<script lang="ts">
  import { layerVisibility, partVisibility } from '../lib/stores/visibility';
  import type { PartName } from '../lib/skin/types';

  interface PartShape {
    id: PartName;
    x: number;
    y: number;
    w: number;
    h: number;
    label: string;
  }

  // A simple front-view paper-doll layout, in the same proportions as the
  // 3D model (head 8×8, torso 8×12, arms/legs 4 wide × 12 tall). Shown as
  // the viewer would see the character face-on, so the character's own
  // right arm/leg render on the left side of the figure.
  const partShapes: PartShape[] = [
    { id: 'head', x: 4, y: 0, w: 8, h: 8, label: 'Head' },
    { id: 'rightArm', x: 0, y: 8, w: 4, h: 12, label: 'Right Arm' },
    { id: 'torso', x: 4, y: 8, w: 8, h: 12, label: 'Torso' },
    { id: 'leftArm', x: 12, y: 8, w: 4, h: 12, label: 'Left Arm' },
    { id: 'rightLeg', x: 4, y: 20, w: 4, h: 12, label: 'Right Leg' },
    { id: 'leftLeg', x: 8, y: 20, w: 4, h: 12, label: 'Left Leg' }
  ];

  let hovered: PartName | null = null;

  function togglePart(id: PartName) {
    partVisibility.update((v) => ({ ...v, [id]: !v[id] }));
  }

  function onKeydown(e: KeyboardEvent, id: PartName) {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      togglePart(id);
    }
  }

  $: hoveredLabel = hovered ? partShapes.find((p) => p.id === hovered)?.label : null;
</script>

<div class="visibility-panel">
  <div class="section">
    <div class="section-label">Layers</div>
    <label class="toggle">
      <input type="checkbox" bind:checked={$layerVisibility.base} />
      Body
    </label>
    <label class="toggle">
      <input type="checkbox" bind:checked={$layerVisibility.overlay} />
      Outer Layer
    </label>
  </div>

  <div class="section">
    <div class="section-label">Body Parts</div>
    <svg viewBox="0 0 16 32" class="figure" role="group" aria-label="Body part visibility — click a part to show or hide it">
      {#each partShapes as p (p.id)}
        <rect
          x={p.x}
          y={p.y}
          width={p.w}
          height={p.h}
          rx="0.6"
          class="part"
          class:part-hidden={!$partVisibility[p.id]}
          class:part-hovered={hovered === p.id}
          role="button"
          tabindex="0"
          aria-pressed={$partVisibility[p.id]}
          aria-label="{p.label} ({$partVisibility[p.id] ? 'visible' : 'hidden'})"
          on:click={() => togglePart(p.id)}
          on:keydown={(e) => onKeydown(e, p.id)}
          on:mouseenter={() => (hovered = p.id)}
          on:mouseleave={() => (hovered = null)}
        >
          <title>{p.label} — click to {$partVisibility[p.id] ? 'hide' : 'show'}</title>
        </rect>
      {/each}
    </svg>
    <p class="hint">{hoveredLabel ?? 'Click a part to show/hide it'}</p>
  </div>
</div>

<style>
  .visibility-panel {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .section {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .section-label {
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    opacity: 0.7;
    margin-bottom: 2px;
  }
  .toggle {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 13px;
    cursor: pointer;
  }
  .figure {
    width: 100%;
    max-width: 110px;
    aspect-ratio: 16 / 32;
    margin: 4px auto 0;
    display: block;
    overflow: visible;
  }
  .part {
    fill: var(--accent-bg, #2b4a6b);
    stroke: var(--accent, #7fb0ff);
    stroke-width: 0.25;
    cursor: pointer;
    transition:
      fill 0.1s ease,
      opacity 0.1s ease;
  }
  .part:focus {
    outline: none;
  }
  .part:focus-visible {
    outline: 1px solid var(--accent, #7fb0ff);
    outline-offset: 1px;
  }
  .part.part-hovered {
    fill: var(--accent, #7fb0ff);
  }
  .part.part-hidden {
    fill: var(--panel-bg-dark, #17181a);
    stroke: var(--border, #555);
    stroke-dasharray: 1 0.8;
    opacity: 0.55;
  }
  .part.part-hidden.part-hovered {
    fill: var(--panel-bg-hover, #444);
  }
  .hint {
    font-size: 11px;
    opacity: 0.6;
    margin: 6px 0 0;
    text-align: center;
    min-height: 1.2em;
  }
</style>
