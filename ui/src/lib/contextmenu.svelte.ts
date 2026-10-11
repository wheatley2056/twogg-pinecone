// Eigenes Rechtsklick-Menü. Bereiche rufen `contextMenu(e, items)` auf (oder nutzen `use:menu`);
// überall sonst zeigt `ContextMenu.svelte` bei Textfeldern Ausschneiden/Kopieren/Einfügen und
// unterdrückt das Menü des Browsers.

import type { IconName } from "../components/Icon.svelte";

export interface MenuItem {
  label: string;
  icon?: IconName;
  danger?: boolean;
  disabled?: boolean;
  action: () => unknown;
}

/** `null`/`false` werden ausgelassen, `"-"` ist ein Trenner. */
export type MenuEntry = MenuItem | "-" | null | false | undefined;

export const ctx: { open: { x: number; y: number; items: (MenuItem | "-")[] } | null } = $state({ open: null });

export function contextMenu(e: MouseEvent, entries: MenuEntry[]) {
  e.preventDefault();
  e.stopPropagation();
  const items = entries.filter((i): i is MenuItem | "-" => !!i);
  // Keine Trenner am Rand oder doppelt.
  const clean = items.filter((i, n) => i !== "-" || (n > 0 && n < items.length - 1 && items[n - 1] !== "-"));
  if (clean.length > 0) ctx.open = { x: e.clientX, y: e.clientY, items: clean };
}

export function closeMenu() {
  ctx.open = null;
}

/** `<div use:menu={() => [...]}>` */
export function menu(node: HTMLElement, items: () => MenuEntry[]) {
  let get = items;
  const handler = (e: MouseEvent) => contextMenu(e, get());
  node.addEventListener("contextmenu", handler);
  return {
    update(next: () => MenuEntry[]) {
      get = next;
    },
    destroy() {
      node.removeEventListener("contextmenu", handler);
    },
  };
}
