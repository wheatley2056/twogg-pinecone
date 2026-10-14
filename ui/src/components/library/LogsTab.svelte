<script lang="ts">
  // Tab „Logs“: Spielausgabe der laufenden bzw. letzten Sitzung, Diagnose-Export.
  import { onMount, tick } from "svelte";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { api, errorText } from "../../lib/api";
  import { crashText } from "../../lib/crash";
  import { exportDiagnostics } from "../../lib/diagnostics";
  import { t } from "../../lib/i18n.svelte";
  import { ensureRun, runOf } from "../../lib/runs.svelte";
  import { isTwogg } from "../../lib/profiles";
  import type { Profile } from "../../lib/types";
  import Icon from "../Icon.svelte";
  import Segmented from "../ui/Segmented.svelte";
  import LogShareDialog from "./LogShareDialog.svelte";

  let { profile }: { profile: Profile } = $props();

  const run = $derived(runOf(profile.id));
  let logEl: HTMLElement | undefined = $state();
  let stick = true;
  let notice = $state<string | null>(null);
  let error = $state<string | null>(null);
  let sharing = $state(false);
  // Absturztafel (F-35): Log oder Crash-Report anzeigen, Event-Pack prüfen.
  const crash = $derived(run.crash);
  let view = $state<"log" | "report">("log");
  let checking = $state(false);
  const shown = $derived(view === "report" && crash?.reportText ? crash.reportText.split("\n") : run.lines);

  async function act(action: () => Promise<unknown>) {
    error = notice = null;
    try {
      await action();
    } catch (e) {
      error = errorText(e);
    }
  }

  async function checkPack() {
    checking = true;
    await act(async () => {
      const r = await api.eventIntegrityCheck(profile.id);
      const count = (r?.repaired.length ?? 0) + (r?.disabled.length ?? 0);
      notice = count ? t("crash.packRepaired", { count }) : t("crash.packOk");
    });
    checking = false;
  }

  onMount(async () => {
    // Nach Neustart des Launchers die letzte Ausgabe zeigen.
    if (run.lines.length === 0 && run.status === "idle") {
      const lines = await api.lastLog(profile.id, 500).catch(() => []);
      if (lines.length > 0) ensureRun(profile.id).lines = lines;
    }
  });

  // Crash-Report von oben lesen, das Log wie gewohnt vom Ende.
  $effect(() => {
    const v = view;
    tick().then(() => {
      if (!logEl) return;
      stick = v === "log";
      logEl.scrollTop = v === "log" ? logEl.scrollHeight : 0;
    });
  });

  $effect(() => {
    void run.lines.length;
    if (stick && logEl) tick().then(() => logEl && (logEl.scrollTop = logEl.scrollHeight));
  });

  function onScroll() {
    if (logEl) stick = logEl.scrollHeight - logEl.scrollTop - logEl.clientHeight < 40;
  }

  async function exportLogs() {
    error = notice = null;
    try {
      notice = await exportDiagnostics(profile.id, profile.name);
    } catch (e) {
      error = errorText(e);
    }
  }
</script>

<div class="tab">
  <div class="tools">
    <span class="state">
      {#if run.status === "running"}
        <span class="square"></span>{run.server ? t("profile.joining", { server: run.server }) : t("profile.running")}{run.java ? ` · ${run.java}` : ""}
      {:else if run.status === "preparing"}
        {run.stage ? t(`stage.${run.stage}`) : t("profile.login")}
      {:else}
        <span class="dim">{t("logs.lastSession")}</span>
      {/if}
    </span>
    <button class="small" onclick={() => (sharing = true)} disabled={run.status !== "idle" || run.lines.length === 0}>{t("logshare.button")}</button>
    <button class="small" onclick={exportLogs}>{t("diag.exportProfile")}</button>
    <button class="small ghost" onclick={() => (ensureRun(profile.id).lines = [])} disabled={run.lines.length === 0}>{t("profile.clear")}</button>
  </div>
  {#if crash}
    <section class="crash" aria-labelledby="crash-title">
      <div class="crash-head">
        <span class="event-badge crash-badge">{t("crash.badge")}</span>
        <button class="icon ghost close" aria-label={t("crash.close")} title={t("crash.close")} onclick={() => ((ensureRun(profile.id).crash = null), (view = "log"))}>
          <Icon name="close" size={16} />
        </button>
      </div>
      <h3 id="crash-title">{t("crash.title")}</h3>
      <p class="cause">{crashText(crash.cause)}</p>
      {#if crash.trigger}<p class="trigger mono">{crash.trigger}</p>{/if}
      <div class="crash-actions">
        <button class="primary cut" onclick={() => act(async () => (notice = await exportDiagnostics(profile.id, profile.name)))}>{t("crash.export")}</button>
        {#if crash.report}
          {@const report = crash.report}
          <button onclick={() => act(() => api.openCrashReport(profile.id, report))}>{t("crash.openReport")}</button>
        {/if}
        {#if isTwogg(profile)}
          <button disabled={checking} onclick={checkPack}>{checking ? t("common.loading") : t("crash.checkPack")}</button>
        {/if}
        <button class="link" onclick={() => openUrl("https://twogg.chat").catch(() => window.open("https://twogg.chat", "_blank"))}>
          {t("crash.discord")}<Icon name="external" size={15} stroke={2} />
        </button>
      </div>
      {#if crash.others.length > 0}
        <div class="others">
          <span class="others-label">{t("crash.others")}</span>
          {#each crash.others as name (name)}
            <button class="tag mono" onclick={() => act(() => api.openCrashReport(profile.id, name))}>{name}</button>
          {/each}
        </div>
      {/if}
    </section>
    {#if crash.reportText}
      <Segmented
        items={[
          { id: "log", label: t("crash.showLog") },
          { id: "report", label: t("crash.showReport") },
        ]}
        value={view}
        label={t("crash.showReport")}
        variant="soft"
        onchange={(v) => (view = v as "log" | "report")}
      />
    {/if}
  {:else if run.error}<p class="error">{run.error}</p>{/if}
  {#if error}<p class="error">{error}</p>{/if}
  {#if notice}<p class="notice">{notice}</p>{/if}
  <pre class="log mono" bind:this={logEl} onscroll={onScroll}>{#each shown as line, i (i)}<span
        class:warn={line.includes("/WARN]")}
        class:err={line.includes("/ERROR]") || line.startsWith("\tat ") || line.includes("Exception")}>{line}
</span>{:else}<span class="dim">{t("profile.noOutput")}</span>{/each}</pre>
</div>

{#if sharing}
  <LogShareDialog {profile} onClose={() => (sharing = false)} />
{/if}

<style>
  .tab {
    flex: 1;
    min-height: 0;
    padding: 14px 28px 16px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .tools {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .state {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    color: var(--nadel);
  }

  .square {
    width: 7px;
    height: 7px;
    background: var(--nadel);
  }

  .dim {
    color: var(--text-dim);
  }

  .crash {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 14px 16px;
    background: color-mix(in srgb, var(--danger) 10%, var(--surface));
    border-left: 3px solid var(--danger);
  }

  .crash-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .crash-badge {
    background: var(--danger);
  }

  .close {
    min-height: 28px;
    width: 28px;
    padding: 0;
  }

  .crash h3 {
    margin: 0;
    font-size: 16px;
  }

  .cause {
    margin: 0;
    font-size: 13.5px;
    color: var(--text-soft);
  }

  .trigger {
    margin: 0;
    font-size: 12px;
    color: var(--text-muted);
    overflow-wrap: anywhere;
  }

  .crash-actions {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    margin-top: 4px;
  }

  .others {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
  }

  .others-label {
    font-size: 12px;
    color: var(--text-muted);
  }

  .tag {
    min-height: 26px;
    padding: 0 8px;
    font-size: 11.5px;
  }

  .small {
    min-height: 34px;
    font-size: 13px;
  }

  .log {
    flex: 1;
    min-height: 0;
    margin: 0;
    padding: 12px 14px;
    overflow: auto;
    background: var(--bar);
    font-size: 12px;
    line-height: 1.5;
    white-space: pre-wrap;
    word-break: break-word;
    user-select: text;
    color: var(--text-soft);
  }

  .warn {
    color: var(--accent-fg);
  }

  .err {
    color: var(--danger);
  }

  .notice {
    margin: 0;
    font-size: 13px;
    color: var(--nadel);
    user-select: text;
  }
</style>
