<script lang="ts">
  // Bibliothek nach docs/design/referenz/02-bibliothek-instanz.html: Liste links (280 px),
  // rechts die gewählte Instanz oder ein Sync-Pack. Instanzen stehen in Gruppen (F-50):
  // automatisch „TwoGG Events“ bzw. „Eigene Instanzen“ oder in einer eigenen Gruppe. Gruppen
  // lassen sich einklappen; Instanzen per Ziehen oder „In Gruppe verschieben …“ zuordnen.
  import type { Snippet } from "svelte";
  import { api, errorText } from "../../lib/api";
  import { t } from "../../lib/i18n.svelte";
  import { byName, byPlaytime, byRecent, isTwogg, versionLabel } from "../../lib/profiles";
  import { settings, updateSettings } from "../../lib/settings.svelte";
  import GroupDialog from "../library/GroupDialog.svelte";
  import { runOf } from "../../lib/runs.svelte";
  import { contextMenu, menu, type MenuEntry } from "../../lib/contextmenu.svelte";
  import type { Profile, SyncPack } from "../../lib/types";
  import Icon from "../Icon.svelte";
  import ProfileIcon from "../ProfileIcon.svelte";
  import SearchField from "../ui/SearchField.svelte";

  interface Props {
    profiles: Profile[];
    syncPacks: SyncPack[];
    /** Gewählte Instanz oder `sync:<id>`. */
    current: string | null;
    onSelect: (id: string) => void;
    onSelectSync: (id: string) => void;
    onCreate: () => void;
    onCreateSync: () => void;
    onImport: () => void;
    instanceMenu: (p: Profile) => MenuEntry[];
    /** Gruppe einer Instanz wurde geändert: Liste neu laden. */
    onProfilesChange: () => void;
    children: Snippet;
  }
  let { profiles, syncPacks, current, onSelect, onSelectSync, onCreate, onCreateSync, onImport, instanceMenu, onProfilesChange, children }: Props = $props();

  type Sort = "recent" | "name" | "playtime";
  interface Group {
    /** `auto:twogg`, `auto:own` oder `group:<name>`. */
    key: string;
    label: string;
    items: Profile[];
  }

  let filter = $state("");
  let error = $state<string | null>(null);
  let dialog = $state<{ kind: "move"; profile: Profile } | { kind: "rename"; group: string } | null>(null);

  const q = $derived(filter.trim().toLowerCase());
  const sort = $derived<Sort>(settings.current?.librarySort ?? "recent");
  const collapsed = $derived(new Set(settings.current?.collapsedGroups ?? []));
  const compare = $derived(sort === "name" ? byName : sort === "playtime" ? byPlaytime : byRecent);
  const sorted = $derived(profiles.filter((p) => !q || p.name.toLowerCase().includes(q)).sort(compare));
  const autoKey = (p: Profile) => (isTwogg(p) ? "auto:twogg" : "auto:own");
  const keyOf = (p: Profile) => (p.group ? `group:${p.group}` : autoKey(p));
  const groups = $derived.by(() => {
    const custom = [...new Set(profiles.map((p) => p.group).filter((g): g is string => !!g))].sort((a, b) => a.localeCompare(b));
    const list: Group[] = [
      { key: "auto:twogg", label: t("library.groupTwogg"), items: [] },
      ...custom.map((g) => ({ key: `group:${g}`, label: g, items: [] as Profile[] })),
      { key: "auto:own", label: t("library.groupOwn"), items: [] },
    ];
    const byKey = new Map(list.map((g) => [g.key, g]));
    for (const p of sorted) byKey.get(keyOf(p))?.items.push(p);
    // TwoGG nur mit Inhalt, eigene Gruppen nur ohne Filter auch leer (sonst verschwinden sie beim Suchen).
    return list.filter((g) => g.key === "auto:own" || g.items.length > 0 || (g.key.startsWith("group:") && !q));
  });
  const sortLabel = $derived({ recent: t("library.sortRecent"), name: t("library.sortName"), playtime: t("library.sortPlaytime") });
  /** Sortier-Menü am Symbol-Knopf; die aktive Sortierung trägt ein Häkchen. */
  function sortMenu(e: MouseEvent) {
    contextMenu(
      e,
      (["recent", "name", "playtime"] as const).map((v) => ({
        label: sortLabel[v],
        icon: v === sort ? ("check" as const) : undefined,
        action: () => updateSettings((s) => (s.librarySort = v)).catch((err) => (error = errorText(err))),
      })),
    );
  }
  const syncs = $derived(syncPacks.filter((s) => !q || s.name.toLowerCase().includes(q)));

  async function toggleGroup(key: string) {
    await updateSettings((s) => {
      const set = new Set(s.collapsedGroups);
      if (!set.delete(key)) set.add(key);
      s.collapsedGroups = [...set];
    }).catch((e) => (error = errorText(e)));
  }

  async function moveTo(p: Profile, key: string) {
    if (key === keyOf(p)) return;
    // Automatische Gruppen nehmen nur Instanzen der passenden Art auf.
    if (key.startsWith("auto:") && key !== autoKey(p)) return;
    error = null;
    try {
      await api.setProfileGroup(p.id, key.startsWith("group:") ? key.slice(6) : null);
      onProfilesChange();
    } catch (e) {
      error = errorText(e);
    }
  }

  async function dissolve(group: string) {
    error = null;
    try {
      for (const p of profiles.filter((p) => p.group === group)) await api.setProfileGroup(p.id, null);
      onProfilesChange();
    } catch (e) {
      error = errorText(e);
    }
  }

  // Ziehen mit Zeiger-Ereignissen (HTML-Drag&Drop kollidiert mit dem Datei-Drop von Tauri).
  let drag = $state<{ profile: Profile; x: number; y: number; over: string | null } | null>(null);
  let pressed: { profile: Profile; x: number; y: number; id: number } | null = null;
  let suppressClick = false;

  function groupAt(x: number, y: number): string | null {
    return (document.elementFromPoint(x, y)?.closest("[data-group]") as HTMLElement | null)?.dataset.group ?? null;
  }

  function onPointerDown(e: PointerEvent, p: Profile) {
    if (e.button !== 0) return;
    pressed = { profile: p, x: e.clientX, y: e.clientY, id: e.pointerId };
  }

  function onPointerMove(e: PointerEvent) {
    if (!pressed || e.pointerId !== pressed.id) return;
    if (!drag && Math.hypot(e.clientX - pressed.x, e.clientY - pressed.y) < 6) return;
    drag = { profile: pressed.profile, x: e.clientX, y: e.clientY, over: groupAt(e.clientX, e.clientY) };
  }

  function onPointerUp(e: PointerEvent) {
    if (drag) {
      const { profile, over } = drag;
      drag = null;
      suppressClick = true;
      setTimeout(() => (suppressClick = false));
      if (over) moveTo(profile, over);
    }
    pressed = null;
  }

  const canDrop = (key: string) => !!drag && key !== keyOf(drag.profile) && (!key.startsWith("auto:") || key === autoKey(drag.profile));
</script>

<svelte:window onpointermove={onPointerMove} onpointerup={onPointerUp} onpointercancel={() => ((drag = null), (pressed = null))} />

{#snippet item(p: Profile)}
  <button
    class="item"
    class:active={current === p.id}
    class:dragging={drag?.profile.id === p.id}
    onclick={() => !suppressClick && onSelect(p.id)}
    onpointerdown={(e) => onPointerDown(e, p)}
    aria-current={current === p.id ? "true" : undefined}
    use:menu={() => [...instanceMenu(p), "-", { label: t("groups.moveMenu"), icon: "layers", action: () => (dialog = { kind: "move", profile: p }) }]}
  >
    <ProfileIcon profile={p} size={36} />
    <span class="text">
      <span class="name">{p.name}</span>
      <span class="version mono">{versionLabel(p)}</span>
    </span>
    {#if runOf(p.id).status === "running"}<span class="live" title={t("play.running")}></span>{/if}
  </button>
{/snippet}

<div class="library">
  <section class="side" aria-label={t("tab.library")}>
    <div class="head">
      <h2>{t("tab.library")}</h2>
      <button class="add" aria-label={t("library.newInstance")} title={t("library.newInstance")} onclick={onCreate}>
        <Icon name="plus" size={16} stroke={2.6} />
      </button>
    </div>
    <div class="tools">
      <SearchField bind:value={filter} placeholder={t("library.filter")} height={36} />
      <button class="sort-btn" aria-label="{t('library.sort')}: {sortLabel[sort]}" title="{t('library.sort')}: {sortLabel[sort]}" onclick={sortMenu}>
        <Icon name="sort" size={18} />
      </button>
    </div>
    {#if error}<p class="error">{error}</p>{/if}

    <div class="groups">
      {#each groups as g (g.key)}
        {@const isCollapsed = collapsed.has(g.key) && !q}
        <div class="group" data-group={g.key} class:drop={drag?.over === g.key && canDrop(g.key)}>
          <span
            class="label head-row"
            use:menu={() =>
              g.key.startsWith("group:")
                ? [
                    { label: t("groups.renameMenu"), icon: "edit", action: () => (dialog = { kind: "rename", group: g.label }) },
                    { label: t("groups.dissolve"), icon: "trash", action: () => dissolve(g.label) },
                  ]
                : []}
          >
            <button class="ghost collapse" aria-expanded={!isCollapsed} onclick={() => toggleGroup(g.key)}>
              <span class="chev" class:closed={isCollapsed} aria-hidden="true">▾</span>
              {g.label}
              <span class="count">{g.items.length}</span>
            </button>
          </span>
          {#if !isCollapsed}
            {#each g.items as p (p.id)}{@render item(p)}{:else}
              <p class="empty">{g.key === "auto:own" ? (q ? t("library.noMatch") : t("library.noOwn")) : t("groups.empty")}</p>
            {/each}
          {/if}
        </div>
      {/each}
      <div class="group">
        <span class="label with-action">
          {t("library.groupSync")}
          <button class="ghost tiny" aria-label={t("sync.new")} title={t("sync.new")} onclick={onCreateSync}><Icon name="plus" size={14} /></button>
        </span>
        {#each syncs as s (s.id)}
          <button class="item" class:active={current === `sync:${s.id}`} onclick={() => onSelectSync(s.id)}>
            <span class="sync-icon"><Icon name="layers" size={18} /></span>
            <span class="text">
              <span class="name">{s.name}</span>
              <span class="version mono">{s.mode === "template" ? t("sync.modeTemplate") : t("sync.modeSync")} · {t("sync.entries", { count: s.entries.length })}</span>
            </span>
          </button>
        {:else}
          <p class="empty">{t("library.noSync")}</p>
        {/each}
      </div>
    </div>

    <button class="dashed import" onclick={onImport}><Icon name="download" size={16} />{t("library.import")}</button>
  </section>

  <main class="detail">
    {@render children()}
  </main>
</div>

{#if drag}
  <div class="ghost-drag" style:left="{drag.x + 12}px" style:top="{drag.y + 8}px" aria-hidden="true">
    <ProfileIcon profile={drag.profile} size={24} />{drag.profile.name}
  </div>
{/if}

{#if dialog}
  <GroupDialog
    mode={dialog}
    {profiles}
    onClose={() => (dialog = null)}
    onDone={() => {
      dialog = null;
      onProfilesChange();
    }}
  />
{/if}

<style>
  .library {
    display: grid;
    grid-template-columns: 280px minmax(0, 1fr);
    height: 100%;
    min-height: 0;
  }

  .side {
    background: var(--side);
    border-right: 1px solid var(--line);
    padding: 18px 14px;
    display: flex;
    flex-direction: column;
    gap: 16px;
    min-height: 0;
  }

  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .head h2 {
    font-weight: 600;
    font-size: 22px;
  }

  .add {
    width: 34px;
    min-height: 34px;
    height: 34px;
    padding: 0;
    background: var(--accent);
    color: var(--ink);
    box-shadow: var(--edge-accent);
    clip-path: polygon(6px 0, 100% 0, 100% calc(100% - 6px), calc(100% - 6px) 100%, 0 100%, 0 6px);
  }

  .add:hover:not(:disabled) {
    background: var(--accent-hover);
  }

  .groups {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 16px;
    margin-right: -6px;
    padding-right: 6px;
  }

  .group {
    display: flex;
    flex-direction: column;
    gap: 2px;
    outline: 1px dashed transparent;
    outline-offset: 2px;
  }

  .group.drop {
    outline-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 6%, transparent);
  }

  .tools {
    display: flex;
    gap: 6px;
  }

  .tools > :global(:first-child) {
    flex: 1;
    min-width: 0;
  }

  .sort-btn {
    width: 36px;
    height: 36px;
    min-height: 36px;
    padding: 0;
    flex: none;
    background: var(--surface);
    color: var(--text-soft);
  }

  .head-row {
    padding: 0 0 4px;
  }

  .collapse {
    width: 100%;
    min-height: 26px;
    justify-content: flex-start;
    gap: 6px;
    padding: 0 6px;
    font: inherit;
    letter-spacing: inherit;
    text-transform: inherit;
    color: inherit;
  }

  .collapse:hover:not(:disabled) {
    color: var(--text-soft);
    background: transparent;
  }

  .chev {
    display: inline-block;
    transition: transform 0.12s;
  }

  .chev.closed {
    transform: rotate(-90deg);
  }

  .count {
    margin-left: auto;
    color: var(--text-dim);
  }

  .item.dragging {
    opacity: 0.45;
  }

  .ghost-drag {
    position: fixed;
    z-index: 50;
    pointer-events: none;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 10px;
    font-size: 13px;
    font-weight: 600;
    background: var(--surface-active);
    box-shadow: 0 6px 18px rgb(0 0 0 / 0.35);
  }

  .error {
    margin: 0;
    font-size: 12px;
  }

  .label {
    font-family: var(--font-mono);
    font-size: 11px;
    letter-spacing: 1px;
    text-transform: uppercase;
    color: var(--text-dim);
    padding: 0 6px 6px;
  }

  .with-action {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding-bottom: 2px;
  }

  .tiny {
    width: 24px;
    min-height: 24px;
    padding: 0;
  }

  .item {
    position: relative;
    justify-content: flex-start;
    gap: 10px;
    padding: 8px;
    min-height: 0;
    background: transparent;
    text-align: left;
  }

  .item:hover:not(:disabled) {
    background: var(--surface);
  }

  .item.active {
    background: var(--surface-active);
    clip-path: polygon(7px 0, 100% 0, 100% 100%, 0 100%, 0 7px);
  }

  .text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .name {
    font-size: 14px;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .active .name {
    font-weight: 700;
    color: var(--accent-hover);
  }

  .version {
    font-size: 11px;
    font-weight: 400;
    color: var(--text-muted);
  }

  .sync-icon {
    width: 36px;
    height: 36px;
    flex: none;
    display: grid;
    place-items: center;
    background: var(--surface-2);
    color: var(--text-soft);
  }

  .live {
    width: 8px;
    height: 8px;
    background: var(--nadel);
  }

  .empty {
    margin: 0;
    padding: 4px 8px;
    font-size: 12px;
    color: var(--text-dim);
  }

  .import {
    height: 40px;
  }

  .detail {
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

</style>
