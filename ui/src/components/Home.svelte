<script lang="ts">
  import { onMount } from "svelte";
  import { api, errorText } from "../lib/api";
  import { formatAgo, formatDuration, t } from "../lib/i18n.svelte";
  import { runOf, stopGame } from "../lib/runs.svelte";
  import type { Dashboard as DashboardData, Profile } from "../lib/types";
  import Dashboard from "./Dashboard.svelte";
  import Icon from "./Icon.svelte";
  import ProfileIcon from "./ProfileIcon.svelte";

  interface Props {
    profiles: Profile[];
    hasAccount: boolean;
    onPlay: (id: string, server?: string) => void;
    onOpen: (id: string) => void;
  }
  let { profiles, hasAccount, onPlay, onOpen }: Props = $props();

  let data = $state<DashboardData | null>(null);
  let error = $state<string | null>(null);
  let chosenId = $state<string | null>(null);

  // Standard: zuletzt gespieltes Profil, sonst das erste.
  const currentId = $derived(chosenId ?? data?.recent.find((r) => r.stats.lastPlayed)?.profile.id ?? profiles[0]?.id ?? null);
  const current = $derived(profiles.find((p) => p.id === currentId) ?? null);
  const currentStats = $derived(data?.recent.find((r) => r.profile.id === currentId)?.stats ?? null);
  const run = $derived(current ? runOf(current.id) : null);

  onMount(async () => {
    try {
      data = await api.dashboard();
    } catch (e) {
      error = errorText(e);
    }
  });

  function loaderLabel(p: Profile) {
    return p.loader ? `${p.loader.kind === "fabric" ? "Fabric" : "Quilt"} ${p.loader.version}` : t("common.vanilla");
  }
</script>

<div class="home">
  {#if current}
    <section class="hero" style:--profile-color={current.color ?? "var(--accent)"}>
      <div class="hero-info">
        <ProfileIcon profile={current} size={72} />
        <div class="hero-text">
          <select class="profile-select" value={current.id} onchange={(e) => (chosenId = e.currentTarget.value)} aria-label={t("home.selectProfile")}>
            {#each profiles as p (p.id)}
              <option value={p.id}>{p.name}</option>
            {/each}
          </select>
          <div class="hero-meta">
            Minecraft {current.minecraft} · {loaderLabel(current)}
          </div>
          <div class="hero-meta muted">
            {#if currentStats?.lastPlayed}
              {t("home.lastPlayed", { ago: formatAgo(currentStats.lastPlayed) })} · {formatDuration(currentStats.playtimeSecs)}
            {:else}
              {t("home.neverPlayed")}
            {/if}
          </div>
        </div>
      </div>

      <div class="hero-actions">
        {#if run && run.status === "running"}
          <button class="danger-solid big-play" onclick={() => stopGame(current.id)}>
            <Icon name="stop" size={22} filled />{t("profile.stop").replace("■ ", "")}
          </button>
          <button class="ghost" onclick={() => onOpen(current.id)}><Icon name="log" />{t("profile.tabLog")}</button>
        {:else if run && run.status === "preparing"}
          <button class="primary big-play" disabled>{t("profile.preparing")}</button>
          <button class="ghost" onclick={() => onOpen(current.id)}><Icon name="log" />{t("profile.tabLog")}</button>
        {:else}
          <button class="primary big-play" disabled={!hasAccount} onclick={() => onPlay(current.id)}>
            <Icon name="play" size={22} filled />{t("home.play")}
          </button>
          {#if !hasAccount}<span class="muted small">{t("profile.needAccount")}</span>{/if}
        {/if}
      </div>
    </section>
  {/if}

  {#if error}<p class="error">{error}</p>{/if}
  {#if data}
    <Dashboard {data} {hasAccount} {onPlay} {onOpen} />
  {/if}
</div>

<style>
  .home {
    display: flex;
    flex-direction: column;
    gap: 18px;
    overflow-y: auto;
    padding-right: 4px;
    min-height: 0;
  }

  .hero {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 24px;
    padding: 28px 30px;
    border-radius: 14px;
    border: 1px solid var(--border);
    background:
      radial-gradient(120% 140% at 0% 0%, color-mix(in srgb, var(--profile-color) 32%, transparent), transparent 60%),
      linear-gradient(135deg, color-mix(in srgb, var(--surface) 92%, transparent), color-mix(in srgb, var(--surface-2) 92%, transparent));
    flex-wrap: wrap;
  }

  .hero-info {
    display: flex;
    align-items: center;
    gap: 18px;
    min-width: 0;
  }

  .hero-text {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
  }

  .profile-select {
    font-size: 24px;
    font-weight: 700;
    border: none;
    background: transparent;
    padding: 0 26px 0 0;
    width: auto;
    max-width: 100%;
    cursor: pointer;
  }

  .profile-select option {
    font-size: 14px;
    background: var(--surface);
  }

  .hero-meta {
    font-size: 13px;
  }

  .hero-actions {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 8px;
  }

  .big-play {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 10px;
    font-size: 20px;
    font-weight: 800;
    letter-spacing: 0.06em;
    padding: 16px 44px;
    min-width: 220px;
    border-radius: 12px;
    box-shadow: 0 6px 24px color-mix(in srgb, var(--accent) 35%, transparent);
  }

  .hero-actions .ghost {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .small {
    font-size: 12px;
  }
</style>
