// Einstellungen laden, speichern und auf die Oberfläche anwenden (Theme, Akzent, Hintergrund, Sprache).

import { convertFileSrc } from "@tauri-apps/api/core";
import { api } from "./api";
import { i18n } from "./i18n.svelte";
import type { Settings } from "./types";

export const settings: { current: Settings | null } = $state({ current: null });

export const ACCENTS = ["#4caf50", "#2e9bd6", "#8e6fd8", "#e5534b", "#f0883e", "#d4a72c", "#1abc9c", "#e45ba6"];

/** Lesbare Textfarbe auf der Akzentfarbe (WCAG-Luminanz). */
function textOn(hex: string): string {
  const [r, g, b] = [1, 3, 5].map((i) => {
    const c = parseInt(hex.slice(i, i + 2), 16) / 255;
    return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * r + 0.7152 * g + 0.0722 * b > 0.4 ? "#10140f" : "#ffffff";
}

export function apply(s: Settings) {
  const root = document.documentElement;
  root.dataset.theme = s.theme.mode;
  root.style.setProperty("--accent", s.theme.accent);
  root.style.setProperty("--accent-text", textOn(s.theme.accent));
  root.style.setProperty("--bg-dim", `${s.theme.backgroundDim}%`);
  if (s.backgroundPath) {
    root.style.setProperty("--bg-image", `url("${convertFileSrc(s.backgroundPath)}")`);
    root.dataset.background = "on";
  } else {
    root.style.removeProperty("--bg-image");
    delete root.dataset.background;
  }
  root.lang = s.language;
  i18n.lang = s.language;
}

export async function loadSettings() {
  const s = await api.getSettings();
  settings.current = s;
  apply(s);
}

/** Änderung sofort anzeigen und speichern; der gespeicherte Stand gewinnt. */
export async function updateSettings(change: (s: Settings) => void) {
  if (!settings.current) return;
  const next: Settings = $state.snapshot(settings.current) as Settings;
  change(next);
  apply(next);
  settings.current = next;
  const saved = await api.saveSettings(next);
  settings.current = saved;
  apply(saved);
}

export function useSettings(s: Settings) {
  settings.current = s;
  apply(s);
}
