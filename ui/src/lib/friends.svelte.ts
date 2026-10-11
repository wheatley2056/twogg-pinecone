// Pinecone-Freundesliste: regelmäßig abrufen und eigenen Status melden, solange der Launcher läuft.

import { untrack } from "svelte";
import { api, errorText } from "./api";
import type { FriendActivity, FriendsState } from "./types";

export const friends: { state: FriendsState | null; error: string | null } = $state({ state: null, error: null });

const REFRESH_MS = 60_000;
const PRESENCE_MS = 45_000;

export async function refreshFriends() {
  try {
    friends.state = await api.friends();
    friends.error = null;
  } catch (e) {
    friends.error = errorText(e);
  }
}

/**
 * Startet Abruf und Status-Meldung. `activity` liefert den aktuellen Stand (gewählte Instanz,
 * ob gespielt wird, Server). Gibt eine Stopp-Funktion zurück.
 */
export function startFriends(activity: () => FriendActivity): () => void {
  const report = () => {
    if (!untrack(() => friends.state?.available)) return;
    api.friendsPresence(activity()).catch(() => {});
  };
  refreshFriends().then(report);
  const a = setInterval(refreshFriends, REFRESH_MS);
  const b = setInterval(report, PRESENCE_MS);
  return () => {
    clearInterval(a);
    clearInterval(b);
  };
}

/** Gleichbleibende Farbe je Spieler für den Avatar-Stapel. */
export function friendColor(uuid: string): string {
  const colors = ["#5e7d57", "#7a5a3a", "#4e6a82", "#8a6a3a", "#6b5578", "#4f7470"];
  let h = 0;
  for (const c of uuid) h = (h * 31 + c.charCodeAt(0)) >>> 0;
  return colors[h % colors.length];
}
