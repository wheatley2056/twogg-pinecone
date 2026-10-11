<script lang="ts" module>
  export type InstanceTab = "content" | "worlds" | "servers" | "screenshots" | "logs" | "settings";
</script>

<script lang="ts">
  // Instanz-Detail nach docs/design/referenz/02-bibliothek-instanz.html: Header mit Schuppenmuster,
  // Meta-Chips und Aktionen, darunter die Unter-Tabs. Gespielt wird über die Spielleiste.
  import { onMount } from "svelte";
  import { api, errorText } from "../../lib/api";
  import { t } from "../../lib/i18n.svelte";
  import { isTwogg, versionLabel } from "../../lib/profiles";
  import { runOf } from "../../lib/runs.svelte";
  import type { ContentKind, ImportResult, PackInfo, PackUpdate, Profile } from "../../lib/types";
  import Icon from "../Icon.svelte";
  import MigrateDialog from "../MigrateDialog.svelte";
  import PackUpdateDialog from "../PackUpdateDialog.svelte";
  import ProfileIcon from "../ProfileIcon.svelte";
  import ShareDialog from "../ShareDialog.svelte";
  import ContentTable from "./ContentTable.svelte";
  import LogsTab from "./LogsTab.svelte";
  import ScreenshotsTab from "./ScreenshotsTab.svelte";
  import ServersTab from "./ServersTab.svelte";
  import SettingsTab from "./SettingsTab.svelte";
  import WorldsTab from "./WorldsTab.svelte";

  interface Props {
    profile: Profile;
    tab: InstanceTab;
    onChanged: () => void;
    onDeleted: () => void;
    onAddContent: (kind: ContentKind) => void;
    onCreated: (result: ImportResult) => void;
    onJoin: (address: string) => void;
  }
  let { profile, tab = $bindable(), onChanged, onDeleted, onAddContent, onCreated, onJoin }: Props = $props();

  const run = $derived(runOf(profile.id));
  const locked = $derived(run.status !== "idle");
  let packInfo = $state<PackInfo | null>(null);
  let packUpdate = $state<PackUpdate | null | undefined>(undefined);
  let contentCount = $state<number | null>(null);
  let contentKey = $state(0);
  let menuOpen = $state(false);
  let menuEl: HTMLElement | undefined = $state();
  let error = $state<string | null>(null);
  let sharing = $state(false);
  let migrating = $state(false);
  // null: geschlossen; target null: nur gesicherte Stände.
  let updateDialog = $state<{ target: { version: string | null; file: string | null } | null } | null>(null);

  const label = $derived(
    isTwogg(profile)
      ? t("instance.kindEvent")
      : packInfo && packInfo.source !== "file"
        ? t("instance.kindPack", { source: packInfo.source === "modrinth" ? "Modrinth" : packInfo.source === "curseforge" ? "CurseForge" : "TwoGG" })
        : t("instance.kindOwn"),
  );

  onMount(async () => {
    packInfo = await api.packInfo(profile.id).catch(() => null);
    if (packInfo && packInfo.source !== "file") {
      packUpdate = await api.packCheckUpdate(profile.id).catch(() => undefined);
    }
  });

  async function menuAction(action: () => unknown) {
    menuOpen = false;
    error = null;
    try {
      await action();
    } catch (e) {
      error = errorText(e);
    }
  }

  const tabs = $derived<{ id: InstanceTab; label: string; count?: number | null }[]>([
    { id: "content", label: t("instance.tabContent"), count: contentCount },
    { id: "worlds", label: t("instance.tabWorlds") },
    { id: "servers", label: t("instance.tabServers") },
    { id: "screenshots", label: t("instance.tabScreenshots") },
    { id: "logs", label: t("instance.tabLogs") },
    { id: "settings", label: t("instance.tabSettings") },
  ]);
</script>

<svelte:window onclick={(e) => menuOpen && menuEl && !menuEl.contains(e.target as Node) && (menuOpen = false)} />

<header class="header">
  <span class="scales-right scales" aria-hidden="true"></span>
  <span class="big-icon"><ProfileIcon {profile} size={96} cut={16} /></span>
  <div class="title">
    <span class="kind" class:event={isTwogg(profile)}>{label}</span>
    <h1>{profile.name}</h1>
    <div class="chips">
      <span class="chip">{versionLabel(profile)}</span>
      <span class="chip">{t("instance.ram", { gb: ((profile.memoryMb ?? 4096) / 1024).toFixed(1).replace(".0", "") })}</span>
      {#if packInfo && packInfo.source !== "file"}
        {#if packUpdate}
          <button class="chip update" disabled={locked} onclick={() => (updateDialog = { target: { version: packUpdate!.versionId, file: null } })}>
            <Icon name="arrow-up" size={13} stroke={2.2} />{t("instance.packUpdate", { version: packUpdate.latest })}
          </button>
        {:else if packUpdate === null}
          <span class="chip ok">{isTwogg(profile) ? t("instance.syncedEvent") : t("instance.syncedPack", { name: packInfo.name })}</span>
        {:else}
          <span class="chip">{packInfo.name} {packInfo.version}</span>
        {/if}
      {/if}
      {#if run.status === "running"}<span class="chip ok">{t("play.running")}</span>{/if}
    </div>
  </div>
  <div class="actions">
    <button class="icon" aria-label={t("profile.openFolder")} title={t("profile.openFolder")} onclick={() => menuAction(() => api.openProfileFolder(profile.id))}>
      <Icon name="folder" size={19} />
    </button>
    <button class="export" onclick={() => (sharing = true)}>{t("instance.export")}</button>
    <div class="menu" bind:this={menuEl}>
      <button class="icon" aria-label={t("profile.more")} title={t("profile.more")} aria-expanded={menuOpen} aria-haspopup="menu" onclick={() => (menuOpen = !menuOpen)}>
        <Icon name="more" size={19} />
      </button>
      {#if menuOpen}
        <div class="menu-pop" role="menu">
          <button role="menuitem" onclick={() => menuAction(() => (sharing = true))}><Icon name="share" size={16} />{t("share.menu")}</button>
          <button role="menuitem" disabled={locked} onclick={() => menuAction(() => (migrating = true))}><Icon name="refresh" size={16} />{t("migrate.menu")}</button>
          {#if packInfo}
            <button role="menuitem" onclick={() => menuAction(() => (updateDialog = { target: null }))}><Icon name="clock" size={16} />{t("update.snapshots")}</button>
          {/if}
          <button role="menuitem" onclick={() => menuAction(() => (tab = "settings"))}><Icon name="settings" size={16} />{t("instance.tabSettings")}</button>
        </div>
      {/if}
    </div>
  </div>
</header>

<div class="tabs" role="tablist" aria-label={t("instance.areas")}>
  {#each tabs as item (item.id)}
    <button role="tab" aria-selected={tab === item.id} class:active={tab === item.id} onclick={() => (tab = item.id)}>
      {item.label}
      {#if item.count != null}<span class="count mono">{item.count}</span>{/if}
    </button>
  {/each}
</div>

{#if error}<p class="error top">{error}</p>{/if}

{#if tab === "content"}
  {#key contentKey}
    <ContentTable {profile} {locked} {onAddContent} onCount={(n) => (contentCount = n)} />
  {/key}
{:else if tab === "worlds"}
  <WorldsTab {profile} {locked} />
{:else if tab === "servers"}
  <ServersTab {profile} {onJoin} />
{:else if tab === "screenshots"}
  <ScreenshotsTab {profile} />
{:else if tab === "logs"}
  <LogsTab {profile} />
{:else}
  <SettingsTab {profile} {locked} {onChanged} {onDeleted} />
{/if}

{#if sharing}<ShareDialog {profile} onClose={() => (sharing = false)} />{/if}

{#if migrating}
  <MigrateDialog {profile} onClose={() => (migrating = false)} {onCreated} />
{/if}

{#if updateDialog}
  <PackUpdateDialog
    profileId={profile.id}
    target={updateDialog.target}
    onClose={() => (updateDialog = null)}
    onDone={async () => {
      packInfo = await api.packInfo(profile.id).catch(() => null);
      packUpdate = null;
      contentKey++;
      onChanged();
    }}
  />
{/if}

<style>
  .header {
    position: relative;
    padding: 24px 28px 20px;
    display: flex;
    align-items: flex-end;
    gap: 20px;
    background: var(--stage);
    border-bottom: 1px solid var(--line);
  }

  .scales-right {
    position: absolute;
    top: 0;
    bottom: 0;
    left: 50%;
    right: 0;
  }

  .big-icon,
  .title,
  .actions {
    position: relative;
  }

  .big-icon {
    display: flex;
  }

  .title {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .kind {
    font-family: var(--font-mono);
    font-size: 11px;
    letter-spacing: 1.5px;
    text-transform: uppercase;
    color: var(--text-dim);
  }

  .kind.event {
    color: var(--accent-fg);
  }

  h1 {
    font-weight: 700;
    font-size: 40px;
    line-height: 1;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .chips {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  .chips .chip {
    padding: 4px 8px;
  }

  button.chip {
    min-height: 0;
    font-weight: 400;
    color: var(--accent-fg);
  }

  .actions {
    display: flex;
    gap: 8px;
  }

  .actions .icon {
    width: 44px;
    height: 44px;
    background: var(--surface-2);
  }

  .export {
    height: 44px;
    padding: 0 16px;
    font-size: 13px;
    font-weight: 700;
  }

  .menu {
    position: relative;
  }

  .menu-pop {
    position: absolute;
    right: 0;
    top: calc(100% + 6px);
    z-index: 20;
    min-width: 230px;
    display: flex;
    flex-direction: column;
    padding: 6px;
    background: var(--bar);
    border: 1px solid var(--line-strong);
  }

  .menu-pop button {
    justify-content: flex-start;
    background: transparent;
    font-weight: 500;
  }

  .menu-pop button:hover:not(:disabled) {
    background: var(--surface);
  }

  .tabs {
    display: flex;
    gap: 2px;
    padding: 0 20px;
    border-bottom: 1px solid var(--line);
  }

  .tabs button {
    position: relative;
    height: 44px;
    padding: 0 14px;
    background: transparent;
    font-weight: 600;
    font-size: 14px;
    color: var(--text-muted);
  }

  .tabs button:hover:not(:disabled) {
    background: transparent;
    color: var(--text);
  }

  .tabs button.active {
    font-weight: 700;
    color: var(--text);
  }

  .tabs button.active::after {
    content: "";
    position: absolute;
    left: 8px;
    right: 8px;
    bottom: 0;
    height: 3px;
    background: var(--accent);
  }

  .count {
    font-size: 11px;
    font-weight: 400;
    color: var(--text-muted);
  }

  .top {
    margin: 10px 28px 0;
  }
</style>
