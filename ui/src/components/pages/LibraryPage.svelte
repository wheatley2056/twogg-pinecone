<script lang="ts">
  // Bibliothek nach docs/design/referenz/02-bibliothek-instanz.html: Liste links (280 px),
  // rechts die gewählte Instanz oder ein Sync-Pack.
  import type { Snippet } from "svelte";
  import { t } from "../../lib/i18n.svelte";
  import { byRecent, isTwogg, versionLabel } from "../../lib/profiles";
  import { runOf } from "../../lib/runs.svelte";
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
    children: Snippet;
  }
  let { profiles, syncPacks, current, onSelect, onSelectSync, onCreate, onCreateSync, onImport, children }: Props = $props();

  let filter = $state("");

  const q = $derived(filter.trim().toLowerCase());
  const sorted = $derived(profiles.filter((p) => !q || p.name.toLowerCase().includes(q)).sort(byRecent));
  const twogg = $derived(sorted.filter(isTwogg));
  const own = $derived(sorted.filter((p) => !isTwogg(p)));
  const syncs = $derived(syncPacks.filter((s) => !q || s.name.toLowerCase().includes(q)));
</script>

{#snippet item(p: Profile)}
  <button class="item" class:active={current === p.id} onclick={() => onSelect(p.id)} aria-current={current === p.id ? "true" : undefined}>
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
    <SearchField bind:value={filter} placeholder={t("library.filter")} height={36} />

    <div class="groups">
      {#if twogg.length > 0}
        <div class="group">
          <span class="label">{t("library.groupTwogg")}</span>
          {#each twogg as p (p.id)}{@render item(p)}{/each}
        </div>
      {/if}
      <div class="group">
        <span class="label">{t("library.groupOwn")}</span>
        {#each own as p (p.id)}{@render item(p)}{:else}<p class="empty">{q ? t("library.noMatch") : t("library.noOwn")}</p>{/each}
      </div>
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
