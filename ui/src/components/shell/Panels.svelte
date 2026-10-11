<script lang="ts">
  // Overlays unter der Kopfleiste: Aktivität (laufende Downloads/Starts) und Benachrichtigungen.
  import { t } from "../../lib/i18n.svelte";
  import { dismiss, markAllRead, notices } from "../../lib/notifications.svelte";
  import { installs } from "../../lib/installs.svelte";
  import { runs } from "../../lib/runs.svelte";
  import type { Profile } from "../../lib/types";
  import Icon from "../Icon.svelte";
  import ProfileIcon from "../ProfileIcon.svelte";

  interface Props {
    panel: "activity" | "notifications";
    profiles: Profile[];
    onClose: () => void;
    onOpenProfile: (id: string) => void;
  }
  let { panel, profiles, onClose, onOpenProfile }: Props = $props();

  let root: HTMLElement | undefined = $state();

  const active = $derived(
    Object.entries(runs)
      .filter(([, r]) => r.status !== "idle")
      .map(([id, r]) => ({ id, run: r, profile: profiles.find((p) => p.id === id) }))
      .filter((x) => x.profile),
  );

  $effect(() => {
    if (panel === "notifications") return () => markAllRead();
  });

  function onWindowClick(e: MouseEvent) {
    const target = e.target as HTMLElement;
    // Klicks auf die Buttons der Kopfleiste schalten selbst um.
    if (root && !root.contains(target) && !target.closest("header")) onClose();
  }

  function time(ms: number) {
    return new Date(ms).toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit" });
  }
</script>

<svelte:window onclick={onWindowClick} onkeydown={(e) => e.key === "Escape" && onClose()} />

<div class="panel" bind:this={root} role="dialog" aria-label={panel === "activity" ? t("shell.activity") : t("shell.notifications")}>
  {#if panel === "activity"}
    <h2 class="title">{t("shell.activity")}</h2>
    {#each installs as job (job.id)}
      <div class="item static">
        {#if job.iconUrl}
          <img class="logo" src={job.iconUrl} alt="" width="36" height="36" />
        {:else}
          <ProfileIcon profile={{ name: job.title, icon: null }} size={36} />
        {/if}
        <span class="body">
          <span class="name">{job.title}</span>
          <span class="meta">{t("shell.installing", { target: job.target })}</span>
          <span class="progress"><span class="bar"><span class="fill"></span></span></span>
        </span>
      </div>
    {/each}
    {#each active as { id, run, profile } (id)}
      <button class="item" onclick={() => ((onClose(), onOpenProfile(id)))}>
        <ProfileIcon profile={profile!} size={36} />
        <span class="body">
          <span class="name">{profile!.name}</span>
          {#if run.status === "running"}
            <span class="meta ok">{t("play.running")}</span>
          {:else}
            <span class="meta">{run.stage ? t(`stage.${run.stage}`) : t("play.loading")}</span>
            <span class="progress">
              <progress max={run.files || 1} value={run.done}></progress>
              <span class="mono">{run.files ? Math.round((run.done / run.files) * 100) : 0}%</span>
            </span>
          {/if}
        </span>
      </button>
    {:else}
      {#if installs.length === 0}<p class="empty">{t("shell.noActivity")}</p>{/if}
    {/each}
  {:else}
    <h2 class="title">{t("shell.notifications")}</h2>
    {#each notices.list as n (n.id)}
      <div class="item static" class:unread={!n.read}>
        <span class="kind" class:error={n.kind === "error"}>
          <Icon name={n.kind === "error" ? "close" : n.kind === "update" ? "download" : n.kind === "friend" ? "users" : "bell"} size={18} />
        </span>
        <span class="body">
          <span class="name">{n.title}</span>
          {#if n.text}<span class="meta">{n.text}</span>{/if}
          {#if n.action}
            {@const action = n.action}
            <button class="link" onclick={() => ((onClose(), action.run()))}>{action.label}</button>
          {/if}
        </span>
        <span class="time mono">{time(n.at)}</span>
        <button class="ghost dismiss" aria-label={t("shell.dismiss")} onclick={() => dismiss(n.id)}><Icon name="close" size={14} /></button>
      </div>
    {:else}
      <p class="empty">{t("shell.noNotifications")}</p>
    {/each}
  {/if}
</div>

<style>
  .logo {
    width: 36px;
    height: 36px;
    flex: none;
    object-fit: cover;
  }

  .bar {
    position: relative;
    flex: 1;
    height: 4px;
    overflow: hidden;
    background: var(--line-strong);
  }

  .fill {
    position: absolute;
    inset: 0 auto 0 0;
    width: 40%;
    background: var(--accent);
    animation: slide 1.1s ease-in-out infinite;
  }

  @keyframes slide {
    from {
      left: -40%;
    }
    to {
      left: 100%;
    }
  }

  .panel {
    position: absolute;
    top: 8px;
    right: 12px;
    z-index: 30;
    width: 360px;
    max-height: calc(100% - 16px);
    overflow-y: auto;
    padding: 14px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    background: var(--bar-glass);
    border: 1px solid var(--line-strong);
  }

  .title {
    font-size: 16px;
    margin-bottom: 6px;
  }

  .item {
    display: flex;
    align-items: flex-start;
    justify-content: flex-start;
    gap: 12px;
    padding: 10px;
    min-height: 0;
    background: var(--surface);
    clip-path: var(--cut-1);
    text-align: left;
    font-weight: 400;
  }

  .item.unread {
    box-shadow: inset 0 -3px 0 var(--accent);
  }

  .body {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .name {
    font-size: 14px;
    font-weight: 700;
  }

  .meta {
    font-size: 12px;
    color: var(--text-muted);
  }

  .meta.ok {
    color: var(--nadel);
  }

  .progress {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 11px;
    color: var(--accent-fg);
  }

  .progress progress {
    flex: 1;
  }

  .kind {
    width: 32px;
    height: 32px;
    flex: none;
    display: grid;
    place-items: center;
    background: var(--surface-2);
    color: var(--accent);
  }

  .kind.error {
    color: var(--danger);
  }

  .time {
    font-size: 11px;
    color: var(--text-dim);
  }

  .dismiss {
    width: 26px;
    min-height: 26px;
    padding: 0;
  }

  .link {
    align-self: flex-start;
    min-height: 24px;
    padding: 0;
    font-size: 13px;
  }

  .empty {
    margin: 4px 0;
    color: var(--text-dim);
  }
</style>
