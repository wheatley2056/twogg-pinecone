// Static Minecraft player-model proportions, in pixel units (1 unit = 1 skin
// pixel = 1/16 block), matching the layout used by every standard skin
// viewer: legs 12 tall, torso 12 tall, head 8 tall, feet at y=0.

import type { ModelType, PartName } from './types'
import { partDims } from './uv'

export interface PartTransform {
  /** Center position of the box, in pixel units. */
  center: { x: number, y: number, z: number }
}

export function partTransform (part: PartName, model: ModelType): PartTransform {
  const dims = partDims(part, model)
  switch (part) {
    case 'head':
      return { center: { x: 0, y: 28, z: 0 } }
    case 'torso':
      return { center: { x: 0, y: 18, z: 0 } }
    case 'rightArm':
      return { center: { x: -4 - dims.dx / 2, y: 18, z: 0 } }
    case 'leftArm':
      return { center: { x: 4 + dims.dx / 2, y: 18, z: 0 } }
    case 'rightLeg':
      return { center: { x: -2, y: 6, z: 0 } }
    case 'leftLeg':
      return { center: { x: 2, y: 6, z: 0 } }
  }
}

/** How much larger the overlay (hat/jacket/sleeves/pants) layer box is than the base layer, per side, in pixel units. */
export const OVERLAY_INSET = 0.5
