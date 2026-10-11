<script lang="ts">
  // Tab „Logs“: Spielausgabe der laufenden bzw. letzten Sitzung, Diagnose-Export.
  import { onMount, tick } from "svelte";
  import { api, errorText } from "../../lib/api";
  import { exportDiagnostics } from "../../lib/diagnostics";
  import { t } from "../../lib/i18n.svelte";
  import { ensureRun, runOf } from "../../lib/runs.svelte";
  import type { Profile } from "../../lib/types";

  let { profile }: { profile: Profile } = $props();

  const run = $derived(runOf(profile.id));
  let logEl: HTMLElement | undefined = $state();
  let stick = true;
  let notice = $state<string | null>(null);
  let error = $state<string | null>(null);

  onMount(async () => {
    // Nach Neustart des Launchers die letzte Ausgabe zeigen.
    if (run.lines.length === 0 && run.status === "idle") {
      const lines = await api.lastLog(profile.id, 500).catch(() => []);
      if (lines.length > 0) ensureRun(profile.id).lines = lines;
    }
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
    <button class="small" onclick={exportLogs}>{t("diag.exportProfile")}</button>
    <button class="small ghost" onclick={() => (ensureRun(profile.id).lines = [])} disabled={run.lines.length === 0}>{t("profile.clear")}</button>
  </div>
  {#if run.error}<p class="error">{run.error}</p>{/if}
  {#if error}<p class="error">{error}</p>{/if}
  {#if notice}<p class="notice">{notice}</p>{/if}
  <pre class="log mono" bind:this={logEl} onscroll={onScroll}>{#each run.lines as line, i (i)}<span
        class:warn={line.includes("/WARN]")}
        class:err={line.includes("/ERROR]") || line.startsWith("\tat ") || line.includes("Exception")}>{line}
</span>{:else}<span class="dim">{t("profile.noOutput")}</span>{/each}</pre>
</div>

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
