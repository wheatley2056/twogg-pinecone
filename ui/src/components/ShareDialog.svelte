<script lang="ts">
  import { onMount } from "svelte";
  import { save } from "@tauri-apps/plugin-dialog";
  import { api, errorText } from "../lib/api";
  import { t } from "../lib/i18n.svelte";
  import type { ExportOptions, Profile } from "../lib/types";
  import Icon from "./Icon.svelte";

  let { profile, onClose }: { profile: Profile; onClose: () => void } = $props();

  let dialog: HTMLDialogElement;
  let link = $state<string | null>(null);
  let copied = $state(false);
  let options = $state<ExportOptions>({ version: "1.0.0", summary: "", resourcepacks: true, shaders: true, config: true, options: false });
  let busy = $state(false);
  let error = $state<string | null>(null);
  let result = $state<string | null>(null);

  onMount(() => {
    dialog.showModal();
    api.packShareLink(profile.id).then((l) => (link = l)).catch(() => {});
  });

  async function copyLink() {
    if (!link) return;
    try {
      await navigator.clipboard.writeText(link);
      copied = true;
      setTimeout(() => (copied = false), 2000);
    } catch (e) {
      error = errorText(e);
    }
  }

  async function exportPack() {
    error = result = null;
    const safeName = profile.name.replace(/[^\w.-]+/g, "_");
    const path = await save({ defaultPath: `${safeName}-${options.version}.mrpack`, filters: [{ name: t("pack.filter"), extensions: ["mrpack"] }] });
    if (!path) return;
    busy = true;
    try {
      const r = await api.packExport(profile.id, path, { ...options, summary: options.summary?.trim() || null });
      result = t("pack.exported", { linked: r.linked, included: r.included });
    } catch (e) {
      error = errorText(e);
    } finally {
      busy = false;
    }
  }

  const folders: [keyof ExportOptions & ("resourcepacks" | "shaders" | "config" | "options"), () => string][] = [
    ["resourcepacks", () => t("share.resourcepacks")],
    ["shaders", () => t("share.shaders")],
    ["config", () => t("share.config")],
    ["options", () => t("share.options")],
  ];
</script>

<dialog bind:this={dialog} oncancel={(e) => (e.preventDefault(), !busy && onClose())}>
  <div class="dialog-body">
    <h2>{t("share.title", { name: profile.name })}</h2>

    {#if link}
      <section>
        <h3><Icon name="link" size={16} />{t("share.linkTitle")}</h3>
        <div class="link-row">
          <input value={link} readonly aria-label={t("share.linkTitle")} onfocus={(e) => e.currentTarget.select()} />
          <button onclick={copyLink}>{copied ? t("share.copied") : t("share.copy")}</button>
        </div>
        <p class="muted">{t("share.linkHint")}</p>
      </section>
    {/if}

    <section>
      <h3><Icon name="file" size={16} />{t("share.fileTitle")}</h3>
      <p class="muted">{t("pack.exportHint")}</p>
      <div class="row">
        <label>
          {t("pack.exportVersion")}
          <input bind:value={options.version} maxlength="32" spellcheck="false" />
        </label>
        <label class="grow">
          {t("share.summary")}
          <input bind:value={options.summary} maxlength="200" placeholder={t("share.summaryPlaceholder")} />
        </label>
      </div>
      <fieldset>
        <legend>{t("share.include")}</legend>
        <span class="check fixed"><input type="checkbox" checked disabled />{t("share.mods")}</span>
        {#each folders as [key, label] (key)}
          <label class="check"><input type="checkbox" bind:checked={options[key]} />{label()}</label>
        {/each}
      </fieldset>
      <p class="muted small">{t("share.never")}</p>
    </section>

    {#if error}<p class="error">{error}</p>{/if}
    {#if result}<p class="notice">{result}</p>{/if}
  </div>
  <div class="dialog-actions">
    <button onclick={onClose} disabled={busy}>{t("common.close")}</button>
    <button class="primary" onclick={exportPack} disabled={busy || !options.version.trim()}>{busy ? t("common.loading") : t("pack.exportButton")}</button>
  </div>
</dialog>

<style>
  p {
    margin: 0;
  }

  section {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  h3 {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 14px;
    margin: 0;
  }

  .link-row,
  .row {
    display: flex;
    gap: 8px;
  }

  .link-row input {
    flex: 1;
    font-family: ui-monospace, monospace;
    font-size: 13px;
  }

  .row label:first-child {
    width: 120px;
  }

  .grow {
    flex: 1;
  }

  fieldset {
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 8px 12px 10px;
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 6px 14px;
  }

  legend {
    color: var(--muted);
    font-size: 12px;
    padding: 0 4px;
  }

  .check {
    flex-direction: row;
    align-items: center;
    gap: 6px;
    font-size: 13px;
    color: var(--text);
  }

  .check input {
    width: auto;
  }

  .fixed {
    display: flex;
    color: var(--muted);
  }

  .small {
    font-size: 12px;
  }

  .notice {
    color: var(--accent);
  }
</style>
