<script lang="ts">
  // Eine Zeile in den Einstellungen: Titel, Hinweis, rechts das Bedienelement.
  import type { Snippet } from "svelte";

  let { title, hint, children }: { title: string; hint?: string; children?: Snippet } = $props();
</script>

<div class="row">
  <span class="text">
    <span class="title">{title}</span>
    {#if hint}<span class="hint">{hint}</span>{/if}
  </span>
  {@render children?.()}
</div>

<style>
  .row {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 11px 14px;
    background: var(--surface);
  }

  .text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .title {
    font-size: 14px;
    font-weight: 700;
  }

  .hint {
    margin-top: 2px;
    font-size: 12px;
    color: var(--text-muted);
    user-select: text;
    overflow-wrap: anywhere;
  }
</style>
