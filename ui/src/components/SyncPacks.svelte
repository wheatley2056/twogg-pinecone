<script lang="ts">
  import { onMount } from "svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { api, errorText } from "../lib/api";
  import { t } from "../lib/i18n.svelte";
  import type { ContentKind, LoaderKind, Profile, SearchHit, SyncEntry, SyncMode, SyncPack } from "../lib/types";
  import ConfirmDialog from "./ConfirmDialog.svelte";
  import FromProfileDialog from "./FromProfileDialog.svelte";
  import Icon from "./Icon.svelte";

  let { profiles }: { profiles: Profile[] } = $props();

  let packs = $state<SyncPack[]>([]);
  let selectedId = $state<string | null>(null);
  let draft = $state<SyncPack | null>(null);
  let dirty = $state(false);
  let error = $state<string | null>(null);
  let busy = $state(false);
  let newName = $state("");
  let newMode = $state<SyncMode>("sync");
  let confirmDelete = $state(false);
  let fromProfile = $state(false);

  // Suche
  let query = $state("");
  let searchKind = $state<ContentKind>("mod");
  let hits = $state<SearchHit[]>([]);
  let searching = $state(false);

  // Eigene Datei: erst wählen, dann Ziel bestätigen.
  let pendingFile = $state<{ path: string; target: string } | null>(null);

  // Neue Option
  let optKey = $state("");
  let optValue = $state("");

  onMount(load);

  async function load() {
    try {
      packs = await api.syncpackList();
      if (!selectedId && packs.length > 0) select(packs[0]);
    } catch (e) {
      error = errorText(e);
    }
  }

  function select(p: SyncPack) {
    selectedId = p.id;
    draft = structuredClone($state.snapshot(p)) as SyncPack;
    dirty = false;
    hits = [];
    pendingFile = null;
  }

  function change(fn: (d: SyncPack) => void) {
    if (!draft) return;
    fn(draft);
    dirty = true;
  }

  async function run(action: () => Promise<SyncPack | void>) {
    busy = true;
    error = null;
    try {
      const updated = await action();
      packs = await api.syncpackList();
      if (updated) select(updated);
    } catch (e) {
      error = errorText(e);
    } finally {
      busy = false;
    }
  }

  const create = () =>
    run(async () => {
      const p = await api.syncpackCreate(newName.trim(), newMode);
      newName = "";
      return p;
    });

  const save = () => run(() => api.syncpackSave($state.snapshot(draft!) as SyncPack));

  /** Vor Aktionen, die serverseitig speichern, ungespeicherte Änderungen sichern. */
  async function saved(): Promise<string> {
    if (dirty && draft) await api.syncpackSave($state.snapshot(draft) as SyncPack);
    return draft!.id;
  }

  async function search() {
    if (!query.trim()) return;
    searching = true;
    error = null;
    try {
      const r = await api.modrinthSearch({
        query: query.trim(),
        projectType: searchKind,
        profileId: null,
        loaders: [],
        gameVersion: null,
        categories: [],
        sort: "relevance",
        offset: 0,
        limit: 8,
      });
      hits = r.hits;
    } catch (e) {
      error = errorText(e);
    } finally {
      searching = false;
    }
  }

  const inPack = (hit: SearchHit) => draft?.entries.some((e) => e.type === "modrinth" && e.projectId === hit.projectId) ?? false;

  function addHit(hit: SearchHit) {
    change((d) => d.entries.push({ type: "modrinth", projectId: hit.projectId, kind: searchKind, name: hit.title }));
  }

  async function chooseFile() {
    const path = await open({ multiple: false });
    if (typeof path !== "string") return;
    const fileName = path.split(/[\\/]/).pop() ?? "datei";
    const folder = fileName.endsWith(".zip") ? "resourcepacks" : fileName.endsWith(".jar") ? "mods" : "config";
    pendingFile = { path, target: `${folder}/${fileName}` };
  }

  const addFile = () =>
    run(async () => {
      const id = await saved();
      const p = await api.syncpackAddFile(id, pendingFile!.path, pendingFile!.target.trim());
      pendingFile = null;
      return p;
    });

  function addOption() {
    const key = optKey.trim();
    if (!key) return;
    change((d) => (d.options[key] = optValue));
    optKey = optValue = "";
  }

  function toggleLoader(l: LoaderKind | null) {
    change((d) => {
      d.auto.loaders = d.auto.loaders.includes(l) ? d.auto.loaders.filter((x) => x !== l) : [...d.auto.loaders, l];
    });
  }

  function entryKey(e: SyncEntry) {
    return e.type === "file" ? `file:${e.path}` : `${e.type}:${e.projectId}`;
  }

  function entryIcon(e: SyncEntry) {
    return e.type === "file" ? "file" : e.kind === "mod" ? "box" : e.kind === "shader" ? "discover" : "library";
  }

  function entryKind(e: SyncEntry) {
    return e.type === "file" ? t("sync.ownFile") : t(`content.${e.kind}`);
  }

  const LOADERS: [LoaderKind | null, string][] = [
    ["fabric", "Fabric"],
    ["quilt", "Quilt"],
    [null, "Vanilla"],
  ];
</script>

<section class="syncpacks">
  <header>
    <div>
      <h1>{t("sync.title")}</h1>
      <span class="muted">{t("sync.subtitle")}</span>
    </div>
  </header>
  {#if error}<p class="error">{error}</p>{/if}

  <div class="layout">
    <aside>
      {#each packs as p (p.id)}
        <button class="pack" class:active={p.id === selectedId} onclick={() => select(p)}>
          <span class="name">{p.name}</span>
          <span class="muted small">{p.mode === "sync" ? t("sync.modeSync") : t("sync.modeTemplate")} · {t("sync.entries", { count: p.entries.length })}</span>
        </button>
      {/each}
      <form class="new" onsubmit={(e) => (e.preventDefault(), newName.trim() && create())}>
        <input bind:value={newName} placeholder={t("sync.newName")} maxlength="48" aria-label={t("sync.newName")} />
        <div class="new-row">
          <select bind:value={newMode} aria-label={t("sync.mode")}>
            <option value="sync">{t("sync.modeSync")}</option>
            <option value="template">{t("sync.modeTemplate")}</option>
          </select>
          <button class="primary" disabled={!newName.trim() || busy}><Icon name="plus" size={16} />{t("sync.create")}</button>
        </div>
      </form>
    </aside>

    {#if draft}
      <div class="editor">
        <div class="row">
          <label class="grow">
            {t("sync.name")}
            <input value={draft.name} oninput={(e) => change((d) => (d.name = e.currentTarget.value))} maxlength="48" />
          </label>
          <label>
            {t("sync.mode")}
            <select value={draft.mode} onchange={(e) => change((d) => (d.mode = e.currentTarget.value as SyncMode))}>
              <option value="sync">{t("sync.modeSync")}</option>
              <option value="template">{t("sync.modeTemplate")}</option>
            </select>
          </label>
        </div>
        <p class="muted small">{draft.mode === "sync" ? t("sync.modeSyncHint") : t("sync.modeTemplateHint")}</p>

        <div class="auto">
          <label class="check">
            <input type="checkbox" checked={draft.auto.enabled} onchange={(e) => change((d) => (d.auto.enabled = e.currentTarget.checked))} />
            {t("sync.auto")}
          </label>
          {#if draft.auto.enabled}
            <span class="muted small">{t("sync.autoOnly")}</span>
            {#each LOADERS as [l, label] (label)}
              <label class="check chip"><input type="checkbox" checked={draft.auto.loaders.includes(l)} onchange={() => toggleLoader(l)} />{label}</label>
            {/each}
          {/if}
        </div>

        <h3>{t("sync.contents")}</h3>
        {#if draft.entries.length === 0}
          <p class="muted small">{t("sync.empty")}</p>
        {/if}
        <ul class="entries">
          {#each draft.entries as e, i (entryKey(e))}
            <li>
              <span class="icon"><Icon name={entryIcon(e)} size={18} /></span>
              <span class="name">{e.type === "file" ? e.path : e.name}</span>
              <span class="badge">{entryKind(e)}</span>
              <button class="icon-btn" aria-label={t("sync.remove")} title={t("sync.remove")} onclick={() => change((d) => d.entries.splice(i, 1))}>
                <Icon name="close" size={15} />
              </button>
            </li>
          {/each}
        </ul>

        <div class="add">
          <form class="search" onsubmit={(e) => (e.preventDefault(), search())}>
            <select bind:value={searchKind} aria-label={t("sync.searchKind")}>
              <option value="mod">{t("content.mod")}</option>
              <option value="resourcepack">{t("content.resourcepack")}</option>
              <option value="shader">{t("content.shader")}</option>
            </select>
            <input bind:value={query} placeholder={t("sync.searchPlaceholder")} aria-label={t("sync.searchPlaceholder")} />
            <button disabled={searching || !query.trim()}><Icon name="search" size={15} />{t("sync.search")}</button>
          </form>
          {#if hits.length > 0}
            <ul class="hits">
              {#each hits as hit (hit.projectId)}
                <li>
                  {#if hit.iconUrl}<img src={hit.iconUrl} alt="" width="28" height="28" loading="lazy" />{:else}<span class="icon"><Icon name="box" size={16} /></span>{/if}
                  <span class="name">{hit.title}</span>
                  <button class="small" disabled={inPack(hit)} onclick={() => addHit(hit)}>{inPack(hit) ? t("sync.added") : t("sync.add")}</button>
                </li>
              {/each}
            </ul>
          {/if}
          <div class="more">
            <button onclick={() => (fromProfile = true)} disabled={busy || profiles.length === 0}><Icon name="library" size={15} />{t("sync.fromProfile")}</button>
            <button onclick={chooseFile} disabled={busy}><Icon name="file" size={15} />{t("sync.addFile")}</button>
          </div>
          {#if pendingFile}
            <form class="pending" onsubmit={(e) => (e.preventDefault(), addFile())}>
              <label class="grow">
                {t("sync.fileTarget")}
                <input bind:value={pendingFile.target} spellcheck="false" />
              </label>
              <button class="primary" disabled={busy || !pendingFile.target.trim()}>{t("sync.add")}</button>
              <button type="button" class="ghost" onclick={() => (pendingFile = null)}>{t("common.cancel")}</button>
            </form>
          {/if}
        </div>

        <h3>{t("sync.options")}</h3>
        <p class="muted small">{t("sync.optionsHint")}</p>
        <ul class="options">
          {#each Object.entries(draft.options) as [k, v] (k)}
            <li>
              <code>{k}</code><span class="muted">=</span>
              <input value={v} oninput={(e) => change((d) => (d.options[k] = e.currentTarget.value))} aria-label={k} />
              <button class="icon-btn" aria-label={t("sync.remove")} onclick={() => change((d) => delete d.options[k])}><Icon name="close" size={15} /></button>
            </li>
          {/each}
        </ul>
        <form class="option-new" onsubmit={(e) => (e.preventDefault(), addOption())}>
          <input bind:value={optKey} placeholder="fov" aria-label={t("sync.optionKey")} spellcheck="false" />
          <input bind:value={optValue} placeholder="0.25" aria-label={t("sync.optionValue")} spellcheck="false" />
          <button disabled={!optKey.trim() || optKey.includes(":")}><Icon name="plus" size={15} />{t("sync.addOption")}</button>
        </form>

        <div class="actions">
          <button class="danger" onclick={() => (confirmDelete = true)} disabled={busy}><Icon name="trash" size={16} />{t("sync.delete")}</button>
          <div class="spacer"></div>
          {#if dirty}<span class="muted small">{t("sync.unsaved")}</span>{/if}
          <button onclick={() => select(packs.find((p) => p.id === draft?.id)!)} disabled={!dirty || busy}>{t("sync.discard")}</button>
          <button class="primary" onclick={save} disabled={!dirty || busy || !draft.name.trim()}>{t("common.save")}</button>
        </div>
      </div>
    {:else}
      <div class="editor empty">
        <p class="muted">{t("sync.intro")}</p>
      </div>
    {/if}
  </div>
</section>

{#if confirmDelete && draft}
  <ConfirmDialog
    title={t("sync.deleteTitle", { name: draft.name })}
    confirmLabel={t("sync.delete")}
    onCancel={() => (confirmDelete = false)}
    onConfirm={() => {
      const id = draft!.id;
      confirmDelete = false;
      draft = null;
      selectedId = null;
      run(async () => {
        await api.syncpackDelete(id);
      });
    }}
  >
    <p>{t("sync.deleteText")}</p>
  </ConfirmDialog>
{/if}

{#if fromProfile && draft}
  <FromProfileDialog
    {profiles}
    onClose={() => (fromProfile = false)}
    onPick={(profileId, keys) => {
      fromProfile = false;
      run(async () => api.syncpackAddFromProfile(await saved(), profileId, keys));
    }}
  />
{/if}

<style>
  .syncpacks {
    display: flex;
    flex-direction: column;
    gap: 16px;
    min-height: 0;
    overflow-y: auto;
    padding-right: 4px;
  }

  h1 {
    font-size: 24px;
  }

  h3 {
    font-size: 14px;
    margin: 6px 0 0;
  }

  p {
    margin: 0;
  }

  .layout {
    display: grid;
    grid-template-columns: 240px 1fr;
    gap: 16px;
    align-items: start;
  }

  aside {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .pack {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
    text-align: left;
    padding: 10px 12px;
  }

  .pack.active {
    border-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 12%, var(--surface));
  }

  .new {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-top: 8px;
    padding-top: 10px;
    border-top: 1px solid var(--border);
  }

  .new-row {
    display: flex;
    gap: 6px;
  }

  .new-row select {
    flex: 1;
    min-width: 0;
  }

  .new-row button,
  .more button,
  .search button,
  .option-new button,
  .actions button {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .editor {
    border: 1px solid var(--border);
    border-radius: 12px;
    background: color-mix(in srgb, var(--surface) 92%, transparent);
    padding: 16px 18px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .editor.empty {
    min-height: 160px;
    display: grid;
    place-items: center;
    text-align: center;
  }

  .row,
  .pending {
    display: flex;
    gap: 10px;
    align-items: flex-end;
  }

  .grow {
    flex: 1;
  }

  .auto {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px 12px;
  }

  .check {
    flex-direction: row;
    align-items: center;
    gap: 6px;
    font-size: 13px;
    color: var(--text);
  }

  .check input {
    width: auto;
  }

  .chip {
    padding: 2px 10px 2px 6px;
    border: 1px solid var(--border);
    border-radius: 999px;
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .entries li,
  .hits li,
  .options li {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 6px 8px 6px 10px;
    border-radius: var(--radius);
    background: var(--surface-2);
    font-size: 13px;
  }

  .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .icon {
    display: grid;
    place-items: center;
    color: var(--muted);
  }

  .hits img {
    border-radius: 6px;
  }

  .add {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 10px;
    border: 1px dashed var(--border);
    border-radius: var(--radius);
  }

  .search,
  .option-new {
    display: flex;
    gap: 6px;
  }

  .search select {
    width: auto;
  }

  .search input,
  .option-new input {
    flex: 1;
    min-width: 0;
  }

  .more {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }

  .options input {
    flex: 1;
    min-width: 0;
  }

  .options code {
    font-size: 12px;
  }

  button.small {
    padding: 4px 10px;
    font-size: 12px;
  }

  .actions {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 6px;
    padding-top: 12px;
    border-top: 1px solid var(--border);
  }

  .spacer {
    flex: 1;
  }

  .icon-btn {
    padding: 4px;
    border: none;
    background: transparent;
    color: var(--muted);
  }

  .icon-btn:hover:not(:disabled) {
    color: var(--danger);
    background: transparent;
  }

  .small {
    font-size: 12px;
  }

  @media (max-width: 900px) {
    .layout {
      grid-template-columns: 1fr;
    }
  }
</style>
