<script lang="ts" generics="T extends string">
  // Segment-Tabs nach Design-System 9.4 (Inhaltstypen, Filter).
  interface Props {
    items: { id: T; label: string; count?: number }[];
    value: T;
    label: string;
    /** "accent": aktive Fläche in Akzentfarbe; "soft": dunkler (in Detailseiten). */
    variant?: "accent" | "soft";
    onchange: (id: T) => void;
    /** Nur die aktive Auswahl bleibt bedienbar sichtbar. */
    disabled?: boolean;
  }
  let { items, value, label, variant = "accent", onchange, disabled = false }: Props = $props();
</script>

<div class="segmented {variant}" role="tablist" aria-label={label}>
  {#each items as item (item.id)}
    <button
      type="button"
      role="tab"
      aria-selected={value === item.id}
      class:active={value === item.id}
      disabled={disabled && value !== item.id}
      onclick={() => onchange(item.id)}
    >
      {item.label}
      {#if item.count !== undefined}<span class="count">{item.count}</span>{/if}
    </button>
  {/each}
</div>

<style>
  .segmented {
    display: inline-flex;
    gap: 2px;
    padding: 3px;
    background: var(--surface);
    flex-wrap: wrap;
  }

  button {
    min-height: 30px;
    padding: 0 14px;
    font-size: 13px;
    font-weight: 600;
    background: transparent;
    color: var(--text-muted);
  }

  button:hover:not(:disabled) {
    background: var(--surface-2);
    color: var(--text);
  }

  .accent button.active,
  .accent button.active:hover {
    background: var(--accent);
    color: var(--ink);
  }

  .soft button.active,
  .soft button.active:hover {
    background: var(--segment-active);
    color: var(--text);
  }

  .count {
    font-family: var(--font-mono);
    font-size: 11px;
    opacity: 0.75;
  }
</style>
