import { get } from 'svelte/store'
import { skinStore } from './stores/skin'
import { activeTool, activeColorHex, brushSize, mirrorEnabled } from './stores/tool'
import { beginAction, commitAction } from './stores/history'
import { hexToRgb, rgbToHex } from './color'
import type { RGBA } from './skin/types'

function currentRGBA (alpha: number): RGBA {
  const rgb = hexToRgb(get(activeColorHex)) ?? { r: 0, g: 0, b: 0 }
  return { ...rgb, a: alpha }
}

/** Call on pointerdown, before the first applyToolAt of a drag. */
export function beginStroke (): void {
  const tool = get(activeTool)
  if (tool === 'pencil' || tool === 'eraser') beginAction()
}

/** Call on pointerup/pointercancel/pointerleave to close out a drag. */
export function endStroke (): void {
  const tool = get(activeTool)
  if (tool === 'pencil' || tool === 'eraser') commitAction()
}

/** Applies the currently active tool at an absolute skin-texture pixel coordinate. */
export function applyToolAt (x: number, y: number): void {
  if (!skinStore.inBounds(x, y)) return
  const tool = get(activeTool)
  const mirror = get(mirrorEnabled)
  const size = get(brushSize)

  switch (tool) {
    case 'pencil':
      skinStore.paintBrush(x, y, size, currentRGBA(255), mirror)
      break
    case 'eraser':
      skinStore.paintBrush(x, y, size, currentRGBA(0), mirror)
      break
    case 'fill':
      beginAction()
      skinStore.floodFill(x, y, currentRGBA(255), mirror)
      commitAction()
      break
    case 'eyedropper': {
      const p = skinStore.getPixel(x, y)
      activeColorHex.set(rgbToHex(p.r, p.g, p.b))
      break
    }
  }
}

/** Bresenham line so a fast pointer drag doesn't skip pixels between move events. */
export function applyToolAlongLine (x0: number, y0: number, x1: number, y1: number): void {
  const dx = Math.abs(x1 - x0)
  const dy = -Math.abs(y1 - y0)
  const sx = x0 < x1 ? 1 : -1
  const sy = y0 < y1 ? 1 : -1
  let err = dx + dy
  let x = x0
  let y = y0
  // eslint-disable-next-line no-constant-condition
  while (true) {
    applyToolAt(x, y)
    if (x === x1 && y === y1) break
    const e2 = 2 * err
    if (e2 >= dy) {
      err += dy
      x += sx
    }
    if (e2 <= dx) {
      err += dx
      y += sy
    }
  }
}
