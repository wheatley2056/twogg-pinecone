import { writable, type Writable } from 'svelte/store'
import type { ModelType, ResolutionId, RGBA } from '../skin/types'
import { RESOLUTIONS } from '../skin/resolutions'
import { buildPixelIndex, mirrorPixel, storedFaceRect, type PixelLocation } from '../skin/uv'

export interface SkinState {
  resolution: ResolutionId
  model: ModelType
  width: number
  height: number
  pixels: Uint8ClampedArray // RGBA, row-major
  version: number
}

export interface SkinSnapshot {
  resolution: ResolutionId
  model: ModelType
  width: number
  height: number
  pixels: Uint8ClampedArray
}

function blankPixels (width: number, height: number): Uint8ClampedArray {
  return new Uint8ClampedArray(width * height * 4)
}

function blankState (resolution: ResolutionId, model: ModelType): SkinState {
  const info = RESOLUTIONS[resolution]
  return {
    resolution,
    model,
    width: info.width,
    height: info.height,
    pixels: blankPixels(info.width, info.height),
    version: 0
  }
}

/** Nearest-neighbor resample of an RGBA buffer to new dimensions. */
function resample (src: Uint8ClampedArray, sw: number, sh: number, dw: number, dh: number): Uint8ClampedArray {
  const dst = blankPixels(dw, dh)
  for (let y = 0; y < dh; y++) {
    const sy = Math.min(sh - 1, Math.floor((y * sh) / dh))
    for (let x = 0; x < dw; x++) {
      const sx = Math.min(sw - 1, Math.floor((x * sw) / dw))
      const si = (sy * sw + sx) * 4
      const di = (y * dw + x) * 4
      dst[di] = src[si]
      dst[di + 1] = src[si + 1]
      dst[di + 2] = src[si + 2]
      dst[di + 3] = src[si + 3]
    }
  }
  return dst
}

export interface SkinStore {
  subscribe: Writable<SkinState>['subscribe']
  newBlank: (resolution: ResolutionId, model: ModelType) => void
  setResolution: (resolution: ResolutionId) => void
  setModel: (model: ModelType) => void
  getPixel: (x: number, y: number) => RGBA
  inBounds: (x: number, y: number) => boolean
  paintBrush: (cx: number, cy: number, size: number, color: RGBA, mirror: boolean) => void
  floodFill: (startX: number, startY: number, color: RGBA, mirror: boolean) => void
  loadImageData: (imageData: ImageData, resolution: ResolutionId, model: ModelType) => void
  snapshot: () => SkinSnapshot
  restoreSnapshot: (snap: SkinSnapshot) => void
  toImageData: () => ImageData
}

function createSkinStore (): SkinStore {
  const { subscribe, update, set } = writable<SkinState>(blankState('standard', 'classic'))

  let current: SkinState = blankState('standard', 'classic')
  subscribe((s) => (current = s))

  let pixelIndex: Map<string, PixelLocation> = buildPixelIndex(current.model, current.resolution)

  function rebuildIndex (): void {
    pixelIndex = buildPixelIndex(current.model, current.resolution)
  }

  function bump (mutator: (s: SkinState) => void): void {
    update((s) => {
      mutator(s)
      return { ...s, version: s.version + 1 }
    })
  }

  function inBounds (x: number, y: number): boolean {
    return x >= 0 && y >= 0 && x < current.width && y < current.height
  }

  function readPixel (x: number, y: number): RGBA {
    const i = (y * current.width + x) * 4
    const p = current.pixels
    return { r: p[i], g: p[i + 1], b: p[i + 2], a: p[i + 3] }
  }

  function writePixel (x: number, y: number, color: RGBA): void {
    if (!inBounds(x, y)) return
    const i = (y * current.width + x) * 4
    current.pixels[i] = color.r
    current.pixels[i + 1] = color.g
    current.pixels[i + 2] = color.b
    current.pixels[i + 3] = color.a
  }

  return {
    subscribe,

    newBlank (resolution: ResolutionId, model: ModelType) {
      set(blankState(resolution, model))
      rebuildIndex()
    },

    setResolution (resolution: ResolutionId) {
      if (resolution === current.resolution) return
      const info = RESOLUTIONS[resolution]
      bump((s) => {
        s.pixels = resample(s.pixels, s.width, s.height, info.width, info.height)
        s.width = info.width
        s.height = info.height
        s.resolution = resolution
      })
      rebuildIndex()
    },

    setModel (model: ModelType) {
      if (model === current.model) return
      bump((s) => {
        s.model = model
      })
      rebuildIndex()
    },

    getPixel (x: number, y: number): RGBA {
      return readPixel(x, y)
    },

    inBounds,

    /** Paints a square brush centered on (cx, cy). size=1 -> 1x1, size=2 -> 3x3, etc. */
    paintBrush (cx: number, cy: number, size: number, color: RGBA, mirror: boolean) {
      const radius = size - 1
      bump(() => {
        for (let dy = -radius; dy <= radius; dy++) {
          for (let dx = -radius; dx <= radius; dx++) {
            const x = cx + dx
            const y = cy + dy
            if (!inBounds(x, y)) continue
            writePixel(x, y, color)
            if (mirror) {
              const m = mirrorPixel(x, y, current.model, current.resolution, pixelIndex)
              if (m != null) writePixel(m.x, m.y, color)
            }
          }
        }
      })
    },

    floodFill (startX: number, startY: number, color: RGBA, mirror: boolean) {
      if (!inBounds(startX, startY)) return
      const target = readPixel(startX, startY)
      if (target.r === color.r && target.g === color.g && target.b === color.b && target.a === color.a) return
      // Bound the fill to the clicked pixel's own face (and thus its layer)
      // so it can't leak across the shared edges the box-UV layout packs
      // faces along in the atlas.
      const loc = pixelIndex.get(`${startX},${startY}`)
      const bounds = loc != null ? storedFaceRect(loc.part, loc.layer, loc.face, current.model, current.resolution) : null
      const minX = bounds != null ? bounds.x : 0
      const minY = bounds != null ? bounds.y : 0
      const maxX = bounds != null ? bounds.x + bounds.w - 1 : current.width - 1
      const maxY = bounds != null ? bounds.y + bounds.h - 1 : current.height - 1
      bump(() => {
        const stack: Array<[number, number]> = [[startX, startY]]
        const w = current.width
        const h = current.height
        const visited = new Uint8Array(w * h)
        while (stack.length > 0) {
          const next = stack.pop()
          if (next === undefined) break
          const [x, y] = next
          if (x < minX || y < minY || x > maxX || y > maxY) continue
          const vi = y * w + x
          if (visited[vi] !== 0) continue
          const p = readPixel(x, y)
          if (p.r !== target.r || p.g !== target.g || p.b !== target.b || p.a !== target.a) continue
          visited[vi] = 1
          writePixel(x, y, color)
          if (mirror) {
            const m = mirrorPixel(x, y, current.model, current.resolution, pixelIndex)
            if (m != null) writePixel(m.x, m.y, color)
          }
          stack.push([x + 1, y], [x - 1, y], [x, y + 1], [x, y - 1])
        }
      })
    },

    loadImageData (imageData: ImageData, resolution: ResolutionId, model: ModelType) {
      set({
        resolution,
        model,
        width: imageData.width,
        height: imageData.height,
        pixels: Uint8ClampedArray.from(imageData.data),
        version: 0
      })
      rebuildIndex()
    },

    snapshot (): SkinSnapshot {
      return {
        resolution: current.resolution,
        model: current.model,
        width: current.width,
        height: current.height,
        pixels: current.pixels.slice()
      }
    },

    restoreSnapshot (snap: SkinSnapshot) {
      set({
        resolution: snap.resolution,
        model: snap.model,
        width: snap.width,
        height: snap.height,
        pixels: snap.pixels.slice(),
        version: 0
      })
      rebuildIndex()
    },

    toImageData (): ImageData {
      return new ImageData(new Uint8ClampedArray(current.pixels), current.width, current.height)
    }
  }
}

export const skinStore = createSkinStore()
