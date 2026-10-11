<script lang="ts">
  import type { ToolId } from '../lib/skin/types';
  import { activeTool, brushSize, mirrorEnabled, MIN_BRUSH_SIZE, MAX_BRUSH_SIZE } from '../lib/stores/tool';
  import { canUndo, canRedo, undo, redo } from '../lib/stores/history';
  import { ripple } from '../lib/actions/ripple';

  const tools: { id: ToolId; label: string; icon: string; hotkey: string }[] = [
    { id: 'pencil', label: 'Pencil', icon: '✏️', hotkey: 'B' },
    { id: 'eraser', label: 'Eraser', icon: '🧹', hotkey: 'E' },
    { id: 'eyedropper', label: 'Eyedropper', icon: '💧', hotkey: 'I' },
    { id: 'fill', label: 'Fill', icon: '🪣', hotkey: 'G' }
  ];

  function selectTool(id: ToolId) {
    activeTool.set(id);
  }
</script>

<div class="toolbar">
  <div class="tool-group" role="group" aria-label="Tools">
    {#each tools as t}
      <button
        type="button"
        class="tool-btn"
        class:active={$activeTool === t.id}
        title="{t.label} ({t.hotkey})"
        on:click={() => selectTool(t.id)}
      >
        <span class="icon" aria-hidden="true">{t.icon}</span>
        <span class="label">{t.label}</span>
      </button>
    {/each}
  </div>

  <div class="row">
    <label class="brush-label" for="brush-size">Brush size</label>
    <input
      id="brush-size"
      type="range"
      min={MIN_BRUSH_SIZE}
      max={MAX_BRUSH_SIZE}
      step="1"
      bind:value={$brushSize}
    />
    <span class="brush-value">{$brushSize}</span>
  </div>

  <label class="mirror-toggle">
    <input type="checkbox" bind:checked={$mirrorEnabled} />
    Mirror (left ↔ right)
  </label>

  <div class="row">
    <button type="button" use:ripple on:click={undo} disabled={!$canUndo} title="Undo (Ctrl+Z)">↶ Undo</button>
    <button type="button" use:ripple on:click={redo} disabled={!$canRedo} title="Redo (Ctrl+Y)">↷ Redo</button>
  </div>
</div>

<style>
  .toolbar {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .tool-group {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 6px;
  }
  .tool-btn {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
    padding: 8px 4px;
    border-radius: 6px;
    border: 1px solid var(--border, #555);
    background: var(--panel-bg, #333);
    color: var(--text, #eee);
    cursor: pointer;
    font-size: 11px;
  }
  .tool-btn .icon {
    font-size: 18px;
  }
  .tool-btn:hover {
    background: var(--panel-bg-hover, #444);
  }
  .tool-btn.active {
    border-color: var(--accent, #7fb0ff);
    background: var(--accent-bg, #2b4a6b);
  }
  .row {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .brush-label {
    font-size: 12px;
    opacity: 0.8;
    white-space: nowrap;
  }
  .brush-value {
    font-size: 12px;
    width: 1.2em;
    text-align: right;
  }
  input[type='range'] {
    flex: 1;
  }
  .mirror-toggle {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 13px;
    cursor: pointer;
  }
  .row button {
    flex: 1;
    padding: 6px 4px;
    border-radius: 6px;
    border: 1px solid var(--border, #555);
    background: var(--panel-bg, #333);
    color: var(--text, #eee);
    cursor: pointer;
    font-size: 12px;
  }
  .row button:hover:not(:disabled) {
    background: var(--panel-bg-hover, #444);
  }
  .row button:disabled {
    opacity: 0.4;
    cursor: default;
  }
</style>
