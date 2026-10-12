// Fenster-Buttons der eigenen Kopfleiste. Im Browser (Mock-Modus) ohne Wirkung.

import { getCurrentWindow } from "@tauri-apps/api/window";

export const inTauri = "__TAURI_INTERNALS__" in window;

function win() {
  return inTauri ? getCurrentWindow() : null;
}

export const appWindow = {
  minimize: () => win()?.minimize(),
  toggleMaximize: () => win()?.toggleMaximize(),
  close: () => win()?.close(),
  hide: () => win()?.hide(),
  /** Wieder sichtbar machen und nach vorn holen (nach Ausblenden oder Minimieren). */
  restore: async () => {
    const w = win();
    if (!w) return;
    await w.show();
    await w.unminimize();
    await w.setFocus();
  },
  /** Ziehen per Maus; Doppelklick maximiert wie bei normalen Titelleisten. */
  drag: (e: MouseEvent) => {
    if (e.button !== 0) return;
    if (e.detail === 2) win()?.toggleMaximize();
    else win()?.startDragging();
  },
};
