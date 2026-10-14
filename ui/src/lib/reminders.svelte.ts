// Hinweise vor Eventstart (F-14): zu jeder Vorlaufzeit ein Eintrag im Panel und eine
// Desktop-Benachrichtigung, solange Pinecone läuft. Beim ersten Hinweis wird ein Pflicht-Update
// des Event-Packs schon eingespielt, damit der Start am Eventtag nicht wartet.
import { api } from "./api";
import { loadTwogg, participationOf, twogg } from "./events.svelte";
import { t } from "./i18n.svelte";
import { notify } from "./notifications.svelte";
import { osNotify } from "./osNotify";
import { settings } from "./settings.svelte";
import type { AppEvent, NotificationSettings } from "./types";

const STORE_KEY = "pinecone.reminders.fired";
/** So lange nach dem Hinweiszeitpunkt wird er noch nachgeholt (z. B. nach dem Aufwachen). */
const GRACE_MS = 15 * 60_000;

export interface Due {
  event: AppEvent;
  lead: number;
  key: string;
  /** Erster Hinweis zu diesem Termin (für das Vorab-Laden). */
  first: boolean;
}

/** Fällige Hinweise. `fired` enthält bereits gezeigte Schlüssel `<id>@<termin>:<vorlauf>`;
 *  ein verschobener Termin bekommt dadurch neue Hinweise (AK-14.5). */
export function dueReminders(
  events: AppEvent[],
  prefs: NotificationSettings,
  registered: (id: string) => boolean,
  now: number,
  fired: Set<string>,
): Due[] {
  if (!prefs.eventReminders) return [];
  const leads = [...new Set(prefs.leadMinutes.filter((m) => m > 0))].sort((a, b) => b - a);
  const due: Due[] = [];
  for (const event of events) {
    if (event.ended || !event.eventDate) continue;
    if (!prefs.allEvents && !registered(event.id)) continue;
    const start = new Date(event.eventDate).getTime();
    if (!(start > now)) continue;
    const base = `${event.id}@${event.eventDate}`;
    const anyFired = leads.some((l) => fired.has(`${base}:${l}`));
    // Nur die kürzeste fällige Vorlaufzeit, damit nach einer Pause nicht mehrere auf einmal kommen.
    const lead = leads.filter((l) => now >= start - l * 60_000 && now < start - l * 60_000 + GRACE_MS && !fired.has(`${base}:${l}`)).at(-1);
    if (lead === undefined) continue;
    due.push({ event, lead, key: `${base}:${lead}`, first: !anyFired });
  }
  return due;
}

function loadFired(): Set<string> {
  try {
    return new Set(JSON.parse(localStorage.getItem(STORE_KEY) ?? "[]") as string[]);
  } catch {
    return new Set();
  }
}

function saveFired(fired: Set<string>) {
  try {
    localStorage.setItem(STORE_KEY, JSON.stringify([...fired].slice(-200)));
  } catch {
    // Ohne Speicher kann ein Hinweis nach einem Neustart doppelt kommen.
  }
}

function leadText(minutes: number): string {
  if (minutes >= 60 && minutes % 60 === 0) return t(minutes === 60 ? "reminder.hour" : "reminder.hours", { count: minutes / 60 });
  return t("reminder.minutes", { count: minutes });
}

/** Pflicht-Update der passenden Event-Instanz jetzt einspielen. */
async function prefetch(event: AppEvent, profileIdOfPack: (packId: string) => string | null) {
  const packs = await api.twoggPacks().catch(() => []);
  const pack = packs.find((p) => p.eventId === event.id);
  const profileId = pack ? profileIdOfPack(pack.id) : null;
  if (!profileId) return;
  const r = await api.eventUpdateNow(profileId).catch(() => null);
  if (r) notify({ id: `updated-${profileId}-${Date.now()}`, kind: "info", title: t("eventUpdate.title"), text: t("eventUpdate.text", { from: r[0], to: r[1] }) });
}

/** Startet den Planer; prüft alle 30 Sekunden und lädt Termine höchstens alle 5 Minuten neu. */
export function startReminders(opts: { onOpen: (eventId: string) => void; profileIdOfPack: (packId: string) => string | null }) {
  const fired = loadFired();
  const tick = async () => {
    const prefs = settings.current?.notifications;
    if (!prefs?.eventReminders) return;
    await loadTwogg();
    const due = dueReminders(twogg.events ?? [], prefs, (id) => !!participationOf(id), Date.now(), fired);
    for (const d of due) {
      fired.add(d.key);
      const title = t("reminder.title", { event: d.event.title, when: leadText(d.lead) });
      notify({ id: `reminder-${d.key}`, kind: "event", title, text: t("reminder.text"), action: { label: t("reminder.open"), run: () => opts.onOpen(d.event.id) } });
      osNotify(title, t("reminder.text"));
      if (d.first) prefetch(d.event, opts.profileIdOfPack);
    }
    if (due.length) saveFired(fired);
  };
  tick();
  const timer = setInterval(tick, 30_000);
  return () => clearInterval(timer);
}
