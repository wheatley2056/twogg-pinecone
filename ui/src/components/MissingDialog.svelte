<script lang="ts">
  import { onMount } from "svelte";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { api } from "../lib/api";
  import { t } from "../lib/i18n.svelte";
  import type { ManualDownload } from "../lib/types";

  let { profileId, missing, onClose }: { profileId: string; missing: ManualDownload[]; onClose: () => void } = $props();
  let dialog: HTMLDialogElement;
  onMount(() => dialog.showModal());
</script>

<dialog bind:this={dialog} oncancel={(e) => (e.preventDefault(), onClose())}>
  <div class="dialog-body">
    <h2>{t("missing.title")}</h2>
    <p class="muted">{t("missing.text")}</p>
    <ul>
      {#each missing as m (m.path)}
        <li>
          <div class="text">
            <strong>{m.name}</strong>
            <code>{m.path}</code>
          </div>
          <button onclick={() => openUrl(m.url)}>{t("missing.open")}</button>
        </li>
      {/each}
    </ul>
  </div>
  <div class="dialog-actions">
    <button class="ghost" onclick={() => api.openProfileFolder(profileId)}>{t("missing.folder")}</button>
    <button class="primary" onclick={onClose}>{t("common.close")}</button>
  </div>
</dialog>

<style>
  p {
    margin: 0;
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  li {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    background: var(--surface-2);
  }

  .text {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  code {
    font-size: 12px;
    color: var(--text-muted);
    overflow-wrap: anywhere;
    user-select: text;
  }
</style>
