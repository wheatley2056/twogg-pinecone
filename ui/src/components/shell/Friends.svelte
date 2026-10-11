<script lang="ts">
  // Freunde in der Spielleiste: Avatar-Stapel „2 online“, Panel nach oben mit Liste, Anfragen
  // und Beitreten (Design-System 9.2, rechts).
  import { api, errorText } from "../../lib/api";
  import { friendColor, friends, refreshFriends } from "../../lib/friends.svelte";
  import { t } from "../../lib/i18n.svelte";
  import type { Friend } from "../../lib/types";
  import Icon from "../Icon.svelte";

  interface Props {
    /** Auf den Server eines Freundes mit der gewählten Instanz verbinden. */
    onJoin: (server: string) => void;
  }
  let { onJoin }: Props = $props();

  let open = $state(false);
  let root: HTMLElement | undefined = $state();
  let name = $state("");
  let busy = $state(false);
  let message = $state<string | null>(null);
  let error = $state<string | null>(null);

  const list = $derived(friends.state?.friends ?? []);
  const online = $derived(list.filter((f) => f.online));
  const stack = $derived((online.length > 0 ? online : list).slice(0, 2));
  const rest = $derived(Math.max(0, list.length - stack.length));
  const incoming = $derived(friends.state?.incoming ?? []);

  function onWindowClick(e: MouseEvent) {
    if (open && root && !root.contains(e.target as Node)) open = false;
  }

  async function run(action: () => Promise<unknown>, ok?: string) {
    busy = true;
    error = message = null;
    try {
      await action();
      if (ok) message = ok;
      await refreshFriends();
    } catch (e) {
      error = errorText(e);
    } finally {
      busy = false;
    }
  }

  async function add(e: SubmitEvent) {
    e.preventDefault();
    const n = name.trim();
    if (!/^[A-Za-z0-9_]{1,16}$/.test(n)) return;
    await run(async () => {
      const status = await api.friendRequest(n);
      message = status === "friends" ? t("friends.nowFriends", { name: n }) : t("friends.requested", { name: n });
      name = "";
    });
  }

  function status(f: Friend): string {
    if (!f.online) return t("friends.offline");
    if (f.activity?.playing && f.activity.instance) return t("friends.playing", { instance: f.activity.instance });
    return t("friends.inLauncher");
  }
</script>

<svelte:window onclick={onWindowClick} />

<div class="friends" bind:this={root}>
  <button class="trigger" onclick={() => ((open = !open), open && refreshFriends())} aria-expanded={open} aria-label={t("friends.title")}>
    <span class="stack" aria-hidden="true">
      {#each stack as f (f.uuid)}
        <span class="face" style:background={friendColor(f.uuid)}></span>
      {/each}
      {#if rest > 0}<span class="face more">+{rest}</span>{/if}
      {#if list.length === 0}<span class="face empty"><Icon name="users" size={14} /></span>{/if}
    </span>
    <span class="label">
      <span class="count">{friends.state?.available === false ? t("friends.title") : t("friends.online", { count: online.length })}</span>
      <span class="sub">{incoming.length === 1
          ? t("friends.requestOne")
          : incoming.length > 1
            ? t("friends.requests", { count: incoming.length })
            : t("friends.title")}</span>
    </span>
    {#if incoming.length > 0}<span class="dot"></span>{/if}
  </button>

  {#if open}
    <div class="panel" role="dialog" aria-label={t("friends.title")}>
      <div class="head">
        <h2>{t("friends.title")}</h2>
        <span class="section-label">PINECONE</span>
      </div>

      {#if friends.state?.available === false}
        <p class="hint">{t("friends.needMicrosoft")}</p>
      {:else if friends.error && !friends.state}
        <p class="error">{friends.error}</p>
      {:else}
        <form onsubmit={add}>
          <input bind:value={name} placeholder={t("friends.addPlaceholder")} maxlength="16" spellcheck="false" aria-label={t("friends.add")} />
          <button class="primary cut" type="submit" disabled={busy || !/^[A-Za-z0-9_]{1,16}$/.test(name.trim())}>
            <Icon name="user-plus" size={16} />{t("friends.add")}
          </button>
        </form>
        {#if message}<p class="ok">{message}</p>{/if}
        {#if error}<p class="error">{error}</p>{/if}

        {#if incoming.length > 0}
          <span class="section-label">{t("friends.incoming")}</span>
          {#each incoming as r (r.uuid)}
            <div class="row">
              <span class="face" style:background={friendColor(r.uuid)}>{r.name[0].toUpperCase()}</span>
              <span class="who"><span class="name">{r.name}</span><span class="meta">{t("friends.wantsToBe")}</span></span>
              <button class="primary small" disabled={busy} onclick={() => run(() => api.friendAccept(r.uuid))}>{t("friends.accept")}</button>
              <button class="ghost small" disabled={busy} aria-label={t("friends.decline")} onclick={() => run(() => api.friendDecline(r.uuid))}>
                <Icon name="close" size={14} />
              </button>
            </div>
          {/each}
        {/if}

        <span class="section-label">{t("friends.list", { count: list.length })}</span>
        <div class="list">
          {#each list as f (f.uuid)}
            <div class="row" class:offline={!f.online}>
              <span class="face" style:background={friendColor(f.uuid)}>
                {f.name[0].toUpperCase()}
                {#if f.online}<span class="online"></span>{/if}
              </span>
              <span class="who">
                <span class="name">{f.name}</span>
                <span class="meta" class:live={f.online && f.activity?.playing}>{status(f)}</span>
              </span>
              {#if f.online && f.activity?.server}
                {@const server = f.activity.server}
                <button class="small" onclick={() => ((open = false), onJoin(server))} title={server}>{t("friends.join")}</button>
              {/if}
              <button class="ghost small" aria-label={t("friends.remove", { name: f.name })} title={t("friends.remove", { name: f.name })}
                disabled={busy} onclick={() => run(() => api.friendRemove(f.uuid))}>
                <Icon name="close" size={14} />
              </button>
            </div>
          {:else}
            <p class="hint">{t("friends.empty")}</p>
          {/each}
        </div>

        {#if (friends.state?.outgoing ?? []).length > 0}
          <span class="section-label">{t("friends.outgoing")}</span>
          {#each friends.state?.outgoing ?? [] as r (r.uuid)}
            <div class="row offline">
              <span class="face" style:background={friendColor(r.uuid)}>{r.name[0].toUpperCase()}</span>
              <span class="who"><span class="name">{r.name}</span><span class="meta">{t("friends.pending")}</span></span>
              <button class="ghost small" disabled={busy} onclick={() => run(() => api.friendDecline(r.uuid))}>{t("friends.cancel")}</button>
            </div>
          {/each}
        {/if}
      {/if}
    </div>
  {/if}
</div>

<style>
  .friends {
    position: relative;
  }

  .trigger {
    position: relative;
    gap: 12px;
    min-height: 56px;
    padding: 0 4px;
    background: transparent;
    text-align: left;
  }

  .trigger:hover:not(:disabled) {
    background: transparent;
  }

  .stack {
    display: flex;
  }

  .face {
    position: relative;
    width: 28px;
    height: 28px;
    flex: none;
    display: grid;
    place-items: center;
    font-family: var(--font-pixel);
    font-size: 13px;
    color: var(--text);
    outline: 2px solid var(--bar);
  }

  .stack .face + .face {
    margin-left: -10px;
  }

  .face.more {
    background: var(--surface-2);
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--text-soft);
  }

  .face.empty {
    background: var(--surface-2);
    color: var(--text-dim);
  }

  .label {
    display: flex;
    flex-direction: column;
    line-height: 1.25;
  }

  .count {
    font-size: 13px;
    font-weight: 700;
  }

  .sub {
    font-size: 12px;
    font-weight: 500;
    color: var(--text-dim);
  }

  .dot {
    position: absolute;
    top: 8px;
    right: -6px;
    width: 7px;
    height: 7px;
    background: var(--accent);
  }

  .panel {
    position: absolute;
    right: 0;
    bottom: calc(100% + 14px);
    z-index: 20;
    width: 360px;
    max-height: min(560px, 72vh);
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 16px;
    background: var(--bar);
    border: 1px solid var(--line-strong);
  }

  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .head h2 {
    font-size: 16px;
  }

  form {
    display: flex;
    gap: 6px;
  }

  form button {
    flex: none;
  }

  .list {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px;
    background: var(--surface);
    clip-path: var(--cut-1);
  }

  .row .face {
    width: 34px;
    height: 34px;
    outline: none;
  }

  .row.offline {
    background: var(--surface-past);
  }

  .row.offline .name {
    color: var(--text-soft);
  }

  .online {
    position: absolute;
    right: -2px;
    bottom: -2px;
    width: 9px;
    height: 9px;
    background: var(--nadel);
    outline: 2px solid var(--surface);
  }

  .who {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .name {
    font-size: 14px;
    font-weight: 700;
  }

  .meta {
    font-size: 12px;
    color: var(--text-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .meta.live {
    color: var(--nadel);
  }

  .small {
    min-height: 30px;
    padding: 0 10px;
    font-size: 13px;
  }

  .ghost.small {
    padding: 0 8px;
  }

  .hint {
    margin: 2px 0;
    color: var(--text-dim);
    font-size: 13px;
  }

  .ok {
    margin: 0;
    color: var(--nadel);
    font-size: 13px;
  }
</style>
