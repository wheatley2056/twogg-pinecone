<script lang="ts">
  // Taste für eine Steuerung wählen: anklicken, dann Taste oder Maustaste drücken. Esc bricht ab.
  import { t } from "../../lib/i18n.svelte";
  import { keyFromEvent, keyFromMouse, keyLabel } from "../../lib/sync";

  interface Props {
    value: string;
    label: string;
    onchange: (value: string) => void;
  }
  let { value, label, onchange }: Props = $props();

  let listening = $state(false);
  let button: HTMLButtonElement;

  function stop() {
    listening = false;
    button?.blur();
  }

  function onkeydown(e: KeyboardEvent) {
    if (!listening) return;
    e.preventDefault();
    e.stopPropagation();
    if (e.code === "Escape") return stop();
    const key = keyFromEvent(e);
    if (key) {
      onchange(key);
      stop();
    }
  }

  function onmousedown(e: MouseEvent) {
    if (!listening) return;
    e.preventDefault();
    e.stopPropagation();
    onchange(keyFromMouse(e.button));
    stop();
  }
</script>

<button
  bind:this={button}
  type="button"
  class="key mono"
  class:listening
  aria-label={label}
  title={t("sync.keyHint")}
  onclick={() => !listening && (listening = true)}
  {onkeydown}
  {onmousedown}
  oncontextmenu={(e) => listening && e.preventDefault()}
  onblur={() => (listening = false)}
>
  {listening ? t("sync.keyPress") : keyLabel(value)}
</button>

<style>
  .key {
    min-height: 32px;
    padding: 0 12px;
    justify-content: flex-start;
    font-size: 13px;
  }

  .listening {
    box-shadow: inset 0 0 0 2px var(--accent);
    color: var(--accent-fg);
  }
</style>
