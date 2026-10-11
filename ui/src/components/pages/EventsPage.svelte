<script lang="ts">
  // Events (im Prototyp noch nicht gestaltet, gebaut nach dem Design-System): Liste links,
  // rechts Detail mit Countdown, Anmeldestand und Beitreten, sobald twogg.app die Adresse freigibt.
  import { onDestroy, onMount } from "svelte";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { api, errorText } from "../../lib/api";
  import { tracked } from "../../lib/installs.svelte";
  import { badgeText, calendarLine, eventDate, featured, loadTwogg, participationOf, twogg, typeLabel } from "../../lib/events.svelte";
  import { t } from "../../lib/i18n.svelte";
  import type { AppEvent, ImportResult, JoinInfo, Profile, TwoggPack } from "../../lib/types";
  import EventIcon from "../EventIcon.svelte";
  import Icon from "../Icon.svelte";
  import Forest from "../ui/Forest.svelte";

  interface Props {
    initialId: string | null;
    profiles: Profile[];
    hasInstance: boolean;
    onJoin: (address: string) => void;
    onLink: () => void;
    onInstalled: (result: ImportResult) => void;
    onOpenInstance: (id: string) => void;
  }
  let { initialId, profiles, hasInstance, onJoin, onLink, onInstalled, onOpenInstance }: Props = $props();

  // TwoGG-Packs, die in der Admin-Seite einem Event zugeordnet sind.
  let packs = $state<TwoggPack[]>([]);
  let installing = $state(false);
  let packError = $state<string | null>(null);

  let chosen = $state<string | null>(null);
  let now = $state(Date.now());
  let join = $state<JoinInfo | null>(null);
  let joinError = $state<string | null>(null);
  let timer: ReturnType<typeof setInterval>;

  onMount(() => {
    loadTwogg();
    api
      .twoggPacks()
      .then((list) => (packs = list))
      .catch(() => {});
    timer = setInterval(() => (now = Date.now()), 30_000);
  });
  onDestroy(() => clearInterval(timer));

  const events = $derived(twogg.events ?? []);
  const upcoming = $derived(events.filter((e) => !e.ended));
  const past = $derived(events.filter((e) => e.ended));
  const current = $derived(events.find((e) => e.id === (chosen ?? initialId)) ?? featured(events));
  const participation = $derived(current ? participationOf(current.id) : null);
  const eventPack = $derived(current ? (packs.find((p) => p.eventId === current.id) ?? null) : null);
  const packInstance = $derived(eventPack ? (profiles.find((p) => p.packId === eventPack.id) ?? null) : null);

  async function installPack(pack: TwoggPack) {
    installing = true;
    packError = null;
    try {
      onInstalled(await tracked({ title: pack.title, iconUrl: pack.iconUrl, target: t("discover.newInstance") }, () => api.packInstallTwogg(pack.id, null)));
    } catch (e) {
      packError = errorText(e);
    } finally {
      installing = false;
    }
  }

  // Server-Adresse erst nachfragen, wenn angemeldet und das Event nicht vorbei ist.
  $effect(() => {
    const e = current;
    join = null;
    joinError = null;
    if (!e || e.ended || !twogg.user) return;
    api
      .appJoin(e.id)
      .then((j) => (join = j))
      .catch((err) => (joinError = errorText(err)));
  });

  const countdown = $derived.by(() => {
    if (!current?.eventDate || current.ended) return null;
    const ms = new Date(current.eventDate).getTime() - now;
    if (ms <= 0) return null;
    const minutes = Math.floor(ms / 60_000);
    return { days: Math.floor(minutes / 1440), hours: Math.floor((minutes % 1440) / 60), minutes: minutes % 60 };
  });

  function open(url: string) {
    openUrl(url).catch(() => window.open(url, "_blank"));
  }
</script>

{#snippet item(e: AppEvent)}
  <button class="item" class:active={current?.id === e.id} class:past={e.ended} onclick={() => (chosen = e.id)}>
    <EventIcon event={e} size={38} />
    <span class="text">
      <span class="name">{e.title}</span>
      <span class="line">{calendarLine(e)}</span>
    </span>
    {#if participationOf(e.id)}<span class="mark" title={t("events.registered")}><Icon name="check" size={14} stroke={2.4} /></span>{/if}
  </button>
{/snippet}

<div class="events">
  <section class="side" aria-label={t("tab.events")}>
    <h2>{t("tab.events")}</h2>
    {#if twogg.error && !twogg.events}<p class="error">{twogg.error}</p>{/if}
    <div class="groups">
      <div class="group">
        <span class="label">{t("events.upcoming")}</span>
        {#each upcoming as e (e.id)}{@render item(e)}{:else}<p class="empty">{twogg.loading ? t("events.loading") : t("events.none")}</p>{/each}
      </div>
      {#if past.length > 0}
        <div class="group">
          <span class="label">{t("events.past")}</span>
          {#each past as e (e.id)}{@render item(e)}{/each}
        </div>
      {/if}
    </div>
    {#if !twogg.user}
      <button class="dashed" onclick={onLink}><Icon name="link" size={16} />{t("twoggLink.connectTitle")}</button>
    {/if}
  </section>

  <main class="detail">
    {#if current}
      <header class="hero">
        {#if current.bannerUrl}
          <img class="banner" src={current.bannerUrl} alt="" />
          <span class="shade"></span>
        {:else}
          <Forest />
        {/if}
        <div class="hero-text">
          <span class="event-badge">{badgeText(current)}</span>
          <h1>{current.title}</h1>
          <div class="chips">
            <span class="chip">{typeLabel(current)}</span>
            {#if current.minecraftVersion}<span class="chip">Minecraft {current.minecraftVersion}</span>{/if}
            {#if current.teamSize && current.eventType !== "STANDARD"}<span class="chip">{t("events.teamSize", { count: current.teamSize })}</span>{/if}
            {#if current.maxPlayers}<span class="chip">{t("events.maxPlayers", { count: current.maxPlayers })}</span>{/if}
            {#if current.requiresApplication}<span class="chip">{t("events.application")}</span>{/if}
          </div>
        </div>
      </header>

      <div class="body">
        {#if countdown}
          <section class="countdown" aria-label={t("events.countdown")}>
            <span class="cd"><span class="num mono">{countdown.days}</span><span class="unit">{t("events.days")}</span></span>
            <span class="cd"><span class="num mono">{countdown.hours}</span><span class="unit">{t("events.hours")}</span></span>
            <span class="cd"><span class="num mono">{countdown.minutes}</span><span class="unit">{t("events.minutes")}</span></span>
            <span class="cd-when">{t("events.startsAt", { when: eventDate(current, true) ?? "" })}</span>
          </section>
        {/if}

        {#if current.summary}<p class="summary">{current.summary}</p>{/if}

        {#if eventPack}
          <section class="status">
            <h3 class="section-label">{t("events.pack")}</h3>
            <div class="row">
              <span class="grow">
                {#if eventPack.pageUrl}
                  {@const url = eventPack.pageUrl}
                  <button class="title link-title" title={t("events.packPage")} onclick={() => open(url)}>{eventPack.title}</button>
                {:else}
                  <span class="title">{eventPack.title}</span>
                {/if}
                <span class="hint">{eventPack.latest ? `${eventPack.latest.version} · ${eventPack.latest.loader ?? "Vanilla"} ${eventPack.latest.minecraft}` : eventPack.summary}</span>
              </span>
              {#if packInstance}
                <button onclick={() => onOpenInstance(packInstance.id)}>{t("events.openInstance")}</button>
              {:else}
                <button class="primary cut" disabled={installing} onclick={() => installPack(eventPack)}>
                  <Icon name="download" size={16} />{installing ? t("discover.loading") : t("events.installPack")}
                </button>
              {/if}
            </div>
            {#if packError}<p class="error">{packError}</p>{/if}
          </section>
        {/if}

        <section class="status">
          <h3 class="section-label">{t("events.yourStatus")}</h3>
          {#if !twogg.user}
            <div class="row">
              <span class="grow"><span class="title">{t("events.notLinked")}</span><span class="hint">{t("events.notLinkedHint")}</span></span>
              <button class="primary cut" onclick={onLink}>{t("twoggLink.connectTitle")}</button>
            </div>
          {:else if participation}
            <div class="row">
              <span class="grow">
                <span class="title ok">{participation.teamName ? t("events.inTeam", { team: participation.teamName }) : participation.kind === "pre" ? t("events.preRegistered") : t("events.registered")}</span>
                <span class="hint">{t("events.statusValue", { status: participation.status })}</span>
              </span>
              {#if join?.serverAddress}
                {@const address = join.serverAddress}
                <button class="primary cut" disabled={!hasInstance} title={hasInstance ? address : t("events.needInstance")} onclick={() => onJoin(address)}>
                  <Icon name="play" size={16} filled />{t("events.joinServer")}
                </button>
              {:else if !current.ended}
                <span class="hint">{t("events.addressLater")}</span>
              {/if}
            </div>
          {:else}
            <div class="row">
              <span class="grow"><span class="title">{t("events.notRegistered")}</span><span class="hint">{current.registrationClosed ? t("events.closed") : t("events.registerHint")}</span></span>
            </div>
          {/if}
          {#if joinError}<p class="hint">{joinError}</p>{/if}
        </section>

        <div class="actions">
          <button class="link" onclick={() => open(current.url)}>
            {current.ended ? t("events.toResults") : t("events.toRegistration")}<Icon name="external" size={15} stroke={2} />
          </button>
          <button class="link" onclick={() => open("https://twogg.chat")}>{t("events.discord")}<Icon name="external" size={15} stroke={2} /></button>
        </div>
      </div>
    {:else if !twogg.loading}
      <p class="empty center">{t("events.none")}</p>
    {/if}
  </main>
</div>

<style>
  .events {
    display: grid;
    grid-template-columns: 300px minmax(0, 1fr);
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

  h2 {
    font-weight: 600;
    font-size: 22px;
  }

  .groups {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .group {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .label {
    font-family: var(--font-mono);
    font-size: 11px;
    letter-spacing: 1px;
    text-transform: uppercase;
    color: var(--text-dim);
    padding: 0 6px 4px;
  }

  .item {
    justify-content: flex-start;
    gap: 12px;
    padding: 10px;
    min-height: 0;
    background: var(--surface);
    clip-path: var(--cut-1);
    text-align: left;
  }

  .item:hover:not(:disabled) {
    background: var(--surface-2);
  }

  .item.past {
    background: var(--surface-past);
  }

  .item.active {
    background: var(--surface-active);
    box-shadow: inset 0 -3px 0 var(--accent);
  }

  .text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .name {
    font-size: 14px;
    font-weight: 700;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .active .name {
    color: var(--accent-hover);
  }

  .past .name {
    color: var(--text-soft);
  }

  .line {
    font-size: 12px;
    font-weight: 400;
    color: var(--text-muted);
  }

  .mark {
    color: var(--nadel);
    display: flex;
  }

  .empty {
    margin: 0;
    padding: 4px 8px;
    font-size: 13px;
    color: var(--text-dim);
  }

  .center {
    margin: auto;
  }

  .detail {
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
  }

  .hero {
    position: relative;
    min-height: 260px;
    display: flex;
    align-items: flex-end;
    padding: 28px;
    background: var(--stage);
    overflow: hidden;
    flex: none;
  }

  .banner {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .shade {
    position: absolute;
    inset: auto 0 0 0;
    height: 100%;
    background: color-mix(in srgb, var(--bar) 55%, transparent);
  }

  .hero-text {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 18px 20px;
    max-width: 720px;
    background: var(--bar-glass);
    border: 1px solid var(--line-strong);
  }

  h1 {
    font-weight: 700;
    font-size: 40px;
    line-height: 1;
  }

  .chips {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }

  .chips .chip {
    padding: 4px 8px;
  }

  .body {
    padding: 22px 28px 28px;
    display: flex;
    flex-direction: column;
    gap: 20px;
    max-width: 900px;
  }

  .countdown {
    display: flex;
    align-items: flex-end;
    gap: 10px;
  }

  .cd {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    min-width: 72px;
    padding: 10px 12px;
    background: var(--surface);
    clip-path: var(--cut-2);
  }

  .num {
    font-size: 30px;
    font-weight: 500;
    color: var(--accent-fg);
    line-height: 1;
  }

  .unit {
    font-size: 11px;
    color: var(--text-dim);
    text-transform: uppercase;
    letter-spacing: 1px;
  }

  .cd-when {
    padding-left: 8px;
    font-size: 13px;
    color: var(--text-muted);
  }

  .summary {
    margin: 0;
    font-size: 16px;
    line-height: 1.55;
    color: var(--text-soft);
  }

  .status {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  h3 {
    margin: 0;
    font-weight: 500;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 14px;
    background: var(--surface);
    clip-path: var(--cut-1);
  }

  .grow {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .title {
    font-weight: 700;
  }

  .link-title {
    justify-content: flex-start;
    min-height: 0;
    padding: 0;
    background: transparent;
    font-size: inherit;
  }

  .link-title:hover:not(:disabled) {
    background: transparent;
    color: var(--accent-hover);
  }

  .title.ok {
    color: var(--nadel);
  }

  .hint {
    margin: 0;
    font-size: 12px;
    color: var(--text-muted);
  }

  .row .primary {
    height: 42px;
  }

  .actions {
    display: flex;
    align-items: center;
    gap: 14px;
    flex-wrap: wrap;
  }

  .actions .link {
    font-size: 14px;
  }
</style>
