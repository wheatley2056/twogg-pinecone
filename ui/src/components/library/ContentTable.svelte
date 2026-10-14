<script lang="ts">
  // Tab „Inhalt“ (Design-System 9.11): Suche, Filter, Updates, Tabelle Name · Status · Quelle · Aktiv.
  // Inhalte aus einem TwoGG-Event-Pack sind gesperrt (vom Event vorgegeben).
  import { onMount, untrack } from "svelte";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { menu, type MenuEntry } from "../../lib/contextmenu.svelte";
  import { api, errorText } from "../../lib/api";
  import { t } from "../../lib/i18n.svelte";
  import { syncReportText } from "../../lib/sync";
  import { isTwogg } from "../../lib/profiles";
  import type { ContentKind, InstalledContent, Profile, ProfileSyncPack, UpdateInfo } from "../../lib/types";
  import ConfirmDialog from "../ConfirmDialog.svelte";
  import AddContentDialog from "./AddContentDialog.svelte";
  import Icon from "../Icon.svelte";
  import Segmented from "../ui/Segmented.svelte";
  import SearchField from "../ui/SearchField.svelte";
  import Switch from "../ui/Switch.svelte";

  interface Props {
    profile: Profile;
    /** Spiel läuft: Dateien nicht anfassen. */
    locked: boolean;
    onAddContent: (kind: ContentKind) => void;
    onCount: (count: number) => void;
    /** Zählt hoch, wenn der Hinzufügen-Dialog aufgehen soll (Rechtsklick auf eine Instanz). */
    addSignal?: number;
  }
  let { profile, locked, onAddContent, onCount, addSignal = 0 }: Props = $props();

  type Filter = "all" | ContentKind;
  let filter = $state<Filter>("all");
  let query = $state("");
  let items = $state<InstalledContent[]>([]);
  let loaded = $state(false);
  let updates = $state<Record<string, UpdateInfo>>({});
  let checked = $state(false);
  let checking = $state(false);
  let busy = $state<Set<string>>(new Set());
  let error = $state<string | null>(null);
  let message = $state<string | null>(null);
  let removing = $state<InstalledContent | null>(null);
  let syncPacks = $state<ProfileSyncPack[]>([]);
  let syncBusy = $state<string | null>(null);
  let adding = $state<ContentKind | null>(null);
  let dragging = $state(false);
  let seenSignal = untrack(() => addSignal);

  $effect(() => {
    if (addSignal !== seenSignal) {
      seenSignal = addSignal;
      if (!locked) adding = "mod";
    }
  });

  const eventPack = $derived(isTwogg(profile));
  const syncNames = $derived(Object.fromEntries(syncPacks.map((s) => [s.pack.id, s.pack.name])));
  const shown = $derived(
    items
      .filter((i) => filter === "all" || i.kind === filter)
      .filter((i) => !query.trim() || i.name.toLowerCase().includes(query.trim().toLowerCase()))
      // Vorgegebene Inhalte oben, dann alphabetisch.
      .sort((a, b) => Number(isLocked(b)) - Number(isLocked(a)) || a.name.localeCompare(b.name)),
  );
  const pending = $derived(Object.values(updates));
  const filters = $derived<{ id: Filter; label: string }[]>([
    { id: "all", label: t("content.all") },
    { id: "mod", label: t("content.mod") },
    { id: "resourcepack", label: t("content.resources") },
    { id: "shader", label: t("content.shader") },
  ]);

  function isLocked(i: InstalledContent) {
    return !!i.locked || (eventPack && i.fromPack);
  }

  async function load() {
    try {
      items = await api.contentList(profile.id);
      onCount(items.length);
      loaded = true;
      if (items.some((i) => i.unidentified)) {
        items = await api.contentIdentify(profile.id);
      }
    } catch (e) {
      error = errorText(e);
    }
  }

  async function checkUpdates(quiet = false) {
    checking = true;
    try {
      const list = await api.contentUpdates(profile.id);
      updates = Object.fromEntries(list.map((u) => [u.key, u]));
      checked = true;
      if (!quiet && list.length === 0) message = t("content.upToDate");
    } catch (e) {
      if (!quiet) error = errorText(e);
    } finally {
      checking = false;
    }
  }

  // Dateien ins Fenster ziehen: Mods (.jar), Ressourcenpakete und Shader (.zip) landen direkt hier.
  onMount(() => {
    let stop: (() => void) | undefined;
    let gone = false;
    api
      .onFileDrop(async (e) => {
        if (locked || eventPack) return;
        if (e.type === "over") dragging = true;
        else if (e.type === "leave") dragging = false;
        else {
          dragging = false;
          await addDropped(e.paths);
        }
      })
      .then((u) => (gone ? u() : (stop = u)))
      .catch(() => {});
    return () => {
      gone = true;
      stop?.();
    };
  });

  async function addDropped(paths: string[]) {
    error = message = null;
    const names: string[] = [];
    for (const path of paths) {
      try {
        const key = await api.contentAddDropped(profile.id, path);
        names.push(key.split("/").pop() ?? key);
      } catch (e) {
        error = errorText(e);
      }
    }
    if (names.length) {
      message = t("content.installedMsg", { names: names.join(", ") });
      await load();
    }
  }

  onMount(async () => {
    await load();
    syncPacks = await api.syncpackForProfile(profile.id).catch(() => []);
    // Status „Aktuell“ braucht einen Abgleich; leise im Hintergrund.
    if (items.length > 0) checkUpdates(true);
  });

  async function act(key: string, action: () => Promise<unknown>) {
    error = message = null;
    busy = new Set([...busy, key]);
    try {
      await action();
      items = await api.contentList(profile.id);
      onCount(items.length);
    } catch (e) {
      error = errorText(e);
    } finally {
      busy = new Set([...busy].filter((k) => k !== key));
    }
  }

  async function update(u: UpdateInfo) {
    await act(u.key, async () => {
      await api.contentUpdate(profile.id, u);
      const { [u.key]: _, ...rest } = updates;
      updates = rest;
    });
  }

  async function updateAll() {
    for (const u of pending) await update(u);
  }

  async function toggleSync(s: ProfileSyncPack) {
    error = message = null;
    syncBusy = s.pack.id;
    try {
      const r = await api.syncpackSetEnabled(profile.id, s.pack.id, s.pack.mode === "template" || !s.enabled);
      message = `${s.pack.name}: ${syncReportText(r)}`;
      syncPacks = await api.syncpackForProfile(profile.id);
      await load();
    } catch (e) {
      error = errorText(e);
    } finally {
      syncBusy = null;
    }
  }

  function rowMenu(item: InstalledContent): MenuEntry[] {
    const fixed = isLocked(item);
    const up = updates[item.key];
    const project =
      item.source.type === "modrinth"
        ? `https://modrinth.com/project/${item.source.project_id}`
        : item.source.type === "curseforge"
          ? `https://www.curseforge.com/projects/${item.source.project_id}`
          : null;
    return [
      !fixed && {
        label: item.enabled ? t("content.disable") : t("content.enable"),
        icon: item.enabled ? ("close" as const) : ("check" as const),
        disabled: locked || busy.has(item.key),
        action: () => act(item.key, () => api.contentSetEnabled(profile.id, item.key, !item.enabled)),
      },
      up && { label: t("content.update"), icon: "arrow-up" as const, disabled: locked || busy.has(item.key), action: () => update(up) },
      !!project && {
        label: t("menu.openSource", { source: item.source.type === "modrinth" ? "Modrinth" : "CurseForge" }),
        icon: "external" as const,
        action: () => openUrl(project).catch(() => window.open(project, "_blank")),
      },
      { label: t("profile.openFolder"), icon: "folder" as const, action: () => api.openProfileFolder(profile.id) },
      !fixed && "-",
      !fixed && { label: t("content.remove"), icon: "trash" as const, danger: true, disabled: locked, action: () => (removing = item) },
    ];
  }

  function source(i: InstalledContent): string {
    if (isLocked(i) && i.source.type === "local") return "TwoGG";
    if (i.sync) return syncNames[i.sync] ?? t("content.sync");
    return i.source.type === "modrinth" ? "Modrinth" : i.source.type === "curseforge" ? "CurseForge" : t("content.local");
  }

  function subtitle(i: InstalledContent): string {
    if (isLocked(i)) return t("content.byEvent");
    if (i.fromPack) return t("content.fromPackShort");
    if (i.sync) return t("content.fromSync", { name: syncNames[i.sync] ?? i.sync });
    return i.version ?? i.fileName;
  }
</script>

<div class="content" class:dragging>
  {#if dragging}
    <div class="drop" aria-hidden="true">
      <Icon name="download" size={28} />
      <span>{t("content.dropHere")}</span>
      <span class="drop-sub">{t("content.dropKinds")}</span>
    </div>
  {/if}
  {#if syncPacks.length > 0}
    <div class="sync">
      <span class="section-label">{t("library.groupSync")}</span>
      {#each syncPacks as s (s.pack.id)}
        <span class="sync-item">
          {#if s.pack.mode === "template"}
            <button class="small" disabled={locked || syncBusy !== null} title={t("sync.templateApply")} onclick={() => toggleSync(s)}>{t("sync.applyOnce")}</button>
          {:else}
            <Switch checked={s.enabled} label={s.pack.name} disabled={locked || syncBusy !== null} onchange={() => toggleSync(s)} />
          {/if}
          {s.pack.name}
        </span>
      {/each}
    </div>
  {/if}

  <div class="toolbar">
    <SearchField bind:value={query} placeholder={t("content.search")} width="240px" />
    <Segmented items={filters} value={filter} label={t("content.filterLabel")} variant="soft" onchange={(id) => (filter = id)} />
    <span class="spacer"></span>
    {#if pending.length > 0}
      <button class="small" onclick={updateAll} disabled={locked || busy.size > 0}>
        <Icon name="arrow-up" size={14} stroke={2.2} />{t("content.updateAll", { count: pending.length })}
      </button>
    {:else}
      <button class="small" onclick={() => checkUpdates()} disabled={checking || items.length === 0}>
        {checking ? t("content.checking") : t("content.checkUpdates")}
      </button>
    {/if}
    <button class="small primary" onclick={() => (adding = filter === "all" ? "mod" : filter)} disabled={locked}>
      <Icon name="plus" size={14} stroke={2.6} />{t("content.addContent")}
    </button>
  </div>

  {#if error}<p class="error">{error}</p>{/if}
  {#if message}<p class="message">{message}</p>{/if}

  <div class="table" role="table" aria-label={t("profile.tabContent")}>
    <div class="row head" role="row">
      <span role="columnheader">{t("content.colName")}</span>
      <span role="columnheader">{t("content.colStatus")}</span>
      <span role="columnheader">{t("content.colSource")}</span>
      <span role="columnheader" class="right">{t("content.colActive")}</span>
    </div>
    <div class="rows">
      {#each shown as item (item.key)}
        {@const up = updates[item.key]}
        <div class="row" class:locked={isLocked(item)} class:off={!item.enabled} role="row" use:menu={() => rowMenu(item)}>
          <span class="name-cell" role="cell">
            {#if item.iconUrl}
              <img class="icon" src={item.iconUrl} alt="" width="34" height="34" loading="lazy" />
            {:else}
              <span class="icon letter">{[...item.name][0]?.toUpperCase() ?? "?"}</span>
            {/if}
            <span class="text">
              <span class="name">{item.name}</span>
              <span class="sub">{subtitle(item)}</span>
            </span>
            {#if !isLocked(item)}
              <button class="ghost remove" disabled={locked || busy.has(item.key)} onclick={() => (removing = item)}
                aria-label={t("content.removeNamed", { name: item.name })} title={t("content.remove")}>
                <Icon name="trash" size={15} />
              </button>
            {/if}
          </span>
          <span role="cell">
            {#if up}
              <button class="status update" disabled={locked || busy.has(item.key)} onclick={() => update(up)} title={up.latest}>
                <Icon name="arrow-up" size={14} stroke={2.2} />{busy.has(item.key) ? t("content.updating") : t("content.updateAvailable")}
              </button>
            {:else if checked}
              <span class="status ok">{t("content.current")}</span>
            {:else}
              <span class="status dim">{checking ? t("content.checking") : "—"}</span>
            {/if}
          </span>
          <span class="source mono" class:twogg={source(item) === "TwoGG"} role="cell">{source(item)}</span>
          <span class="right" role="cell">
            {#if isLocked(item)}
              <span class="lock" title={t("content.lockedHint")} aria-label={t("content.lockedHint")}><Icon name="lock" size={18} /></span>
            {:else}
              <Switch
                checked={item.enabled}
                label={`${item.enabled ? t("content.disable") : t("content.enable")}: ${item.name}`}
                disabled={locked || busy.has(item.key)}
                onchange={(on) => act(item.key, () => api.contentSetEnabled(profile.id, item.key, on))}
              />
            {/if}
          </span>
        </div>
      {:else}
        <p class="empty">{loaded ? (items.length ? t("library.noMatch") : t("content.empty")) : t("common.loading")}</p>
      {/each}
    </div>
  </div>
</div>

{#if adding}
  <AddContentDialog
    {profile}
    kind={adding}
    installed={items}
    onClose={() => (adding = null)}
    onAdded={load}
    onDiscover={onAddContent}
  />
{/if}

{#if removing}
  {@const item = removing}
  <ConfirmDialog
    title={t("content.removeTitle")}
    confirmLabel={t("content.remove")}
    onCancel={() => (removing = null)}
    onConfirm={() => {
      const key = item.key;
      removing = null;
      act(key, () => api.contentRemove(profile.id, key));
    }}
  >
    <p>{t("content.removeText", { name: item.name })}</p>
  </ConfirmDialog>
{/if}

<style>
  .content {
    position: relative;
  }

  .drop {
    position: absolute;
    inset: 0;
    z-index: 5;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    background: var(--bar-glass);
    outline: 2px dashed var(--accent);
    outline-offset: -8px;
    color: var(--accent);
    font-family: var(--font-pixel);
    font-size: 20px;
    pointer-events: none;
  }

  .drop-sub {
    font-family: var(--font-ui);
    font-size: 13px;
    color: var(--text-muted);
  }

  .content {
    flex: 1;
    min-height: 0;
    padding: 14px 28px 0;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .sync {
    display: flex;
    align-items: center;
    gap: 16px;
    flex-wrap: wrap;
    font-size: 13px;
  }

  .sync-item {
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }

  .toolbar {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
  }

  .spacer {
    flex: 1;
  }

  .small {
    min-height: 34px;
    padding: 0 14px;
    font-size: 13px;
  }

  .message {
    margin: 0;
    font-size: 13px;
    color: var(--nadel);
  }

  .table {
    display: flex;
    flex-direction: column;
    min-height: 0;
    flex: 1;
  }

  .rows {
    overflow-y: auto;
    min-height: 0;
    padding-bottom: 14px;
  }

  .row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 170px 120px 60px;
    gap: 16px;
    padding: 9px 14px;
    align-items: center;
    border-bottom: 1px solid var(--line-row);
  }

  .row.head {
    padding: 8px 14px;
    font-family: var(--font-mono);
    font-size: 11px;
    letter-spacing: 1px;
    text-transform: uppercase;
    color: var(--text-dim);
    border-bottom-color: var(--line);
  }

  .row.locked {
    background: var(--surface-locked);
  }

  .row.off .name,
  .row.off .icon {
    opacity: 0.55;
  }

  .name-cell {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
  }

  .icon {
    width: 34px;
    height: 34px;
    flex: none;
    object-fit: cover;
  }

  .letter {
    display: grid;
    place-items: center;
    background: var(--surface-2);
    font-family: var(--font-pixel);
    font-weight: 700;
    color: var(--text-soft);
  }

  .text {
    min-width: 0;
    flex: 1;
    display: flex;
    flex-direction: column;
  }

  .name {
    font-weight: 700;
    font-size: 14px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .sub {
    font-size: 12px;
    color: var(--text-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .remove {
    width: 30px;
    min-height: 30px;
    padding: 0;
    opacity: 0;
  }

  .row:hover .remove,
  .remove:focus-visible {
    opacity: 1;
  }

  .status {
    font-size: 13px;
  }

  .status.ok {
    color: var(--nadel);
  }

  .status.dim {
    color: var(--text-dim);
  }

  .status.update {
    min-height: 0;
    padding: 0;
    gap: 6px;
    background: transparent;
    font-weight: 400;
    color: var(--accent-fg);
  }

  .status.update:hover:not(:disabled) {
    background: transparent;
    color: var(--accent-hover);
  }

  .source {
    font-size: 12px;
    color: var(--text-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .source.twogg {
    color: var(--accent-fg);
  }

  .right {
    display: flex;
    justify-content: flex-end;
  }

  .lock {
    display: flex;
    color: var(--text-muted);
  }

  .empty {
    margin: 16px 14px;
    color: var(--text-dim);
  }
</style>
