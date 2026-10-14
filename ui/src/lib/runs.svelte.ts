// Laufzustand je Profil: Vorbereitung, Fortschritt, Log. Bleibt beim Profilwechsel erhalten.

import { api, errorText } from "./api";
import { t } from "./i18n.svelte";
import { notify } from "./notifications.svelte";
import type { LaunchEvent, Stage } from "./types";

export type RunStatus = "idle" | "preparing" | "running";

export interface RunState {
  status: RunStatus;
  stage: Stage | null;
  done: number;
  files: number;
  bytesDone: number;
  bytes: number;
  java: string | null;
  lines: string[];
  error: string | null;
  exitCode: number | null;
  /** Vom Nutzer beendet: Exit-Code ist dann kein Fehler. */
  stopping: boolean;
  /** Quick-Join-Ziel des laufenden Starts. */
  server: string | null;
}

const MAX_LINES = 5000;

export const runs: Record<string, RunState> = $state({});

function idle(): RunState {
  return { status: "idle", stage: null, done: 0, files: 0, bytesDone: 0, bytes: 0, java: null, lines: [], error: null, exitCode: null, stopping: false, server: null };
}

const IDLE: RunState = Object.freeze(idle()) as RunState;

/** Nur lesen (auch in Templates erlaubt): noch nie gestartete Profile sind im Leerlauf. */
export function runOf(id: string): RunState {
  return runs[id] ?? IDLE;
}

/** Eintrag anlegen, um ihn zu ändern. Nicht aus Templates oder `$derived` aufrufen. */
export function ensureRun(id: string): RunState {
  runs[id] ??= idle();
  return runs[id];
}

function appendLines(run: RunState, lines: string[]) {
  run.lines.push(...lines);
  if (run.lines.length > MAX_LINES) run.lines.splice(0, run.lines.length - MAX_LINES);
}

/** Startet ein Profil, optional direkt auf einen Server. `onStarted` folgt, sobald das Spiel läuft,
 *  `onExit` nach Spielende bzw. Fehler. */
export async function startGame(id: string, onExit: () => void, server: string | null = null, onStarted?: () => void) {
  const run = ensureRun(id);
  Object.assign(run, { status: "preparing", stage: null, done: 0, files: 0, bytesDone: 0, bytes: 0, error: null, exitCode: null, stopping: false, server });
  run.lines = [];
  const handle = (e: LaunchEvent) => {
    switch (e.type) {
      case "stage":
        run.stage = e.stage;
        run.done = run.files = 0;
        break;
      case "progress":
        run.done = e.done;
        run.files = e.files;
        run.bytesDone = e.bytesDone;
        if (e.bytes) run.bytes = e.bytes;
        break;
      case "java":
        run.java = `Java ${e.version}`;
        break;
      case "integrity": {
        const name = (p: string) => p.split("/").pop() ?? p;
        const parts = [
          e.repaired.length ? t("integrity.repaired", { files: e.repaired.map(name).join(", ") }) : "",
          e.disabled.length ? t("integrity.disabled", { files: e.disabled.map(name).join(", ") }) : "",
        ].filter(Boolean);
        notify({ id: `integrity-${id}-${Date.now()}`, kind: "info", title: t("integrity.title"), text: parts.join(" ") });
        break;
      }
      case "started":
        run.status = "running";
        onStarted?.();
        break;
      case "log":
        appendLines(run, e.lines);
        break;
      case "exited":
        run.status = "idle";
        run.exitCode = e.code;
        if (!e.success && e.code !== null && !run.stopping) run.error = t("profile.exitError", { code: e.code });
        onExit();
        break;
    }
  };
  try {
    await api.launch(id, server, handle);
  } catch (e) {
    run.status = "idle";
    run.error = errorText(e);
    onExit();
  }
}

/** Beendet das laufende Spiel eines Profils. */
export async function stopGame(id: string) {
  const run = ensureRun(id);
  run.stopping = true;
  try {
    await api.stopGame(id);
  } catch (e) {
    run.stopping = false;
    throw e;
  }
}
