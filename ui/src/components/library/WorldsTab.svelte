<script lang="ts">
  // Tab „Welten“: Welten der Instanz mit Spielmodus, letzter Änderung und Größe; sichern,
  // aus Backups wiederherstellen (auch gelöschte Welten), Ordner öffnen, löschen.
  // Oben: automatische Backups vor jedem Start mit Anzahl, die behalten wird.
  import { menu } from "../../lib/contextmenu.svelte";
  import { onMount } from "svelte";
  import { api, errorText, fileSrc } from "../../lib/api";
  import { i18n, t } from "../../lib/i18n.svelte";
  import type { Profile, World, WorldBackup } from "../../lib/types";
  import ConfirmDialog from "../ConfirmDialog.svelte";
  import Icon from "../Icon.svelte";
  import Switch from "../ui/Switch.svelte";

  let { profile, locked }: { profile: Profile; locked: boolean } = $props();

  let worlds = $state<World[] | null>(null);
  let busy = $state<string | null>(null);
  let error = $state<string | null>(null);
  let message = $state<string | null>(null);
  let deleting = $state<World | null>(null);
  let backups = $state<WorldBackup[]>([]);
  let open = $state<Set<string>>(new Set());
  let restoring = $state<{ backup: WorldBackup; name: string; exists: boolean } | null>(null);
  let removingBackup = $state<WorldBackup | null>(null);
  // svelte-ignore state_referenced_locally
  let autoBackup = $state(profile.backupOnLaunch);
  // svelte-ignore state_referenced_locally
  let keep = $state(profile.backupKeep);

  async function load() {
    try {
      [worlds, backups] = await Promise.all([api.worlds(profile.id), api.worldBackups(profile.id)]);
    } catch (e) {
      error = errorText(e);
      worlds ??= [];
    }
  }

  const backupsOf = (folder: string) => backups.filter((b) => b.folder === folder);
  // Backups von Welten, die es nicht mehr gibt, nach Ordner gruppiert.
  const orphans = $derived.by(() => {
    const present = new Set((worlds ?? []).map((w) => w.folder));
    const folders = [...new Set(backups.map((b) => b.folder))].filter((f) => !present.has(f));
    return folders.map((folder) => ({ folder, items: backupsOf(folder) }));
  });

  function toggle(folder: string) {
    const next = new Set(open);
    if (!next.delete(folder)) next.add(folder);
    open = next;
  }

  async function saveAuto(enabled: boolean, count: number) {
    const before = { autoBackup, keep };
    autoBackup = enabled;
    keep = Math.min(50, Math.max(1, Math.round(count) || 1));
    try {
      await api.setBackupOnLaunch(profile.id, autoBackup, keep);
    } catch (e) {
      ({ autoBackup, keep } = before);
      error = errorText(e);
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

  function backupLabel(b: WorldBackup) {
    return `${when(b.created)} · ${size(b.size)}`;
  }

  function when(ms: number | null) {
    return ms ? new Date(ms).toLocaleString(i18n.lang === "de" ? "de-DE" : "en-GB", { dateStyle: "medium", timeStyle: "short" }) : "—";
  }
</script>

{#snippet backupList(items: WorldBackup[], name: string, exists: boolean)}
  <ul class="backups" aria-label={t("worlds.backupsOf", { name })}>
    {#each items as b (b.file)}
      <li>
        <Icon name="clock" size={16} />
        <span class="b-when">{backupLabel(b)}</span>
        {#if b.auto}<span class="chip">{t("worlds.backupAuto")}</span>{/if}
        <span class="grow"></span>
        <button class="small" disabled={locked || busy !== null} title={locked ? t("worlds.lockedHint") : undefined} onclick={() => (restoring = { backup: b, name, exists })}>
          {busy === b.file ? t("worlds.restoring") : t("worlds.restore")}
        </button>
        <button class="icon-btn danger" aria-label={t("worlds.backupDelete")} title={t("worlds.backupDelete")} disabled={busy !== null} onclick={() => (removingBackup = b)}>
          <Icon name="trash" size={16} />
        </button>
      </li>
    {/each}
  </ul>
{/snippet}

<div class="tab">
  <div class="auto">
    <Switch checked={autoBackup} label={t("worlds.autoBackup")} onchange={(on) => saveAuto(on, keep)} />
    <span class="auto-text">
      <span class="title">{t("worlds.autoBackup")}</span>
      <span class="hint">{t("worlds.autoBackupHint")}</span>
    </span>
    {#if autoBackup}
      <label class="keep">
        {t("worlds.keep")}
        <input type="number" min="1" max="50" value={keep} onchange={(e) => saveAuto(true, Number(e.currentTarget.value))} />
      </label>
    {/if}
  </div>

  {#if error}<p class="error">{error}</p>{/if}
  {#if message}<p class="message">{message}</p>{/if}

  {#if worlds === null}
    <p class="empty">{t("common.loading")}</p>
  {:else if worlds.length === 0}
    <p class="empty">{t("worlds.empty")}</p>
  {/if}
  {#if worlds && worlds.length > 0}
    <div class="list">
      {#each worlds as w (w.folder)}
        {@const own = backupsOf(w.folder)}
        <div class="row" use:menu={() => [
          { label: t("worlds.openFolder"), icon: "folder", action: () => run(w.folder, () => api.worldOpenFolder(profile.id, w.folder)) },
          { label: t("worlds.backup"), icon: "download", disabled: busy !== null, action: () => run(w.folder, async () => {
            const path = await api.worldBackup(profile.id, w.folder);
            message = t("worlds.backedUp", { path });
          }) },
          "-",
          { label: t("worlds.delete"), icon: "trash", danger: true, disabled: locked || busy !== null, action: () => (deleting = w) },
        ]}>
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
          {#if own.length > 0}
            <button class="small ghost" aria-expanded={open.has(w.folder)} onclick={() => toggle(w.folder)}>
              {t("worlds.backupsCount", { n: own.length })}
            </button>
          {/if}
          <button class="small" disabled={busy !== null} onclick={() => run(w.folder, async () => {
            const path = await api.worldBackup(profile.id, w.folder);
            message = t("worlds.backedUp", { path });
            await load();
          })}>
            {busy === w.folder ? t("worlds.backingUp") : t("worlds.backup")}
          </button>
          <button class="icon-btn danger" aria-label={t("worlds.delete")} title={locked ? t("worlds.lockedHint") : t("worlds.delete")} disabled={locked || busy !== null} onclick={() => (deleting = w)}>
            <Icon name="trash" size={18} />
          </button>
        </div>
        {#if open.has(w.folder) && own.length > 0}
          {@render backupList(own, w.name, true)}
        {/if}
      {/each}
    </div>
  {/if}

  {#if orphans.length > 0}
    <h3 class="section-label">{t("worlds.deletedWorlds")}</h3>
    <div class="list">
      {#each orphans as o (o.folder)}
        <div class="row orphan">
          <span class="icon placeholder"><Icon name="globe" size={22} /></span>
          <span class="text">
            <span class="name">{o.folder}</span>
            <span class="when">{t("worlds.deletedHint")}</span>
          </span>
          <button class="small ghost" aria-expanded={open.has(o.folder)} onclick={() => toggle(o.folder)}>
            {t("worlds.backupsCount", { n: o.items.length })}
          </button>
        </div>
        {#if open.has(o.folder)}
          {@render backupList(o.items, o.folder, false)}
        {/if}
      {/each}
    </div>
  {/if}
</div>

{#if restoring}
  {@const r = restoring}
  <ConfirmDialog
    title={t("worlds.restoreTitle")}
    confirmLabel={t("worlds.restore")}
    onCancel={() => (restoring = null)}
    onConfirm={() => {
      const { backup, name } = r;
      restoring = null;
      run(backup.file, async () => {
        const safety = await api.worldRestore(profile.id, backup.file);
        message = safety ? t("worlds.restoredSafety", { name, path: safety }) : t("worlds.restored", { name });
        await load();
      });
    }}
  >
    <p>{t(r.exists ? "worlds.restoreText" : "worlds.restoreTextDeleted", { name: r.name, when: when(r.backup.created) })}</p>
  </ConfirmDialog>
{/if}

{#if removingBackup}
  {@const b = removingBackup}
  <ConfirmDialog
    title={t("worlds.backupDeleteTitle")}
    confirmLabel={t("worlds.backupDelete")}
    onCancel={() => (removingBackup = null)}
    onConfirm={() => {
      const file = b.file;
      removingBackup = null;
      run(file, async () => {
        await api.worldBackupDelete(profile.id, file);
        await load();
      });
    }}
  >
    <p>{t("worlds.backupDeleteText", { when: when(b.created) })}</p>
  </ConfirmDialog>
{/if}

{#if deleting}
  {@const w = deleting}
  <ConfirmDialog
    title={t("worlds.deleteTitle")}
    confirmLabel={t("worlds.delete")}
    onCancel={() => (deleting = null)}
    onConfirm={() => {
      const folder = w.folder;
      deleting = null;
      run(folder, async () => {
        await api.worldDelete(profile.id, folder);
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

  .auto {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 12px 14px;
    background: var(--surface);
    clip-path: var(--cut-1);
  }

  .auto-text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .auto-text .title {
    font-size: 14px;
    font-weight: 700;
  }

  .auto-text .hint {
    font-size: 12px;
    color: var(--text-muted);
  }

  .keep {
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    color: var(--text-soft);
  }

  .keep input {
    width: 72px;
    background: var(--surface-2);
  }

  .backups {
    list-style: none;
    margin: -2px 0 4px 62px;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .backups li {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 6px 10px;
    background: var(--surface-2);
    color: var(--text-soft);
    font-size: 13px;
  }

  .backups .chip {
    padding: 2px 6px;
    font-size: 11px;
  }

  .b-when {
    font-family: var(--font-mono);
    font-size: 12px;
  }

  .grow {
    flex: 1;
  }

  .backups .small {
    min-height: 32px;
  }

  .backups .icon-btn {
    width: 32px;
    min-height: 32px;
  }

  .orphan .name {
    color: var(--text-soft);
  }

  .section-label {
    margin: 12px 0 0;
  }
</style>
