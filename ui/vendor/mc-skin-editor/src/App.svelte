<script lang="ts">
  import { onMount } from 'svelte';
  import { skinStore } from './lib/stores/skin';
  import { ensureInitialSkin, persistCurrentSkin } from './lib/stores/savedSkins';
  import { undo, redo } from './lib/stores/history';
  import { activeTool, brushSize, mirrorEnabled, MIN_BRUSH_SIZE, MAX_BRUSH_SIZE } from './lib/stores/tool';
  import type { ToolId } from './lib/skin/types';

  import SkinManagerBar from './components/SkinManagerBar.svelte';
  import Toolbar from './components/Toolbar.svelte';
  import ColorPicker from './components/ColorPicker.svelte';
  import Palette from './components/Palette.svelte';
  import VisibilityPanel from './components/VisibilityPanel.svelte';
  import Canvas2D from './components/Canvas2D.svelte';

  let ready = false;
  let saveTimer: ReturnType<typeof setTimeout> | undefined;

  // Loaded via dynamic import (rather than a static import) so the three.js-based
  // 3D viewport — by far the largest chunk of the app's JS — is split into its own
  // chunk and only fetched once the app is actually ready to show it.
  const viewport3DModule = import('./components/Viewport3D.svelte');

  onMount(() => {
    ensureInitialSkin().then(() => {
      ready = true;
    });
  });

  // Auto-save the current skin's pixels to localStorage on every change, debounced.
  $: if (ready) {
    $skinStore; // track dependency
    if (saveTimer) clearTimeout(saveTimer);
    saveTimer = setTimeout(persistCurrentSkin, 250);
  }

  const hotkeys: Record<string, ToolId> = { b: 'pencil', e: 'eraser', i: 'eyedropper', g: 'fill' };

  function onKeydown(e: KeyboardEvent) {
    const target = e.target as HTMLElement;
    if (target && ['INPUT', 'TEXTAREA', 'SELECT'].includes(target.tagName)) return;

    const key = e.key.toLowerCase();
    if ((e.ctrlKey || e.metaKey) && key === 'z') {
      e.preventDefault();
      if (e.shiftKey) redo();
      else undo();
      return;
    }
    if ((e.ctrlKey || e.metaKey) && key === 'y') {
      e.preventDefault();
      redo();
      return;
    }
    if (e.ctrlKey || e.metaKey || e.altKey) return;

    if (key in hotkeys) {
      activeTool.set(hotkeys[key]);
      return;
    }
    if (key === 'm') {
      mirrorEnabled.update((v) => !v);
      return;
    }
    if (key === '[') {
      brushSize.update((v) => Math.max(MIN_BRUSH_SIZE, v - 1));
      return;
    }
    if (key === ']') {
      brushSize.update((v) => Math.min(MAX_BRUSH_SIZE, v + 1));
      return;
    }
  }
</script>

<svelte:window on:keydown={onKeydown} />

<div class="app">
  <SkinManagerBar />

  <div class="workspace">
    <aside class="sidebar left">
      <section class="panel">
        <Toolbar />
      </section>
      <section class="panel">
        <ColorPicker />
      </section>
      <section class="panel">
        <Palette />
      </section>
    </aside>

    <main class="viewport-area">
      {#if ready}
        {#await viewport3DModule then { default: Viewport3D }}
          <Viewport3D />
        {/await}
      {/if}
    </main>

    <aside class="sidebar right">
      <section class="panel">
        <VisibilityPanel />
      </section>
      <section class="panel grow">
        <div class="panel-label">2D Texture Map</div>
        <Canvas2D />
      </section>
    </aside>
  </div>
</div>

<style>
  .app {
    display: flex;
    flex-direction: column;
    height: 100vh;
  }
  .workspace {
    flex: 1;
    display: grid;
    grid-template-columns: 200px 1fr 260px;
    grid-template-rows: minmax(0, 1fr);
    gap: 12px;
    padding: 12px;
    min-height: 0;
  }
  .sidebar {
    display: flex;
    flex-direction: column;
    gap: 12px;
    min-height: 0;
    overflow-y: auto;
  }
  .panel {
    background: var(--panel-bg);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 12px;
  }
  .panel.grow {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 260px;
  }
  .panel-label {
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    opacity: 0.7;
    margin-bottom: 8px;
  }
  .viewport-area {
    min-height: 0;
    min-width: 0;
    background: var(--panel-bg-dark, #17181a);
    border-radius: 8px;
    border: 1px solid var(--border);
    overflow: hidden;
  }

  @media (max-width: 900px) {
    .workspace {
      grid-template-columns: 1fr;
      grid-template-rows: none;
      grid-auto-rows: min-content;
    }
    .viewport-area {
      height: 60vh;
    }
    .sidebar {
      overflow-y: visible;
    }
    .sidebar.left {
      flex-direction: row;
      flex-wrap: wrap;
    }
    .sidebar.left .panel {
      flex: 1 1 200px;
    }
  }
</style>
