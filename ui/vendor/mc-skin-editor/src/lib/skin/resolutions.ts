import type { ResolutionId, ResolutionInfo } from './types'

export const RESOLUTIONS: Record<ResolutionId, ResolutionInfo> = {
  legacy: {
    id: 'legacy',
    label: 'Legacy (64×32)',
    width: 64,
    height: 32,
    scale: 1,
    hasBodyOverlay: false,
    hasSeparateLimbs: false
  },
  standard: {
    id: 'standard',
    label: 'Standard (64×64)',
    width: 64,
    height: 64,
    scale: 1,
    hasBodyOverlay: true,
    hasSeparateLimbs: true
  },
  high: {
    id: 'high',
    label: 'High (128×128)',
    width: 128,
    height: 128,
    scale: 2,
    hasBodyOverlay: true,
    hasSeparateLimbs: true
  }
}

export const RESOLUTION_LIST: ResolutionInfo[] = [RESOLUTIONS.legacy, RESOLUTIONS.standard, RESOLUTIONS.high]

export function resolutionFromDims (w: number, h: number): ResolutionId | null {
  if (w === 64 && h === 32) return 'legacy'
  if (w === 64 && h === 64) return 'standard'
  if (w === 128 && h === 128) return 'high'
  return null
}
