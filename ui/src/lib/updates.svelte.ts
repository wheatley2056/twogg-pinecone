// Zustand des Selbst-Updates (gemeinsam für Start-Hinweis und Einstellungen).
import { api, errorText } from "./api";
import { t } from "./i18n.svelte";
import { notify } from "./notifications.svelte";
import { runs } from "./runs.svelte";
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
  /** Während eines Spiels angefordert: wird nach Spielende installiert (AK-34.3). */
  deferred: false,
});

/** Erneut prüfen, solange Pinecone läuft (F-34). */
const CHECK_EVERY_MS = 6 * 60 * 60_000;

function gameActive(): boolean {
  return Object.values(runs).some((r) => r.status !== "idle");
}

/** Prüft beim Start und danach alle 6 Stunden still auf Updates. */
export function startUpdateChecks() {
  checkUpdates(true);
  const timer = setInterval(() => checkUpdates(true), CHECK_EVERY_MS);
  return () => clearInterval(timer);
}

/** Nach Spielende: ein aufgeschobenes Update jetzt installieren. */
export function afterGameExit() {
  if (updates.deferred && !gameActive()) {
    updates.deferred = false;
    installUpdate();
  }
}

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
  if (gameActive()) {
    updates.deferred = true;
    notify({ id: "app-update-deferred", kind: "update", title: t("appUpdate.deferred") });
    return;
  }
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
