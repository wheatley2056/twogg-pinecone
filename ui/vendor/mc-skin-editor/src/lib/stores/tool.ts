import { writable } from 'svelte/store'
import type { ToolId } from '../skin/types'

export const activeTool = writable<ToolId>('pencil')
export const activeColorHex = writable<string>('#4c7f4c')
export const brushSize = writable<number>(1) // 1..5, square side = 2*size-1
export const mirrorEnabled = writable<boolean>(false)

export const MIN_BRUSH_SIZE = 1
export const MAX_BRUSH_SIZE = 5
