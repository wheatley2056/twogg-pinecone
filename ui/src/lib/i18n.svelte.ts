// Übersetzungen: `t("schlüssel", { name })`. Reaktiv, Sprachwechsel greift sofort.

import de, { type MessageKey } from "./locales/de";
import en from "./locales/en";

export type Lang = "de" | "en";

const dictionaries: Record<Lang, Record<MessageKey, string>> = { de, en };

export const i18n = $state({ lang: "de" as Lang });

export function t(key: MessageKey, vars?: Record<string, string | number>): string {
  let text = dictionaries[i18n.lang][key] ?? de[key];
  if (vars) for (const [k, v] of Object.entries(vars)) text = text.replaceAll(`{${k}}`, String(v));
  return text;
}

export function locale(): string {
  return i18n.lang === "de" ? "de-DE" : "en-US";
}

/** Dauer wie „3 h 12 min“. */
export function formatDuration(secs: number): string {
  if (secs < 60) return `${secs} s`;
  if (secs < 3600) return `${Math.floor(secs / 60)} min`;
  return `${Math.floor(secs / 3600)} h ${Math.floor((secs % 3600) / 60)} min`;
}

/** Relative Zeit wie „vor 2 Tagen“ / „2 days ago“. */
export function formatAgo(unixSecs: number): string {
  const diff = unixSecs - Date.now() / 1000;
  const rtf = new Intl.RelativeTimeFormat(locale(), { numeric: "auto" });
  const steps: [Intl.RelativeTimeFormatUnit, number][] = [
    ["year", 31536000],
    ["month", 2592000],
    ["week", 604800],
    ["day", 86400],
    ["hour", 3600],
    ["minute", 60],
  ];
  for (const [unit, secs] of steps) {
    if (Math.abs(diff) >= secs) return rtf.format(Math.round(diff / secs), unit);
  }
  return rtf.format(0, "minute");
}
