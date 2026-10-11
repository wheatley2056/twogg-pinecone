// Zustand des Selbst-Updates (gemeinsam für Start-Hinweis und Einstellungen).
import { api, errorText } from "./api";
import type { AppUpdate } from "./types";

export const updates = $state({
  checking: false,
  checked: false,
  info: null as AppUpdate | null,
  error: null as string | null,
  /** 0–1 während des Downloads, null sonst. */
  progress: null as number | null,
  installing: false,
  dismissed: false,
});

export async function checkUpdates(silent = false) {
  updates.checking = true;
  updates.error = null;
  try {
    updates.info = await api.updateCheck();
    updates.checked = true;
  } catch (e) {
    // Beim automatischen Prüfen (z. B. offline) keine Fehlermeldung anzeigen.
    if (!silent) updates.error = errorText(e);
  } finally {
    updates.checking = false;
  }
}

export async function installUpdate() {
  updates.error = null;
  updates.progress = 0;
  try {
    await api.updateInstall((e) => {
      if (e.type === "progress") updates.progress = e.total ? e.done / e.total : null;
      else updates.installing = true;
    });
  } catch (e) {
    updates.error = errorText(e);
    updates.progress = null;
    updates.installing = false;
  }
}
