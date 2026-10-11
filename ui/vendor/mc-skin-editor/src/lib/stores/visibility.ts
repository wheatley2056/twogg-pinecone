import { writable } from 'svelte/store'
import type { PartName } from '../skin/types'

export interface LayerVisibility {
  base: boolean
  overlay: boolean
}

export const layerVisibility = writable<LayerVisibility>({ base: true, overlay: true })

export const partVisibility = writable<Record<PartName, boolean>>({
  head: true,
  torso: true,
  leftArm: true,
  rightArm: true,
  leftLeg: true,
  rightLeg: true
})
