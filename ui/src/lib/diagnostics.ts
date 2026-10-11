import { save } from "@tauri-apps/plugin-dialog";
import { api } from "./api";
import { t } from "./i18n.svelte";

/** Fragt nach dem Speicherort und schreibt das Diagnose-ZIP; liefert eine Meldung oder null (abgebrochen). */
export async function exportDiagnostics(profileId: string | null = null, name = "pinecone"): Promise<string | null> {
  const day = new Date().toISOString().slice(0, 10);
  const path = await save({
    defaultPath: `${name.replace(/[^\w.-]+/g, "_")}-diagnose-${day}.zip`,
    filters: [{ name: "ZIP", extensions: ["zip"] }],
  });
  if (!path) return null;
  const r = await api.diagnosticsExport(path, profileId);
  return t("diag.done", { files: r.files, size: Math.max(1, Math.round(r.bytes / 1024)) });
}
