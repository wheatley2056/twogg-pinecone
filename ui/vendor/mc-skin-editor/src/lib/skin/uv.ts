// Minecraft player-skin UV layout.
//
// Every cuboid body part is unwrapped onto the texture using the standard
// "box UV" template (the same one Blockbench uses for Minecraft entity
// models): given a box of size (dx width, dy height, dz depth) placed at
// texture origin (u, v), the six faces land at:
//
//   top:    (u + dz,           v),          size (dx, dz)
//   bottom: (u + dz + dx,      v),          size (dx, dz)
//   right:  (u,                v + dz),     size (dz, dy)
//   front:  (u + dz,           v + dz),     size (dx, dy)
//   left:   (u + dz + dx,      v + dz),     size (dz, dy)
//   back:   (u + dz + dx + dz, v + dz),     size (dx, dy)
//
// All coordinates below are expressed in 64-wide-baseline pixel units and
// get multiplied by the resolution's scale factor (1 for 64-wide, 2 for
// 128-wide) to land on the actual texture.
//
// The legacy 64x32 format has no separate texture storage for the left arm
// / left leg (the game just mirrors the right arm / right leg onto them),
// and no overlay layer for anything but the head. `renderFaceSource` below
// resolves that indirection for the 3D mesh builder; `storedFaceRect` never
// does — it only ever points at real, paintable texture pixels.

import type { FaceName, LayerName, ModelType, PartName, Rect, ResolutionId } from './types'
import { RESOLUTIONS } from './resolutions'

export interface Dims {
  dx: number
  dy: number
  dz: number
}

export const PART_NAMES: PartName[] = ['head', 'torso', 'rightArm', 'leftArm', 'rightLeg', 'leftLeg']
export const LAYER_NAMES: LayerName[] = ['base', 'overlay']
export const FACE_NAMES: FaceName[] = ['top', 'bottom', 'right', 'front', 'left', 'back']

export function boxUV (u: number, v: number, dx: number, dy: number, dz: number): Record<FaceName, Rect> {
  return {
    top: { x: u + dz, y: v, w: dx, h: dz },
    bottom: { x: u + dz + dx, y: v, w: dx, h: dz },
    right: { x: u, y: v + dz, w: dz, h: dy },
    front: { x: u + dz, y: v + dz, w: dx, h: dy },
    left: { x: u + dz + dx, y: v + dz, w: dz, h: dy },
    back: { x: u + dz + dx + dz, y: v + dz, w: dx, h: dy }
  }
}

/** Body part dimensions in 64-wide-baseline pixel units. */
export function partDims (part: PartName, model: ModelType): Dims {
  switch (part) {
    case 'head':
      return { dx: 8, dy: 8, dz: 8 }
    case 'torso':
      return { dx: 8, dy: 12, dz: 4 }
    case 'rightArm':
    case 'leftArm':
      return { dx: model === 'slim' ? 3 : 4, dy: 12, dz: 4 }
    case 'rightLeg':
    case 'leftLeg':
      return { dx: 4, dy: 12, dz: 4 }
  }
}

/** UV origin (u,v) in 64-wide-baseline units for a part+layer, or null if that part/layer has no texture storage at this resolution. */
export function partUVOrigin (part: PartName, layer: LayerName, resolution: ResolutionId): { u: number, v: number } | null {
  const info = RESOLUTIONS[resolution]
  if (layer === 'overlay' && part !== 'head' && !info.hasBodyOverlay) return null
  if (!info.hasSeparateLimbs && (part === 'leftArm' || part === 'leftLeg')) return null

  if (layer === 'base') {
    switch (part) {
      case 'head':
        return { u: 0, v: 0 }
      case 'torso':
        return { u: 16, v: 16 }
      case 'rightArm':
        return { u: 40, v: 16 }
      case 'leftArm':
        return { u: 32, v: 48 }
      case 'rightLeg':
        return { u: 0, v: 16 }
      case 'leftLeg':
        return { u: 16, v: 48 }
    }
  } else {
    switch (part) {
      case 'head':
        return { u: 32, v: 0 }
      case 'torso':
        return { u: 16, v: 32 }
      case 'rightArm':
        return { u: 40, v: 32 }
      case 'leftArm':
        return { u: 48, v: 48 }
      case 'rightLeg':
        return { u: 0, v: 32 }
      case 'leftLeg':
        return { u: 0, v: 48 }
    }
  }
}

/** The opposite-side limb, used to resolve legacy mirroring and the Mirror-paint tool. Null for head/torso. */
export function mirrorCounterpart (part: PartName): PartName | null {
  if (part === 'rightArm') return 'leftArm'
  if (part === 'leftArm') return 'rightArm'
  if (part === 'rightLeg') return 'leftLeg'
  if (part === 'leftLeg') return 'rightLeg'
  return null
}

const FACE_SWAP: Record<FaceName, FaceName> = {
  top: 'top',
  bottom: 'bottom',
  right: 'left',
  left: 'right',
  front: 'front',
  back: 'back'
}

/** True if mirroring this face requires flipping the local X axis within the face (all faces except the side faces, which swap identity instead). */
function faceFlipsOnMirror (face: FaceName): boolean {
  return face !== 'left' && face !== 'right'
}

export interface FaceRect extends Rect {
  part: PartName
  layer: LayerName
  face: FaceName
}

/** The real, paintable pixel rect for a part/layer/face, or null if this part/layer has no storage at this resolution. */
export function storedFaceRect (
  part: PartName,
  layer: LayerName,
  face: FaceName,
  model: ModelType,
  resolution: ResolutionId
): FaceRect | null {
  const origin = partUVOrigin(part, layer, resolution)
  if (origin == null) return null
  const scale = RESOLUTIONS[resolution].scale
  const dims = partDims(part, model)
  const r = boxUV(origin.u, origin.v, dims.dx, dims.dy, dims.dz)[face]
  return { part, layer, face, x: r.x * scale, y: r.y * scale, w: r.w * scale, h: r.h * scale }
}

/** All (part, layer, face) rects that have real texture storage at this resolution. */
export function allStoredFaceRects (model: ModelType, resolution: ResolutionId): FaceRect[] {
  const out: FaceRect[] = []
  for (const part of PART_NAMES) {
    for (const layer of LAYER_NAMES) {
      for (const face of FACE_NAMES) {
        const r = storedFaceRect(part, layer, face, model, resolution)
        if (r != null) out.push(r)
      }
    }
  }
  return out
}

/**
 * What the 3D mesh should sample for a given part/layer/face, resolving the
 * legacy left-limb mirroring indirection. `flipX` tells the mesh builder to
 * reverse the U axis within `rect` when assigning per-vertex UVs.
 */
export function renderFaceSource (
  part: PartName,
  layer: LayerName,
  face: FaceName,
  model: ModelType,
  resolution: ResolutionId
): { rect: FaceRect, flipX: boolean } | null {
  const direct = storedFaceRect(part, layer, face, model, resolution)
  if (direct != null) return { rect: direct, flipX: false }

  const counterpart = mirrorCounterpart(part)
  if (counterpart == null) return null
  const counterFace = FACE_SWAP[face]
  const rect = storedFaceRect(counterpart, layer, counterFace, model, resolution)
  if (rect == null) return null
  return { rect, flipX: faceFlipsOnMirror(face) }
}

export interface PixelLocation {
  part: PartName
  layer: LayerName
  face: FaceName
  localX: number
  localY: number
}

/** Maps every stored (paintable) absolute pixel coordinate to the part/layer/face/local-coord it belongs to. Rebuild when model or resolution changes. */
export function buildPixelIndex (model: ModelType, resolution: ResolutionId): Map<string, PixelLocation> {
  const index = new Map<string, PixelLocation>()
  for (const rect of allStoredFaceRects(model, resolution)) {
    for (let dy = 0; dy < rect.h; dy++) {
      for (let dx = 0; dx < rect.w; dx++) {
        index.set(`${rect.x + dx},${rect.y + dy}`, {
          part: rect.part,
          layer: rect.layer,
          face: rect.face,
          localX: dx,
          localY: dy
        })
      }
    }
  }
  return index
}

/** Given an absolute pixel that's part of a limb (arm/leg), returns the mirrored absolute pixel on the opposite limb, or null if not mirrorable. */
export function mirrorPixel (
  x: number,
  y: number,
  model: ModelType,
  resolution: ResolutionId,
  index: Map<string, PixelLocation>
): { x: number, y: number } | null {
  const loc = index.get(`${x},${y}`)
  if (loc == null) return null
  const counterpart = mirrorCounterpart(loc.part)
  if (counterpart == null) return null
  const counterRect = storedFaceRect(counterpart, loc.layer, FACE_SWAP[loc.face], model, resolution)
  if (counterRect == null) return null
  const sourceRect = storedFaceRect(loc.part, loc.layer, loc.face, model, resolution)
  if (sourceRect == null) return null
  const mirroredLocalX = faceFlipsOnMirror(loc.face) ? sourceRect.w - 1 - loc.localX : loc.localX
  return { x: counterRect.x + mirroredLocalX, y: counterRect.y + loc.localY }
}
