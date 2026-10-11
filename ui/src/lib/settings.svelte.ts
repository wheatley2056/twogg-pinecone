// Einstellungen laden, speichern und auf die Oberfläche anwenden (Theme, Akzent, Details, Sprache).

import { api } from "./api";
import { i18n } from "./i18n.svelte";
import type { Settings, ThemeName } from "./types";

export const settings: { current: Settings | null } = $state({ current: null });

/** Wählbare Akzentfarben aus dem Design-System; Text darauf ist immer `--ink`. */
export const ACCENTS = [
  { id: "harz", color: "#e9a23b" },
  { id: "nadel", color: "#8ccb7e" },
  { id: "gletscher", color: "#7fb8e0" },
  { id: "beere", color: "#e58a98" },
  { id: "flechte", color: "#cfc870" },
] as const;

export const THEMES: ThemeName[] = ["rinde", "kiefer", "winterland"];

/** Mischt eine Farbe mit Weiß (t > 0) oder Schwarz (t < 0); für Hover und Kanten der Akzentfarbe. */
function shade(hex: string, t: number): string {
  const target = t > 0 ? 255 : 0;
  const k = Math.abs(t);
  const channel = (i: number) => Math.round(parseInt(hex.slice(i, i + 2), 16) * (1 - k) + target * k);
  return `#${[1, 3, 5].map((i) => channel(i).toString(16).padStart(2, "0")).join("")}`;
}

export function apply(s: Settings) {
  const root = document.documentElement;
  const accent = s.theme.accent.toLowerCase();
  root.dataset.theme = s.theme.name;
  // Harz hat feste Töne aus dem Design-System, andere Akzente werden daraus abgeleitet.
  const harz = accent === "#e9a23b";
  root.style.setProperty("--accent", accent);
  root.style.setProperty("--accent-hover", harz ? "#f3bc64" : shade(accent, 0.25));
  root.style.setProperty("--accent-split", harz ? "#d48f2c" : shade(accent, -0.09));
  root.style.setProperty("--accent-edge", harz ? "#b8742a" : shade(accent, -0.22));
  root.style.setProperty("--accent-light", harz ? "#f6c77a" : shade(accent, 0.35));
  // Akzent als Textfarbe: auf hellem Grund abgedunkelt, damit der Kontrast reicht.
  root.style.setProperty("--accent-fg", s.theme.name === "winterland" ? shade(accent, -0.5) : accent);
  root.dataset.pattern = s.theme.pattern ? "on" : "off";
  root.dataset.pixel = s.theme.pixelTitles ? "on" : "off";
  root.dataset.motion = s.theme.reduceMotion ? "reduce" : "full";
  root.dataset.frame = s.systemFrame ? "system" : "custom";
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
