import { writable, type Writable } from 'svelte/store'
import { loadJSON, saveJSON } from '../persist'

const KEY = 'mcSkinEditor.palette'
export const SLOT_COUNT = 16

export interface PaletteStore {
  subscribe: Writable<Array<string | null>>['subscribe']
  setSlot: (index: number, color: string) => void
  clearSlot: (index: number) => void
  addToFirstEmpty: (color: string) => number
}

function initial (): Array<string | null> {
  const loaded = loadJSON<Array<string | null>>(KEY, [])
  const arr: Array<string | null> = new Array(SLOT_COUNT).fill(null)
  loaded.forEach((c, i) => {
    if (i < SLOT_COUNT) arr[i] = c
  })
  return arr
}

function createPaletteStore (): PaletteStore {
  const { subscribe, update } = writable<Array<string | null>>(initial())
  subscribe((v) => saveJSON(KEY, v))

  return {
    subscribe,
    setSlot (index: number, color: string) {
      update((arr) => {
        const copy = [...arr]
        copy[index] = color
        return copy
      })
    },
    clearSlot (index: number) {
      update((arr) => {
        const copy = [...arr]
        copy[index] = null
        return copy
      })
    },
    /** Stores `color` in the first empty slot, or does nothing if the palette is full. Returns the slot index used, or -1. */
    addToFirstEmpty (color: string): number {
      let usedIndex = -1
      update((arr) => {
        const idx = arr.indexOf(null)
        if (idx === -1) return arr
        usedIndex = idx
        const copy = [...arr]
        copy[idx] = color
        return copy
      })
      return usedIndex
    }
  }
}

export const paletteStore = createPaletteStore()
