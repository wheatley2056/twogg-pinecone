<script lang="ts">
  import { onMount } from "svelte";
  import { save } from "@tauri-apps/plugin-dialog";
  import { api, errorText } from "../lib/api";
  import { t } from "../lib/i18n.svelte";
  import type { Profile } from "../lib/types";

  let { profile, onClose }: { profile: Profile; onClose: () => void } = $props();

  let dialog: HTMLDialogElement;
  let version = $state("1.0.0");
  let busy = $state(false);
  let error = $state<string | null>(null);
  let result = $state<string | null>(null);

  onMount(() => dialog.showModal());

  async function exportPack() {
    error = result = null;
    const safeName = profile.name.replace(/[^\w.-]+/g, "_");
    const path = await save({ defaultPath: `${safeName}-${version}.mrpack`, filters: [{ name: t("pack.filter"), extensions: ["mrpack"] }] });
    if (!path) return;
    busy = true;
    try {
      const r = await api.packExport(profile.id, path, version);
      result = t("pack.exported", { linked: r.linked, included: r.included });
    } catch (e) {
      error = errorText(e);
    } finally {
      busy = false;
    }
  }
</script>

<dialog bind:this={dialog} oncancel={(e) => (e.preventDefault(), !busy && onClose())}>
  <div class="dialog-body">
    <h2>{t("pack.exportTitle")}</h2>
    <p class="muted">{t("pack.exportHint")}</p>
    <label>
      {t("pack.exportVersion")}
      <input bind:value={version} maxlength="32" spellcheck="false" />
    </label>
    {#if error}<p class="error">{error}</p>{/if}
    {#if result}<p class="notice">{result}</p>{/if}
  </div>
  <div class="dialog-actions">
    <button onclick={onClose} disabled={busy}>{t("common.close")}</button>
    <button class="primary" onclick={exportPack} disabled={busy || !version.trim()}>{busy ? t("common.loading") : t("pack.exportButton")}</button>
  </div>
</dialog>

<style>
  p {
    margin: 0;
  }

  .notice {
    color: var(--accent);
  }
</style>
