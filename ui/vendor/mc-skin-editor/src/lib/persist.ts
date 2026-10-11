// Thin, error-tolerant localStorage JSON wrapper. Storage can throw (quota,
// private mode, disabled) — every call is wrapped so a persistence failure
// never breaks editing.

export function loadJSON<T> (key: string, fallback: T): T {
  try {
    const raw = localStorage.getItem(key)
    if (raw == null) return fallback
    return JSON.parse(raw) as T
  } catch {
    return fallback
  }
}

export function saveJSON (key: string, value: unknown): void {
  try {
    localStorage.setItem(key, JSON.stringify(value))
  } catch (e) {
    console.warn(`Failed to save "${key}" to localStorage`, e)
  }
}

export function removeKey (key: string): void {
  try {
    localStorage.removeItem(key)
  } catch {
    // ignore
  }
}
