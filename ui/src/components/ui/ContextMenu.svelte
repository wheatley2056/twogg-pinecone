<script lang="ts">
  // Rechtsklick-Menü der App (Design-System: dunkle Leiste, Kante statt Schatten). Ohne eigenes
  // Menü bekommen Textfelder Ausschneiden/Kopieren/Einfügen, sonst bleibt der Rechtsklick stumm.
  import { tick } from "svelte";
  import { closeMenu, contextMenu, ctx, type MenuItem } from "../../lib/contextmenu.svelte";
  import { t } from "../../lib/i18n.svelte";
  import Icon from "../Icon.svelte";

  let el = $state<HTMLDivElement>();
  let pos = $state({ x: 0, y: 0 });

  $effect(() => {
    const open = ctx.open;
    if (!open) return;
    pos = { x: open.x, y: open.y };
    tick().then(() => {
      if (!el) return;
      const r = el.getBoundingClientRect();
      pos = {
        x: Math.max(4, Math.min(open.x, window.innerWidth - r.width - 4)),
        y: Math.max(4, Math.min(open.y, window.innerHeight - r.height - 4)),
      };
      el.querySelector<HTMLButtonElement>("button:not(:disabled)")?.focus();
    });
  });

  function editable(target: EventTarget | null): HTMLInputElement | HTMLTextAreaElement | null {
    if (target instanceof HTMLTextAreaElement) return target;
    if (target instanceof HTMLInputElement && !["checkbox", "radio", "range", "button", "submit", "color", "file"].includes(target.type))
      return target;
    return null;
  }

  function onContextMenu(e: MouseEvent) {
    if (e.defaultPrevented) return;
    e.preventDefault();
    const field = editable(e.target);
    const selection = window.getSelection()?.toString() ?? "";
    const fieldSelection = field ? (field.value ?? "").slice(field.selectionStart ?? 0, field.selectionEnd ?? 0) : "";
    if (!field && !selection) {
      closeMenu();
      return;
    }
    const writable = !!field && !field.readOnly && !field.disabled;
    contextMenu(e, [
      writable && { label: t("menu.cut"), icon: "cut", disabled: !fieldSelection, action: () => document.execCommand("cut") },
      { label: t("menu.copy"), icon: "copy", disabled: !(fieldSelection || selection), action: () => copy(fieldSelection || selection) },
      writable && { label: t("menu.paste"), icon: "paste", action: () => paste(field!) },
      field && "-",
      field && { label: t("menu.selectAll"), icon: "check", action: () => field.select() },
    ]);
  }

  async function copy(text: string) {
    try {
      await navigator.clipboard.writeText(text);
    } catch {
      document.execCommand("copy");
    }
  }

  async function paste(field: HTMLInputElement | HTMLTextAreaElement) {
    field.focus();
    try {
      const text = await navigator.clipboard.readText();
      document.execCommand("insertText", false, text);
    } catch {
      document.execCommand("paste");
    }
  }

  function run(item: MenuItem) {
    closeMenu();
    item.action();
  }

  function onKey(e: KeyboardEvent) {
    if (!ctx.open || !el) return;
    if (e.key === "Escape") {
      e.preventDefault();
      closeMenu();
      return;
    }
    if (e.key !== "ArrowDown" && e.key !== "ArrowUp") return;
    e.preventDefault();
    const buttons = [...el.querySelectorAll<HTMLButtonElement>("button:not(:disabled)")];
    const i = buttons.indexOf(document.activeElement as HTMLButtonElement);
    const next = e.key === "ArrowDown" ? (i + 1) % buttons.length : (i - 1 + buttons.length) % buttons.length;
    buttons[next]?.focus();
  }

  function onPointerDown(e: PointerEvent) {
    if (ctx.open && el && !el.contains(e.target as Node)) closeMenu();
  }
</script>

<svelte:window
  oncontextmenu={onContextMenu}
  onkeydown={onKey}
  onpointerdown={onPointerDown}
  onblur={closeMenu}
  onresize={closeMenu}
/>
<svelte:document onscroll={closeMenu} />

{#if ctx.open}
  <div class="ctx" role="menu" bind:this={el} style:left="{pos.x}px" style:top="{pos.y}px">
    {#each ctx.open.items as item, i (i)}
      {#if item === "-"}
        <span class="sep" role="separator"></span>
      {:else}
        <button role="menuitem" class:danger={item.danger} disabled={item.disabled} onclick={() => run(item)}>
          {#if item.icon}<Icon name={item.icon} size={16} />{:else}<span class="no-icon"></span>{/if}
          {item.label}
        </button>
      {/if}
    {/each}
  </div>
{/if}

<style>
  .ctx {
    position: fixed;
    z-index: 1000;
    min-width: 210px;
    max-width: 320px;
    display: flex;
    flex-direction: column;
    padding: 5px;
    background: var(--bar);
    border: 1px solid var(--line-strong);
    box-shadow: inset 0 -3px 0 var(--bar);
  }

  button {
    justify-content: flex-start;
    gap: 10px;
    min-height: 32px;
    padding: 0 10px;
    background: transparent;
    font-size: 13.5px;
    font-weight: 500;
    color: var(--text);
    text-align: left;
  }

  button:hover:not(:disabled),
  button:focus-visible {
    background: var(--surface);
    outline: none;
  }

  button :global(svg) {
    color: var(--text-muted);
    flex: none;
  }

  button.danger,
  button.danger :global(svg) {
    color: var(--danger);
  }

  button:disabled {
    color: var(--text-dim);
  }

  .no-icon {
    width: 16px;
  }

  .sep {
    height: 1px;
    margin: 4px 6px;
    background: var(--line);
  }
</style>
