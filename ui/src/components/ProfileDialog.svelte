<script lang="ts">
  // Dialog „Neue Instanz“ (Bearbeiten passiert im Tab „Einstellungen“ der Instanz).
  import { onMount } from "svelte";
  import { t } from "../lib/i18n.svelte";
  import type { Profile } from "../lib/types";
  import InstanceForm from "./library/InstanceForm.svelte";

  interface Props {
    onClose: () => void;
    onSaved: (p: Profile) => void;
  }
  let { onClose, onSaved }: Props = $props();

  let dialog: HTMLDialogElement;
  onMount(() => dialog.showModal());
</script>

<dialog bind:this={dialog} class="wide" oncancel={(e) => (e.preventDefault(), onClose())}>
  <div class="dialog-body">
    <h2>{t("dialog.newProfile")}</h2>
    <InstanceForm id="new-instance" {onSaved}>
      {#snippet actions({ saving, canSave })}
        <div class="dialog-actions">
          <button type="button" onclick={onClose}>{t("common.cancel")}</button>
          <button type="submit" class="primary cut" disabled={!canSave}>{saving ? t("dialog.saving") : t("dialog.create")}</button>
        </div>
      {/snippet}
    </InstanceForm>
  </div>
</dialog>

<style>
  dialog.wide {
    width: min(680px, calc(100vw - 32px));
  }

  .dialog-actions {
    margin: 6px -22px -20px;
  }
</style>
