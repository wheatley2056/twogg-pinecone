<script lang="ts">
  // Instanz in eine Gruppe verschieben oder eine Gruppe umbenennen (F-50).
  import { onMount } from "svelte";
  import { api, errorText } from "../../lib/api";
  import { t } from "../../lib/i18n.svelte";
  import type { Profile } from "../../lib/types";

  type Mode = { kind: "move"; profile: Profile } | { kind: "rename"; group: string };
  interface Props {
    mode: Mode;
    /** Alle Instanzen (für Umbenennen und die Liste vorhandener Gruppen). */
    profiles: Profile[];
    onClose: () => void;
    onDone: () => void;
  }
  let { mode, profiles, onClose, onDone }: Props = $props();

  let dialog: HTMLDialogElement;
  const groups = $derived([...new Set(profiles.map((p) => p.group).filter((g): g is string => !!g))].sort((a, b) => a.localeCompare(b)));
  // svelte-ignore state_referenced_locally
  let choice = $state<string>(mode.kind === "move" ? (mode.profile.group ?? "") : "");
  // svelte-ignore state_referenced_locally
  let newName = $state(mode.kind === "rename" ? mode.group : "");
  let busy = $state(false);
  let error = $state<string | null>(null);
  const NEW = "\u0000neu";

  onMount(() => dialog.showModal());

  async function submit(e: SubmitEvent) {
    e.preventDefault();
    busy = true;
    error = null;
    try {
      if (mode.kind === "move") {
        const target = choice === NEW ? newName.trim() : choice;
        await api.setProfileGroup(mode.profile.id, target || null);
      } else {
        const from = mode.group;
        const to = newName.trim();
        for (const p of profiles.filter((p) => p.group === from)) await api.setProfileGroup(p.id, to || null);
      }
      onDone();
    } catch (err) {
      error = errorText(err);
      busy = false;
    }
  }

  const canSubmit = $derived(!busy && (mode.kind === "rename" ? !!newName.trim() : choice !== NEW || !!newName.trim()));
</script>

<dialog bind:this={dialog} oncancel={(e) => (e.preventDefault(), !busy && onClose())}>
  <form onsubmit={submit}>
    <div class="dialog-body">
      {#if mode.kind === "move"}
        <h2>{t("groups.moveTitle", { name: mode.profile.name })}</h2>
        <div class="choices" role="radiogroup" aria-label={t("groups.target")}>
          <label class="check"><input type="radio" name="group" value="" bind:group={choice} />{t("groups.auto")}</label>
          {#each groups as g (g)}
            <label class="check"><input type="radio" name="group" value={g} bind:group={choice} />{g}</label>
          {/each}
          <label class="check"><input type="radio" name="group" value={NEW} bind:group={choice} />{t("groups.new")}</label>
          {#if choice === NEW}
            <!-- svelte-ignore a11y_autofocus (erscheint nach der Wahl „Neue Gruppe“) -->
            <input class="new" bind:value={newName} maxlength="32" placeholder={t("groups.newPlaceholder")} autofocus />
          {/if}
        </div>
        <p class="muted">{t("groups.autoHint")}</p>
      {:else}
        <h2>{t("groups.renameTitle")}</h2>
        <label>
          {t("groups.name")}
          <!-- svelte-ignore a11y_autofocus (Erstes Feld im modalen Dialog) -->
          <input bind:value={newName} maxlength="32" autofocus />
        </label>
      {/if}
      {#if error}<p class="error">{error}</p>{/if}
    </div>
    <div class="dialog-actions">
      <button type="button" onclick={onClose} disabled={busy}>{t("common.cancel")}</button>
      <button type="submit" class="primary cut" disabled={!canSubmit}>{mode.kind === "move" ? t("groups.move") : t("groups.rename")}</button>
    </div>
  </form>
</dialog>

<style>
  p {
    margin: 0;
    font-size: 13px;
  }

  .choices {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .check {
    flex-direction: row;
    align-items: center;
    gap: 8px;
    font-size: 14px;
    color: var(--text);
  }

  .check input {
    width: auto;
    min-height: 0;
  }

  .new {
    margin-left: 24px;
    width: calc(100% - 24px);
    background: var(--surface-2);
  }
</style>
