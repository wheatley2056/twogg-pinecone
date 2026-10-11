// Benachrichtigungen der Kopfleiste (Glocke): App-Updates, Spielfehler, später Freunde und Events.

import { untrack } from "svelte";

export interface Notice {
  id: string;
  title: string;
  text?: string;
  /** Unix-Zeit in Millisekunden. */
  at: number;
  read: boolean;
  kind: "update" | "error" | "info" | "friend" | "event";
  action?: { label: string; run: () => void };
}

export const notices: { list: Notice[] } = $state({ list: [] });

/** Fügt eine Benachrichtigung hinzu; gleiche `id` ersetzt die alte. */
export function notify(n: Omit<Notice, "at" | "read"> & { at?: number }) {
  // Auch aus Effekten aufrufbar, ohne die Liste als Abhängigkeit zu lesen.
  untrack(() => {
    const rest = notices.list.filter((x) => x.id !== n.id);
    notices.list = [{ ...n, at: n.at ?? Date.now(), read: false }, ...rest].slice(0, 50);
  });
}

export function dismiss(id: string) {
  notices.list = notices.list.filter((x) => x.id !== id);
}

export function markAllRead() {
  for (const n of notices.list) n.read = true;
}

export function hasUnread() {
  return notices.list.some((n) => !n.read);
}
