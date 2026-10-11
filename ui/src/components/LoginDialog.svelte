<script lang="ts">
  import { onMount } from "svelte";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { api, errorText } from "../lib/api";
  import { t } from "../lib/i18n.svelte";
  import type { LoginStart } from "../lib/types";

  interface Props {
    onClose: () => void;
    onDone: () => void;
  }
  let { onClose, onDone }: Props = $props();

  let dialog: HTMLDialogElement;
  let start: LoginStart | null = $state(null);
  let error: string | null = $state(null);
  let copied = $state(false);
  let finished = false;

  onMount(() => {
    dialog.showModal();
    (async () => {
      try {
        start = await api.loginStart();
        await api.loginFinish();
        finished = true;
        onDone();
      } catch (e) {
        error = errorText(e);
      }
    })();
    return () => {
      if (!finished) api.loginCancel();
    };
  });

  async function copy() {
    if (!start) return;
    await navigator.clipboard.writeText(start.userCode);
    copied = true;
  }
</script>

<dialog bind:this={dialog} oncancel={(e) => (e.preventDefault(), onClose())}>
  <div class="dialog-body">
    <h2>{t("login.title")}</h2>
    {#if error}
      <p class="error">{error}</p>
    {:else if !start}
      <p class="muted">{t("login.preparing")}</p>
    {:else}
      <p>{t("login.text")}</p>
      <div class="code">{start.userCode}</div>
      <div class="buttons">
        <button onclick={copy}>{copied ? t("login.copied") : t("login.copy")}</button>
        <button class="primary" onclick={() => start && openUrl(start.verificationUri)}>{t("login.open")}</button>
      </div>
      <p class="muted waiting">{t("login.waiting")}</p>
    {/if}
  </div>
  <div class="dialog-actions">
    <button onclick={onClose}>{error ? t("common.close") : t("common.cancel")}</button>
  </div>
</dialog>

<style>
  .code {
    font-family: ui-monospace, monospace;
    font-size: 30px;
    letter-spacing: 0.15em;
    text-align: center;
    padding: 14px;
    background: var(--bg);
    border: 1px dashed var(--border);
    border-radius: var(--radius);
    user-select: all;
  }

  .buttons {
    display: flex;
    gap: 8px;
    justify-content: center;
  }

  .waiting {
    text-align: center;
    margin: 0;
  }
</style>
