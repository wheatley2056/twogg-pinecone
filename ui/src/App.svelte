<script lang="ts">
  import { onMount } from "svelte";
  import { crashText } from "./lib/crash";
  import { api, errorText } from "./lib/api";
  import { t } from "./lib/i18n.svelte";
  import { notify } from "./lib/notifications.svelte";
  import { osNotify } from "./lib/osNotify";
  import { startReminders } from "./lib/reminders.svelte";
  import { hasUnread } from "./lib/notifications.svelte";
  import { friends, refreshFriends, startFriends } from "./lib/friends.svelte";
  import { byRecent, versionLabel } from "./lib/profiles";
  import { installs } from "./lib/installs.svelte";
  import { runOf, startGame, stopGame } from "./lib/runs.svelte";
  import type { MenuEntry } from "./lib/contextmenu.svelte";
  import { loadSettings, settings } from "./lib/settings.svelte";
  import { appWindow } from "./lib/window";
  import { afterGameExit, installUpdate, startUpdateChecks, updates } from "./lib/updates.svelte";
  import type { Account, AppInfo, ContentKind, ImportResult, Invite, OpenRequest, Profile, ProjectType, SyncPack } from "./lib/types";
  import DiscoverPage from "./components/pages/DiscoverPage.svelte";
  import PlayPage from "./components/pages/PlayPage.svelte";
  import ImportDialog from "./components/ImportDialog.svelte";
  import InstanceView, { type InstanceTab } from "./components/library/InstanceView.svelte";
  import DuplicateDialog from "./components/library/DuplicateDialog.svelte";
  import SyncShareDialog from "./components/library/SyncShareDialog.svelte";
  import NewSyncDialog from "./components/library/NewSyncDialog.svelte";
  import SyncPackView from "./components/library/SyncPackView.svelte";
  import EventsPage from "./components/pages/EventsPage.svelte";
  import Onboarding from "./components/Onboarding.svelte";
  import LibraryPage from "./components/pages/LibraryPage.svelte";
  import MissingDialog from "./components/MissingDialog.svelte";
  import ProfileDialog from "./components/ProfileDialog.svelte";
  import SettingsPage, { type SettingsSection } from "./components/pages/SettingsPage.svelte";
  import SkinsPage from "./components/pages/SkinsPage.svelte";
  import Friends from "./components/shell/Friends.svelte";
  import Panels from "./components/shell/Panels.svelte";
  import PlayBar from "./components/shell/PlayBar.svelte";
  import TitleBar, { type Tab } from "./components/shell/TitleBar.svelte";
  import ContextMenu from "./components/ui/ContextMenu.svelte";

  type View = Tab | "settings";

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
  let discoverTarget = $state<{ profileId: string | null; type: ProjectType }>({ profileId: null, type: "modpack" });
  let instanceTab = $state<InstanceTab>("content");
  // Rechts in der Bibliothek: Instanz (selectedId) oder ein Sync-Pack.
  let syncId = $state<string | null>(null);
  let syncPacks = $state<SyncPack[]>([]);
  let creatingSync = $state(false);
  let settingsSection = $state<SettingsSection>("general");
  let eventId = $state<string | null>(null);
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
    if (value.startsWith("pinecone://friend/")) {
      addFriendByLink(value);
      return;
    }
    if (value.startsWith("pinecone://sync/")) {
      syncShareCode = value;
      return;
    }
    openPack(value.startsWith("pinecone://") ? { kind: "link", link: value } : { kind: "file", path: value });
  }

  /** `pinecone://friend/<code>` (F-57): Anfrage schicken und Ergebnis melden. */
  async function addFriendByLink(link: string) {
    try {
      const r = await api.friendRequestCode(link);
      notify({
        id: `friend-code-${Date.now()}`,
        kind: "friend",
        title: r.status === "friends" ? t("friends.nowFriends", { name: r.name }) : t("friends.codeAdded", { name: r.name }),
      });
      refreshFriends();
    } catch (e) {
      notify({ id: `friend-code-${Date.now()}`, kind: "error", title: t("friends.add"), text: errorText(e) });
    }
  }

  /** Geöffneter Link auf ein geteiltes Sync-Pack (F-58). */
  let syncShareCode = $state<string | null>(null);

  /** Nach dem Installieren aus einer Einladung direkt beitreten. */
  let joinAfterImport: string | null = null;

  async function onImported(result: ImportResult) {
    await reloadProfiles();
    openProfile(result.profile.id);
    if (result.missing.length > 0) missing = result;
    if (joinAfterImport) {
      const server = joinAfterImport;
      joinAfterImport = null;
      play(result.profile.id, server);
    }
  }

  /** Teilen-Link ohne Versionsangabe, damit eine andere Version desselben Packs passt. */
  const packBase = (link: string) => link.replace(/^(pinecone:\/\/[^/]+\/[^/?#]+).*$/, "$1");

  /** Einladung annehmen (F-56): passende Instanz suchen, sonst das Pack installieren, dann beitreten. */
  async function acceptInvite(inv: Invite) {
    try {
      await api.friendInviteAnswer(inv.id, true);
    } catch (e) {
      notify({ id: `invite-${inv.id}`, kind: "error", title: t("invite.notice", { name: inv.name }), text: errorText(e) });
      refreshFriends();
      return;
    }
    refreshFriends();
    if (inv.pack) {
      const wanted = packBase(inv.pack);
      for (const p of profiles) {
        const link = await api.packShareLink(p.id).catch(() => null);
        if (link && packBase(link) === wanted) {
          play(p.id, inv.server);
          return;
        }
      }
      joinAfterImport = inv.server;
      notify({ id: `invite-${inv.id}`, kind: "info", title: t("invite.notice", { name: inv.name }), text: t("invite.installFirst") });
      openPack({ kind: "link", link: inv.pack });
      return;
    }
    if (selected) play(selected.id, inv.server);
  }

  // Eigener Server zum Einladen: nur wenn über Pinecone beigetreten (sonst kennt Pinecone ihn nicht).
  const inviteFrom = $derived.by(() => {
    if (!selected) return null;
    const run = runOf(selected.id);
    return run.status === "running" && run.server ? { server: run.server, instance: selected.name, profileId: selected.id } : null;
  });

  // Neue Freundschaftsanfragen (F-37): Panel und, falls gewünscht, Desktop-Benachrichtigung.
  // Beim ersten Laden gelten vorhandene Anfragen als bekannt.
  let seenRequests: Set<string> | null = null;
  $effect(() => {
    const incoming = friends.state?.incoming;
    if (!incoming) return;
    if (!seenRequests) {
      seenRequests = new Set(incoming.map((r) => r.uuid));
      return;
    }
    for (const r of incoming) {
      if (seenRequests.has(r.uuid)) continue;
      seenRequests.add(r.uuid);
      const title = t("friends.requestNotice", { name: r.name });
      notify({ id: `friend-request-${r.uuid}`, kind: "friend", title });
      if (settings.current?.notifications.osFriends) osNotify(title);
    }
  });

  // Neue Einladungen als Benachrichtigung mit „Beitreten“.
  const seenInvites = new Set<string>();
  $effect(() => {
    for (const inv of friends.state?.invites ?? []) {
      if (seenInvites.has(inv.id)) continue;
      seenInvites.add(inv.id);
      notify({
        id: `invite-${inv.id}`,
        kind: "friend",
        title: t("invite.notice", { name: inv.name }),
        text: t("invite.noticeText", { instance: inv.instance ?? "Minecraft", server: inv.server }),
        action: { label: t("friends.join"), run: () => acceptInvite(inv) },
      });
    }
  });

  const selected = $derived(profiles.find((p) => p.id === selectedId) ?? null);
  const hasAccount = $derived(accounts.some((a) => a.active));
  const tab = $derived<Tab | null>(view === "settings" ? null : view);
  const syncPack = $derived(syncId ? (syncPacks.find((s) => s.id === syncId) ?? null) : null);
  const activity = $derived(profiles.filter((p) => runOf(p.id).status === "preparing").length + installs.length);
  const customFrame = $derived(!(settings.current?.systemFrame ?? false));

  async function reloadProfiles() {
    try {
      profiles = await api.profiles();
      if (!profiles.some((p) => p.id === selectedId)) {
        const first = [...profiles].sort(byRecent)[0];
        selectedId = first?.id ?? null;
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
    if (info?.updatesConfigured) startUpdateChecks();
    // Hinweise vor Eventstart (F-14).
    startReminders({
      onOpen: (id) => ((eventId = id), (view = "events"), void appWindow.restore()?.catch(() => {})),
      profileIdOfPack: (packId) => profiles.find((p) => p.packId === packId)?.id ?? null,
    });
    // Ältere Pack-Instanzen ohne Symbol bekommen das Logo ihres Packs.
    api
      .packFillIcons()
      .then((n) => void (n > 0 && reloadProfiles()))
      .catch(() => {});
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

  function openProfile(id: string, tab: InstanceTab = "content") {
    select(id);
    syncId = null;
    instanceTab = tab;
    view = "library";
  }

  async function reloadSyncPacks() {
    syncPacks = await api.syncpackList().catch(() => []);
  }

  $effect(() => {
    if (view === "library") reloadSyncPacks();
  });

  function onGameExit(id: string) {
    const run = runOf(id);
    if (run.error) {
      const name = profiles.find((p) => p.id === id)?.name ?? id;
      if (run.crash && settings.current?.notifications.osCrash) osNotify(t("notice.crashed", { name }), crashText(run.crash.cause));
      notify({
        id: `game-${id}-${Date.now()}`,
        kind: "error",
        title: run.crash ? t("notice.crashed", { name }) : t("notice.gameError", { name }),
        text: run.crash ? crashText(run.crash.cause) : run.error,
        action: { label: run.crash ? t("crash.view") : t("profile.tabLog"), run: () => openProfile(id, "logs") },
      });
    }
    reloadProfiles();
    homeKey++;
    afterGameExit();
  }

  /** Wie das Fenster nach dem letzten Start verändert wurde (F-52); `null` = gar nicht. */
  let windowAway: "minimize" | "hide" | null = null;

  /** Fenster nach dem Spielstart minimieren, ausblenden oder den Launcher schließen. */
  function afterStart() {
    const behavior = settings.current?.launchBehavior ?? "keep";
    // Laufen noch Downloads oder Vorbereitungen, wird nicht geschlossen, sondern ausgeblendet.
    const busy = installs.length > 0 || profiles.some((p) => runOf(p.id).status === "preparing");
    if (behavior === "close" && !busy) {
      appWindow.close();
    } else if (behavior === "hide" || behavior === "close") {
      windowAway = "hide";
      appWindow.hide();
    } else if (behavior === "minimize") {
      windowAway = "minimize";
      appWindow.minimize();
    }
  }

  /** Nach Spielende: ausgeblendet immer, minimiert nach Einstellung, bei Absturz immer zurück. */
  function afterExit(id: string) {
    if (!windowAway) return;
    const crashed = runOf(id).error !== null;
    if (windowAway === "hide" || crashed || (settings.current?.showAfterExit ?? true)) {
      windowAway = null;
      appWindow.restore();
    }
  }

  /** Starten aus Spielleiste, Startseite oder Server-Schnellbeitritt. */
  function play(id: string, server?: string) {
    select(id);
    startGame(
      id,
      () => {
        onGameExit(id);
        afterExit(id);
      },
      server ?? null,
      afterStart,
    );
  }

  /** Instanz, die gerade dupliziert wird (Dialog offen). */
  let duplicating = $state<Profile | null>(null);

  /** Signal an den Inhalt-Tab, den Hinzufügen-Dialog zu öffnen. */
  let addSignal = $state(0);

  /** Rechtsklick auf eine Instanz (Bibliothek, Startseite). */
  function instanceMenu(p: Profile): MenuEntry[] {
    const running = runOf(p.id).status === "running";
    return [
      running
        ? { label: t("menu.stop"), icon: "stop", action: () => stopGame(p.id) }
        : { label: t("menu.play"), icon: "play", disabled: runOf(p.id).status === "preparing", action: () => play(p.id) },
      { label: t("menu.open"), icon: "library", action: () => openProfile(p.id) },
      { label: t("content.addContent"), icon: "plus", disabled: running, action: () => (openProfile(p.id, "content"), addSignal++) },
      { label: t("duplicate.menu"), icon: "copy", action: () => (duplicating = p) },
      "-",
      { label: t("profile.openFolder"), icon: "folder", action: () => api.openProfileFolder(p.id) },
      { label: t("instance.tabWorlds"), icon: "globe", action: () => openProfile(p.id, "worlds") },
      { label: t("instance.tabScreenshots"), icon: "image", action: () => openProfile(p.id, "screenshots") },
      { label: t("instance.tabLogs"), icon: "log", action: () => openProfile(p.id, "logs") },
      { label: t("instance.tabSettings"), icon: "settings", action: () => openProfile(p.id, "settings") },
    ];
  }

  function discover(profileId: string | null, kind: ContentKind) {
    discoverTarget = { profileId, type: kind };
    view = "discover";
  }

  function togglePanel(p: "activity" | "notifications") {
    panel = panel === p ? null : p;
  }

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

  <main class="view" class:bleed={(view === "home" && profiles.length > 0) || view === "library" || view === "discover" || view === "settings" || view === "events" || view === "skins"}>
    {#if loadError}
      <p class="error">{loadError}</p>
    {/if}
    {#if view === "home"}
      {#if profiles.length === 0}
        <Onboarding
          {info}
          {accounts}
          onAccountsChange={reloadAccounts}
          onCreate={() => (creating = true)}
          onEvents={() => ((eventId = null), (view = "events"))}
          onDiscover={() => (view = "discover")}
        />
      {:else}
        {#key homeKey}
          <PlayPage
            {profiles}
            {selectedId}
            onSelect={select}
            onPlay={(id) => (select(id), play(id))}
            {instanceMenu}
            onJoin={(address) => selected && play(selected.id, address)}
            onLibrary={() => (view = "library")}
            onEvents={(id) => ((eventId = id ?? null), (view = "events"))}
          />
        {/key}
      {/if}
    {:else if view === "library"}
      <LibraryPage
        {profiles}
        {syncPacks}
        current={syncId ? `sync:${syncId}` : selectedId}
        onSelect={(id) => openProfile(id, instanceTab)}
        onSelectSync={(id) => (syncId = id)}
        onCreate={() => (creating = true)}
        onCreateSync={() => (creatingSync = true)}
        onImport={() => openPack({ kind: "link", link: "" })}
        {instanceMenu}
        onProfilesChange={reloadProfiles}
      >
        {#if syncPack}
          {#key syncPack.id}
            <SyncPackView
              pack={syncPack}
              {profiles}
              onChanged={async (p) => {
                await reloadSyncPacks();
                if (!p) syncId = null;
              }}
            />
          {/key}
        {:else if selected}
          {#key selected.id}
            <InstanceView
              profile={selected}
              bind:tab={instanceTab}
              onChanged={() => onGameExit(selected.id)}
              onDeleted={reloadProfiles}
              onAddContent={(kind) => discover(selected.id, kind)}
              {addSignal}
              onCreated={onImported}
              onJoin={(address) => play(selected.id, address)}
              onDuplicate={() => (duplicating = selected)}
            />
          {/key}
        {:else}
          <div class="welcome">
            <h2>{t("welcome.title")}</h2>
            <p class="muted">{t("welcome.text")}</p>
            <button class="primary cut" onclick={() => (creating = true)}>{t("welcome.create")}</button>
          </div>
        {/if}
      </LibraryPage>
    {:else if view === "discover"}
      {#key discoverTarget}
        <DiscoverPage
          {profiles}
          initialProfileId={discoverTarget.profileId ?? selectedId}
          initialType={discoverTarget.type}
          curseforge={info?.curseforgeConfigured ?? false}
          onPackInstalled={onImported}
        />
      {/key}
    {:else if view === "events"}
      <EventsPage
        initialId={eventId}
        {profiles}
        hasInstance={!!selected}
        onInstalled={onImported}
        onOpenInstance={(id) => openProfile(id)}
        onJoin={(address) => selected && play(selected.id, address)}
        onLink={() => ((settingsSection = "twogg"), (view = "settings"))}
      />
    {:else if view === "skins"}
      <SkinsPage {accounts} />
    {:else if view === "settings"}
      <SettingsPage {info} {accounts} bind:section={settingsSection} onAccountsChange={reloadAccounts} />
    {/if}

    {#if panel}
      <Panels {panel} {profiles} onClose={() => (panel = null)} onOpenProfile={(id) => openProfile(id, "logs")} />
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
      <Friends onJoin={(server) => selected && play(selected.id, server)} {inviteFrom} onAcceptInvite={acceptInvite} />
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

{#if creatingSync}
  <NewSyncDialog
    onClose={() => (creatingSync = false)}
    onCreated={async (p) => {
      creatingSync = false;
      await reloadSyncPacks();
      syncId = p.id;
      view = "library";
    }}
  />
{/if}

{#if syncShareCode}
  <SyncShareDialog
    code={syncShareCode}
    onClose={() => (syncShareCode = null)}
    onImported={async (p) => {
      syncShareCode = null;
      await reloadSyncPacks();
      syncId = p.id;
      view = "library";
    }}
  />
{/if}

{#if duplicating}
  <DuplicateDialog
    profile={duplicating}
    onClose={() => (duplicating = null)}
    onCreated={async (p) => {
      duplicating = null;
      await reloadProfiles();
      openProfile(p.id);
    }}
  />
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

<ContextMenu />

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

  .view.bleed {
    padding: 0;
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
</style>
