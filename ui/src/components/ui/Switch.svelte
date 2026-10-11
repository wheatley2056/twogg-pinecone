<script lang="ts">
  // Schalter nach Design-System 9.10: eckig, Knopf rechts in Tinte, wenn an.
  interface Props {
    checked: boolean;
    label: string;
    disabled?: boolean;
    onchange: (checked: boolean) => void;
  }
  let { checked, label, disabled = false, onchange }: Props = $props();
</script>

<button
  type="button"
  class="switch"
  class:on={checked}
  aria-pressed={checked}
  aria-label={label}
  {disabled}
  onclick={() => onchange(!checked)}
>
  <span class="knob"></span>
</button>

<style>
  .switch {
    flex: none;
    position: relative;
    width: 44px;
    min-height: 0;
    height: 24px;
    padding: 0;
    border-radius: 3px;
    background: var(--switch-off);
  }

  .switch:hover:not(:disabled) {
    background: var(--switch-off);
    filter: brightness(1.1);
  }

  .switch.on,
  .switch.on:hover:not(:disabled) {
    background: var(--accent);
  }

  .knob {
    position: absolute;
    top: 4px;
    left: 4px;
    width: 16px;
    height: 16px;
    border-radius: 2px;
    background: var(--toggle-knob-off);
    transition: left 0.12s;
  }

  .on .knob {
    left: 24px;
    background: var(--ink);
  }
</style>
