<script lang="ts">
  // Spielen (Start): oben das aktuelle TwoGG-Event mit dem Banner von twogg.app, darunter
  // „Zuletzt gespielt“ und Neuigkeiten; rechts Event-Kalender, Community-Server und Freunde.
  import { onMount } from "svelte";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { api } from "../../lib/api";
  import { badgeText, calendarLine, featured, loadTwogg, participationOf, twogg } from "../../lib/events.svelte";
  import { friendColor, friends } from "../../lib/friends.svelte";
  import { formatAgo, locale, t } from "../../lib/i18n.svelte";
  import { byRecent, versionLabel } from "../../lib/profiles";
  import { menu, type MenuEntry } from "../../lib/contextmenu.svelte";
  import type { NewsItem, Profile, ServerBanner, ServerInfo, ServerStatus } from "../../lib/types";
  import { checkUpdates, installUpdate, updates } from "../../lib/updates.svelte";
  import EventIcon from "../EventIcon.svelte";
  import Icon from "../Icon.svelte";
  import ProfileIcon from "../ProfileIcon.svelte";
  import Forest from "../ui/Forest.svelte";

  interface Props {
    profiles: Profile[];
    selectedId: string | null;
    onSelect: (id: string) => void;
    onPlay: (id: string) => void;
    onJoin: (address: string) => void;
    onLibrary: () => void;
    onEvents: (eventId?: string) => void;
    instanceMenu: (p: Profile) => MenuEntry[];
  }
  let { profiles, selectedId, onSelect, onPlay, onJoin, onLibrary, onEvents, instanceMenu }: Props = $props();

  let chosenId = $state<string | null>(null);
  let server = $state<ServerInfo["gameServer"]>(null);
  let status = $state<ServerStatus | null | "offline">(null);
  let news = $state<NewsItem[] | null>(null);
  let reading = $state<NewsItem | null>(null);
  let bannerFailed = $state<string | null>(null);
  let newsDialog = $state<HTMLDialogElement>();
  let notice = $state<ServerBanner | null>(null);
  let required = $state<{ min: string; current: string } | null>(null);

  const CLOSED_KEY = "pinecone.closedBanners";

  function closedBanners(): string[] {
    try {
      return JSON.parse(localStorage.getItem(CLOSED_KEY) ?? "[]");
    } catch {
      return [];
    }
  }

  function closeNotice() {
    if (!notice) return;
    const id = notice.id;
    notice = null;
    try {
      localStorage.setItem(CLOSED_KEY, JSON.stringify([...closedBanners().filter((c) => c !== id), id].slice(-20)));
    } catch {
      // Ohne Speicher erscheint der Hinweis beim nächsten Start wieder.
    }
  }

  /** true, wenn Version `a` älter ist als `b` (x.y.z, fehlende Teile = 0). */
  function versionOlder(a: string, b: string): boolean {
    const parts = (v: string) => v.split("-")[0].split(".").map((n) => parseInt(n, 10) || 0);
    const [x, y] = [parts(a), parts(b)];
    for (let i = 0; i < 3; i++) if ((x[i] ?? 0) !== (y[i] ?? 0)) return (x[i] ?? 0) < (y[i] ?? 0);
    return false;
  }

  function updateNow() {
    if (updates.info) installUpdate();
    else checkUpdates();
  }

  onMount(() => {
    loadTwogg();
    api
      .serverInfo()
      .then(async (info) => {
        server = info.gameServer;
        if (info.banner && !closedBanners().includes(info.banner.id)) notice = info.banner;
        const min = info.minLauncherVersion;
        if (min) {
          const current = (await api.appInfo().catch(() => null))?.version;
          if (current && versionOlder(current, min)) required = { min, current };
        }
        if (server) status = await api.pingServer(server.address).catch(() => "offline" as const);
      })
      .catch(() => {});
    api
      .twoggNews(6)
      .then((n) => (news = n))
      .catch(() => (news = []));
  });

  $effect(() => {
    if (reading) newsDialog?.showModal();
  });

  const events = $derived(twogg.events ?? []);
  const hero = $derived(events.find((e) => e.id === chosenId) ?? featured(events));
  const calendar = $derived(events.slice(0, 4));
  const recent = $derived([...profiles].sort(byRecent).slice(0, 6));
  const heroParticipation = $derived(hero ? participationOf(hero.id) : null);
  const banner = $derived(hero?.bannerUrl && bannerFailed !== hero.bannerUrl ? hero.bannerUrl : null);
  const online = $derived((friends.state?.friends ?? []).filter((f) => f.online));

  function open(url: string) {
    openUrl(url).catch(() => window.open(url, "_blank"));
  }

  /** Klartext-Anriss aus dem vom Server erzeugten HTML. */
  function excerpt(html: string): string {
    const doc = new DOMParser().parseFromString(html, "text/html");
    return (doc.body.textContent ?? "").replace(/\s+/g, " ").trim();
  }

  function newsDate(secs: number): string {
    return new Date(secs * 1000).toLocaleDateString(locale(), { day: "numeric", month: "short" });
  }

  function closeNews() {
    newsDialog?.close();
    reading = null;
  }
</script>

<div class="home">
  <div class="main">
    {#if required}
      <div class="notice danger" role="alert">
        <Icon name="download" size={18} stroke={2} />
        <span class="text">
          <strong>{t("home.updateRequired")}</strong>
          <span>{t("home.updateRequiredText", { min: required.min, current: required.current })}</span>
          {#if updates.error}<span class="error">{updates.error}</span>{/if}
        </span>
        <button class="small primary" disabled={updates.checking || updates.progress != null} onclick={updateNow}>
          {updates.progress != null
            ? `${Math.round((updates.progress ?? 0) * 100)} %`
            : updates.info
              ? t("home.updateInstall", { version: updates.info.version })
              : t("home.updateCheck")}
        </button>
      </div>
    {/if}
    {#if notice}
      {@const n = notice}
      <div class="notice" class:danger={n.level === "warning"} role={n.level === "warning" ? "alert" : "status"}>
        <Icon name={n.level === "warning" ? "alert" : "bell"} size={18} stroke={2} />
        <span class="text"><span class="body">{n.text}</span></span>
        {#if n.link}
          <a href={n.link} class="external" onclick={(e) => (e.preventDefault(), open(n.link!))}>
            {t("home.noticeMore")}
            <Icon name="external" size={15} stroke={2} />
          </a>
        {/if}
        <button class="icon" aria-label={t("home.noticeClose")} title={t("home.noticeClose")} onclick={closeNotice}>
          <Icon name="close" size={16} />
        </button>
      </div>
    {/if}
    <section class="hero" class:with-banner={banner}>
      {#if banner}
        <img class="banner" src={banner} alt="" onerror={() => (bannerFailed = banner)} />
      {:else}
        <Forest />
      {/if}
      {#if hero}
        <div class="panel">
          <span class="event-badge">{badgeText(hero)}</span>
          <h1 class:long={hero.title.length > 22}>{hero.title}</h1>
          {#if hero.summary}<p class="summary">{hero.summary}</p>{/if}
          <div class="actions">
            <button class="primary cut" onclick={() => onEvents(hero.id)}>{t("home.toEvent")}</button>
            {#if heroParticipation}
              <span class="chip ok">
                <Icon name="check" size={14} />
                {heroParticipation.teamName
                  ? t("events.inTeam", { team: heroParticipation.teamName })
                  : heroParticipation.kind === "pre"
                    ? t("events.preRegistered")
                    : t("events.registered")}
              </span>
            {:else}
              <a href={hero.url} class="external" onclick={(e) => (e.preventDefault(), open(hero.url))}>
                {hero.ended ? t("events.toResults") : t("events.toRegistration")}
                <Icon name="external" size={15} stroke={2} />
              </a>
            {/if}
          </div>
        </div>
      {:else}
        <div class="panel">
          <span class="event-badge">{t("events.badge")}</span>
          <h1>{t("home.offlineTitle")}</h1>
          <p class="summary">{twogg.loading ? t("events.loading") : twogg.error ? t("home.offlineText") : t("home.noEvent")}</p>
        </div>
      {/if}
    </section>

    <section class="block">
      <div class="block-head">
        <h2>{t("home.recent")}</h2>
        <button class="link" onclick={onLibrary}>{t("home.allInLibrary")}</button>
      </div>
      {#if recent.length > 0}
        <div class="recent">
          {#each recent as p (p.id)}
            <div class="tile" class:selected={p.id === selectedId} use:menu={() => instanceMenu(p)}>
              <button class="tile-main" onclick={() => onSelect(p.id)} aria-pressed={p.id === selectedId}>
                <ProfileIcon profile={p} size={46} />
                <span class="tile-text">
                  <span class="tile-name">{p.name}</span>
                  <span class="tile-version mono">{versionLabel(p)}</span>
                  <span class="tile-when">{p.lastPlayed ? formatAgo(p.lastPlayed) : t("home.neverPlayed")}</span>
                </span>
              </button>
              <button class="tile-play" onclick={() => onPlay(p.id)} title={t("home.playInstance", { name: p.name })}
                aria-label={t("home.playInstance", { name: p.name })} disabled={p.running}>
                <Icon name="play" size={16} filled />
              </button>
            </div>
          {/each}
        </div>
      {:else}
        <button class="dashed empty-recent" onclick={onLibrary}><Icon name="plus" size={16} />{t("home.firstInstance")}</button>
      {/if}
    </section>

    <section class="block">
      <div class="block-head">
        <h2>{t("home.news")}</h2>
      </div>
      {#if news === null}
        <p class="muted small">{t("home.newsLoading")}</p>
      {:else if news.length === 0}
        <p class="muted small">{t("home.noNews")}</p>
      {:else}
        <div class="news">
          {#each news as n (n.id)}
            <button class="news-item" onclick={() => (reading = n)}>
              {#if n.imageUrl}
                <img class="thumb" src={n.imageUrl} alt="" loading="lazy" />
              {:else}
                <span class="thumb placeholder"><Icon name="bell" size={22} /></span>
              {/if}
              <span class="news-text">
                <span class="news-meta mono">
                  {#if n.pinned}<span class="pin">{t("home.pinned")}</span>{/if}
                  {newsDate(n.publishedAt)}
                </span>
                <span class="news-title">{n.title}</span>
                <span class="news-excerpt">{excerpt(n.bodyHtml)}</span>
              </span>
            </button>
          {/each}
        </div>
      {/if}
    </section>
  </div>

  <aside class="side">
    <section class="card">
      <div class="card-head">
        <span class="card-title">{t("events.calendar")}</span>
        <span class="mono dim">TWOGG</span>
      </div>
      {#each calendar as e (e.id)}
        <button class="cal-item" class:past={e.ended} class:current={e.id === hero?.id} onclick={() => (chosenId = e.id)}
          use:menu={() => [
            { label: t("menu.showHere"), icon: "home", action: () => (chosenId = e.id) },
            { label: t("home.toEvent"), icon: "calendar", action: () => onEvents(e.id) },
            { label: t("menu.openTwogg"), icon: "external", action: () => open(e.url) },
          ]}>
          <EventIcon event={e} size={38} />
          <span class="cal-text">
            <span class="cal-name">{e.title}</span>
            <span class="cal-line">{calendarLine(e)}</span>
          </span>
          {#if e.id === featured(events)?.id && !e.ended}<span class="new-badge">{t("events.new")}</span>{/if}
        </button>
      {:else}
        <p class="muted small">{twogg.loading ? t("events.loading") : t("events.none")}</p>
      {/each}
      <button class="link more" onclick={() => onEvents()}>{t("home.allEvents")}</button>
    </section>

    {#if server}
      {@const address = server.address}
      <section class="card">
        <div class="card-head">
          <span class="card-title">{t("home.communityServer")}</span>
          {#if status && status !== "offline"}
            <span class="status ok"><span class="square"></span>{t("home.online")}</span>
          {:else if status === "offline"}
            <span class="status"><span class="square off"></span>{t("home.serverOffline")}</span>
          {:else}
            <span class="status"><span class="square off"></span>{t("home.serverChecking")}</span>
          {/if}
        </div>
        <div class="server">
          <ProfileIcon profile={{ name: server.name, icon: "motif:server" }} size={42} />
          <span class="server-text">
            <span class="server-name">{server.name}</span>
            <span class="mono dim">{address}</span>
          </span>
        </div>
        {#if status && status !== "offline"}
          <p class="server-line mono">{t("home.players", { online: status.online, max: status.max })} · {status.version}</p>
        {/if}
        <button class="primary cut join" onclick={() => onJoin(address)} disabled={!selectedId}>
          <Icon name="play" size={14} filled />{t("home.join")}
        </button>
      </section>
    {/if}

    {#if friends.state?.available && online.length > 0}
      <section class="card">
        <div class="card-head">
          <span class="card-title">{t("home.friendsOnline")}</span>
          <span class="mono dim">{online.length}</span>
        </div>
        {#each online.slice(0, 5) as f (f.uuid)}
          <div class="friend">
            <span class="face" style:background={friendColor(f.uuid)}>{f.name[0]?.toUpperCase()}</span>
            <span class="friend-text">
              <span class="friend-name">{f.name}</span>
              <span class="friend-line">
                {f.activity?.playing && f.activity.instance
                  ? t("friends.playing", { instance: f.activity.instance })
                  : t("friends.inLauncher")}
              </span>
            </span>
          </div>
        {/each}
      </section>
    {/if}

    <a class="discord" href="https://twogg.chat" onclick={(e) => (e.preventDefault(), open("https://twogg.chat"))}>
      <Icon name="chat" size={17} />
      <span>{t("events.discord")}</span>
      <Icon name="external" size={14} stroke={2} />
    </a>
  </aside>
</div>

{#if reading}
  {@const item = reading}
  <dialog bind:this={newsDialog} class="news-dialog" oncancel={(e) => (e.preventDefault(), closeNews())}>
    {#if item.imageUrl}<img class="news-image" src={item.imageUrl} alt="" />{/if}
    <div class="dialog-body">
      <span class="mono dim">{newsDate(item.publishedAt)}</span>
      <h2>{item.title}</h2>
      <div class="news-body">{@html item.bodyHtml}</div>
    </div>
    <div class="dialog-actions">
      {#if item.link}
        {@const link = item.link}
        <button class="ghost" onclick={() => open(link)}>{t("home.readMore")}<Icon name="external" size={14} stroke={2} /></button>
      {/if}
      <button class="primary" onclick={closeNews}>{t("common.close")}</button>
    </div>
  </dialog>
{/if}

<style>
  .home {
    height: 100%;
    overflow: auto;
    display: grid;
    grid-template-columns: minmax(0, 1fr) 312px;
    gap: 24px;
    padding: 24px 28px 28px;
    align-items: start;
  }

  .main {
    display: flex;
    flex-direction: column;
    gap: 26px;
    min-width: 0;
  }

  /* Hinweise vom Pinecone-Server */
  .notice {
    --tone: var(--accent);
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 10px 10px 14px;
    margin-bottom: -12px;
    background: color-mix(in srgb, var(--tone) 12%, var(--surface));
    border-left: 3px solid var(--tone);
    color: var(--text);
    font-size: 13px;
  }

  .notice.danger {
    --tone: var(--danger);
  }

  .notice > :global(svg) {
    flex: none;
    color: var(--tone);
  }

  .notice .text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .notice .text span:not(.body) {
    color: var(--text-soft);
  }

  .notice .text .error {
    color: var(--danger);
  }

  .notice .external {
    flex: none;
    display: inline-flex;
    align-items: center;
    gap: 4px;
    white-space: nowrap;
  }

  .notice .small {
    min-height: 34px;
    font-size: 13px;
    white-space: nowrap;
  }

  /* Event-Bühne */
  .hero {
    position: relative;
    height: 300px;
    overflow: hidden;
    background: var(--stage);
    clip-path: var(--cut-2);
  }

  .banner {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .panel {
    position: absolute;
    left: 24px;
    bottom: 24px;
    max-width: min(560px, calc(100% - 48px));
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 12px;
  }

  .with-banner .panel {
    padding: 18px 20px 20px;
    background: var(--bar-glass);
    clip-path: var(--cut-2);
  }

  h1 {
    margin: 0;
    font-weight: 700;
    font-size: 46px;
    line-height: 0.95;
    overflow-wrap: anywhere;
  }

  h1.long {
    font-size: 36px;
  }

  .summary {
    margin: 0;
    font-size: 14.5px;
    line-height: 1.5;
    color: var(--text-soft);
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .actions {
    display: flex;
    align-items: center;
    gap: 14px;
    margin-top: 2px;
  }

  .actions .primary {
    height: 40px;
    padding: 0 18px;
    font-weight: 700;
  }

  .external {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13.5px;
    font-weight: 700;
  }

  /* Abschnitte */
  .block {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .block-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .block-head h2 {
    font-size: 18px;
  }

  .block-head .link {
    min-height: 0;
    padding: 0;
    font-size: 13px;
    font-weight: 600;
  }

  .recent {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 10px;
  }

  .tile {
    position: relative;
    display: flex;
    background: var(--surface);
    clip-path: var(--cut-2);
  }

  .tile:hover {
    background: var(--surface-2);
  }

  .tile.selected {
    background: var(--surface-active);
    box-shadow: inset 0 -3px 0 var(--accent);
  }

  .tile-main {
    flex: 1;
    min-width: 0;
    justify-content: flex-start;
    gap: 12px;
    padding: 12px;
    min-height: 0;
    background: transparent;
    text-align: left;
  }

  .tile-main:hover:not(:disabled) {
    background: transparent;
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

  .tile-when {
    font-size: 12px;
    font-weight: 400;
    color: var(--text-dim);
    margin-top: 2px;
  }

  .tile-play {
    align-self: center;
    width: 36px;
    height: 36px;
    min-height: 0;
    margin-right: 12px;
    padding: 0;
    flex: none;
    background: var(--accent);
    color: var(--ink);
    clip-path: var(--cut-1);
    box-shadow: var(--edge-accent);
    opacity: 0;
    transition: opacity 0.12s;
  }

  .tile:hover .tile-play,
  .tile.selected .tile-play,
  .tile-play:focus-visible {
    opacity: 1;
  }

  .tile-play:hover:not(:disabled) {
    background: var(--accent-hover);
  }

  .empty-recent {
    justify-content: center;
    height: 72px;
  }

  .news {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .news-item {
    justify-content: flex-start;
    align-items: stretch;
    gap: 14px;
    padding: 10px;
    min-height: 0;
    background: var(--surface);
    clip-path: var(--cut-1);
    text-align: left;
  }

  .news-item:hover:not(:disabled) {
    background: var(--surface-2);
  }

  .thumb {
    width: 112px;
    height: 64px;
    flex: none;
    object-fit: cover;
  }

  .thumb.placeholder {
    display: grid;
    place-items: center;
    background: var(--surface-2);
    color: var(--accent);
  }

  .news-text {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .news-meta {
    display: flex;
    gap: 8px;
    align-items: center;
    font-size: 11px;
    font-weight: 400;
    color: var(--text-dim);
  }

  .pin {
    color: var(--accent);
    text-transform: uppercase;
    letter-spacing: 1px;
  }

  .news-title {
    font-size: 14.5px;
    font-weight: 700;
  }

  .news-excerpt {
    font-size: 13px;
    font-weight: 400;
    color: var(--text-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .small {
    margin: 0;
    font-size: 13px;
  }

  /* Seitenspalte */
  .side {
    display: flex;
    flex-direction: column;
    gap: 14px;
    position: sticky;
    top: 0;
  }

  .card {
    padding: 14px;
    background: var(--side);
    border: 1px solid var(--line-strong);
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .card-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 2px;
  }

  .card-title {
    font-family: var(--font-pixel);
    font-weight: 600;
    font-size: 15px;
  }

  .dim {
    font-size: 11px;
    color: var(--text-dim);
  }

  .cal-item {
    justify-content: flex-start;
    gap: 12px;
    padding: 9px;
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
    font-size: 13.5px;
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

  .more {
    align-self: flex-start;
    min-height: 22px;
    padding: 0;
    font-size: 13px;
  }

  .status {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    font-weight: 500;
    color: var(--text-dim);
  }

  .status.ok {
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

  .server {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .server-text {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .server-name {
    font-size: 14px;
    font-weight: 700;
  }

  .server-line {
    margin: 0;
    font-size: 11.5px;
    color: var(--text-muted);
  }

  .join {
    margin-top: 4px;
    height: 36px;
    gap: 8px;
    font-weight: 700;
  }

  .friend {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .face {
    width: 28px;
    height: 28px;
    flex: none;
    display: grid;
    place-items: center;
    font-family: var(--font-pixel);
    font-size: 13px;
    color: #f1e8da;
  }

  .friend-text {
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .friend-name {
    font-size: 13px;
    font-weight: 700;
  }

  .friend-line {
    font-size: 12px;
    color: var(--nadel);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .discord {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 4px 4px;
    font-size: 13px;
    font-weight: 600;
  }

  .discord span {
    flex: 1;
  }

  /* Neuigkeit lesen */
  .news-dialog {
    width: min(620px, calc(100vw - 64px));
    padding: 0;
  }

  .news-image {
    display: block;
    width: 100%;
    max-height: 240px;
    object-fit: cover;
  }

  .news-body {
    font-size: 14.5px;
    line-height: 1.55;
    color: var(--text-soft);
    max-height: 50vh;
    overflow: auto;
  }

  .news-body :global(a) {
    color: var(--accent);
  }

  @media (max-width: 1150px) {
    .home {
      grid-template-columns: minmax(0, 1fr) 280px;
    }

    .recent {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
</style>
