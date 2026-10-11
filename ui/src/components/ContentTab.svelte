<script lang="ts">
  import { onMount } from "svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { api, errorText } from "../lib/api";
  import { t } from "../lib/i18n.svelte";
  import type { ContentKind, InstalledContent, InstallReport, Profile, ProfileSyncPack, SyncReport, UpdateInfo } from "../lib/types";
  import ConfirmDialog from "./ConfirmDialog.svelte";
  import Icon from "./Icon.svelte";

  interface Props {
    profile: Profile;
    /** Spiel läuft: Dateien nicht anfassen. */
    locked: boolean;
    onAddContent: (kind: ContentKind) => void;
  }
  let { profile, locked, onAddContent }: Props = $props();

  const KINDS: ContentKind[] = ["mod", "resourcepack", "shader"];

  let kind = $state<ContentKind>("mod");
  let items = $state<InstalledContent[]>([]);
  let filter = $state("");
  let identifying = $state(false);
  let checking = $state(false);
  let updates = $state<Record<string, UpdateInfo>>({});
  let checked = $state(false);
  let busy = $state<Set<string>>(new Set());
  let error = $state<string | null>(null);
  let message = $state<string | null>(null);
  let removing = $state<InstalledContent | null>(null);

  const counts = $derived(Object.fromEntries(KINDS.map((k) => [k, items.filter((i) => i.kind === k).length])) as Record<ContentKind, number>);
  const shown = $derived(
    items.filter((i) => i.kind === kind && (!filter.trim() || i.name.toLowerCase().includes(filter.trim().toLowerCase()))),
  );
  const pendingUpdates = $derived(Object.values(updates));
  const vanillaMods = $derived(kind === "mod" && !profile.loader);

  async function load() {
    try {
      items = await api.contentList(profile.id);
      if (items.some((i) => i.unidentified)) {
        identifying = true;
        items = await api.contentIdentify(profile.id);
      }
    } catch (e) {
      error = errorText(e);
    } finally {
      identifying = false;
    }
  }

  // Sync-Packs: Schalter je Profil; Namen auch für die Herkunfts-Markierung.
  let syncPacks = $state<ProfileSyncPack[]>([]);
  let syncBusy = $state<string | null>(null);
  const syncNames = $derived(Object.fromEntries(syncPacks.map((s) => [s.pack.id, s.pack.name])));

  async function loadSync() {
    syncPacks = await api.syncpackForProfile(profile.id).catch(() => []);
  }

  async function toggleSync(s: ProfileSyncPack) {
    error = message = null;
    syncBusy = s.pack.id;
    try {
      const r = await api.syncpackSetEnabled(profile.id, s.pack.id, !s.enabled);
      message = syncSummary(r);
      await Promise.all([loadSync(), load()]);
    } catch (e) {
      error = errorText(e);
    } finally {
      syncBusy = null;
    }
  }

  function syncSummary(r: SyncReport) {
    const parts = [
      r.installed.length && t("sync.reportInstalled", { list: r.installed.join(", ") }),
      r.removed.length && t("sync.reportRemoved", { list: r.removed.join(", ") }),
      r.present.length && t("sync.reportPresent", { list: r.present.join(", ") }),
      r.unavailable.length && t("sync.reportUnavailable", { list: r.unavailable.join(", ") }),
      ...r.warnings,
    ].filter(Boolean);
    return parts.length ? parts.join(" · ") : t("sync.reportNothing");
  }

  onMount(() => {
    load();
    loadSync();
  });

  async function act(key: string, action: () => Promise<unknown>) {
    error = message = null;
    busy = new Set([...busy, key]);
    try {
      await action();
      items = await api.contentList(profile.id);
    } catch (e) {
      error = errorText(e);
    } finally {
      busy = new Set([...busy].filter((k) => k !== key));
    }
  }

  function report(r: InstallReport) {
    const parts = [];
    if (r.installed.length) parts.push(t("content.installedMsg", { names: r.installed.map(([n, v]) => `${n} ${v}`).join(", ") }));
    if (r.warnings.length) parts.push(r.warnings.join(" "));
    message = parts.join(" · ") || null;
  }

  async function checkUpdates() {
    error = message = null;
    checking = true;
    try {
      const list = await api.contentUpdates(profile.id);
      updates = Object.fromEntries(list.map((u) => [u.key, u]));
      checked = true;
      if (list.length === 0) message = t("content.upToDate");
    } catch (e) {
      error = errorText(e);
    } finally {
      checking = false;
    }
  }

  async function update(u: UpdateInfo) {
    await act(u.key, async () => {
      report(await api.contentUpdate(profile.id, u));
      const { [u.key]: _, ...rest } = updates;
      updates = rest;
    });
  }

  async function updateAll() {
    for (const u of pendingUpdates) await update(u);
  }

  async function addFile() {
    const path = await open({
      multiple: false,
      filters: [{ name: t("content.fileFilter"), extensions: kind === "mod" ? ["jar"] : ["zip"] }],
    });
    if (typeof path === "string") await act("__add", () => api.contentAddLocal(profile.id, kind, path));
  }

  function sourceLabel(i: InstalledContent) {
    if (i.fromPack) return t("content.fromPack");
    if (i.sync) return t("content.fromSync", { name: syncNames[i.sync] ?? i.sync });
    return i.source.type === "modrinth" ? "Modrinth" : i.source.type === "curseforge" ? "CurseForge" : t("content.local");
  }
</script>

<div class="content-tab">
  {#if syncPacks.length > 0}
    <div class="sync-strip">
      <span class="muted small"><Icon name="layers" size={15} />{t("sync.strip")}</span>
      {#each syncPacks as s (s.pack.id)}
        <label class="sync-toggle" class:on={s.enabled} title={s.pack.mode === "template" ? t("sync.templateApply") : undefined}>
          <input
            type="checkbox"
            checked={s.enabled}
            disabled={locked || syncBusy !== null}
            onchange={(e) => (e.currentTarget.checked = s.enabled, toggleSync(s))}
          />
          {s.pack.name}{#if s.pack.mode === "template"}<span class="badge">{t("sync.modeTemplate")}</span>{/if}
          {#if syncBusy === s.pack.id}<span class="muted small">…</span>{/if}
        </label>
      {/each}
    </div>
  {/if}
  <div class="toolbar">
    <div class="kinds" role="tablist">
      {#each KINDS as k (k)}
        <button role="tab" aria-selected={kind === k} class:active={kind === k} onclick={() => (kind = k)}>
          {t(`content.${k}`)}<span class="count">{counts[k]}</span>
        </button>
      {/each}
    </div>
    <div class="buttons">
      <label class="search">
        <Icon name="search" size={15} />
        <input bind:value={filter} placeholder={t("content.filter")} aria-label={t("content.filter")} />
      </label>
      {#if pendingUpdates.length > 0}
        <button class="primary" onclick={updateAll} disabled={locked || busy.size > 0}>
          <Icon name="download" size={16} />{t("content.updateAll", { count: pendingUpdates.length })}
        </button>
      {:else}
        <button onclick={checkUpdates} disabled={checking || items.length === 0}>
          <Icon name="refresh" size={16} />{checking ? t("common.loading") : t("content.checkUpdates")}
        </button>
      {/if}
      <button onclick={addFile} disabled={locked || vanillaMods}><Icon name="file" size={16} />{t("content.addFile")}</button>
      <button class="primary" onclick={() => onAddContent(kind)} disabled={vanillaMods}><Icon name="plus" size={16} />{t("content.add").replace("+ ", "")}</button>
    </div>
  </div>

  {#if error}<p class="error">{error}</p>{/if}
  {#if message}<p class="notice">{message}</p>{/if}
  {#if identifying}<p class="muted small">{t("content.identifying")}</p>{/if}

  {#if vanillaMods}
    <p class="hint">{t("content.vanillaHint")}</p>
  {:else if shown.length === 0}
    <div class="empty">
      <p class="muted">{t("content.empty")}</p>
      <button class="primary" onclick={() => onAddContent(kind)}>{t("content.add")}</button>
    </div>
  {:else}
    <ul class="items">
      {#each shown as item (item.key)}
        {@const up = updates[item.key]}
        <li class:disabled={!item.enabled}>
          {#if item.iconUrl}
            <img class="icon" src={item.iconUrl} alt="" width="40" height="40" loading="lazy" />
          {:else}
            <span class="icon placeholder"><Icon name={item.kind === "mod" ? "box" : "file"} size={20} /></span>
          {/if}
          <div class="text">
            <span class="name">{item.name}</span>
            <span class="muted small">
              {item.version ?? item.fileName}
              <span class="badge">{sourceLabel(item)}</span>
            </span>
          </div>
          {#if up}
            <button class="update" onclick={() => update(up)} disabled={locked || busy.has(item.key)} title={t("content.update")}>
              <Icon name="download" size={15} />{busy.has(item.key) ? t("content.updating") : t("content.updateTo", { version: up.latest })}
            </button>
          {/if}
          <label class="switch" title={item.enabled ? t("content.disable") : t("content.enable")}>
            <input
              type="checkbox"
              checked={item.enabled}
              disabled={locked || busy.has(item.key)}
              aria-label={`${item.enabled ? t("content.disable") : t("content.enable")}: ${item.name}`}
              onchange={(e) => act(item.key, () => api.contentSetEnabled(profile.id, item.key, e.currentTarget.checked))}
            />
            <span class="slider"></span>
          </label>
          <button class="icon-btn" disabled={locked || busy.has(item.key)} onclick={() => (removing = item)} aria-label={`${t("content.remove")}: ${item.name}`}>
            <Icon name="trash" size={17} />
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</div>

{#if removing}
  {@const item = removing}
  <ConfirmDialog
    title={t("content.removeTitle")}
    confirmLabel={t("content.remove")}
    onCancel={() => (removing = null)}
    onConfirm={() => {
      removing = null;
      act(item.key, () => api.contentRemove(profile.id, item.key));
    }}
  >
    <p>{t("content.removeText", { name: item.name })}</p>
  </ConfirmDialog>
{/if}

<style>
  .content-tab {
    display: flex;
    flex-direction: column;
    gap: 12px;
    min-height: 0;
    flex: 1;
  }

  .toolbar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
  }

  .kinds {
    display: flex;
    gap: 4px;
    background: var(--surface-2);
    padding: 3px;
    border-radius: 10px;
  }

  .kinds button {
    border: none;
    background: transparent;
    color: var(--muted);
    border-radius: 8px;
    padding: 6px 12px;
    display: inline-flex;
    gap: 6px;
    align-items: center;
  }

  .kinds button.active {
    background: var(--surface);
    color: var(--text);
    box-shadow: 0 1px 3px rgb(0 0 0 / 0.25);
  }

  .count {
    font-size: 11px;
    padding: 0 6px;
    border-radius: 999px;
    background: var(--surface-3);
  }

  .buttons {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }

  .buttons button {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .search {
    flex-direction: row;
    align-items: center;
    gap: 6px;
    padding: 0 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg);
    color: var(--muted);
  }

  .search input {
    border: none;
    background: transparent;
    padding: 6px 0;
    width: 150px;
  }

  .search input:focus-visible {
    outline: none;
  }

  .notice {
    margin: 0;
    color: var(--accent);
    user-select: text;
  }

  .hint {
    margin: 0;
    color: var(--warning);
  }

  .small {
    font-size: 12px;
  }

  .empty {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 10px;
  }

  .items {
    list-style: none;
    margin: 0;
    padding: 0 4px 0 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
    overflow-y: auto;
    min-height: 0;
  }

  .items li {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 10px;
    border-radius: var(--radius);
    background: color-mix(in srgb, var(--surface) 92%, transparent);
    border: 1px solid var(--border);
  }

  .items li.disabled .icon,
  .items li.disabled .text {
    opacity: 0.45;
  }

  .icon {
    width: 40px;
    height: 40px;
    border-radius: 8px;
    flex-shrink: 0;
    object-fit: cover;
  }

  .placeholder {
    display: grid;
    place-items: center;
    background: var(--surface-3);
    color: var(--muted);
  }

  .text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .text > span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .name {
    font-weight: 600;
  }

  .update {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 4px 10px;
    font-size: 12px;
    color: var(--accent);
    border-color: color-mix(in srgb, var(--accent) 50%, var(--border));
  }

  .sync-strip {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px 10px;
  }

  .sync-strip > span {
    display: inline-flex;
    align-items: center;
    gap: 5px;
  }

  .sync-toggle {
    flex-direction: row;
    align-items: center;
    gap: 6px;
    padding: 3px 10px 3px 7px;
    border: 1px solid var(--border);
    border-radius: 999px;
    font-size: 13px;
    color: var(--text);
    cursor: pointer;
  }

  .sync-toggle.on {
    border-color: color-mix(in srgb, var(--accent) 60%, var(--border));
    background: color-mix(in srgb, var(--accent) 12%, transparent);
  }

  .sync-toggle input {
    width: auto;
  }

  .icon-btn {
    padding: 6px;
    border: none;
    background: transparent;
    color: var(--muted);
  }

  .icon-btn:hover:not(:disabled) {
    color: var(--danger);
  }

  /* Schalter statt Checkbox, wie in Modrinth/CurseForge. */
  .switch {
    position: relative;
    width: 38px;
    height: 22px;
    flex-shrink: 0;
  }

  .switch input {
    position: absolute;
    inset: 0;
    opacity: 0;
    margin: 0;
    cursor: pointer;
    z-index: 1;
  }

  .slider {
    position: absolute;
    inset: 0;
    border-radius: 999px;
    background: var(--surface-3);
    transition: background 0.15s;
  }

  .slider::after {
    content: "";
    position: absolute;
    top: 3px;
    left: 3px;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: #fff;
    transition: transform 0.15s;
  }

  .switch input:checked + .slider {
    background: var(--accent);
  }

  .switch input:checked + .slider::after {
    transform: translateX(16px);
  }

  .switch input:focus-visible + .slider {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .switch input:disabled + .slider {
    opacity: 0.5;
  }
</style>
