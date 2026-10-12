<script lang="ts">
  // Log teilen (F-53): bereinigte Vorschau zeigen, erst nach Bestätigung zu mclo.gs hochladen.
  import { onMount } from "svelte";
  import { api, errorText } from "../../lib/api";
  import { t } from "../../lib/i18n.svelte";
  import type { Profile } from "../../lib/types";

  let { profile, onClose }: { profile: Profile; onClose: () => void } = $props();

  let dialog: HTMLDialogElement;
  let text = $state<string | null>(null);
  let link = $state<string | null>(null);
  let copied = $state(false);
  let busy = $state(false);
  let error = $state<string | null>(null);

  onMount(() => {
    dialog.showModal();
    api
      .logSharePreview(profile.id)
      .then((s) => (text = s))
      .catch((e) => (error = errorText(e)));
  });

  async function upload() {
    if (!text) return;
    busy = true;
    error = null;
    try {
      link = await api.logShareUpload(text);
      await copy();
    } catch (e) {
      error = errorText(e);
    } finally {
      busy = false;
    }
  }

  async function copy() {
    if (!link) return;
    try {
      await navigator.clipboard.writeText(link);
      copied = true;
    } catch {
      copied = false;
    }
  }

  const lines = $derived(text ? text.split("\n").length : 0);
</script>

<dialog bind:this={dialog} oncancel={(e) => (e.preventDefault(), !busy && onClose())}>
  <div class="dialog-body">
    <h2>{t("logshare.title")}</h2>
    {#if link}
      <p>{copied ? t("logshare.doneCopied") : t("logshare.done")}</p>
      <div class="link-row">
        <input value={link} readonly aria-label={t("logshare.link")} onfocus={(e) => e.currentTarget.select()} />
        <button onclick={copy}>{copied ? t("share.copied") : t("share.copy")}</button>
      </div>
    {:else}
      <p class="muted">{t("logshare.hint")}</p>
      {#if text !== null}
        <pre class="preview" aria-label={t("logshare.preview")}>{text}</pre>
        <p class="muted small">{t("logshare.lines", { n: lines })}</p>
      {:else if !error}
        <p class="muted">{t("common.loading")}</p>
      {/if}
    {/if}
    {#if error}<p class="error">{error}</p>{/if}
  </div>
  <div class="dialog-actions">
    <button onclick={onClose} disabled={busy}>{link ? t("common.close") : t("common.cancel")}</button>
    {#if !link}
      <button class="primary cut" onclick={upload} disabled={busy || !text}>{busy ? t("logshare.uploading") : t("logshare.upload")}</button>
    {/if}
  </div>
</dialog>

<style>
  p {
    margin: 0;
    font-size: 13px;
  }

  .small {
    font-size: 12px;
  }

  .preview {
    margin: 0;
    max-height: 320px;
    overflow: auto;
    padding: 10px 12px;
    background: var(--surface-2);
    font-family: var(--font-mono);
    font-size: 12px;
    line-height: 1.5;
    white-space: pre;
    user-select: text;
  }

  .link-row {
    display: flex;
    gap: 8px;
  }

  .link-row input {
    flex: 1;
    font-family: var(--font-mono);
    font-size: 13px;
    background: var(--surface-2);
  }

  dialog {
    width: min(760px, calc(100vw - 48px));
  }
</style>
