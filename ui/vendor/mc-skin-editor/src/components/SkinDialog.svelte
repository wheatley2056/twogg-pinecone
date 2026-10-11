<script lang="ts">
  import { RESOLUTION_LIST, RESOLUTIONS } from '../lib/skin/resolutions';
  import type { ModelType, ResolutionId } from '../lib/skin/types';

  export let title: string;
  export let confirmLabel = 'Create';
  export let initialName = 'My Skin';
  export let initialResolution: ResolutionId = 'standard';
  export let initialModel: ModelType = 'classic';
  export let allowResolutionChange = true;
  export let onConfirm: (name: string, resolution: ResolutionId, model: ModelType) => void;
  export let onCancel: () => void;

  let name = initialName;
  let resolution = initialResolution;
  let model: ModelType = initialModel;

  function confirm() {
    onConfirm(name.trim() || 'Untitled', resolution, model);
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') onCancel();
    if (e.key === 'Enter' && (e.target as HTMLElement).tagName !== 'TEXTAREA') confirm();
  }
</script>

<svelte:window on:keydown={onKeydown} />

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="overlay" on:click|self={onCancel}>
  <div class="dialog" role="dialog" aria-modal="true" aria-label={title}>
    <h2>{title}</h2>

    <label class="field">
      <span>Name</span>
      <!-- svelte-ignore a11y_autofocus -->
      <input type="text" bind:value={name} autofocus />
    </label>

    {#if allowResolutionChange}
      <label class="field">
        <span>Resolution</span>
        <select bind:value={resolution}>
          {#each RESOLUTION_LIST as r}
            <option value={r.id}>{r.label}</option>
          {/each}
        </select>
      </label>
    {:else}
      <div class="field">
        <span>Resolution</span>
        <div class="static-value">{RESOLUTIONS[resolution].label}</div>
      </div>
    {/if}

    <fieldset class="field">
      <legend>Model</legend>
      <label class="radio"><input type="radio" bind:group={model} value="classic" /> Classic (4px arms)</label>
      <label class="radio"><input type="radio" bind:group={model} value="slim" /> Slim (3px arms)</label>
    </fieldset>

    <div class="actions">
      <button type="button" on:click={onCancel}>Cancel</button>
      <button type="button" class="primary" on:click={confirm}>{confirmLabel}</button>
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
  }
  .dialog {
    background: var(--panel-bg, #2a2a2a);
    color: var(--text, #eee);
    border-radius: 10px;
    padding: 20px;
    width: 300px;
    display: flex;
    flex-direction: column;
    gap: 14px;
    box-shadow: 0 10px 40px rgba(0, 0, 0, 0.5);
  }
  .dialog h2 {
    margin: 0;
    font-size: 16px;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 13px;
    border: none;
    padding: 0;
    margin: 0;
  }
  .field legend {
    font-size: 13px;
    padding: 0;
    margin-bottom: 4px;
  }
  .field input[type='text'],
  .field select {
    padding: 7px 8px;
    border-radius: 6px;
    border: 1px solid var(--border, #555);
    background: var(--input-bg, #222);
    color: var(--text, #eee);
    font-size: 14px;
  }
  .static-value {
    padding: 7px 8px;
    opacity: 0.8;
  }
  .radio {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-top: 2px;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 4px;
  }
  .actions button {
    padding: 7px 14px;
    border-radius: 6px;
    border: 1px solid var(--border, #555);
    background: var(--panel-bg-hover, #3a3a3a);
    color: var(--text, #eee);
    cursor: pointer;
  }
  .actions button.primary {
    background: var(--accent, #3a7bd5);
    border-color: var(--accent, #3a7bd5);
    color: white;
  }
</style>
