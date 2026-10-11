// Hilfen für Sync-Packs: Abgleich-Bericht als Text, Tastennamen wie in options.txt.
import { t } from "./i18n.svelte";
import type { SyncReport } from "./types";

/** Kurzer Bericht nach Ein-/Ausschalten oder Abgleich. */
export function syncReportText(r: SyncReport): string {
  const parts: string[] = [];
  if (r.installed.length) parts.push(t("sync.reportInstalled", { list: r.installed.join(", ") }));
  if (r.removed.length) parts.push(t("sync.reportRemoved", { list: r.removed.join(", ") }));
  if (r.present.length) parts.push(t("sync.reportPresent", { list: r.present.join(", ") }));
  if (r.unavailable.length) parts.push(t("sync.reportUnavailable", { list: r.unavailable.join(", ") }));
  parts.push(...r.warnings);
  return parts.length ? parts.join(" · ") : t("sync.reportNothing");
}

/** Tastenbelegungen stehen in options.txt als `key_<Aktion>:<Taste>`. */
export const isControl = (option: string) => option.startsWith("key_");

const CODES: Record<string, string> = {
  Space: "space",
  Tab: "tab",
  Enter: "enter",
  Backspace: "backspace",
  CapsLock: "caps.lock",
  ShiftLeft: "left.shift",
  ShiftRight: "right.shift",
  ControlLeft: "left.control",
  ControlRight: "right.control",
  AltLeft: "left.alt",
  AltRight: "right.alt",
  MetaLeft: "left.win",
  MetaRight: "right.win",
  ArrowUp: "up",
  ArrowDown: "down",
  ArrowLeft: "left",
  ArrowRight: "right",
  Minus: "minus",
  Equal: "equal",
  BracketLeft: "left.bracket",
  BracketRight: "right.bracket",
  Backslash: "backslash",
  Semicolon: "semicolon",
  Quote: "apostrophe",
  Comma: "comma",
  Period: "period",
  Slash: "slash",
  Backquote: "grave.accent",
  IntlBackslash: "world.2",
  Insert: "insert",
  Delete: "delete",
  Home: "home",
  End: "end",
  PageUp: "page.up",
  PageDown: "page.down",
  PrintScreen: "print.screen",
  ScrollLock: "scroll.lock",
  Pause: "pause",
  NumLock: "num.lock",
  ContextMenu: "menu",
  NumpadAdd: "keypad.add",
  NumpadSubtract: "keypad.subtract",
  NumpadMultiply: "keypad.multiply",
  NumpadDivide: "keypad.divide",
  NumpadDecimal: "keypad.decimal",
  NumpadEnter: "keypad.enter",
  NumpadEqual: "keypad.equal",
};

/** Taste aus einem Tastendruck, z. B. `KeyC` → `key.keyboard.c`; unbekannt → `null`. */
export function keyFromEvent(e: KeyboardEvent): string | null {
  const c = e.code;
  let name: string | undefined = CODES[c];
  if (!name && /^Key[A-Z]$/.test(c)) name = c.slice(3).toLowerCase();
  else if (!name && /^Digit\d$/.test(c)) name = c.slice(5);
  else if (!name && /^Numpad\d$/.test(c)) name = `keypad.${c.slice(6)}`;
  else if (!name && /^F\d{1,2}$/.test(c)) name = c.toLowerCase();
  return name ? `key.keyboard.${name}` : null;
}

/** Maustaste: 0 links, 1 Mitte, 2 rechts, 3/4 Seitentasten. */
export function keyFromMouse(button: number): string {
  return ["key.mouse.left", "key.mouse.middle", "key.mouse.right"][button] ?? `key.mouse.${button + 1}`;
}

/** Lesbarer Name einer Taste, z. B. `key.keyboard.left.shift` → „Left Shift“. */
export function keyLabel(value: string): string {
  if (value === "key.keyboard.unknown") return t("sync.keyNone");
  const mouse = value.match(/^key\.mouse\.(.+)$/);
  if (mouse) {
    const b = mouse[1];
    const name = b === "left" ? t("sync.mouseLeft") : b === "right" ? t("sync.mouseRight") : b === "middle" ? t("sync.mouseMiddle") : b;
    return t("sync.mouse", { button: name });
  }
  const rest = value.replace(/^key\.keyboard\./, "").replace(/^keypad\./, "Num ");
  return rest
    .split(".")
    .map((w) => (w.length === 1 ? w.toUpperCase() : w[0].toUpperCase() + w.slice(1)))
    .join(" ");
}

/** Aktion einer Tastenbelegung ohne Präfix, z. B. `key_key.zoom` → `key.zoom`. */
export const controlName = (option: string) => option.replace(/^key_/, "");
