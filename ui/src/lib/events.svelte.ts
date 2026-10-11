// TwoGG-Events von twogg.app und der eigene Stand (Anmeldung, Team/Gilde), einmal geladen und
// von Spielen- und Events-Seite geteilt.

import { api, errorText } from "./api";
import { i18n, t } from "./i18n.svelte";
import type { AppEvent, AppUser, Participation } from "./types";

export const twogg: {
  events: AppEvent[] | null;
  user: AppUser | null;
  participations: Participation[];
  error: string | null;
  loading: boolean;
} = $state({ events: null, user: null, participations: [], error: null, loading: false });

let loaded = 0;

/** Lädt Events (höchstens alle 5 Minuten, außer `force`) und, falls verknüpft, den eigenen Stand. */
export async function loadTwogg(force = false) {
  if (twogg.loading || (!force && Date.now() - loaded < 5 * 60_000 && twogg.events)) return;
  twogg.loading = true;
  try {
    const [events, user] = await Promise.all([api.appEvents(), api.appMe().catch(() => null)]);
    twogg.events = events;
    twogg.user = user;
    twogg.participations = user ? await api.appParticipations().catch(() => []) : [];
    twogg.error = null;
    loaded = Date.now();
  } catch (e) {
    twogg.error = errorText(e);
  } finally {
    twogg.loading = false;
  }
}

export function participationOf(eventId: string): Participation | null {
  const list = twogg.participations.filter((p) => p.eventId === eventId);
  return list.find((p) => p.kind === "team") ?? list.find((p) => p.kind === "registered") ?? list[0] ?? null;
}

/** Event für den Hero: das nächste offene, sonst das zuletzt beendete. */
export function featured(events: AppEvent[]): AppEvent | null {
  return events.find((e) => !e.ended) ?? events[0] ?? null;
}

function locale() {
  return i18n.lang === "de" ? "de-DE" : "en-GB";
}

/** „28.11.“ bzw. „28.11. · 18:00“ */
export function eventDate(e: AppEvent, withTime = false): string | null {
  if (!e.eventDate) return null;
  const d = new Date(e.eventDate);
  const date = d.toLocaleDateString(locale(), { day: "2-digit", month: "2-digit" });
  return withTime ? `${date} · ${d.toLocaleTimeString(locale(), { hour: "2-digit", minute: "2-digit" })}` : date;
}

/** Text für das Event-Badge: TWOGG EVENT · TERMIN FOLGT */
export function badgeText(e: AppEvent): string {
  const when = e.ended ? t("events.ended") : (eventDate(e, true) ?? t("events.dateTbd"));
  return `${t("events.badge")} · ${when}`;
}

export function typeLabel(e: AppEvent): string {
  return e.eventType === "ROLEPLAY" ? t("events.typeRoleplay") : e.eventType === "TEAM" ? t("events.typeTeam") : t("events.typeSolo");
}

/** Untertitel im Kalender: „Rollenspiel · Termin folgt“, „Team-Event · 03.10. · beendet“ */
export function calendarLine(e: AppEvent): string {
  const parts = [typeLabel(e), eventDate(e) ?? t("events.dateTbd")];
  if (e.ended) parts.push(t("events.endedLower"));
  return parts.join(" · ");
}

/** Motiv für Events ohne Logo, passend zum Design-System. */
export function eventMotif(e: AppEvent): "shield" | "snowflake" | "waves" | "bolt" {
  const title = e.title.toLowerCase();
  if (title.includes("winter")) return "snowflake";
  if (title.includes("latest") || title.includes("atlantis")) return "waves";
  if (e.eventType === "ROLEPLAY") return "shield";
  return "bolt";
}
