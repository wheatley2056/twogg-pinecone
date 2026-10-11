<script lang="ts">
  // Neues Sync-Pack oder neue Vorlage anlegen.
  import { onMount } from "svelte";
  import { api, errorText } from "../../lib/api";
  import { t } from "../../lib/i18n.svelte";
  import type { SyncMode, SyncPack } from "../../lib/types";
  import Segmented from "../ui/Segmented.svelte";

  let { onClose, onCreated }: { onClose: () => void; onCreated: (p: SyncPack) => void } = $props();

  let dialog: HTMLDialogElement;
  let name = $state("");
  let mode = $state<SyncMode>("sync");
  let busy = $state(false);
  let error = $state<string | null>(null);

  onMount(() => dialog.showModal());

  async function create(e: SubmitEvent) {
    e.preventDefault();
    if (!name.trim()) return;
    busy = true;
    try {
      onCreated(await api.syncpackCreate(name.trim(), mode));
    } catch (err) {
      error = errorText(err);
      busy = false;
    }
  }
</script>

<dialog bind:this={dialog} oncancel={(e) => (e.preventDefault(), onClose())}>
  <form onsubmit={create}>
    <div class="dialog-body">
      <h2>{t("sync.new")}</h2>
      <label>
        {t("sync.name")}
        <!-- svelte-ignore a11y_autofocus (Erstes Feld im modalen Dialog) -->
        <input bind:value={name} maxlength="48" placeholder={t("sync.newName")} autofocus />
      </label>
      <Segmented
        items={[
          { id: "sync", label: t("sync.modeSync") },
          { id: "template", label: t("sync.modeTemplate") },
        ]}
        value={mode}
        label={t("sync.mode")}
        variant="soft"
        onchange={(m) => (mode = m as SyncMode)}
      />
      <p class="muted">{mode === "sync" ? t("sync.modeSyncHint") : t("sync.modeTemplateHint")}</p>
      {#if error}<p class="error">{error}</p>{/if}
    </div>
    <div class="dialog-actions">
      <button type="button" onclick={onClose}>{t("common.cancel")}</button>
      <button type="submit" class="primary cut" disabled={busy || !name.trim()}>{t("sync.create")}</button>
    </div>
  </form>
</dialog>

<style>
  p {
    margin: 0;
    font-size: 13px;
  }
</style>
