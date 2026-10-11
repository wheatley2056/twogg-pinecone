<script lang="ts">
  import { paletteStore, SLOT_COUNT } from '../lib/stores/palette';
  import { activeColorHex } from '../lib/stores/tool';

  function onSlotClick(index: number) {
    const color = $paletteStore[index];
    if (color) {
      activeColorHex.set(color);
    } else {
      paletteStore.setSlot(index, $activeColorHex);
    }
  }

  function onSlotContextMenu(e: MouseEvent, index: number) {
    e.preventDefault();
    paletteStore.setSlot(index, $activeColorHex);
  }
</script>

<div class="palette">
  <div class="palette-label">Palette</div>
  <div class="palette-grid">
    {#each Array(SLOT_COUNT) as _, i}
      <button
        type="button"
        class="slot"
        class:empty={!$paletteStore[i]}
        style={$paletteStore[i] ? `background:${$paletteStore[i]};` : ''}
        title={$paletteStore[i] ? $paletteStore[i]! : 'Empty — click to store the current color'}
        on:click={() => onSlotClick(i)}
        on:contextmenu={(e) => onSlotContextMenu(e, i)}
      ></button>
    {/each}
  </div>
  <p class="hint">Click empty slot to save · right-click to overwrite</p>
</div>

<style>
  .palette {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .palette-label {
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    opacity: 0.7;
  }
  .palette-grid {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 6px;
  }
  .slot {
    aspect-ratio: 1;
    border-radius: 5px;
    border: 1px solid var(--border, #555);
    cursor: pointer;
    padding: 0;
  }
  .slot.empty {
    background-image:
      linear-gradient(45deg, #666 25%, transparent 25%),
      linear-gradient(-45deg, #666 25%, transparent 25%),
      linear-gradient(45deg, transparent 75%, #666 75%),
      linear-gradient(-45deg, transparent 75%, #666 75%);
    background-size: 8px 8px;
    background-position: 0 0, 0 4px, 4px -4px, -4px 0;
    background-color: #3a3a3a;
  }
  .slot:hover {
    outline: 2px solid var(--accent, #7fb0ff);
    outline-offset: 1px;
  }
  .hint {
    font-size: 11px;
    opacity: 0.55;
    margin: 0;
  }
</style>
