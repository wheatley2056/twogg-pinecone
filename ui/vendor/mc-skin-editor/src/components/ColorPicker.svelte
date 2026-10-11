<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import iro from '@jaames/iro';
  import { activeColorHex } from '../lib/stores/tool';
  import { adjustLightness, hexToRgb, LIGHTNESS_STEP } from '../lib/color';
  import { ripple } from '../lib/actions/ripple';

  let wheelEl: HTMLDivElement;
  let picker: iro.ColorPicker | null = null;
  let hexInput = $activeColorHex;
  let hexInvalid = false;
  let settingFromStore = false;

  function handleColorChange(color: iro.Color) {
    if (settingFromStore) return;
    activeColorHex.set(color.hexString);
  }

  onMount(() => {
    const instance = iro.ColorPicker(wheelEl, {
      width: 168,
      color: $activeColorHex,
      borderWidth: 1,
      borderColor: '#00000030',
      layout: [
        { component: iro.ui.Wheel, options: {} },
        { component: iro.ui.Slider, options: { sliderType: 'value', sliderShape: 'bar' } }
      ]
    });
    instance.on('color:change', handleColorChange);
    picker = instance;
  });

  onDestroy(() => {
    picker?.off('color:change', handleColorChange);
  });

  $: {
    hexInput = $activeColorHex;
    hexInvalid = false;
    if (picker && picker.color.hexString.toLowerCase() !== $activeColorHex.toLowerCase()) {
      settingFromStore = true;
      picker.color.hexString = $activeColorHex;
      settingFromStore = false;
    }
  }

  function commitHexInput() {
    const rgb = hexToRgb(hexInput);
    if (!rgb) {
      hexInvalid = true;
      return;
    }
    hexInvalid = false;
    activeColorHex.set(('#' + hexInput.replace('#', '')).toLowerCase());
  }

  function onHexKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') commitHexInput();
  }

  const presets = ['#ffffff', '#000000'];

  function selectPreset(hex: string) {
    activeColorHex.set(hex);
  }

  export function darken() {
    activeColorHex.set(adjustLightness($activeColorHex, -LIGHTNESS_STEP));
  }
  export function lighten() {
    activeColorHex.set(adjustLightness($activeColorHex, LIGHTNESS_STEP));
  }
</script>

<div class="color-picker">
  <div class="preset-row">
    {#each presets as hex}
      <button
        type="button"
        class="preset-swatch"
        class:active={$activeColorHex.toLowerCase() === hex}
        style="background: {hex};"
        title={hex}
        aria-label="Select {hex === '#ffffff' ? 'white' : 'black'}"
        on:click={() => selectPreset(hex)}
      ></button>
    {/each}
  </div>

  <div class="wheel" bind:this={wheelEl}></div>

  <div class="swatch-row">
    <div class="swatch" style="background: {$activeColorHex};"></div>
    <input
      class="hex-input"
      class:invalid={hexInvalid}
      type="text"
      maxlength="7"
      bind:value={hexInput}
      on:blur={commitHexInput}
      on:keydown={onHexKeydown}
      aria-label="Hex color"
    />
  </div>

  <div class="lighten-darken">
    <button type="button" use:ripple on:click={darken} title="Darken color">Darken</button>
    <button type="button" use:ripple on:click={lighten} title="Lighten color">Lighten</button>
  </div>
</div>

<style>
  .color-picker {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
  }
  .wheel {
    display: flex;
    justify-content: center;
  }
  .preset-row {
    display: flex;
    gap: 8px;
    width: 100%;
  }
  .preset-swatch {
    flex: 1;
    height: 26px;
    border-radius: 6px;
    border: 1px solid var(--border, #555);
    cursor: pointer;
    padding: 0;
  }
  .preset-swatch:hover {
    outline: 2px solid var(--accent, #7fb0ff);
    outline-offset: 1px;
  }
  .preset-swatch.active {
    border-color: var(--accent, #7fb0ff);
    box-shadow: 0 0 0 1px var(--accent, #7fb0ff);
  }
  .swatch-row {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
  }
  .swatch {
    width: 28px;
    height: 28px;
    border-radius: 6px;
    border: 1px solid var(--border, #555);
    flex-shrink: 0;
    background-image:
      linear-gradient(45deg, #888 25%, transparent 25%),
      linear-gradient(-45deg, #888 25%, transparent 25%),
      linear-gradient(45deg, transparent 75%, #888 75%),
      linear-gradient(-45deg, transparent 75%, #888 75%);
    background-size: 8px 8px;
    background-position: 0 0, 0 4px, 4px -4px, -4px 0;
  }
  .hex-input {
    flex: 1;
    min-width: 0;
    padding: 6px 8px;
    border-radius: 6px;
    border: 1px solid var(--border, #555);
    background: var(--input-bg, #222);
    color: var(--text, #eee);
    font-family: monospace;
    font-size: 14px;
  }
  .hex-input.invalid {
    border-color: #e05555;
  }
  .lighten-darken {
    display: flex;
    gap: 8px;
    width: 100%;
  }
  .lighten-darken button {
    flex: 1;
    padding: 6px 4px;
    border-radius: 6px;
    border: 1px solid var(--border, #555);
    background: var(--panel-bg, #333);
    color: var(--text, #eee);
    cursor: pointer;
    font-size: 12px;
  }
  .lighten-darken button:hover {
    background: var(--panel-bg-hover, #444);
  }
</style>
