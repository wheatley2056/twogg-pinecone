<script lang="ts">
  // JVM-Optimierungen: globales Standard-Preset wählen, eingebaute Presets anpassen oder
  // zurücksetzen, eigene anlegen. Instanzen können im Instanz-Dialog ein anderes wählen.
  import { onMount } from "svelte";
  import { api, errorText } from "../../lib/api";
  import { t } from "../../lib/i18n.svelte";
  import { NO_PRESET, mergePresets, parseArgs, presetId, withPreset } from "../../lib/jvm";
  import { settings, updateSettings } from "../../lib/settings.svelte";
  import type { JvmPreset, JvmSettings } from "../../lib/types";
  import Icon from "../Icon.svelte";

  let builtin = $state<JvmPreset[]>([]);
  let error = $state<string | null>(null);
  /** Offener Editor: bestehendes Preset (ID) oder neues. */
  let editing = $state<{ id: string | null; name: string; args: string; minJava: number } | null>(null);

  const jvm = $derived<JvmSettings>(settings.current?.jvm ?? { defaultPreset: NO_PRESET, presets: [] });
  const presets = $derived(mergePresets(builtin, jvm));

  onMount(async () => {
    builtin = await api.jvmBuiltinPresets().catch(() => []);
  });

  async function save(change: (j: JvmSettings) => JvmSettings) {
    error = null;
    try {
      await updateSettings((s) => (s.jvm = change(s.jvm)));
      return true;
    } catch (e) {
      error = errorText(e);
      return false;
    }
  }

  function edit(p: JvmPreset) {
    editing = { id: p.id, name: p.name, args: p.args.join("\n"), minJava: p.minJava };
  }

  function create() {
    const base = presets.find((p) => p.id === jvm.defaultPreset) ?? presets[0];
    editing = { id: null, name: t("jvm.newName"), args: base ? base.args.join("\n") : "", minJava: base?.minJava ?? 8 };
  }

  async function commit() {
    if (!editing) return;
    const e = editing;
    const args = parseArgs(e.args);
    const id = e.id ?? presetId(e.name.trim(), [NO_PRESET, ...presets.map((p) => p.id)]);
    const isBuiltin = builtin.some((b) => b.id === id);
    const ok = await save((j) => withPreset(j, { id, name: isBuiltin ? "" : e.name.trim(), args, minJava: e.minJava }));
    if (ok) editing = null;
  }

  async function reset(p: JvmPreset) {
    if (await save((j) => ({ ...j, presets: j.presets.filter((s) => s.id !== p.id) }))) editing = null;
  }

  async function remove(p: JvmPreset) {
    const ok = await save((j) => ({
      defaultPreset: j.defaultPreset === p.id ? NO_PRESET : j.defaultPreset,
      presets: j.presets.filter((s) => s.id !== p.id),
    }));
    if (ok) editing = null;
  }
</script>

{#snippet editor(p: JvmPreset | null)}
  {#if editing}
    <div class="editor">
      {#if !p?.builtin}
        <div class="fields">
          <label class="grow">
            {t("jvm.name")}
            <input bind:value={editing.name} maxlength="40" />
          </label>
          <label>
            {t("jvm.minJava")}
            <select bind:value={editing.minJava}>
              {#each [8, 17, 21, 25] as v (v)}<option value={v}>Java {v}+</option>{/each}
            </select>
          </label>
        </div>
      {/if}
      <label>
        {t("jvm.args")}
        <textarea class="mono" rows="9" bind:value={editing.args} spellcheck="false"></textarea>
      </label>
      <p class="hint">{t("jvm.argsHint")}</p>
      <div class="actions">
        {#if p?.builtin && p.modified}
          <button class="ghost" onclick={() => reset(p)}><Icon name="refresh" size={14} />{t("jvm.reset")}</button>
        {/if}
        {#if p && !p.builtin}
          <button class="ghost danger" onclick={() => remove(p)}><Icon name="trash" size={14} />{t("jvm.delete")}</button>
        {/if}
        <span class="spacer"></span>
        <button class="ghost" onclick={() => (editing = null)}>{t("common.cancel")}</button>
        <button class="primary" disabled={!p?.builtin && !editing.name.trim()} onclick={commit}>{t("jvm.save")}</button>
      </div>
    </div>
  {/if}
{/snippet}

<div class="list" role="radiogroup" aria-label={t("jvm.title")}>
  <div class="preset" class:selected={jvm.defaultPreset === NO_PRESET}>
    <button class="pick" role="radio" aria-checked={jvm.defaultPreset === NO_PRESET} onclick={() => save((j) => ({ ...j, defaultPreset: NO_PRESET }))}>
      <span class="dot"></span>
      <span class="text">
        <span class="name">{t("jvm.none")}</span>
        <span class="desc">{t("jvm.noneHint")}</span>
      </span>
    </button>
  </div>
  {#each presets as p (p.id)}
    {@const selected = jvm.defaultPreset === p.id}
    <div class="preset" class:selected>
      <button class="pick" role="radio" aria-checked={selected} onclick={() => save((j) => ({ ...j, defaultPreset: p.id }))}>
        <span class="dot"></span>
        <span class="text">
          <span class="name">
            {p.name}
            {#if p.minJava > 8}<span class="chip">Java {p.minJava}+</span>{/if}
            {#if p.modified}<span class="chip accent">{t("jvm.modified")}</span>{/if}
            {#if !p.builtin}<span class="chip">{t("jvm.custom")}</span>{/if}
          </span>
          <span class="desc">{p.description || t("jvm.argCount", { n: p.args.length })}</span>
        </span>
      </button>
      {#if editing?.id !== p.id}
        <button class="ghost small edit" onclick={() => edit(p)}><Icon name="edit" size={14} />{t("jvm.edit")}</button>
      {/if}
    </div>
    {#if editing?.id === p.id}{@render editor(p)}{/if}
  {/each}
  {#if editing && editing.id === null}
    {@render editor(null)}
  {:else}
    <button class="dashed" onclick={create}><Icon name="plus" size={15} />{t("jvm.create")}</button>
  {/if}
</div>
{#if error}<p class="error">{error}</p>{/if}

<style>
  .list {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .preset {
    display: flex;
    align-items: center;
    gap: 10px;
    padding-right: 12px;
    background: var(--surface);
    border-left: 3px solid transparent;
  }

  .preset.selected {
    border-left-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 7%, var(--surface));
  }

  .pick {
    flex: 1;
    min-width: 0;
    justify-content: flex-start;
    align-items: flex-start;
    gap: 12px;
    padding: 12px 14px;
    min-height: 0;
    background: transparent;
    text-align: left;
    font-weight: 400;
  }

  .pick:hover:not(:disabled) {
    background: transparent;
  }

  .dot {
    flex: none;
    width: 14px;
    height: 14px;
    margin-top: 2px;
    border: 2px solid var(--line-strong);
    border-radius: 50%;
  }

  .selected .dot {
    border-color: var(--accent);
    background: radial-gradient(circle, var(--accent) 0 3px, transparent 4px);
  }

  .text {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .name {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px;
    font-size: 14px;
    font-weight: 700;
  }

  .desc {
    font-size: 12px;
    color: var(--text-muted);
  }

  .chip {
    padding: 2px 7px;
    font: 500 11px/1.3 var(--font-mono);
    font-weight: 500;
    color: var(--text-soft);
    background: var(--surface-2);
  }

  .chip.accent {
    color: var(--accent-fg);
  }

  .small {
    min-height: 28px;
    font-size: 12px;
    gap: 6px;
  }

  .editor {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 14px;
    background: var(--surface);
    border-left: 3px solid var(--line-strong);
  }

  .editor label {
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: 12px;
    color: var(--text-muted);
  }

  .fields {
    display: flex;
    gap: 10px;
  }

  .grow {
    flex: 1;
  }

  textarea {
    resize: vertical;
    font-size: 12px;
    line-height: 1.5;
  }

  .actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .spacer {
    flex: 1;
  }

  .hint {
    margin: 0;
    font-size: 12px;
    color: var(--text-dim);
  }

  .dashed {
    margin-top: 8px;
    gap: 8px;
  }
</style>
