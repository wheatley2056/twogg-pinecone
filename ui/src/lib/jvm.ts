// JVM-Presets wie im Backend (crates/core/src/jvm.rs): eingebaute Sätze, in den Einstellungen
// angepasst oder um eigene ergänzt. Gespeichert werden nur Abweichungen.

import type { JvmPreset, JvmSettings, StoredPreset } from "./types";

export const NO_PRESET = "none";

/** Eingebaute (ggf. angepasste) Presets, danach eigene. */
export function mergePresets(builtin: JvmPreset[], jvm: JvmSettings): JvmPreset[] {
  const out = builtin.map((b) => {
    const stored = jvm.presets.find((s) => s.id === b.id);
    return stored ? { ...b, args: stored.args, modified: !sameArgs(stored.args, b.args) } : b;
  });
  for (const s of jvm.presets) {
    if (!builtin.some((b) => b.id === s.id)) out.push({ id: s.id, name: s.name, description: "", args: s.args, minJava: s.minJava, builtin: false, modified: false });
  }
  return out;
}

export function sameArgs(a: string[], b: string[]): boolean {
  return a.length === b.length && a.every((x, i) => x === b[i]);
}

/** Text aus dem Editor (eine Zeile oder Leerzeichen je Argument) in Argumente zerlegen. */
export function parseArgs(text: string): string[] {
  return text.split(/\s+/).filter(Boolean);
}

/** Kurze, eindeutige ID aus einem Namen („Mein Preset“ → `mein-preset`, bei Bedarf `-2`). */
export function presetId(name: string, taken: string[]): string {
  const base =
    name
      .toLowerCase()
      .normalize("NFKD")
      .replace(/[̀-ͯ]/g, "")
      .replace(/ß/g, "ss")
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/^-+|-+$/g, "")
      .slice(0, 40) || "preset";
  let id = base === NO_PRESET ? "preset" : base;
  for (let n = 2; taken.includes(id); n++) id = `${base}-${n}`;
  return id;
}

/** Gespeicherten Eintrag für ein Preset setzen (Anpassen oder Neuanlegen). */
export function withPreset(jvm: JvmSettings, preset: StoredPreset): JvmSettings {
  const presets = jvm.presets.filter((p) => p.id !== preset.id);
  return { ...jvm, presets: [...presets, preset] };
}
