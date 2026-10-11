<script lang="ts">
  import { onMount } from "svelte";
  import { api, errorText } from "./lib/api";
  import { t } from "./lib/i18n.svelte";
  import { runOf, startGame } from "./lib/runs.svelte";
  import { loadSettings } from "./lib/settings.svelte";
  import type { Account, AppInfo, ContentKind, ImportResult, Profile, ProjectType } from "./lib/types";
  import AccountMenu from "./components/AccountMenu.svelte";
  import Discover from "./components/Discover.svelte";
  import Home from "./components/Home.svelte";
  import Icon, { type IconName } from "./components/Icon.svelte";
  import Library from "./components/Library.svelte";
  import MissingDialog from "./components/MissingDialog.svelte";
  import ProfileDialog from "./components/ProfileDialog.svelte";
  import ProfileIcon from "./components/ProfileIcon.svelte";
  import ProfileView from "./components/ProfileView.svelte";
  import Settings from "./components/Settings.svelte";

  type View = "home" | "library" | "discover" | "profile" | "settings";

  let info = $state<AppInfo | null>(null);
  let profiles = $state<Profile[]>([]);
  let accounts = $state<Account[]>([]);
  let selectedId = $state<string | null>(null);
  let view = $state<View>("home");
  let creating = $state(false);
  let loadError = $state<string | null>(null);
  // Neu laden der Startseite nach Spielende (Spielzeit, letzter Server).
  let homeKey = $state(0);
  // Entdecken aus einem Profil heraus: Ziel und Inhaltsart vorbelegen.
  let discoverTarget = $state<{ profileId: string | null; type: ProjectType }>({ profileId: null, type: "mod" });
  let profileTab = $state<"content" | "log">("content");
  let missing = $state<ImportResult | null>(null);

  async function onImported(result: ImportResult) {
    await reloadProfiles();
    openProfile(result.profile.id);
    if (result.missing.length > 0) missing = result;
  }

  const selected = $derived(profiles.find((p) => p.id === selectedId) ?? null);
  const hasAccount = $derived(accounts.some((a) => a.active));
  const running = $derived(profiles.filter((p) => runOf(p.id).status !== "idle"));

  async function reloadProfiles() {
    try {
      profiles = await api.profiles();
      if (selectedId && !profiles.some((p) => p.id === selectedId)) {
        selectedId = null;
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
  });

  function openProfile(id: string, tab: "content" | "log" = "content") {
    selectedId = id;
    profileTab = tab;
    view = "profile";
  }

  function onGameExit() {
    reloadProfiles();
    homeKey++;
  }

  /** Starten: zum Profil mit Log wechseln, damit Fortschritt und Ausgabe sichtbar sind. */
  function play(id: string, server?: string) {
    openProfile(id, "log");
    startGame(id, onGameExit, server ?? null);
  }

  function discover(profileId: string | null, kind: ContentKind) {
    discoverTarget = { profileId, type: kind };
    view = "discover";
  }

  const nav: [View, IconName, () => string][] = [
    ["home", "home", () => t("nav.home")],
    ["library", "library", () => t("nav.library")],
    ["discover", "discover", () => t("nav.discover")],
  ];
</script>

<div class="shell">
  <nav class="rail" aria-label="Navigation">
    <img class="logo" src="/pinecone.svg" alt="TwoGG Pinecone" width="34" height="34" />
    {#each nav as [id, icon, label] (id)}
      <button class="rail-item" class:active={view === id || (id === "library" && view === "profile")} onclick={() => (view = id)}>
        <Icon name={icon} size={22} />
        <span>{label()}</span>
      </button>
    {/each}

    {#if running.length > 0}
      <div class="rail-sep"></div>
      {#each running as p (p.id)}
        <button class="rail-running" title={`${p.name} · ${t("nav.running")}`} onclick={() => openProfile(p.id, "log")}>
          <ProfileIcon profile={p} size={34} />
          <span class="pulse"></span>
        </button>
      {/each}
    {/if}

    <div class="spacer"></div>
    <button class="rail-item" class:active={view === "settings"} onclick={() => (view = "settings")}>
      <Icon name="settings" size={22} />
      <span>{t("nav.settings")}</span>
    </button>
    <AccountMenu {accounts} msaConfigured={info?.msaConfigured ?? false} onChange={reloadAccounts} compact />
  </nav>

  <main class="view">
    {#if loadError}
      <p class="error">{loadError}</p>
    {/if}
    {#if view === "home"}
      {#if profiles.length === 0}
        <div class="welcome">
          <img src="/pinecone.svg" alt="" width="96" height="96" />
          <h2>{t("welcome.title")}</h2>
          <p class="muted">{t("welcome.text")}</p>
          <button class="primary" onclick={() => (creating = true)}>{t("welcome.create")}</button>
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
        {onImported}
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
    {:else if view === "profile" && selected}
      {#key selected.id}
        <ProfileView
          profile={selected}
          {hasAccount}
          bind:tab={profileTab}
          onPlay={() => play(selected.id)}
          onChanged={onGameExit}
          onDeleted={reloadProfiles}
          onAddContent={(kind) => discover(selected.id, kind)}
        />
      {/key}
    {:else if view === "settings"}
      <Settings {info} />
    {/if}
  </main>
</div>

{#if missing}
  <MissingDialog profileId={missing.profile.id} missing={missing.missing} onClose={() => (missing = null)} />
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
  .shell {
    display: grid;
    grid-template-columns: 84px 1fr;
    height: 100%;
  }

  .rail {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    padding: 14px 8px;
    background: var(--surface);
    border-right: 1px solid var(--border);
    min-height: 0;
    overflow-y: auto;
  }

  :global([data-background="on"]) .rail {
    background: color-mix(in srgb, var(--surface) 82%, transparent);
    backdrop-filter: blur(14px);
  }

  .logo {
    image-rendering: pixelated;
    margin-bottom: 14px;
  }

  .rail-item {
    width: 68px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    padding: 9px 4px 7px;
    border: none;
    background: transparent;
    color: var(--muted);
    font-size: 11px;
    border-radius: 10px;
  }

  .rail-item:hover:not(:disabled) {
    color: var(--text);
  }

  .rail-item.active {
    background: color-mix(in srgb, var(--accent) 16%, transparent);
    color: var(--accent);
  }

  .rail-sep {
    width: 36px;
    height: 1px;
    background: var(--border);
    margin: 8px 0;
  }

  .rail-running {
    position: relative;
    padding: 2px;
    border: none;
    background: transparent;
  }

  .pulse {
    position: absolute;
    right: -1px;
    bottom: -1px;
    width: 11px;
    height: 11px;
    border-radius: 50%;
    background: var(--accent);
    border: 2px solid var(--surface);
    animation: pulse 1.6s ease-in-out infinite;
  }

  @keyframes pulse {
    50% {
      opacity: 0.45;
    }
  }

  .spacer {
    flex: 1;
  }

  .view {
    min-width: 0;
    min-height: 0;
    padding: 22px 28px;
    display: flex;
    flex-direction: column;
    gap: 12px;
    overflow: hidden;
  }

  .welcome {
    margin: auto;
    text-align: center;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    max-width: 420px;
  }

  .welcome img {
    image-rendering: pixelated;
  }
</style>
