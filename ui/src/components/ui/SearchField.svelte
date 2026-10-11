<script lang="ts">
  // Suchfeld nach Design-System 9.6: Fläche ohne Rahmen, Lupe links.
  import Icon from "../Icon.svelte";

  interface Props {
    value: string;
    placeholder: string;
    height?: number;
    width?: string;
  }
  let { value = $bindable(), placeholder, height = 34, width = "auto" }: Props = $props();
</script>

<label class="search" style:height="{height}px" style:width>
  <Icon name="search" size={15} />
  <input type="search" bind:value {placeholder} aria-label={placeholder} />
</label>

<style>
  .search {
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: 8px;
    padding: 0 10px;
    background: var(--surface);
    color: var(--text-dim);
  }

  input {
    flex: 1;
    min-width: 0;
    min-height: 0;
    height: 100%;
    padding: 0;
    background: transparent;
    font-size: 13px;
  }

  input:focus-visible {
    outline: none;
  }

  .search:focus-within {
    outline: 2px solid var(--accent);
    outline-offset: 0;
  }
</style>
