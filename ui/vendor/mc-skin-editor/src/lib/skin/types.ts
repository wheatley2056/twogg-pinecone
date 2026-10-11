// Core shared types for the skin editor.

export type ResolutionId = 'legacy' | 'standard' | 'high'

export interface ResolutionInfo {
  id: ResolutionId
  label: string
  width: number
  height: number
  /** Pixel scale relative to the 64-wide baseline UV layout (1 for 64-wide, 2 for 128-wide). */
  scale: number
  /** Legacy (64x32) only has a hat overlay; no jacket/sleeves/pants overlay, no separate left-limb storage. */
  hasBodyOverlay: boolean
  hasSeparateLimbs: boolean
}

export type ModelType = 'classic' | 'slim'

export type PartName = 'head' | 'torso' | 'rightArm' | 'leftArm' | 'rightLeg' | 'leftLeg'
export type LayerName = 'base' | 'overlay'
export type FaceName = 'top' | 'bottom' | 'right' | 'front' | 'left' | 'back'

export type ToolId = 'pencil' | 'eraser' | 'eyedropper' | 'fill'

export interface RGBA {
  r: number
  g: number
  b: number
  a: number
}

export interface Rect {
  x: number
  y: number
  w: number
  h: number
}

export interface SavedSkinMeta {
  id: string
  name: string
  resolution: ResolutionId
  model: ModelType
  updatedAt: number
}

export interface SavedSkinRecord extends SavedSkinMeta {
  /** Base64-encoded PNG data URL of the full texture, alpha included. */
  dataUrl: string
}
