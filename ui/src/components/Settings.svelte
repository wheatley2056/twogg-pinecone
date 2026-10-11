<script lang="ts">
  import { onMount } from "svelte";
  import { open, save } from "@tauri-apps/plugin-dialog";
  import { api, errorText } from "../lib/api";
  import { t } from "../lib/i18n.svelte";
  import { ACCENTS, settings, updateSettings, useSettings } from "../lib/settings.svelte";
  import { exportDiagnostics } from "../lib/diagnostics";
  import { checkUpdates, installUpdate, updates } from "../lib/updates.svelte";
  import type { AppInfo, JavaInstallation, Settings, ThemeMode, WidgetId } from "../lib/types";

  let { info }: { info: AppInfo | null } = $props();

  let javas: JavaInstallation[] | null = $state(null);
  let error: string | null = $state(null);
  let diagNotice = $state<string | null>(null);
  let diagBusy = $state(false);

  async function diagnose() {
    diagBusy = true;
    diagNotice = null;
    try {
      diagNotice = await exportDiagnostics();
    } catch (e) {
      error = errorText(e);
    } finally {
      diagBusy = false;
    }
  }
  let notice: string | null = $state(null);

  const s = $derived(settings.current);

  async function loadJava() {
    javas = null;
    try {
      javas = await api.javaList();
    } catch (e) {
      error = errorText(e);
    }
  }

  onMount(loadJava);

  async function change(fn: (s: Settings) => void) {
    error = notice = null;
    await updateSettings(fn).catch((e) => (error = errorText(e)));
  }

  async function run(action: () => Promise<Settings | void>, success?: string) {
    error = notice = null;
    try {
      const result = await action();
      if (result) useSettings(result);
      if (success) notice = success;
    } catch (e) {
      error = errorText(e);
    }
  }

  async function chooseBackground() {
    const path = await open({ multiple: false, filters: [{ name: t("settings.imageFilter"), extensions: ["png", "jpg", "jpeg", "webp"] }] });
    if (typeof path === "string") await run(() => api.setBackground(path));
  }

  async function exportTheme() {
    const path = await save({ defaultPath: "pinecone-theme.json", filters: [{ name: t("settings.themeFilter"), extensions: ["json"] }] });
    if (path) await run(() => api.exportTheme(path));
  }

  async function importTheme() {
    const path = await open({ multiple: false, filters: [{ name: t("settings.themeFilter"), extensions: ["json"] }] });
    if (typeof path === "string") await run(() => api.importTheme(path), t("settings.imported"));
  }

  function moveWidget(index: number, delta: number) {
    change((s) => {
      const [w] = s.widgets.splice(index, 1);
      s.widgets.splice(index + delta, 0, w);
    });
  }

  const modes: [ThemeMode, () => string][] = [
    ["system", () => t("settings.modeSystem")],
    ["light", () => t("settings.modeLight")],
    ["dark", () => t("settings.modeDark")],
  ];
</script>

<section class="settings">
  <h1>{t("settings.title")}</h1>
  {#if error}<p class="error">{error}</p>{/if}
  {#if notice}<p class="notice">{notice}</p>{/if}

  <div class="card">
    <h2>{t("settings.general")}</h2>
    <dl>
      <dt>{t("settings.version")}</dt>
      <dd>TwoGG Pinecone {info?.version ?? "?"}</dd>
      <dt>{t("settings.dataDir")}</dt>
      <dd class="path">{info?.dataDir ?? "?"}</dd>
      <dt>{t("settings.msa")}</dt>
      <dd>
        {#if info?.msaConfigured}
          {t("settings.msaOk")}
        {:else}
          <span class="warning">{t("settings.msaMissing")}</span>: {t("settings.msaHint")}
        {/if}
      </dd>
      <dt>{t("settings.curseforge")}</dt>
      <dd>
        {#if info?.curseforgeConfigured}
          {t("settings.msaOk")}
        {:else}
          <span class="warning">{t("settings.msaMissing")}</span>: {t("settings.cfHint")}
        {/if}
      </dd>
      <dt>{t("settings.language")}</dt>
      <dd>
        <select class="inline" value={s?.language} onchange={(e) => change((x) => (x.language = e.currentTarget.value as "de" | "en"))}>
          <option value="de">Deutsch</option>
          <option value="en">English</option>
        </select>
      </dd>
    </dl>
  </div>

  {#if s}
    <div class="card">
      <h2>{t("settings.appearance")}</h2>
      <div class="row">
        <span class="label">{t("settings.mode")}</span>
        <div class="segmented" role="radiogroup">
          {#each modes as [mode, label] (mode)}
            <button
              role="radio"
              aria-checked={s.theme.mode === mode}
              class:selected={s.theme.mode === mode}
              onclick={() => change((x) => (x.theme.mode = mode))}>{label()}</button
            >
          {/each}
        </div>
      </div>
      <div class="row">
        <span class="label">{t("settings.accent")}</span>
        <div class="swatches" role="radiogroup">
          {#each ACCENTS as c (c)}
            <button
              class="swatch"
              class:selected={s.theme.accent === c}
              role="radio"
              aria-checked={s.theme.accent === c}
              aria-label={c}
              style:background={c}
              onclick={() => change((x) => (x.theme.accent = c))}
            ></button>
          {/each}
          <input
            type="color"
            value={s.theme.accent}
            aria-label={t("settings.accent")}
            onchange={(e) => change((x) => (x.theme.accent = e.currentTarget.value))}
          />
        </div>
      </div>
      <div class="row">
        <span class="label">{t("settings.background")}</span>
        <div class="bg">
          <button onclick={chooseBackground}>{t("settings.chooseImage")}</button>
          {#if s.background}
            <button class="ghost" onclick={() => run(api.clearBackground)}>{t("settings.removeImage")}</button>
          {/if}
        </div>
      </div>
      {#if s.background}
        <label class="dim">
          {t("settings.dim", { percent: s.theme.backgroundDim })}
          <input
            type="range"
            min="0"
            max="90"
            step="5"
            value={s.theme.backgroundDim}
            onchange={(e) => change((x) => (x.theme.backgroundDim = Number(e.currentTarget.value)))}
          />
        </label>
      {/if}
      <div class="row end">
        <button class="ghost" onclick={importTheme}>{t("settings.import")}</button>
        <button class="ghost" onclick={exportTheme}>{t("settings.export")}</button>
      </div>
    </div>

    <div class="card">
      <h2>{t("settings.dashboard")}</h2>
      <p class="muted">{t("settings.dashboardHint")}</p>
      <ul class="widgets">
        {#each s.widgets as w, i (w.id)}
          <li>
            <label class="toggle">
              <input type="checkbox" checked={w.visible} onchange={(e) => change((x) => (x.widgets[i].visible = e.currentTarget.checked))} />
              {t(`widget.${w.id as WidgetId}`)}
            </label>
            <button class="ghost small" disabled={i === 0} onclick={() => moveWidget(i, -1)} aria-label={t("settings.up")}>↑</button>
            <button class="ghost small" disabled={i === s.widgets.length - 1} onclick={() => moveWidget(i, 1)} aria-label={t("settings.down")}>↓</button>
          </li>
        {/each}
      </ul>
    </div>
  {/if}

  <div class="card">
    <div class="card-head">
      <h2>{t("settings.java")}</h2>
      <button class="ghost" onclick={loadJava} disabled={javas === null}>{t("settings.rescan")}</button>
    </div>
    <p class="muted">{t("settings.javaHint")}</p>
    {#if javas === null}
      <p class="muted">{t("settings.searching")}</p>
    {:else if javas.length === 0}
      <p class="muted">{t("settings.noJava")}</p>
    {:else}
      <table>
        <tbody>
          {#each javas as j (j.path)}
            <tr>
              <td class="major">Java {j.major}</td>
              <td>{j.version}</td>
              <td><span class="badge">{j.managed ? t("settings.byPinecone") : t("settings.system")}</span></td>
              <td class="path">{j.path}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </div>

  <div class="card">
    <h2>{t("appUpdate.title")}</h2>
    <p class="muted">{t("appUpdate.current", { version: info?.version ?? "?" })}</p>
    {#if !info?.updatesConfigured}
      <p class="muted">{t("appUpdate.notConfigured")}</p>
    {:else}
      {#if updates.info}
        <p>{t("appUpdate.available", { version: updates.info.version })}</p>
        {#if updates.info.notes}<pre class="notes">{updates.info.notes}</pre>{/if}
        {#if updates.progress !== null}
          <progress max="1" value={updates.progress}></progress>
          <p class="muted">{updates.installing ? t("appUpdate.installing") : t("appUpdate.downloading")}</p>
        {/if}
      {:else if updates.checked}
        <p class="notice">{t("appUpdate.upToDate")}</p>
      {/if}
      {#if updates.progress === null}
        <div class="row-buttons">
          {#if updates.info}<button class="primary" onclick={installUpdate}>{t("appUpdate.install")}</button>{/if}
          <button onclick={() => checkUpdates()} disabled={updates.checking}>{updates.checking ? t("appUpdate.checking") : t("appUpdate.check")}</button>
        </div>
      {/if}
      {#if updates.error}<p class="error">{updates.error}</p>{/if}
    {/if}
  </div>

  <div class="card">
    <h2>{t("diag.title")}</h2>
    <p class="muted">{t("diag.hint")}</p>
    <div class="row-buttons">
      <button class="primary" onclick={diagnose} disabled={diagBusy}>{diagBusy ? t("common.loading") : t("diag.export")}</button>
      <button onclick={() => api.openLogsFolder().catch((e) => (error = errorText(e)))}>{t("diag.openLogs")}</button>
    </div>
    {#if diagNotice}<p class="notice">{diagNotice}</p>{/if}
  </div>
</section>

<style>
  .notes {
    margin: 0;
    padding: 8px 10px;
    border-radius: var(--radius);
    background: var(--surface-2);
    font-family: inherit;
    font-size: 13px;
    white-space: pre-wrap;
    max-height: 160px;
    overflow-y: auto;
  }

  progress {
    width: 100%;
  }

  .row-buttons {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  .notice {
    color: var(--accent);
  }

  .settings {
    display: flex;
    flex-direction: column;
    gap: 16px;
    overflow-y: auto;
    padding-right: 4px;
  }

  h1 {
    font-size: 24px;
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
    align-items: center;
  }

  .notice {
    margin: 0;
    color: var(--accent);
  }

  dl {
    display: grid;
    grid-template-columns: 150px 1fr;
    gap: 8px 12px;
    margin: 0;
    align-items: center;
  }

  dt,
  .label {
    color: var(--muted);
  }

  dd {
    margin: 0;
  }

  select.inline {
    width: auto;
    min-width: 160px;
  }

  .row {
    display: grid;
    grid-template-columns: 150px 1fr;
    align-items: center;
    gap: 12px;
  }

  .row.end {
    display: flex;
    justify-content: flex-end;
  }

  .segmented {
    display: inline-flex;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    overflow: hidden;
    justify-self: start;
  }

  .segmented button {
    border: none;
    border-radius: 0;
    background: var(--bg);
    min-width: 90px;
  }

  .segmented button + button {
    border-left: 1px solid var(--border);
  }

  .segmented button.selected {
    background: var(--accent);
    color: var(--accent-text);
    font-weight: 600;
  }

  .swatches {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
  }

  .swatch {
    width: 26px;
    height: 26px;
    padding: 0;
    border-radius: 50%;
    border: 2px solid transparent;
  }

  .swatch.selected {
    border-color: var(--text);
    box-shadow: 0 0 0 2px var(--surface);
  }

  input[type="color"] {
    width: 34px;
    height: 28px;
    padding: 2px;
    cursor: pointer;
  }

  .bg {
    display: flex;
    gap: 8px;
  }

  .dim input {
    padding: 0;
    accent-color: var(--accent);
  }

  .widgets {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .widgets li {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 8px;
    border-radius: var(--radius);
    background: var(--surface-2);
  }

  .toggle {
    flex: 1;
    flex-direction: row;
    align-items: center;
    gap: 8px;
    color: var(--text);
    font-size: 14px;
  }

  .toggle input {
    width: auto;
    accent-color: var(--accent);
  }

  .small {
    padding: 2px 10px;
  }

  .path {
    font-family: ui-monospace, monospace;
    font-size: 12px;
    user-select: text;
    overflow-wrap: anywhere;
  }

  .warning {
    color: var(--warning);
  }

  table {
    border-collapse: collapse;
    width: 100%;
  }

  td {
    padding: 6px 8px;
    border-top: 1px solid var(--border);
  }

  .major {
    font-weight: 600;
    white-space: nowrap;
  }
</style>
