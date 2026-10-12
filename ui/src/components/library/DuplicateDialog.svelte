<script lang="ts">
  // Instanz duplizieren (F-49): neuer Name, Welten und Screenshots auf Wunsch mitkopieren.
  import { onMount } from "svelte";
  import { api, errorText } from "../../lib/api";
  import { t } from "../../lib/i18n.svelte";
  import type { Profile } from "../../lib/types";

  interface Props {
    profile: Profile;
    onClose: () => void;
    onCreated: (p: Profile) => void;
  }
  let { profile, onClose, onCreated }: Props = $props();

  let dialog: HTMLDialogElement;
  // svelte-ignore state_referenced_locally
  let name = $state(t("duplicate.defaultName", { name: profile.name }).slice(0, 64));
  let worlds = $state(false);
  let screenshots = $state(false);
  let busy = $state(false);
  let error = $state<string | null>(null);
  const isEvent = $derived(profile.packSource === "twogg");

  onMount(() => dialog.showModal());

  async function submit(e: SubmitEvent) {
    e.preventDefault();
    if (!name.trim()) return;
    busy = true;
    error = null;
    try {
      onCreated(await api.duplicateProfile(profile.id, name.trim(), { worlds, screenshots }));
    } catch (err) {
      error = errorText(err);
      busy = false;
    }
  }
</script>

<dialog bind:this={dialog} oncancel={(e) => (e.preventDefault(), !busy && onClose())}>
  <form onsubmit={submit}>
    <div class="dialog-body">
      <h2>{t("duplicate.title")}</h2>
      <label>
        {t("duplicate.name")}
        <!-- svelte-ignore a11y_autofocus (Erstes Feld im modalen Dialog) -->
        <input bind:value={name} maxlength="64" autofocus />
      </label>
      <label class="check"><input type="checkbox" bind:checked={worlds} />{t("duplicate.worlds")}</label>
      <label class="check"><input type="checkbox" bind:checked={screenshots} />{t("duplicate.screenshots")}</label>
      <p class="muted">{t("duplicate.hint")}</p>
      {#if isEvent}<p class="muted">{t("duplicate.eventHint")}</p>{/if}
      {#if error}<p class="error">{error}</p>{/if}
    </div>
    <div class="dialog-actions">
      <button type="button" onclick={onClose} disabled={busy}>{t("common.cancel")}</button>
      <button type="submit" class="primary cut" disabled={busy || !name.trim()}>
        {busy ? t("duplicate.busy") : t("duplicate.submit")}
      </button>
    </div>
  </form>
</dialog>

<style>
  p {
    margin: 0;
    font-size: 13px;
  }

  .check {
    flex-direction: row;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    color: var(--text);
  }

  .check input {
    width: auto;
  }
</style>
