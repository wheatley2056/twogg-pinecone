// Color conversion / adjustment helpers.

export interface HSL {
  h: number // 0-360
  s: number // 0-100
  l: number // 0-100
}

export function hexToRgb (hex: string): { r: number, g: number, b: number } | null {
  const m = /^#?([0-9a-f]{6})$/i.exec(hex.trim())
  if (m == null) return null
  const n = parseInt(m[1], 16)
  return { r: (n >> 16) & 0xff, g: (n >> 8) & 0xff, b: n & 0xff }
}

export function rgbToHex (r: number, g: number, b: number): string {
  const clamp = (v: number): number => Math.max(0, Math.min(255, Math.round(v)))
  return '#' + [clamp(r), clamp(g), clamp(b)].map((v) => v.toString(16).padStart(2, '0')).join('')
}

export function rgbToHsl (r: number, g: number, b: number): HSL {
  r /= 255
  g /= 255
  b /= 255
  const max = Math.max(r, g, b)
  const min = Math.min(r, g, b)
  let h = 0
  let s = 0
  const l = (max + min) / 2
  const d = max - min
  if (d !== 0) {
    s = d / (1 - Math.abs(2 * l - 1))
    switch (max) {
      case r:
        h = 60 * (((g - b) / d) % 6)
        break
      case g:
        h = 60 * ((b - r) / d + 2)
        break
      case b:
        h = 60 * ((r - g) / d + 4)
        break
    }
  }
  if (h < 0) h += 360
  return { h, s: s * 100, l: l * 100 }
}

export function hslToRgb (h: number, s: number, l: number): { r: number, g: number, b: number } {
  s /= 100
  l /= 100
  const c = (1 - Math.abs(2 * l - 1)) * s
  const x = c * (1 - Math.abs(((h / 60) % 2) - 1))
  const m = l - c / 2
  let r1 = 0
  let g1 = 0
  let b1 = 0
  if (h < 60) [r1, g1, b1] = [c, x, 0]
  else if (h < 120) [r1, g1, b1] = [x, c, 0]
  else if (h < 180) [r1, g1, b1] = [0, c, x]
  else if (h < 240) [r1, g1, b1] = [0, x, c]
  else if (h < 300) [r1, g1, b1] = [x, 0, c]
  else [r1, g1, b1] = [c, 0, x]
  return { r: (r1 + m) * 255, g: (g1 + m) * 255, b: (b1 + m) * 255 }
}

/** Shifts a hex color's lightness by `deltaPercent` (positive = lighter, negative = darker), clamped to 0-100. */
export function adjustLightness (hex: string, deltaPercent: number): string {
  const rgb = hexToRgb(hex)
  if (rgb == null) return hex
  const hsl = rgbToHsl(rgb.r, rgb.g, rgb.b)
  hsl.l = Math.max(0, Math.min(100, hsl.l + deltaPercent))
  const out = hslToRgb(hsl.h, hsl.s, hsl.l)
  return rgbToHex(out.r, out.g, out.b)
}

export const LIGHTNESS_STEP = 3
