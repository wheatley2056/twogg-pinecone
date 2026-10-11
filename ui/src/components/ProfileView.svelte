<script lang="ts">
  import { onMount, tick } from "svelte";
  import { api, errorText } from "../lib/api";
  import { t } from "../lib/i18n.svelte";
  import { ensureRun, runOf, stopGame } from "../lib/runs.svelte";
  import { open as openFile } from "@tauri-apps/plugin-dialog";
  import type { ContentKind, PackInfo, PackUpdate, Profile } from "../lib/types";
  import ConfirmDialog from "./ConfirmDialog.svelte";
  import ContentTab from "./ContentTab.svelte";
  import ExportDialog from "./ExportDialog.svelte";
  import PackUpdateDialog from "./PackUpdateDialog.svelte";
  import Icon from "./Icon.svelte";
  import ProfileDialog from "./ProfileDialog.svelte";
  import ProfileIcon from "./ProfileIcon.svelte";

  interface Props {
    profile: Profile;
    hasAccount: boolean;
    tab: "content" | "log";
    onPlay: () => void;
    onChanged: () => void;
    onDeleted: () => void;
    onAddContent: (kind: ContentKind) => void;
  }
  let { profile, hasAccount, tab = $bindable(), onPlay, onChanged, onDeleted, onAddContent }: Props = $props();

  const run = $derived(runOf(profile.id));
  let editing = $state(false);
  let confirmDelete = $state(false);
  let menuOpen = $state(false);
  let menuEl: HTMLElement | undefined = $state();
  let actionError = $state<string | null>(null);
  let logEl: HTMLElement | undefined = $state();
  let stickToBottom = true;
  let packInfo = $state<PackInfo | null>(null);
  let packUpdate = $state<PackUpdate | null>(null);
  let packChecking = $state(false);
  let packMessage = $state<string | null>(null);
  // null: geschlossen; target null: nur gesicherte Stände.
  let updateDialog = $state<{ target: { version: string | null; file: string | null } | null } | null>(null);
  let exporting = $state(false);
  // Neu zeichnen der Inhalte nach Pack-Update oder Rollback.
  let contentKey = $state(0);

  const percent = $derived(run.files > 0 ? Math.round((run.done / run.files) * 100) : null);
  const gb = (mb: number) => (mb / 1024).toFixed(1).replace(".0", "");

  async function loadPack() {
    packInfo = await api.packInfo(profile.id).catch(() => null);
  }

  async function checkPack() {
    packChecking = true;
    packMessage = null;
    try {
      packUpdate = await api.packCheckUpdate(profile.id);
      if (!packUpdate && packInfo) packMessage = t("pack.current", { name: `${packInfo.name} ${packInfo.version}` });
    } catch (e) {
      actionError = errorText(e);
    } finally {
      packChecking = false;
    }
  }

  async function updateFromFile() {
    const file = await openFile({ multiple: false, filters: [{ name: t("pack.filter"), extensions: ["mrpack"] }] });
    if (typeof file === "string") updateDialog = { target: { version: null, file } };
  }

  onMount(async () => {
    loadPack();
    // Nach Neustart des Launchers die letzte Ausgabe zeigen.
    if (run.lines.length === 0 && run.status === "idle") {
      const lines = await api.lastLog(profile.id, 500).catch(() => []);
      if (lines.length > 0) ensureRun(profile.id).lines = lines;
    }
  });

  $effect(() => {
    void run.lines.length;
    void tab;
    if (stickToBottom && logEl) tick().then(() => logEl && (logEl.scrollTop = logEl.scrollHeight));
  });

  function onLogScroll() {
    if (!logEl) return;
    stickToBottom = logEl.scrollHeight - logEl.scrollTop - logEl.clientHeight < 40;
  }

  function play() {
    actionError = null;
    stickToBottom = true;
    onPlay();
  }

  async function menuAction(action: () => unknown) {
    menuOpen = false;
    actionError = null;
    try {
      await action();
    } catch (e) {
      actionError = errorText(e);
    }
  }

  async function remove() {
    confirmDelete = false;
    try {
      await api.deleteProfile(profile.id);
      onDeleted();
    } catch (e) {
      actionError = errorText(e);
    }
  }

  function mb(bytes: number) {
    return (bytes / 1024 / 1024).toFixed(0);
  }
</script>

<svelte:window onclick={(e) => menuOpen && menuEl && !menuEl.contains(e.target as Node) && (menuOpen = false)} />

<section class="profile-view">
  <header class="header" style:--profile-color={profile.color ?? "var(--accent)"}>
    <div class="title">
      <ProfileIcon {profile} size={64} />
      <div class="title-text">
        <h1>{profile.name}</h1>
        <div class="meta">
          <span class="badge">{t("profile.minecraft", { version: profile.minecraft })}</span>
          {#if profile.loader}
            <span class="badge">{profile.loader.kind === "fabric" ? "Fabric" : "Quilt"} {profile.loader.version}</span>
          {:else}
            <span class="badge">{t("common.vanilla")}</span>
          {/if}
          <span class="badge">{t("profile.ram", { gb: gb(profile.memoryMb ?? 4096) })}</span>
          {#if profile.java}<span class="badge" title={profile.java}>{t("profile.customJava")}</span>{/if}
        </div>
      </div>
    </div>

    <div class="actions">
      {#if run.status === "idle"}
        <button class="primary play" onclick={play} disabled={!hasAccount}><Icon name="play" size={18} filled />{t("home.play")}</button>
      {:else if run.status === "preparing"}
        <button class="primary play" disabled>{t("profile.preparing")}</button>
      {:else}
        <button class="danger-solid play" onclick={() => stopGame(profile.id).catch((e) => (actionError = errorText(e)))}>
          <Icon name="stop" size={18} filled />{t("profile.stop").replace("■ ", "")}
        </button>
      {/if}
      <div class="menu" bind:this={menuEl}>
        <button class="icon-btn" onclick={() => (menuOpen = !menuOpen)} aria-label={t("profile.more")} aria-expanded={menuOpen}>
          <Icon name="more" />
        </button>
        {#if menuOpen}
          <div class="menu-pop" role="menu">
            <button role="menuitem" disabled={run.status !== "idle"} onclick={() => menuAction(() => (editing = true))}>
              <Icon name="edit" size={16} />{t("profile.edit")}
            </button>
            <button role="menuitem" onclick={() => menuAction(() => api.openProfileFolder(profile.id))}>
              <Icon name="folder" size={16} />{t("profile.openFolder")}
            </button>
            <button role="menuitem" onclick={() => menuAction(() => (exporting = true))}>
              <Icon name="download" size={16} />{t("pack.export")}
            </button>
            <button role="menuitem" class="danger" disabled={run.status !== "idle"} onclick={() => menuAction(() => (confirmDelete = true))}>
              <Icon name="trash" size={16} />{t("profile.delete")}
            </button>
          </div>
        {/if}
      </div>
    </div>
  </header>

  {#if packInfo}
    <div class="pack-bar">
      <span class="badge">{t("pack.label")}</span>
      <span>{packInfo.name} <strong>{packInfo.version}</strong></span>
      <span class="spacer"></span>
      {#if packMessage}<span class="muted small">{packMessage}</span>{/if}
      {#if packInfo.source === "modrinth"}
        {#if packUpdate}
          <button class="primary small" disabled={run.status !== "idle"} onclick={() => (updateDialog = { target: { version: packUpdate!.versionId, file: null } })}>
            <Icon name="download" size={15} />{t("pack.updateTo", { version: packUpdate.latest })}
          </button>
        {:else}
          <button class="small" onclick={checkPack} disabled={packChecking}>
            <Icon name="refresh" size={15} />{packChecking ? t("common.loading") : t("pack.check")}
          </button>
        {/if}
      {:else}
        <button class="small" disabled={run.status !== "idle"} onclick={updateFromFile}><Icon name="file" size={15} />{t("pack.updateFromFile")}</button>
      {/if}
      <button class="ghost small" onclick={() => (updateDialog = { target: null })}>{t("update.snapshots")}</button>
    </div>
  {/if}

  {#if !hasAccount && run.status === "idle"}<p class="muted">{t("profile.needAccount")}</p>{/if}

  {#if run.status === "preparing"}
    <div class="progress">
      <div class="progress-label">
        <span>{run.stage ? t(`stage.${run.stage}`) : t("profile.login")}</span>
        {#if run.files > 0}
          <span class="muted">
            {t("profile.files", { done: run.done, files: run.files })}{#if run.bytes > 0}
              · {t("profile.mb", { done: mb(run.bytesDone), total: mb(run.bytes) })}{/if}
          </span>
        {/if}
      </div>
      <div class="bar">
        <div class="fill" class:indeterminate={percent === null} style:width={percent === null ? "30%" : `${percent}%`}></div>
      </div>
    </div>
  {:else if run.status === "running"}
    <p class="muted running">
      ● {run.server ? t("profile.joining", { server: run.server }) : t("profile.running")}{run.java ? ` · ${run.java}` : ""}
    </p>
  {/if}

  {#if run.error}<p class="error">{run.error}</p>{/if}
  {#if actionError}<p class="error">{actionError}</p>{/if}

  <div class="tabs" role="tablist">
    <button role="tab" aria-selected={tab === "content"} class:active={tab === "content"} onclick={() => (tab = "content")}>
      <Icon name="box" size={16} />{t("profile.tabContent")}
    </button>
    <button role="tab" aria-selected={tab === "log"} class:active={tab === "log"} onclick={() => (tab = "log")}>
      <Icon name="log" size={16} />{t("profile.tabLog")}
    </button>
  </div>

  {#if tab === "content"}
    {#key contentKey}
      <ContentTab {profile} locked={run.status !== "idle"} {onAddContent} />
    {/key}
  {:else}
    <div class="log-wrap">
      <button class="ghost small clear" onclick={() => (ensureRun(profile.id).lines = [])} disabled={run.lines.length === 0}>
        {t("profile.clear")}
      </button>
      <pre class="log" bind:this={logEl} onscroll={onLogScroll}>{#each run.lines as line, i (i)}<span
            class:warn={line.includes("/WARN]")}
            class:err={line.includes("/ERROR]") || line.startsWith("\tat ") || line.includes("Exception")}>{line}
</span>{:else}<span class="muted">{t("profile.noOutput")}</span>{/each}</pre>
    </div>
  {/if}
</section>

{#if editing}
  <ProfileDialog
    {profile}
    onClose={() => (editing = false)}
    onSaved={() => {
      editing = false;
      onChanged();
    }}
  />
{/if}

{#if updateDialog}
  <PackUpdateDialog
    profileId={profile.id}
    target={updateDialog.target}
    onClose={() => (updateDialog = null)}
    onDone={async () => {
      packUpdate = null;
      packMessage = null;
      await loadPack();
      contentKey++;
      onChanged();
    }}
  />
{/if}

{#if exporting}
  <ExportDialog {profile} onClose={() => (exporting = false)} />
{/if}

{#if confirmDelete}
  <ConfirmDialog
    title={t("profile.deleteTitle")}
    confirmLabel={t("profile.deleteConfirm")}
    onConfirm={remove}
    onCancel={() => (confirmDelete = false)}
  >
    <p>{t("profile.deleteText", { name: profile.name })}</p>
  </ConfirmDialog>
{/if}

<style>
  .profile-view {
    display: flex;
    flex-direction: column;
    gap: 14px;
    min-height: 0;
    flex: 1;
  }

  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
    padding: 20px 22px;
    border-radius: 14px;
    border: 1px solid var(--border);
    background:
      radial-gradient(120% 160% at 0% 0%, color-mix(in srgb, var(--profile-color) 26%, transparent), transparent 60%),
      color-mix(in srgb, var(--surface) 92%, transparent);
  }

  .title {
    display: flex;
    gap: 16px;
    align-items: center;
    min-width: 0;
  }

  .title-text {
    min-width: 0;
  }

  h1 {
    font-size: 24px;
    overflow-wrap: anywhere;
  }

  .meta {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: 6px;
  }

  .actions {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
  }

  .play {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    font-size: 16px;
    font-weight: 700;
    padding: 12px 30px;
    min-width: 160px;
    border-radius: 10px;
  }

  .icon-btn {
    width: 44px;
    height: 44px;
    padding: 0;
    display: grid;
    place-items: center;
    border-radius: 10px;
  }

  .menu {
    position: relative;
  }

  .menu-pop {
    position: absolute;
    right: 0;
    top: calc(100% + 6px);
    z-index: 10;
    min-width: 190px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 10px;
    box-shadow: var(--shadow);
    padding: 6px;
    display: flex;
    flex-direction: column;
  }

  .menu-pop button {
    display: flex;
    align-items: center;
    gap: 10px;
    border: none;
    background: transparent;
    text-align: left;
  }

  .menu-pop button:hover:not(:disabled) {
    background: var(--surface-2);
  }

  .pack-bar {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 12px;
    border-radius: var(--radius);
    border: 1px solid var(--border);
    background: color-mix(in srgb, var(--surface-2) 92%, transparent);
    font-size: 13px;
    flex-wrap: wrap;
  }

  .pack-bar .spacer {
    flex: 1;
  }

  .pack-bar button {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .progress {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .progress-label {
    display: flex;
    justify-content: space-between;
    font-size: 13px;
  }

  .bar {
    height: 8px;
    border-radius: 4px;
    background: var(--surface-3);
    overflow: hidden;
  }

  .fill {
    height: 100%;
    background: var(--accent);
    transition: width 0.15s;
  }

  .fill.indeterminate {
    animation: slide 1.2s ease-in-out infinite;
  }

  @keyframes slide {
    from {
      transform: translateX(-100%);
    }
    to {
      transform: translateX(340%);
    }
  }

  .running {
    margin: 0;
    color: var(--accent);
  }

  .tabs {
    display: flex;
    gap: 4px;
    border-bottom: 1px solid var(--border);
  }

  .tabs button {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    border: none;
    border-radius: 8px 8px 0 0;
    background: transparent;
    color: var(--muted);
    padding: 8px 14px;
    border-bottom: 2px solid transparent;
  }

  .tabs button.active {
    color: var(--text);
    border-bottom-color: var(--accent);
  }

  .log-wrap {
    position: relative;
    flex: 1;
    min-height: 0;
    display: flex;
  }

  .clear {
    position: absolute;
    top: 8px;
    right: 18px;
    z-index: 1;
  }

  .small {
    padding: 3px 10px;
    font-size: 12px;
  }

  .log {
    flex: 1;
    min-height: 120px;
    margin: 0;
    overflow: auto;
    background: var(--log-bg);
    color: #cfd6dc;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 10px 12px;
    font-family: "JetBrains Mono", "Fira Code", ui-monospace, monospace;
    font-size: 12px;
    line-height: 1.5;
    white-space: pre-wrap;
    word-break: break-word;
    user-select: text;
  }

  .log .warn {
    color: #e3b341;
  }

  .log .err {
    color: #f47067;
  }
</style>
