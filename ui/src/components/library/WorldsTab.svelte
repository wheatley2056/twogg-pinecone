<script lang="ts">
  // Tab „Welten“: Welten der Instanz mit Spielmodus, letzter Änderung und Größe; sichern,
  // Ordner öffnen, löschen.
  import { onMount } from "svelte";
  import { api, errorText, fileSrc } from "../../lib/api";
  import { i18n, t } from "../../lib/i18n.svelte";
  import type { Profile, World } from "../../lib/types";
  import ConfirmDialog from "../ConfirmDialog.svelte";
  import Icon from "../Icon.svelte";

  let { profile, locked }: { profile: Profile; locked: boolean } = $props();

  let worlds = $state<World[] | null>(null);
  let busy = $state<string | null>(null);
  let error = $state<string | null>(null);
  let message = $state<string | null>(null);
  let deleting = $state<World | null>(null);

  async function load() {
    try {
      worlds = await api.worlds(profile.id);
    } catch (e) {
      error = errorText(e);
      worlds = [];
    }
  }

  onMount(load);

  async function run(folder: string, action: () => Promise<unknown>) {
    busy = folder;
    error = message = null;
    try {
      await action();
    } catch (e) {
      error = errorText(e);
    } finally {
      busy = null;
    }
  }

  function size(bytes: number) {
    return bytes >= 1e9 ? `${(bytes / 1e9).toFixed(1)} GB` : `${Math.max(1, Math.round(bytes / 1e6))} MB`;
  }

  function when(ms: number | null) {
    return ms ? new Date(ms).toLocaleString(i18n.lang === "de" ? "de-DE" : "en-GB", { dateStyle: "medium", timeStyle: "short" }) : "—";
  }
</script>

<div class="tab">
  {#if error}<p class="error">{error}</p>{/if}
  {#if message}<p class="message">{message}</p>{/if}

  {#if worlds === null}
    <p class="empty">{t("common.loading")}</p>
  {:else if worlds.length === 0}
    <p class="empty">{t("worlds.empty")}</p>
  {:else}
    <div class="list">
      {#each worlds as w (w.folder)}
        <div class="row">
          {#if w.icon}
            <img class="icon" src={fileSrc(w.icon)} alt="" width="48" height="48" />
          {:else}
            <span class="icon placeholder"><Icon name="globe" size={22} /></span>
          {/if}
          <span class="text">
            <span class="name">{w.name}</span>
            <span class="meta">
              {#if w.gameMode}<span class="chip">{t(`worlds.mode.${w.gameMode}`)}{w.hardcore ? ` · ${t("worlds.hardcore")}` : ""}</span>{/if}
              {#if w.version}<span class="chip">{w.version}</span>{/if}
              <span class="chip">{size(w.size)}</span>
              <span class="when">{t("worlds.lastPlayed", { when: when(w.lastPlayed) })}</span>
            </span>
          </span>
          <button class="icon-btn" aria-label={t("worlds.openFolder")} title={t("worlds.openFolder")} onclick={() => run(w.folder, () => api.worldOpenFolder(profile.id, w.folder))}>
            <Icon name="folder" size={18} />
          </button>
          <button class="small" disabled={busy !== null} onclick={() => run(w.folder, async () => {
            const path = await api.worldBackup(profile.id, w.folder);
            message = t("worlds.backedUp", { path });
          })}>
            {busy === w.folder ? t("worlds.backingUp") : t("worlds.backup")}
          </button>
          <button class="icon-btn danger" aria-label={t("worlds.delete")} title={locked ? t("worlds.lockedHint") : t("worlds.delete")} disabled={locked || busy !== null} onclick={() => (deleting = w)}>
            <Icon name="trash" size={18} />
          </button>
        </div>
      {/each}
    </div>
  {/if}
</div>

{#if deleting}
  {@const w = deleting}
  <ConfirmDialog
    title={t("worlds.deleteTitle")}
    confirmLabel={t("worlds.delete")}
    onCancel={() => (deleting = null)}
    onConfirm={() => {
      deleting = null;
      run(w.folder, async () => {
        await api.worldDelete(profile.id, w.folder);
        await load();
      });
    }}
  >
    <p>{t("worlds.deleteText", { name: w.name })}</p>
  </ConfirmDialog>
{/if}

<style>
  .tab {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 16px 28px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .list {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 10px 12px;
    background: var(--surface);
    clip-path: var(--cut-1);
  }

  .icon {
    width: 48px;
    height: 48px;
    flex: none;
    image-rendering: pixelated;
  }

  .placeholder {
    display: grid;
    place-items: center;
    background: var(--surface-2);
    color: var(--text-muted);
  }

  .text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .name {
    font-size: 15px;
    font-weight: 700;
  }

  .meta {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
  }

  .meta .chip {
    padding: 3px 7px;
    font-size: 11px;
  }

  .when {
    font-size: 12px;
    color: var(--text-muted);
  }

  .icon-btn {
    width: 38px;
    min-height: 38px;
    padding: 0;
    color: var(--text-soft);
  }

  .small {
    min-height: 38px;
    font-size: 13px;
  }

  .message {
    margin: 0;
    font-size: 13px;
    color: var(--nadel);
    user-select: text;
  }

  .empty {
    color: var(--text-dim);
  }
</style>
