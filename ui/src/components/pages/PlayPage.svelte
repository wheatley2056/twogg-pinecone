<script lang="ts">
  // Spielen (Start) nach docs/design/referenz/01-spielen.html: Bühne mit Wald, Event-Teaser,
  // Event-Kalender und „Zuletzt gespielt“ mit Schnellbeitritt zum Community-Server.
  import { onMount } from "svelte";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { api } from "../../lib/api";
  import { badgeText, calendarLine, featured, loadTwogg, participationOf, twogg } from "../../lib/events.svelte";
  import { t } from "../../lib/i18n.svelte";
  import { byRecent, versionLabel } from "../../lib/profiles";
  import type { AppEvent, Profile, ServerInfo, ServerStatus } from "../../lib/types";
  import EventIcon from "../EventIcon.svelte";
  import Icon from "../Icon.svelte";
  import ProfileIcon from "../ProfileIcon.svelte";
  import Forest from "../ui/Forest.svelte";

  interface Props {
    profiles: Profile[];
    selectedId: string | null;
    onSelect: (id: string) => void;
    onJoin: (address: string) => void;
    onLibrary: () => void;
    onEvents: (eventId?: string) => void;
  }
  let { profiles, selectedId, onSelect, onJoin, onLibrary, onEvents }: Props = $props();

  let chosenId = $state<string | null>(null);
  let server = $state<ServerInfo["gameServer"]>(null);
  let status = $state<ServerStatus | null | "offline">(null);

  onMount(() => {
    loadTwogg();
    api
      .serverInfo()
      .then(async (info) => {
        server = info.gameServer;
        if (server) status = await api.pingServer(server.address).catch(() => "offline" as const);
      })
      .catch(() => {});
  });

  const events = $derived(twogg.events ?? []);
  const hero = $derived(events.find((e) => e.id === chosenId) ?? featured(events));
  const calendar = $derived(events.slice(0, 3));
  const recent = $derived([...profiles].sort(byRecent).slice(0, server ? 4 : 5));
  const heroParticipation = $derived(hero ? participationOf(hero.id) : null);

  /** „REVIVAL: Avalon“ → zwei Zeilen wie im Prototyp. */
  function heroLines(title: string): string[] {
    const i = title.indexOf(":");
    return i > 0 ? [title.slice(0, i + 1), title.slice(i + 1).trim()] : [title];
  }

  function teamAction(e: AppEvent): string {
    return e.eventType === "ROLEPLAY" ? t("events.chooseGuild") : e.eventType === "TEAM" ? t("events.chooseTeam") : t("events.details");
  }

  function open(url: string) {
    openUrl(url).catch(() => window.open(url, "_blank"));
  }
</script>

<div class="stage">
  <Forest />

  {#if hero}
    <div class="teaser">
      <span class="event-badge">{badgeText(hero)}</span>
      <h1 class:long={hero.title.length > 24}>
        {#each heroLines(hero.title) as line, i (i)}{#if i > 0}<br />{/if}{line}{/each}
      </h1>
      {#if hero.summary}<p class="summary">{hero.summary}</p>{/if}
      {#if heroParticipation}
        <span class="chip ok">
          <Icon name="check" size={14} />
          {heroParticipation.teamName
            ? t("events.inTeam", { team: heroParticipation.teamName })
            : heroParticipation.kind === "pre"
              ? t("events.preRegistered")
              : t("events.registered")}
        </span>
      {/if}
      <div class="actions">
        {#if !hero.ended}
          <button class="cut edge big" onclick={() => open(`${hero.url}?tab=teams`)}>{teamAction(hero)}</button>
        {/if}
        <a href={hero.url} class="external" onclick={(e) => (e.preventDefault(), open(hero.url))}>
          {hero.ended ? t("events.toResults") : t("events.toRegistration")}
          <Icon name="external" size={15} stroke={2} />
        </a>
      </div>
    </div>
  {:else if twogg.error}
    <div class="teaser">
      <span class="event-badge">{t("events.badge")}</span>
      <h1>{t("home.offlineTitle")}</h1>
      <p class="summary">{t("home.offlineText")}</p>
    </div>
  {/if}

  <aside class="calendar" aria-label={t("events.calendar")}>
    <div class="cal-head">
      <span class="cal-title">{t("events.calendar")}</span>
      <span class="mono dim">TWOGG</span>
    </div>
    {#each calendar as e (e.id)}
      <button class="cal-item" class:past={e.ended} class:current={chosenId !== null && e.id === hero?.id} onclick={() => (chosenId = e.id)}>
        <EventIcon event={e} size={38} />
        <span class="cal-text">
          <span class="cal-name">{e.title}</span>
          <span class="cal-line">{calendarLine(e)}</span>
        </span>
        {#if e.id === featured(events)?.id && !e.ended}<span class="new-badge">{t("events.new")}</span>{/if}
      </button>
    {:else}
      <p class="cal-empty">{twogg.loading ? t("events.loading") : t("events.none")}</p>
    {/each}
    {#if events.length > 3}
      <button class="link more" onclick={() => onEvents()}>{t("events.all", { count: events.length })}</button>
    {/if}
    <a class="discord" href="https://twogg.chat" onclick={(e) => (e.preventDefault(), open("https://twogg.chat"))}>
      <Icon name="chat" size={17} />
      <span>{t("events.discord")}</span>
      <Icon name="external" size={14} stroke={2} />
    </a>
  </aside>

  <section class="recent">
    <div class="recent-head">
      <h2>{t("home.recent")}</h2>
      <button class="link" onclick={onLibrary}>{t("home.allInLibrary")}</button>
    </div>
    <div class="tiles">
      {#each recent as p (p.id)}
        <button class="tile" class:selected={p.id === selectedId} onclick={() => onSelect(p.id)} aria-pressed={p.id === selectedId}>
          <ProfileIcon profile={p} size={44} />
          <span class="tile-text">
            <span class="tile-name">{p.name}</span>
            <span class="tile-version mono">{versionLabel(p)}</span>
          </span>
        </button>
      {/each}
      {#if server}
        {@const address = server.address}
        <button class="tile server" onclick={() => onJoin(address)} title={address}>
          <ProfileIcon profile={{ name: server.name, icon: "motif:server" }} size={44} />
          <span class="tile-text">
            <span class="tile-name">{server.name}</span>
            {#if status && status !== "offline"}
              <span class="tile-status ok"><span class="square"></span>{t("home.serverOnline")}</span>
            {:else if status === "offline"}
              <span class="tile-status"><span class="square off"></span>{t("home.serverOffline")}</span>
            {:else}
              <span class="tile-status"><span class="square off"></span>{t("home.serverChecking")}</span>
            {/if}
          </span>
        </button>
      {/if}
    </div>
  </section>
</div>

<style>
  .stage {
    position: relative;
    height: 100%;
    overflow: hidden;
    background: var(--stage);
  }

  .teaser {
    position: absolute;
    left: 48px;
    top: 44px;
    width: min(560px, calc(100% - 420px));
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  h1 {
    margin: 0;
    font-weight: 700;
    font-size: 66px;
    line-height: 0.95;
    overflow-wrap: anywhere;
  }

  h1.long {
    font-size: 52px;
  }

  .summary {
    margin: 0;
    font-size: 16px;
    line-height: 1.55;
    color: var(--text-soft);
    max-width: 460px;
    display: -webkit-box;
    -webkit-line-clamp: 4;
    line-clamp: 4;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .chip.ok {
    align-self: flex-start;
  }

  .actions {
    display: flex;
    gap: 12px;
    align-items: center;
    margin-top: 6px;
  }

  .big {
    height: 46px;
    padding: 0 20px;
    font-weight: 700;
  }

  .external {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 46px;
    padding: 0 12px;
    font-size: 14px;
    font-weight: 700;
  }

  .calendar {
    position: absolute;
    right: 28px;
    top: 28px;
    width: 300px;
    padding: 16px;
    background: var(--bar-glass);
    border: 1px solid var(--line-strong);
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .cal-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .cal-title {
    font-family: var(--font-pixel);
    font-weight: 600;
    font-size: 16px;
  }

  .dim {
    font-size: 11px;
    color: var(--text-dim);
  }

  .cal-item {
    justify-content: flex-start;
    gap: 12px;
    padding: 10px;
    min-height: 0;
    background: var(--surface);
    clip-path: var(--cut-1);
    text-align: left;
  }

  .cal-item:hover:not(:disabled) {
    background: var(--surface-2);
  }

  .cal-item.past {
    background: var(--surface-past);
  }

  .cal-item.current {
    box-shadow: inset 0 -3px 0 var(--accent);
  }

  .cal-text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .cal-name {
    font-size: 14px;
    font-weight: 700;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .past .cal-name {
    color: var(--text-soft);
  }

  .cal-line {
    font-size: 12px;
    font-weight: 400;
    color: var(--text-muted);
    margin-top: 2px;
  }

  .past .cal-line {
    color: var(--text-dim);
  }

  .cal-empty {
    margin: 4px 0;
    color: var(--text-dim);
    font-size: 13px;
  }

  .more {
    align-self: flex-start;
    min-height: 24px;
    font-size: 13px;
  }

  .discord {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 4px 2px;
    font-size: 13px;
    font-weight: 600;
    border-top: 1px solid var(--line);
  }

  .discord span {
    flex: 1;
  }

  .recent {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    height: 138px;
    padding: 14px 48px 18px;
    background: color-mix(in srgb, var(--bar) 84%, transparent);
    border-top: 1px solid var(--line);
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .recent-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .recent-head h2 {
    font-size: 16px;
  }

  .recent-head .link {
    min-height: 0;
    padding: 0;
    font-size: 13px;
    font-weight: 600;
  }

  .tiles {
    display: grid;
    grid-template-columns: repeat(5, minmax(0, 1fr));
    gap: 12px;
  }

  .tile {
    justify-content: flex-start;
    gap: 12px;
    padding: 10px;
    min-height: 0;
    background: var(--surface);
    clip-path: var(--cut-2);
    text-align: left;
  }

  .tile:hover:not(:disabled) {
    background: var(--surface-2);
  }

  .tile.selected {
    background: var(--surface-active);
    box-shadow: inset 0 -3px 0 var(--accent);
  }

  .tile.server {
    background: transparent;
    clip-path: none;
    outline: 1px dashed var(--line-dashed);
    outline-offset: -1px;
  }

  .tile.server:hover:not(:disabled) {
    background: var(--surface);
  }

  .tile-text {
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .tile-name {
    font-size: 14px;
    font-weight: 700;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .tile-version {
    font-size: 11px;
    font-weight: 400;
    color: var(--text-muted);
    margin-top: 3px;
  }

  .tile-status {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    font-weight: 500;
    color: var(--text-dim);
    margin-top: 3px;
  }

  .tile-status.ok {
    color: var(--nadel);
  }

  .square {
    width: 7px;
    height: 7px;
    background: var(--nadel);
  }

  .square.off {
    background: var(--text-dim);
  }

  @media (max-width: 1150px) {
    .teaser {
      width: calc(100% - 400px);
    }

    h1 {
      font-size: 52px;
    }
  }
</style>
