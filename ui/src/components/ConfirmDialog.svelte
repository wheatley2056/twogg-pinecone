<script lang="ts">
  import { onMount, type Snippet } from "svelte";
  import { t } from "../lib/i18n.svelte";

  interface Props {
    title: string;
    confirmLabel: string;
    onConfirm: () => void;
    onCancel: () => void;
    children: Snippet;
  }
  let { title, confirmLabel, onConfirm, onCancel, children }: Props = $props();
  let dialog: HTMLDialogElement;

  onMount(() => dialog.showModal());
</script>

<dialog bind:this={dialog} oncancel={(e) => (e.preventDefault(), onCancel())}>
  <div class="dialog-body">
    <h2>{title}</h2>
    {@render children()}
  </div>
  <div class="dialog-actions">
    <!-- Abbrechen zuerst fokussiert: Enter löscht nicht versehentlich. -->
    <!-- svelte-ignore a11y_autofocus -->
    <button onclick={onCancel} autofocus>{t("common.cancel")}</button>
    <button class="danger-solid" onclick={onConfirm}>{confirmLabel}</button>
  </div>
</dialog>
