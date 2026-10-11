<script lang="ts">
  import { onMount } from "svelte";
  import { api, errorText } from "./lib/api";
  import { t } from "./lib/i18n.svelte";
  import { notify } from "./lib/notifications.svelte";
  import { hasUnread } from "./lib/notifications.svelte";
  import { startFriends } from "./lib/friends.svelte";
  import { byRecent, versionLabel } from "./lib/profiles";
  import { runOf, startGame } from "./lib/runs.svelte";
  import { loadSettings, settings } from "./lib/settings.svelte";
  import { checkUpdates, installUpdate, updates } from "./lib/updates.svelte";
  import type { Account, AppInfo, ContentKind, ImportResult, OpenRequest, Profile, ProjectType } from "./lib/types";
  import Discover from "./components/Discover.svelte";
  import Home from "./components/Home.svelte";
  import ImportDialog from "./components/ImportDialog.svelte";
  import Library from "./components/Library.svelte";
  import MissingDialog from "./components/MissingDialog.svelte";
  import ProfileDialog from "./components/ProfileDialog.svelte";
  import ProfileView from "./components/ProfileView.svelte";
  import Settings from "./components/Settings.svelte";
  import SyncPacks from "./components/SyncPacks.svelte";
  import Skins from "./components/Skins.svelte";
  import Screenshots from "./components/Screenshots.svelte";
  import Friends from "./components/shell/Friends.svelte";
  import Panels from "./components/shell/Panels.svelte";
  import PlayBar from "./components/shell/PlayBar.svelte";
  import TitleBar, { type Tab } from "./components/shell/TitleBar.svelte";
  import Segmented from "./components/ui/Segmented.svelte";

  type View = Tab | "syncpacks" | "screenshots" | "profile" | "settings";
  type LibraryView = "library" | "syncpacks" | "screenshots";

  const SELECTED_KEY = "pinecone.selected";

  let info = $state<AppInfo | null>(null);
  let profiles = $state<Profile[]>([]);
  let accounts = $state<Account[]>([]);
  // Gewählte Instanz: gilt für Spielleiste und Bibliothek gleichermaßen.
  let selectedId = $state<string | null>(readSelected());
  let view = $state<View>("home");
  let panel = $state<"activity" | "notifications" | null>(null);
  let creating = $state(false);
  let loadError = $state<string | null>(null);
  // Neu laden der Startseite nach Spielende (Spielzeit, letzter Server).
  let homeKey = $state(0);
  // Entdecken aus einem Profil heraus: Ziel und Inhaltsart vorbelegen.
  let discoverTarget = $state<{ profileId: string | null; type: ProjectType }>({ profileId: null, type: "mod" });
  let profileTab = $state<"content" | "log">("content");
  let missing = $state<ImportResult | null>(null);
  // Modpack-Vorschau: aus der Bibliothek, per pinecone://-Link oder beim Öffnen einer .mrpack-Datei.
  let openRequest = $state<OpenRequest | null>(null);
  let openQueue: OpenRequest[] = [];

  function readSelected(): string | null {
    try {
      return localStorage.getItem(SELECTED_KEY);
    } catch {
      return null;
    }
  }

  function select(id: string) {
    selectedId = id;
    try {
      localStorage.setItem(SELECTED_KEY, id);
    } catch {
      // Ohne Speicher bleibt die Wahl nur bis zum Neustart.
    }
  }

  function openPack(request: OpenRequest) {
    if (openRequest) openQueue.push(request);
    else openRequest = request;
  }

  function closeOpenRequest() {
    openRequest = openQueue.shift() ?? null;
  }

  /** Vom System übergeben: Link oder Dateipfad. */
  function openExternal(value: string) {
    openPack(value.startsWith("pinecone://") ? { kind: "link", link: value } : { kind: "file", path: value });
  }

  async function onImported(result: ImportResult) {
    await reloadProfiles();
    openProfile(result.profile.id);
    if (result.missing.length > 0) missing = result;
  }

  const selected = $derived(profiles.find((p) => p.id === selectedId) ?? null);
  const hasAccount = $derived(accounts.some((a) => a.active));
  const tab = $derived<Tab | null>(
    view === "profile" || view === "syncpacks" || view === "screenshots" ? "library" : view === "settings" ? null : view,
  );
  const activity = $derived(profiles.filter((p) => runOf(p.id).status === "preparing").length);
  const customFrame = $derived(!(settings.current?.systemFrame ?? false));

  async function reloadProfiles() {
    try {
      profiles = await api.profiles();
      if (!profiles.some((p) => p.id === selectedId)) {
        const first = [...profiles].sort(byRecent)[0];
        selectedId = first?.id ?? null;
        if (view === "profile") view = "library";
      }
    } catch (e) {
      loadError = errorText(e);
    }
  }

  async function reloadAccounts() {
    try {
      accounts = await api.accounts();
    } catch (e) {
      loadError = errorText(e);
    }
  }

  onMount(async () => {
    await loadSettings().catch((e) => (loadError = errorText(e)));
    info = await api.appInfo().catch(() => null);
    await Promise.all([reloadProfiles(), reloadAccounts()]);
    await api.onOpenPack(openExternal).catch(() => {});
    for (const r of await api.takeOpenRequests().catch(() => [])) openExternal(r);
    if (info?.updatesConfigured) checkUpdates(true);
  });

  // Freundesliste: Status melden (gewählte Instanz, ob gespielt wird, Server beim Schnellbeitritt).
  onMount(() =>
    startFriends(() => {
      const p = selected;
      const run = p ? runOf(p.id) : null;
      return {
        instance: p?.name ?? null,
        version: p ? versionLabel(p) : null,
        server: run?.status === "running" ? run.server : null,
        playing: run?.status === "running",
      };
    }),
  );

  // Gefundenes App-Update als Benachrichtigung anbieten.
  $effect(() => {
    const u = updates.info;
    if (!u) return;
    notify({
      id: `app-update-${u.version}`,
      kind: "update",
      title: t("notice.appUpdate", { version: u.version }),
      action: { label: t("notice.install"), run: installUpdate },
    });
  });

  function openProfile(id: string, tab: "content" | "log" = "content") {
    select(id);
    profileTab = tab;
    view = "profile";
  }

  function onGameExit(id: string) {
    const run = runOf(id);
    if (run.error) {
      const name = profiles.find((p) => p.id === id)?.name ?? id;
      notify({
        id: `game-${id}-${Date.now()}`,
        kind: "error",
        title: t("notice.gameError", { name }),
        text: run.error,
        action: { label: t("profile.tabLog"), run: () => openProfile(id, "log") },
      });
    }
    reloadProfiles();
    homeKey++;
  }

  /** Starten aus Spielleiste, Startseite oder Server-Schnellbeitritt. */
  function play(id: string, server?: string) {
    select(id);
    startGame(id, () => onGameExit(id), server ?? null);
  }

  function discover(profileId: string | null, kind: ContentKind) {
    discoverTarget = { profileId, type: kind };
    view = "discover";
  }

  function togglePanel(p: "activity" | "notifications") {
    panel = panel === p ? null : p;
  }

  const libraryViews = $derived<{ id: LibraryView; label: string }[]>([
    { id: "library", label: t("library.instances") },
    { id: "syncpacks", label: t("nav.syncpacks") },
    { id: "screenshots", label: t("nav.screenshots") },
  ]);
</script>

<div class="app" class:framed={customFrame}>
  <TitleBar
    {tab}
    settingsOpen={view === "settings"}
    {activity}
    unread={hasUnread()}
    {customFrame}
    {panel}
    onTab={(id) => ((view = id), (panel = null))}
    onSettings={() => ((view = "settings"), (panel = null))}
    onPanel={togglePanel}
  />

  <main class="view">
    {#if loadError}
      <p class="error">{loadError}</p>
    {/if}
    {#if view === "library" || view === "syncpacks" || view === "screenshots"}
      <div class="subnav">
        <Segmented items={libraryViews} value={view} label={t("tab.library")} variant="soft" onchange={(id) => (view = id)} />
      </div>
    {/if}
    {#if view === "home"}
      {#if profiles.length === 0}
        <div class="welcome">
          <h2>{t("welcome.title")}</h2>
          <p class="muted">{t("welcome.text")}</p>
          <button class="primary cut" onclick={() => (creating = true)}>{t("welcome.create")}</button>
        </div>
      {:else}
        {#key homeKey}
          <Home {profiles} {hasAccount} onPlay={play} onOpen={(id) => openProfile(id)} />
        {/key}
      {/if}
    {:else if view === "library"}
      <Library
        {profiles}
        {hasAccount}
        onOpen={(id) => openProfile(id)}
        onPlay={play}
        onCreate={() => (creating = true)}
        onOpenPack={openPack}
      />
    {:else if view === "discover"}
      {#key discoverTarget}
        <Discover
          {profiles}
          initialProfileId={discoverTarget.profileId}
          initialType={discoverTarget.type}
          curseforge={info?.curseforgeConfigured ?? false}
          onPackInstalled={onImported}
        />
      {/key}
    {:else if view === "events"}
      <p class="muted placeholder">{t("events.soon")}</p>
    {:else if view === "screenshots"}
      <Screenshots {profiles} />
    {:else if view === "skins"}
      <Skins {accounts} />
    {:else if view === "syncpacks"}
      <SyncPacks {profiles} />
    {:else if view === "profile" && selected}
      {#key selected.id}
        <ProfileView
          profile={selected}
          {hasAccount}
          bind:tab={profileTab}
          onPlay={() => play(selected.id)}
          onChanged={() => onGameExit(selected.id)}
          onDeleted={reloadProfiles}
          onAddContent={(kind) => discover(selected.id, kind)}
          onCreated={onImported}
        />
      {/key}
    {:else if view === "settings"}
      <Settings {info} />
    {/if}

    {#if panel}
      <Panels {panel} {profiles} onClose={() => (panel = null)} onOpenProfile={(id) => openProfile(id, "log")} />
    {/if}
  </main>

  <PlayBar
    {profiles}
    {selected}
    {accounts}
    msaConfigured={info?.msaConfigured ?? false}
    onSelect={select}
    onPlay={(id) => play(id)}
    onCreate={() => (creating = true)}
    onLibrary={() => (selected ? openProfile(selected.id) : (view = "library"))}
    onAccountsChange={reloadAccounts}
  >
    {#snippet friends()}
      <Friends onJoin={(server) => selected && play(selected.id, server)} />
    {/snippet}
  </PlayBar>
</div>

{#if missing}
  <MissingDialog profileId={missing.profile.id} missing={missing.missing} onClose={() => (missing = null)} />
{/if}

{#if openRequest}
  {#key openRequest}
    <ImportDialog request={openRequest} onClose={closeOpenRequest} {onImported} />
  {/key}
{/if}

{#if creating}
  <ProfileDialog
    onClose={() => (creating = false)}
    onSaved={async (p) => {
      creating = false;
      await reloadProfiles();
      openProfile(p.id);
    }}
  />
{/if}

<style>
  .app {
    display: grid;
    grid-template-rows: 52px minmax(0, 1fr) 88px;
    height: 100%;
    background: var(--bg);
  }

  .app.framed {
    border: 1px solid var(--line-strong);
  }

  .view {
    position: relative;
    min-width: 0;
    min-height: 0;
    padding: 22px 28px;
    display: flex;
    flex-direction: column;
    gap: 12px;
    overflow: hidden;
  }

  .subnav {
    display: flex;
  }

  .welcome {
    margin: auto;
    text-align: center;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 14px;
    max-width: 420px;
  }

  .welcome h2 {
    font-size: 28px;
  }

  .placeholder {
    margin: auto;
  }
</style>
