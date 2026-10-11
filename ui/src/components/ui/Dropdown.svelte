<script lang="ts" generics="T extends string">
  // Filter-Dropdown nach Design-System 9.7: „Loader Fabric ▾“. Liste als eigene Ebene.
  import type { Snippet } from "svelte";
  import Icon from "../Icon.svelte";

  interface Props {
    label: string;
    value: T;
    options: { value: T; label: string }[];
    disabled?: boolean;
    title?: string;
    /** Inhalt links im Button (z. B. Instanz-Symbol). */
    leading?: Snippet;
    align?: "left" | "right";
    onchange: (value: T) => void;
  }
  let { label, value, options, disabled = false, title, leading, align = "left", onchange }: Props = $props();

  let open = $state(false);
  let root: HTMLElement | undefined = $state();
  const current = $derived(options.find((o) => o.value === value)?.label ?? value);

  function onWindowClick(e: MouseEvent) {
    if (open && root && !root.contains(e.target as Node)) open = false;
  }

  function pick(v: T) {
    open = false;
    if (v !== value) onchange(v);
  }
</script>

<svelte:window onclick={onWindowClick} onkeydown={(e) => open && e.key === "Escape" && (open = false)} />

<div class="dropdown" bind:this={root}>
  <button class="trigger" {disabled} {title} aria-haspopup="listbox" aria-expanded={open} onclick={() => (open = !open)}>
    {@render leading?.()}
    {#if label}<span class="label">{label}</span>{/if}
    <span class="value">{current}</span>
    <Icon name="chevron" size={14} />
  </button>
  {#if open}
    <div class="list" class:right={align === "right"} role="listbox" aria-label={label}>
      {#each options as o (o.value)}
        <button role="option" aria-selected={o.value === value} class:selected={o.value === value} onclick={() => pick(o.value)}>
          {o.label}
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .dropdown {
    position: relative;
  }

  .trigger {
    height: 40px;
    padding: 0 12px;
    gap: 8px;
    font-size: 13px;
    white-space: nowrap;
  }

  .label {
    font-weight: 500;
    color: var(--text-dim);
  }

  .value {
    font-weight: 600;
    color: var(--text);
    max-width: 200px;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .list {
    position: absolute;
    left: 0;
    top: calc(100% + 4px);
    z-index: 25;
    min-width: 100%;
    max-height: 320px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    padding: 4px;
    background: var(--bar);
    border: 1px solid var(--line-strong);
  }

  .list.right {
    left: auto;
    right: 0;
  }

  .list button {
    justify-content: flex-start;
    min-height: 32px;
    background: transparent;
    font-size: 13px;
    font-weight: 500;
    white-space: nowrap;
  }

  .list button:hover:not(:disabled) {
    background: var(--surface);
  }

  .list button.selected {
    background: var(--surface-active);
    color: var(--accent-hover);
  }
</style>
