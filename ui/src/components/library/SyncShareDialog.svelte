<script lang="ts">
  // Geteiltes Sync-Pack öffnen (F-58, `pinecone://sync/<code>`): erst alle Werte zeigen, dann
  // als eigenes Sync-Pack übernehmen. Eigene Dateien werden nie geteilt.
  import { onMount } from "svelte";
  import { api, errorText } from "../../lib/api";
  import { t } from "../../lib/i18n.svelte";
  import { controlName, isControl, keyLabel } from "../../lib/sync";
  import type { SharedSyncPack, SyncPack } from "../../lib/types";

  let { code, onClose, onImported }: { code: string; onClose: () => void; onImported: (p: SyncPack) => void } = $props();

  let dialog: HTMLDialogElement;
  let shared = $state<SharedSyncPack | null>(null);
  let busy = $state(false);
  let error = $state<string | null>(null);

  onMount(() => {
    dialog.showModal();
    api
      .syncpackSharePreview(code)
      .then((s) => (shared = s))
      .catch((e) => (error = errorText(e)));
  });

  async function take() {
    busy = true;
    error = null;
    try {
      onImported(await api.syncpackShareImport(code));
    } catch (e) {
      error = errorText(e);
      busy = false;
    }
  }

  const options = $derived(Object.entries(shared?.pack.options ?? {}));
</script>

<dialog bind:this={dialog} oncancel={(e) => (e.preventDefault(), !busy && onClose())}>
  <div class="dialog-body">
    <h2>{t("syncShare.openTitle")}</h2>
    {#if shared}
      {@const p = shared.pack}
      <p>{t("syncShare.from", { owner: shared.owner, name: p.name })} · {p.mode === "sync" ? t("sync.modeSync") : t("sync.modeTemplate")}</p>
      {#if p.entries.length > 0}
        <span class="section-label">{t("syncShare.projects")}</span>
        <ul>
          {#each p.entries as e, i (i)}
            {#if e.type !== "file"}<li>{e.name} <span class="muted">· {e.type === "modrinth" ? "Modrinth" : "CurseForge"}</span></li>{/if}
          {/each}
        </ul>
      {/if}
      {#if options.length > 0}
        <span class="section-label">{t("sync.options")}</span>
        <dl>
          {#each options as [k, v] (k)}
            <dt class="mono">{isControl(k) ? controlName(k) : k}</dt>
            <dd class="mono">{isControl(k) ? keyLabel(v) : v}</dd>
          {/each}
        </dl>
      {/if}
      <p class="muted small">{t("syncShare.openHint")}</p>
    {:else if !error}
      <p class="muted">{t("common.loading")}</p>
    {/if}
    {#if error}<p class="error">{error}</p>{/if}
  </div>
  <div class="dialog-actions">
    <button onclick={onClose} disabled={busy}>{t("common.cancel")}</button>
    <button class="primary cut" onclick={take} disabled={busy || !shared}>{busy ? t("common.loading") : t("syncShare.take")}</button>
  </div>
</dialog>

<style>
  p {
    margin: 0;
    font-size: 13px;
  }

  .small {
    font-size: 12px;
  }

  ul {
    margin: 0;
    padding-left: 18px;
    font-size: 13px;
  }

  dl {
    margin: 0;
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 4px 16px;
    max-height: 240px;
    overflow: auto;
    padding: 8px 10px;
    background: var(--surface-2);
    font-size: 12px;
  }

  dt {
    color: var(--text-muted);
  }

  dd {
    margin: 0;
  }
</style>
