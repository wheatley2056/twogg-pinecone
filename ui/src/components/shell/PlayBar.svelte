<script lang="ts">
  // Spielleiste (88 px) auf jedem Screen: Instanzwahl, SPIELEN, Freunde, Konto (Design-System 9.2).
  import type { Snippet } from "svelte";
  import { t } from "../../lib/i18n.svelte";
  import { byRecent, isTwogg, versionLabel } from "../../lib/profiles";
  import { runOf, stopGame } from "../../lib/runs.svelte";
  import type { Account, Profile } from "../../lib/types";
  import AccountMenu from "../AccountMenu.svelte";
  import Icon from "../Icon.svelte";
  import ProfileIcon from "../ProfileIcon.svelte";

  interface Props {
    profiles: Profile[];
    selected: Profile | null;
    accounts: Account[];
    msaConfigured: boolean;
    onSelect: (id: string) => void;
    onPlay: (id: string) => void;
    onCreate: () => void;
    onLibrary: () => void;
    onAccountsChange: () => void;
    friends?: Snippet;
  }
  let { profiles, selected, accounts, msaConfigured, onSelect, onPlay, onCreate, onLibrary, onAccountsChange, friends }: Props =
    $props();

  let pickerOpen = $state(false);
  let filter = $state("");
  let picker: HTMLElement | undefined = $state();
  let accountMenu: ReturnType<typeof AccountMenu> | undefined = $state();

  const run = $derived(selected ? runOf(selected.id) : null);
  const hasAccount = $derived(accounts.some((a) => a.active));
  const progress = $derived(run && run.files > 0 ? Math.round((run.done / run.files) * 100) : null);

  const groups = $derived.by(() => {
    const q = filter.trim().toLowerCase();
    const list = profiles.filter((p) => !q || p.name.toLowerCase().includes(q)).sort(byRecent);
    return [
      { id: "twogg", label: t("library.groupTwogg"), items: list.filter(isTwogg) },
      { id: "own", label: t("library.groupOwn"), items: list.filter((p) => !isTwogg(p)) },
    ].filter((g) => g.items.length > 0);
  });

  function onWindowClick(e: MouseEvent) {
    if (pickerOpen && picker && !picker.contains(e.target as Node)) pickerOpen = false;
  }

  function pick(id: string) {
    onSelect(id);
    pickerOpen = false;
    filter = "";
  }

  function play() {
    if (!selected || !run) return;
    if (run.status === "running") {
      stopGame(selected.id).catch(() => {});
      return;
    }
    if (!hasAccount) {
      accountMenu?.show();
      return;
    }
    onPlay(selected.id);
  }

  const playLabel = $derived.by(() => {
    if (!run || run.status === "idle") return t("play.play");
    if (run.status === "running") return run.stopping ? t("play.stopping") : t("play.stop");
    return progress !== null ? t("play.loadingPercent", { percent: progress }) : t("play.loading");
  });
</script>

<svelte:window onclick={onWindowClick} />

<footer class="bar">
  <div class="left" bind:this={picker}>
    <button
      class="instance"
      onclick={() => (pickerOpen = !pickerOpen)}
      aria-expanded={pickerOpen}
      aria-haspopup="listbox"
      aria-label={t("play.chooseInstance")}
    >
      {#if selected}
        <ProfileIcon profile={selected} size={48} />
        <span class="info">
          <span class="name">{selected.name}</span>
          <span class="version mono">{versionLabel(selected)}{isTwogg(selected) ? ` · ${t("play.event")}` : ""}</span>
        </span>
      {:else}
        <span class="info"><span class="name">{t("play.noInstance")}</span></span>
      {/if}
      <span class="chevron"><Icon name="chevron-up" size={16} /></span>
    </button>

    {#if pickerOpen}
      <div class="picker" role="listbox" aria-label={t("play.chooseInstance")}>
        {#if profiles.length > 6}
          <input class="filter" bind:value={filter} placeholder={t("library.filter")} aria-label={t("library.filter")} />
        {/if}
        <div class="list">
          {#each groups as g (g.id)}
            <span class="section-label">{g.label}</span>
            {#each g.items as p (p.id)}
              <button class="row" class:active={p.id === selected?.id} role="option" aria-selected={p.id === selected?.id} onclick={() => pick(p.id)}>
                <ProfileIcon profile={p} size={36} />
                <span class="info">
                  <span class="row-name">{p.name}</span>
                  <span class="version mono">{versionLabel(p)}</span>
                </span>
                {#if runOf(p.id).status === "running"}<span class="live">{t("play.running")}</span>{/if}
              </button>
            {/each}
          {/each}
          {#if groups.length === 0}<p class="empty">{t("library.noMatch")}</p>{/if}
        </div>
        <div class="picker-actions">
          <button class="dashed" onclick={() => ((pickerOpen = false), onCreate())}><Icon name="plus" size={16} />{t("library.newInstance")}</button>
          <button class="ghost" onclick={() => ((pickerOpen = false), onLibrary())}>{t("play.openLibrary")}</button>
        </div>
      </div>
    {/if}
  </div>

  <div class="center">
    <button
      class="play"
      class:running={run?.status === "running"}
      class:preparing={run?.status === "preparing"}
      disabled={!selected || run?.status === "preparing" || run?.stopping}
      onclick={play}
      title={!hasAccount && selected ? t("play.needAccount") : undefined}
    >
      {#if run?.status === "running"}
        <Icon name="stop" size={20} filled />
      {:else if run?.status !== "preparing"}
        <svg width="22" height="22" viewBox="0 0 24 24" aria-hidden="true"><path d="M6 4l14 8-14 8z" fill="currentColor" /></svg>
      {/if}
      <span>{playLabel}</span>
      {#if run?.status === "preparing"}
        <span class="meter" style:width="{progress ?? 8}%"></span>
      {/if}
    </button>
    {#if run?.error}<span class="play-error" role="alert">{run.error}</span>{/if}
  </div>

  <div class="right">
    {@render friends?.()}
    <div class="sep"></div>
    <AccountMenu bind:this={accountMenu} {accounts} {msaConfigured} onChange={onAccountsChange} />
  </div>
</footer>

<style>
  .bar {
    display: grid;
    grid-template-columns: 1fr auto 1fr;
    align-items: center;
    gap: 20px;
    padding: 0 20px;
    background: var(--bar);
    border-top: 1px solid var(--line);
    min-width: 0;
  }

  .left {
    position: relative;
    min-width: 0;
  }

  .instance {
    width: min(300px, 100%);
    min-height: 64px;
    justify-content: flex-start;
    gap: 12px;
    padding: 8px;
    background: var(--surface);
    clip-path: var(--cut-2);
    text-align: left;
  }

  .instance:hover:not(:disabled) {
    background: var(--surface-2);
  }

  .info {
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
    flex: 1;
  }

  .name {
    font-size: 15px;
    font-weight: 700;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .version {
    font-size: 12px;
    font-weight: 400;
    color: var(--text-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .chevron {
    color: var(--text-muted);
    padding-right: 6px;
  }

  .picker {
    position: absolute;
    left: 0;
    bottom: calc(100% + 12px);
    z-index: 20;
    width: 340px;
    max-height: min(520px, 70vh);
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 12px;
    background: var(--bar);
    border: 1px solid var(--line-strong);
  }

  .list {
    display: flex;
    flex-direction: column;
    gap: 2px;
    overflow-y: auto;
    min-height: 0;
  }

  .list .section-label {
    padding: 8px 4px 4px;
  }

  .row {
    justify-content: flex-start;
    gap: 10px;
    padding: 6px 8px;
    min-height: 0;
    background: transparent;
    text-align: left;
  }

  .row:hover:not(:disabled) {
    background: var(--surface);
  }

  .row.active {
    background: var(--surface-active);
    clip-path: polygon(7px 0, 100% 0, 100% 100%, 0 100%, 0 7px);
  }

  .row.active .row-name {
    color: var(--accent-hover);
  }

  .row-name {
    font-size: 14px;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .live {
    font-size: 11px;
    color: var(--nadel);
  }

  .empty {
    margin: 8px 4px;
    color: var(--text-dim);
  }

  .picker-actions {
    display: flex;
    gap: 6px;
    border-top: 1px solid var(--line);
    padding-top: 10px;
  }

  .picker-actions .dashed {
    flex: 1;
  }

  .center {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
  }

  .play {
    position: relative;
    width: 290px;
    height: 62px;
    gap: 14px;
    background: var(--accent);
    color: var(--ink);
    font-family: var(--font-pixel);
    font-weight: 700;
    font-size: 29px;
    letter-spacing: 3px;
    text-transform: uppercase;
    clip-path: var(--cut-play);
    box-shadow:
      inset 0 -5px 0 var(--accent-edge),
      inset 0 2px 0 var(--accent-light);
    overflow: hidden;
  }

  .play:hover:not(:disabled) {
    background: var(--accent-hover);
  }

  .play:disabled {
    opacity: 1;
  }

  .play:disabled:not(.preparing) {
    filter: saturate(0.4) brightness(0.8);
  }

  .play.preparing {
    font-size: 20px;
    letter-spacing: 2px;
  }

  .play.running {
    background: var(--surface-2);
    color: var(--text);
    box-shadow: inset 0 -5px 0 var(--bar);
    font-size: 24px;
  }

  .play.running:hover:not(:disabled) {
    background: var(--surface-active);
  }

  .meter {
    position: absolute;
    left: 0;
    bottom: 0;
    height: 5px;
    background: var(--ink);
    opacity: 0.35;
    transition: width 0.3s;
  }

  .play-error {
    position: absolute;
    top: calc(100% + 4px);
    max-width: 420px;
    font-size: 11px;
    color: var(--danger);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .right {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 16px;
    min-width: 0;
  }

  .sep {
    width: 1px;
    height: 36px;
    background: var(--line);
  }
</style>
