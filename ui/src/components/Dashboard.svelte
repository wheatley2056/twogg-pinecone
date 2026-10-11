<script lang="ts">
  import { onMount } from "svelte";
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { api } from "../lib/api";
  import { formatAgo, formatDuration, t } from "../lib/i18n.svelte";
  import { runOf } from "../lib/runs.svelte";
  import { settings } from "../lib/settings.svelte";
  import { externalLinks } from "../lib/links";
  import type { Dashboard, NewsItem, PackUpdate, Profile, ServerEntry, ServerStatus, WidgetId } from "../lib/types";
  import ProfileIcon from "./ProfileIcon.svelte";

  interface Props {
    data: Dashboard;
    hasAccount: boolean;
    /** Profil öffnen; mit `server` direkt per Quick-Join starten. */
    onPlay: (profileId: string, server?: string) => void;
    onOpen: (profileId: string) => void;
  }
  let { data, hasAccount, onPlay, onOpen }: Props = $props();

  // Status je Adresse: undefined = lädt, null = nicht erreichbar.
  let statuses: Record<string, ServerStatus | null | undefined> = $state({});
  let packUpdates = $state<{ profile: Profile; update: PackUpdate }[] | null>(null);
  // null = lädt, false = Server nicht erreichbar.
  let news = $state<NewsItem[] | null | false>(null);

  const widgets = $derived(
    (settings.current?.widgets ?? []).filter((w) => w.visible).map((w) => w.id as WidgetId),
  );
  // Gleiche Adresse aus mehreren Profilen nur einmal pingen.
  const uniqueAddresses = $derived([...new Set(data.servers.map((s) => s.address.toLowerCase()))]);
  const maxPlaytime = $derived(Math.max(1, ...data.recent.map((r) => r.stats.playtimeSecs)));

  onMount(() => {
    if (widgets.includes("news")) {
      api
        .twoggNews(5)
        .then((list) => (news = list))
        .catch(() => (news = false));
    }
    if (widgets.includes("packs")) {
      api
        .packUpdatesAll()
        .then((list) => {
          packUpdates = list;
        })
        .catch(() => {
          packUpdates = [];
        });
    }
    for (const address of uniqueAddresses) {
      statuses[address] = undefined;
      api
        .pingServer(address)
        .then((s) => {
          statuses[address] = s;
        })
        .catch(() => {
          statuses[address] = null;
        });
    }
  });

  function statusOf(s: ServerEntry) {
    return statuses[s.address.toLowerCase()];
  }
</script>

<section class="dashboard">
  {#if data}
    {#each widgets as widget (widget)}
      {#if widget === "recent"}
        <div class="card">
          <h2>{t("dash.recent")}</h2>
          {#if data.recent.length === 0}
            <p class="muted">{t("nav.noProfiles")}</p>
          {:else}
            <div class="recent">
              {#each data.recent.slice(0, 4) as { profile, stats } (profile.id)}
                {@const run = runOf(profile.id)}
                <div class="recent-item">
                  <button class="recent-open" onclick={() => onOpen(profile.id)}>
                    <ProfileIcon {profile} size={40} />
                    <span class="recent-text">
                      <span class="name">{profile.name}</span>
                      <span class="muted small">
                        {profile.minecraft} · {profile.loader ? (profile.loader.kind === "fabric" ? "Fabric" : "Quilt") : t("common.vanilla")}
                      </span>
                      <span class="muted small">
                        {stats.lastPlayed
                          ? t("dash.playedAgo", { ago: formatAgo(stats.lastPlayed), time: formatDuration(stats.playtimeSecs) })
                          : t("dash.neverPlayed")}
                      </span>
                    </span>
                  </button>
                  <button
                    class="primary small-play"
                    disabled={!hasAccount || run.status !== "idle"}
                    onclick={() => onPlay(profile.id)}
                    title={run.status !== "idle" ? t("nav.running") : undefined}>▶</button
                  >
                </div>
              {/each}
            </div>
          {/if}
        </div>
      {:else if widget === "packs"}
        {#if packUpdates === null || packUpdates.length > 0}
          <div class="card">
            <h2>{t("dash.packs")}</h2>
            {#if packUpdates === null}
              <p class="muted">{t("dash.packChecking")}</p>
            {:else}
              <ul class="servers">
                {#each packUpdates as { profile, update } (profile.id)}
                  <li>
                    <ProfileIcon {profile} size={40} />
                    <div class="server-text">
                      <span class="name">{profile.name}</span>
                      <div class="muted small">{update.current} → {update.latest}</div>
                    </div>
                    <button onclick={() => onOpen(profile.id)}>{t("pack.updateTo", { version: update.latest })}</button>
                  </li>
                {/each}
              </ul>
            {/if}
          </div>
        {/if}
      {:else if widget === "servers"}
        <div class="card">
          <h2>{t("dash.servers")}</h2>
          {#if data.servers.length === 0}
            <p class="muted">{t("dash.noServers")}</p>
          {:else}
            <ul class="servers">
              {#each data.servers as server (server.profileId + server.address)}
                {@const status = statusOf(server)}
                {@const icon = status?.favicon ?? (server.icon ? `data:image/png;base64,${server.icon}` : null)}
                <li>
                  {#if icon}
                    <img class="favicon" src={icon} alt="" width="40" height="40" />
                  {:else}
                    <span class="favicon placeholder"></span>
                  {/if}
                  <div class="server-text">
                    <div class="server-head">
                      <span class="name">{server.name}</span>
                      {#if server.last}<span class="badge accent">{t("dash.lastServer")}</span>{/if}
                    </div>
                    <div class="muted small motd">
                      {#if status === undefined}
                        {t("common.loading")}
                      {:else if status === null}
                        <span class="offline">● {t("dash.offline")}</span>
                      {:else}
                        <span class="online">●</span>
                        {t("dash.players", { online: status.online, max: status.max })} · {status.pingMs} ms
                        {#if status.motd} · {status.motd.split("\n")[0]}{/if}
                      {/if}
                    </div>
                    <div class="muted small">{server.address} · {t("dash.inProfile", { profile: server.profileName })}</div>
                  </div>
                  <button
                    disabled={!hasAccount || runOf(server.profileId).status !== "idle"}
                    onclick={() => onPlay(server.profileId, server.address)}>{t("dash.join")}</button
                  >
                </li>
              {/each}
            </ul>
          {/if}
        </div>
      {:else if widget === "news"}
        <div class="card">
          <h2>{t("dash.news")}</h2>
          {#if news === null}
            <p class="muted">{t("common.loading")}</p>
          {:else if news === false}
            <p class="muted">{t("dash.newsOffline")}</p>
          {:else if news.length === 0}
            <p class="muted">{t("dash.noNews")}</p>
          {:else}
            <ul class="news">
              {#each news as n (n.id)}
                <li class:pinned={n.pinned}>
                  {#if n.imageUrl}<img src={n.imageUrl} alt="" loading="lazy" />{/if}
                  <div class="news-text">
                    <div class="news-head">
                      <strong>{n.title}</strong>
                      <span class="muted small">{formatAgo(n.publishedAt)}</span>
                    </div>
                    <div class="news-body" use:externalLinks>{@html n.bodyHtml}</div>
                    {#if n.link}
                      <a class="more" href={n.link} use:externalLinks>{t("dash.readMore")}</a>
                    {/if}
                  </div>
                </li>
              {/each}
            </ul>
          {/if}
        </div>
      {:else if widget === "screenshots"}
        <div class="card">
          <h2>{t("dash.screenshots")}</h2>
          {#if data.screenshots.length === 0}
            <p class="muted">{t("dash.noScreenshots")}</p>
          {:else}
            <div class="shots">
              {#each data.screenshots as shot (shot.path)}
                <button class="shot" onclick={() => api.openScreenshot(shot.path)} title={formatAgo(shot.modified)}>
                  <img src={convertFileSrc(shot.path)} alt="" loading="lazy" />
                </button>
              {/each}
            </div>
          {/if}
        </div>
      {:else if widget === "playtime"}
        <div class="card">
          <div class="card-head">
            <h2>{t("dash.playtime")}</h2>
            <span class="muted">{t("dash.total", { time: formatDuration(data.totalPlaytimeSecs) })}</span>
          </div>
          {#if data.totalPlaytimeSecs === 0}
            <p class="muted">{t("dash.noRecent")}</p>
          {:else}
            <ul class="playtime">
              {#each data.recent.filter((r) => r.stats.playtimeSecs > 0).sort((a, b) => b.stats.playtimeSecs - a.stats.playtimeSecs) as { profile, stats } (profile.id)}
                <li>
                  <span class="pt-name">{profile.name}</span>
                  <span class="pt-bar"><span style:width="{(stats.playtimeSecs / maxPlaytime) * 100}%" style:background={profile.color ?? "var(--accent)"}></span></span>
                  <span class="pt-time">{formatDuration(stats.playtimeSecs)}</span>
                  <span class="muted small">{t("dash.sessions", { count: stats.sessions })}</span>
                </li>
              {/each}
            </ul>
          {/if}
        </div>
      {/if}
    {/each}
  {/if}
</section>

<style>
  .news {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .news li {
    display: flex;
    gap: 12px;
    padding: 10px 12px;
    border-radius: var(--radius);
    background: var(--surface-2);
  }

  .news li.pinned {
    border-left: 3px solid var(--accent);
  }

  .news img {
    width: 120px;
    height: 68px;
    object-fit: cover;
    border-radius: 6px;
    flex: none;
  }

  .news-text {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
  }

  .news-head {
    display: flex;
    gap: 10px;
    align-items: baseline;
    flex-wrap: wrap;
  }

  .news-body :global(p) {
    margin: 0;
  }

  .news-body :global(a),
  .more {
    color: var(--accent);
  }

  .more {
    font-size: 13px;
  }

  .dashboard {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  h2 {
    font-size: 15px;
  }

  .card {
    background: color-mix(in srgb, var(--surface) 92%, transparent);
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 16px 18px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .card p {
    margin: 0;
  }

  .card-head {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
  }

  .small {
    font-size: 12px;
  }

  .name {
    font-weight: 600;
  }

  .recent {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(230px, 1fr));
    gap: 10px;
  }

  .recent-item {
    display: flex;
    align-items: center;
    gap: 8px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 8px;
  }

  .recent-open {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 10px;
    border: none;
    background: transparent;
    text-align: left;
    padding: 2px;
    min-width: 0;
  }

  .recent-text {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .recent-text > span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .small-play {
    padding: 8px 12px;
  }

  .servers {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .servers li {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px;
    border-radius: var(--radius);
    background: var(--surface-2);
  }

  .favicon {
    width: 40px;
    height: 40px;
    border-radius: 6px;
    image-rendering: pixelated;
    flex-shrink: 0;
  }

  .placeholder {
    background: var(--surface-3);
  }

  .server-text {
    flex: 1;
    min-width: 0;
  }

  .server-head {
    display: flex;
    gap: 8px;
    align-items: center;
  }

  .motd {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .online {
    color: var(--accent);
  }

  .offline {
    color: var(--danger);
  }

  .badge.accent {
    background: var(--accent);
    color: var(--accent-text);
  }

  .shots {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
    gap: 8px;
  }

  .shot {
    padding: 0;
    border: none;
    border-radius: var(--radius);
    overflow: hidden;
    aspect-ratio: 16 / 9;
    background: var(--surface-3);
  }

  .shot img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
    transition: transform 0.15s;
  }

  .shot:hover img {
    transform: scale(1.04);
  }

  .playtime {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .playtime li {
    display: grid;
    grid-template-columns: minmax(120px, 200px) 1fr auto auto;
    align-items: center;
    gap: 12px;
  }

  .pt-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .pt-bar {
    height: 8px;
    border-radius: 4px;
    background: var(--surface-3);
    overflow: hidden;
  }

  .pt-bar span {
    display: block;
    height: 100%;
    border-radius: 4px;
  }

  .pt-time {
    font-variant-numeric: tabular-nums;
  }
</style>
