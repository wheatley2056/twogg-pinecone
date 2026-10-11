import { writable, get } from 'svelte/store'
import { loadJSON, saveJSON } from '../persist'
import type { SavedSkinRecord, ModelType, ResolutionId } from '../skin/types'
import { skinStore } from './skin'
import { clearHistory } from './history'
import { imageDataToDataUrl, dataUrlToImageData } from '../png'

const LIST_KEY = 'mcSkinEditor.savedSkins'
const CURRENT_KEY = 'mcSkinEditor.currentSkinId'

function makeId (): string {
  if ('randomUUID' in crypto) return crypto.randomUUID()
  return `skin-${Date.now()}-${Math.random().toString(36).slice(2)}`
}

export const savedSkins = writable<SavedSkinRecord[]>(loadJSON(LIST_KEY, []))
export const currentSkinId = writable<string | null>(loadJSON<string | null>(CURRENT_KEY, null))

savedSkins.subscribe((v) => saveJSON(LIST_KEY, v))
currentSkinId.subscribe((v) => saveJSON(CURRENT_KEY, v))

/** Writes the live editor buffer into the matching saved-skin record. Call on every skin change (debounced by the caller). */
export function persistCurrentSkin (): void {
  const id = get(currentSkinId)
  if (id == null) return
  const snap = skinStore.snapshot()
  const dataUrl = imageDataToDataUrl(new ImageData(new Uint8ClampedArray(snap.pixels), snap.width, snap.height))
  savedSkins.update((list) =>
    list.map((s) =>
      s.id === id
        ? { ...s, resolution: snap.resolution, model: snap.model, dataUrl, updatedAt: Date.now() }
        : s
    )
  )
}

export function createNewSkin (name: string, resolution: ResolutionId, model: ModelType): string {
  const id = makeId()
  skinStore.newBlank(resolution, model)
  clearHistory()
  const snap = skinStore.snapshot()
  const dataUrl = imageDataToDataUrl(new ImageData(new Uint8ClampedArray(snap.pixels), snap.width, snap.height))
  const record: SavedSkinRecord = { id, name, resolution, model, dataUrl, updatedAt: Date.now() }
  savedSkins.update((list) => [...list, record])
  currentSkinId.set(id)
  return id
}

/** Creates a new saved skin from imported PNG image data. */
export function importSkinAsNew (name: string, resolution: ResolutionId, model: ModelType, imageData: ImageData): string {
  const id = makeId()
  skinStore.loadImageData(imageData, resolution, model)
  clearHistory()
  const dataUrl = imageDataToDataUrl(imageData)
  const record: SavedSkinRecord = { id, name, resolution, model, dataUrl, updatedAt: Date.now() }
  savedSkins.update((list) => [...list, record])
  currentSkinId.set(id)
  return id
}

export async function switchToSkin (id: string): Promise<void> {
  const record = get(savedSkins).find((s) => s.id === id)
  if (record == null) return
  const imageData = await dataUrlToImageData(record.dataUrl)
  skinStore.loadImageData(imageData, record.resolution, record.model)
  clearHistory()
  currentSkinId.set(id)
}

export function renameSkin (id: string, name: string): void {
  savedSkins.update((list) => list.map((s) => (s.id === id ? { ...s, name } : s)))
}

export function deleteSkin (id: string): void {
  const list = get(savedSkins)
  const remaining = list.filter((s) => s.id !== id)
  savedSkins.set(remaining)
  if (get(currentSkinId) === id) {
    if (remaining.length > 0) {
      void switchToSkin(remaining[0].id)
    } else {
      currentSkinId.set(null)
    }
  }
}

export function duplicateSkin (id: string): string | null {
  const record = get(savedSkins).find((s) => s.id === id)
  if (record == null) return null
  const newId = makeId()
  const copy: SavedSkinRecord = { ...record, id: newId, name: `${record.name} copy`, updatedAt: Date.now() }
  savedSkins.update((list) => [...list, copy])
  return newId
}

/** Ensures there is always a current skin to edit — creates a fresh one on first run. Resolves once skinStore holds the real data, so callers can wait before mounting anything that snapshots skinStore on init. */
export async function ensureInitialSkin (): Promise<void> {
  const list = get(savedSkins)
  const id = get(currentSkinId)
  if (id != null && list.some((s) => s.id === id)) {
    await switchToSkin(id)
  } else if (list.length > 0) {
    await switchToSkin(list[0].id)
  } else {
    createNewSkin('My Skin', 'standard', 'classic')
  }
}
