<script lang="ts">
  // Einstellungen › Darstellung nach docs/design/referenz/04-einstellungen-darstellung.html.
  import { open, save } from "@tauri-apps/plugin-dialog";
  import { api, errorText } from "../../lib/api";
  import { t } from "../../lib/i18n.svelte";
  import { ACCENTS, THEMES, settings, updateSettings, useSettings } from "../../lib/settings.svelte";
  import type { Settings, ThemeName } from "../../lib/types";
  import Switch from "../ui/Switch.svelte";
  import SettingRow from "./SettingRow.svelte";

  let error = $state<string | null>(null);
  let notice = $state<string | null>(null);
  const s = $derived(settings.current);

  // Mini-Vorschau je Thema: Kopfleiste, Inhalt, Spielleiste (Farben fest, damit alle drei sichtbar sind).
  const PREVIEW: Record<ThemeName, { bg: string; bar: string; tile: string }> = {
    rinde: { bg: "#12100d", bar: "#0d0b09", tile: "#241f19" },
    kiefer: { bg: "#0e1512", bar: "#0a0f0d", tile: "#1c2822" },
    winterland: { bg: "#edf1f0", bar: "#d5dedc", tile: "#ffffff" },
  };

  async function change(fn: (x: Settings) => void) {
    error = notice = null;
    await updateSettings(fn).catch((e) => (error = errorText(e)));
  }

  async function exportTheme() {
    const path = await save({ defaultPath: "pinecone-theme.json", filters: [{ name: t("settings.themeFilter"), extensions: ["json"] }] });
    if (path) await api.exportTheme(path).catch((e) => (error = errorText(e)));
  }

  async function importTheme() {
    const path = await open({ multiple: false, filters: [{ name: t("settings.themeFilter"), extensions: ["json"] }] });
    if (typeof path !== "string") return;
    try {
      useSettings(await api.importTheme(path));
      notice = t("settings.imported");
    } catch (e) {
      error = errorText(e);
    }
  }

  const details = [
    ["pattern", "settings.pattern", "settings.patternHint"],
    ["pixelTitles", "settings.pixelTitles", "settings.pixelTitlesHint"],
    ["reduceMotion", "settings.reduceMotion", "settings.reduceMotionHint"],
  ] as const;
</script>

{#if s}
  <div class="head">
    <h1>{t("settings.appearance")}</h1>
    <p class="lead">{t("settings.appearanceLead")}</p>
  </div>

  <section>
    <h3 class="section-label">{t("settings.theme")}</h3>
    <div class="themes">
      {#each THEMES as name (name)}
        {@const p = PREVIEW[name]}
        <button class="theme" class:selected={s.theme.name === name} aria-pressed={s.theme.name === name} onclick={() => change((x) => (x.theme.name = name))}>
          <span class="mini" style:background={p.bg}>
            <span class="mini-bar" style:background={p.bar}></span>
            <span class="mini-body"><span style:background={p.tile} class="a"></span><span class="gap"></span><span style:background={p.tile} class="b"></span></span>
            <span class="mini-play" style:background={p.bar}><span style:background={s.theme.accent}></span></span>
          </span>
          <span class="theme-text">
            <span class="theme-name">{t(`theme.${name}`)}</span>
            <span class="theme-sub">{t(`theme.${name}Hint`)}</span>
          </span>
        </button>
      {/each}
    </div>
  </section>

  <section>
    <h3 class="section-label">{t("settings.accent")}</h3>
    <div class="swatches">
      {#each ACCENTS as a (a.id)}
        <button class="swatch" class:selected={s.theme.accent === a.color} aria-pressed={s.theme.accent === a.color} aria-label={t(`accent.${a.id}`)} onclick={() => change((x) => (x.theme.accent = a.color))}>
          <span class="chip-color" style:background={a.color}></span>
          <span class="swatch-name">{t(`accent.${a.id}`)}</span>
        </button>
      {/each}
    </div>
  </section>

  <section class="split">
    <div class="col">
      <h3 class="section-label">{t("settings.preview")}</h3>
      <div class="preview">
        <div class="pv-card">
          <span class="pv-icon"></span>
          <span class="pv-text">
            <span class="pv-title">REVIVAL: Avalon</span>
            <span class="pv-sub">Fabric 1.21.11 · Event-Modpack</span>
          </span>
        </div>
        <div class="pv-actions">
          <span class="pv-play">
            <svg width="15" height="15" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><path d="M6 4l14 8-14 8z" /></svg>
            {t("play.play")}
          </span>
          <span class="pv-secondary">{t("events.chooseGuild")}</span>
        </div>
      </div>
    </div>
    <div class="col">
      <h3 class="section-label">{t("settings.details")}</h3>
      <div class="rows">
        {#each details as [key, label, hint] (key)}
          <SettingRow title={t(label)} hint={t(hint)}>
            <Switch checked={s.theme[key]} label={t(label)} onchange={(on) => change((x) => (x.theme[key] = on))} />
          </SettingRow>
        {/each}
      </div>
    </div>
  </section>

  <div class="files">
    <button class="link" onclick={importTheme}>{t("settings.import")}</button>
    <button class="link" onclick={exportTheme}>{t("settings.export")}</button>
    {#if notice}<span class="notice">{notice}</span>{/if}
  </div>
  {#if error}<p class="error">{error}</p>{/if}
{/if}

<style>
  .head h1 {
    font-weight: 600;
    font-size: 30px;
  }

  .lead {
    margin: 4px 0 0;
    font-size: 14px;
    color: var(--text-muted);
  }

  section {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  h3 {
    margin: 0;
    font-weight: 500;
  }

  .themes {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 14px;
  }

  .theme {
    flex-direction: column;
    align-items: stretch;
    gap: 10px;
    padding: 10px;
    min-height: 0;
    background: var(--surface);
    border: 2px solid var(--line);
    text-align: left;
  }

  .theme:hover:not(:disabled) {
    background: var(--surface);
    border-color: var(--line-dashed);
  }

  .theme.selected,
  .theme.selected:hover:not(:disabled) {
    border-color: var(--accent);
  }

  .mini {
    display: flex;
    flex-direction: column;
    height: 76px;
    overflow: hidden;
  }

  .mini-bar {
    height: 10px;
  }

  .mini-body {
    flex: 1;
    padding: 8px;
    display: flex;
    gap: 6px;
  }

  .mini-body .a {
    width: 40%;
  }

  .mini-body .gap {
    flex: 1;
  }

  .mini-body .b {
    width: 26%;
  }

  .mini-play {
    height: 16px;
    display: flex;
    justify-content: center;
    align-items: center;
  }

  .mini-play span {
    width: 42px;
    height: 9px;
  }

  .theme-text {
    display: flex;
    flex-direction: column;
  }

  .theme-name {
    font-size: 14px;
    font-weight: 700;
  }

  .theme-sub {
    font-size: 12px;
    font-weight: 400;
    color: var(--text-muted);
  }

  .swatches {
    display: flex;
    gap: 14px;
    align-items: center;
  }

  .swatch {
    flex-direction: column;
    gap: 6px;
    padding: 0;
    min-height: 0;
    background: transparent;
  }

  .swatch:hover:not(:disabled) {
    background: transparent;
  }

  .chip-color {
    width: 44px;
    height: 44px;
    clip-path: polygon(10px 0, 100% 0, 100% calc(100% - 10px), calc(100% - 10px) 100%, 0 100%, 0 10px);
  }

  .selected .chip-color {
    box-shadow: inset 0 0 0 4px var(--bg);
  }

  .swatch-name {
    font-size: 12px;
    font-weight: 700;
    color: var(--text-dim);
  }

  .selected .swatch-name {
    color: var(--text);
  }

  .split {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
    gap: 20px;
  }

  .col {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .preview {
    padding: 16px;
    background: var(--bg);
    border: 1px solid var(--line-strong);
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .pv-card {
    padding: 12px;
    background: var(--surface-2);
    display: flex;
    align-items: center;
    gap: 12px;
    clip-path: var(--cut-1);
  }

  .pv-icon {
    width: 40px;
    height: 40px;
    background: var(--accent);
  }

  .pv-text {
    flex: 1;
    display: flex;
    flex-direction: column;
  }

  .pv-title {
    font-family: var(--font-pixel);
    font-weight: 700;
    font-size: 18px;
  }

  .pv-sub {
    font-size: 12px;
    color: var(--text-muted);
  }

  .pv-actions {
    display: flex;
    gap: 10px;
    align-items: center;
  }

  .pv-play {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 46px;
    padding: 0 24px;
    background: var(--accent);
    color: var(--ink);
    font-family: var(--font-pixel);
    font-weight: 700;
    font-size: 20px;
    letter-spacing: 2px;
    text-transform: uppercase;
    box-shadow: inset 0 -4px 0 var(--accent-edge);
    clip-path: polygon(9px 0, calc(100% - 9px) 0, 100% 9px, 100% calc(100% - 9px), calc(100% - 9px) 100%, 9px 100%, 0 calc(100% - 9px), 0 9px);
  }

  .pv-secondary {
    display: flex;
    align-items: center;
    height: 40px;
    padding: 0 14px;
    background: var(--surface-2);
    font-size: 13px;
    font-weight: 600;
  }

  .rows {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .files {
    display: flex;
    gap: 14px;
    align-items: center;
  }

  .files .link {
    padding: 0;
    min-height: 0;
    font-size: 13px;
  }

  .notice {
    font-size: 13px;
    color: var(--nadel);
  }
</style>
