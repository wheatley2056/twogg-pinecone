<script lang="ts">
  // Sync-Pack bzw. Vorlage bearbeiten: Inhalte (Modrinth, eigene Dateien, aus Instanz),
  // automatische Zuordnung und Standard-Optionen.
  import { untrack } from "svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { api, errorText } from "../../lib/api";
  import { t } from "../../lib/i18n.svelte";
  import type { ContentKind, LoaderKind, Profile, SearchHit, SyncEntry, SyncMode, SyncPack } from "../../lib/types";
  import ConfirmDialog from "../ConfirmDialog.svelte";
  import FromProfileDialog from "../FromProfileDialog.svelte";
  import Icon from "../Icon.svelte";
  import Segmented from "../ui/Segmented.svelte";
  import Switch from "../ui/Switch.svelte";

  interface Props {
    pack: SyncPack;
    profiles: Profile[];
    onChanged: (pack: SyncPack | null) => void;
  }
  let { pack, profiles, onChanged }: Props = $props();

  let draft = $state<SyncPack>(structuredClone($state.snapshot(untrack(() => pack))) as SyncPack);
  let dirty = $state(false);
  let busy = $state(false);
  let error = $state<string | null>(null);
  let confirmDelete = $state(false);
  let fromProfile = $state(false);
  let query = $state("");
  let searchKind = $state<ContentKind>("mod");
  let hits = $state<SearchHit[]>([]);
  let searching = $state(false);
  let pendingFile = $state<{ path: string; target: string } | null>(null);
  let optKey = $state("");
  let optValue = $state("");

  function change(fn: (d: SyncPack) => void) {
    fn(draft);
    dirty = true;
  }

  async function run(action: () => Promise<SyncPack | void>) {
    busy = true;
    error = null;
    try {
      const updated = await action();
      if (updated) {
        draft = structuredClone(updated);
        dirty = false;
        onChanged(updated);
      }
    } catch (e) {
      error = errorText(e);
    } finally {
      busy = false;
    }
  }

  const save = () => run(() => api.syncpackSave($state.snapshot(draft) as SyncPack));

  /** Vor Aktionen, die serverseitig speichern, ungespeicherte Änderungen sichern. */
  async function saved(): Promise<string> {
    if (dirty) await api.syncpackSave($state.snapshot(draft) as SyncPack);
    return draft.id;
  }

  async function search(e: SubmitEvent) {
    e.preventDefault();
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
    } catch (err) {
      error = errorText(err);
    } finally {
      searching = false;
    }
  }

  const inPack = (hit: SearchHit) => draft.entries.some((e) => e.type === "modrinth" && e.projectId === hit.projectId);

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

  function addOption(e: SubmitEvent) {
    e.preventDefault();
    const key = optKey.trim();
    if (!key || key.includes(":")) return;
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

  const LOADERS: [LoaderKind | null, string][] = [
    ["fabric", "Fabric"],
    ["quilt", "Quilt"],
    ["forge", "Forge"],
    ["neoforge", "NeoForge"],
    [null, "Vanilla"],
  ];
</script>

<header class="header">
  <span class="scales-right scales" aria-hidden="true"></span>
  <span class="big-icon"><Icon name="layers" size={48} /></span>
  <div class="title">
    <span class="kind">{draft.mode === "sync" ? t("sync.modeSync") : t("sync.modeTemplate")}</span>
    <input class="name-input" value={draft.name} maxlength="48" aria-label={t("sync.name")} oninput={(e) => change((d) => (d.name = e.currentTarget.value))} />
    <div class="chips">
      <span class="chip">{t("sync.entries", { count: draft.entries.length })}</span>
      {#if draft.auto.enabled}<span class="chip ok">{t("sync.autoShort")}</span>{/if}
    </div>
  </div>
  <div class="actions">
    {#if dirty}<span class="unsaved">{t("sync.unsaved")}</span>{/if}
    <button class="ghost" disabled={!dirty || busy} onclick={() => ((draft = structuredClone($state.snapshot(pack)) as SyncPack), (dirty = false))}>{t("sync.discard")}</button>
    <button class="primary cut" disabled={!dirty || busy || !draft.name.trim()} onclick={save}>{t("common.save")}</button>
  </div>
</header>

<div class="body">
  {#if error}<p class="error">{error}</p>{/if}

  <section>
    <h3 class="section-label">{t("sync.mode")}</h3>
    <div class="mode">
      <Segmented
        items={[
          { id: "sync", label: t("sync.modeSync") },
          { id: "template", label: t("sync.modeTemplate") },
        ]}
        value={draft.mode}
        label={t("sync.mode")}
        variant="soft"
        onchange={(m) => change((d) => (d.mode = m as SyncMode))}
      />
      <span class="hint">{draft.mode === "sync" ? t("sync.modeSyncHint") : t("sync.modeTemplateHint")}</span>
    </div>
    <div class="row-card">
      <span class="grow">
        <span class="row-title">{t("sync.auto")}</span>
        <span class="hint">{t("sync.autoOnly")}</span>
      </span>
      {#if draft.auto.enabled}
        {#each LOADERS as [l, label] (label)}
          <button class="toggle-chip" class:on={draft.auto.loaders.includes(l)} aria-pressed={draft.auto.loaders.includes(l)} onclick={() => toggleLoader(l)}>{label}</button>
        {/each}
      {/if}
      <Switch checked={draft.auto.enabled} label={t("sync.auto")} onchange={(on) => change((d) => (d.auto.enabled = on))} />
    </div>
  </section>

  <section>
    <h3 class="section-label">{t("sync.contents")} · {draft.entries.length}</h3>
    {#if draft.entries.length === 0}<p class="hint">{t("sync.empty")}</p>{/if}
    <div class="entries">
      {#each draft.entries as e, i (entryKey(e))}
        <div class="entry">
          <span class="letter">{[...(e.type === "file" ? (e.path.split("/").pop() ?? "?") : e.name)][0]?.toUpperCase()}</span>
          <span class="grow">
            <span class="row-title">{e.type === "file" ? e.path : e.name}</span>
            <span class="hint">{e.type === "file" ? t("sync.ownFile") : t(`content.${e.kind}`)}</span>
          </span>
          <span class="mono src">{e.type === "file" ? t("content.local") : e.type === "modrinth" ? "Modrinth" : "CurseForge"}</span>
          <button class="ghost tiny" aria-label={t("sync.remove")} title={t("sync.remove")} onclick={() => change((d) => d.entries.splice(i, 1))}>
            <Icon name="close" size={15} />
          </button>
        </div>
      {/each}
    </div>

    <form class="search" onsubmit={search}>
      <select bind:value={searchKind} aria-label={t("sync.searchKind")}>
        <option value="mod">{t("content.mod")}</option>
        <option value="resourcepack">{t("content.resourcepack")}</option>
        <option value="shader">{t("content.shader")}</option>
      </select>
      <input bind:value={query} placeholder={t("sync.searchPlaceholder")} aria-label={t("sync.searchPlaceholder")} />
      <button disabled={searching || !query.trim()}><Icon name="search" size={15} />{t("sync.search")}</button>
      <button type="button" class="ghost" onclick={() => (fromProfile = true)} disabled={busy || profiles.length === 0}>{t("sync.fromProfile")}</button>
      <button type="button" class="ghost" onclick={chooseFile} disabled={busy}>{t("sync.addFile")}</button>
    </form>
    {#if hits.length > 0}
      <div class="entries">
        {#each hits as hit (hit.projectId)}
          <div class="entry">
            {#if hit.iconUrl}<img src={hit.iconUrl} alt="" width="34" height="34" loading="lazy" />{:else}<span class="letter">{hit.title[0]}</span>{/if}
            <span class="grow row-title">{hit.title}</span>
            <button class="small" disabled={inPack(hit)} onclick={() => change((d) => d.entries.push({ type: "modrinth", projectId: hit.projectId, kind: searchKind, name: hit.title }))}>
              {inPack(hit) ? t("sync.added") : t("sync.add")}
            </button>
          </div>
        {/each}
      </div>
    {/if}
    {#if pendingFile}
      <form class="search" onsubmit={(e) => (e.preventDefault(), addFile())}>
        <input class="mono" bind:value={pendingFile.target} spellcheck="false" aria-label={t("sync.fileTarget")} />
        <button class="primary" disabled={busy || !pendingFile.target.trim()}>{t("sync.add")}</button>
        <button type="button" class="ghost" onclick={() => (pendingFile = null)}>{t("common.cancel")}</button>
      </form>
    {/if}
  </section>

  <section>
    <h3 class="section-label">{t("sync.options")}</h3>
    <p class="hint">{t("sync.optionsHint")}</p>
    <div class="options">
      {#each Object.entries(draft.options) as [k, v] (k)}
        <div class="option">
          <code class="mono">{k}</code>
          <input class="mono" value={v} oninput={(e) => change((d) => (d.options[k] = e.currentTarget.value))} aria-label={k} />
          <button class="ghost tiny" aria-label={t("sync.remove")} onclick={() => change((d) => delete d.options[k])}><Icon name="close" size={15} /></button>
        </div>
      {/each}
      <form class="option" onsubmit={addOption}>
        <input class="mono" bind:value={optKey} placeholder="fov" aria-label={t("sync.optionKey")} spellcheck="false" />
        <input class="mono" bind:value={optValue} placeholder="0.25" aria-label={t("sync.optionValue")} spellcheck="false" />
        <button class="small" disabled={!optKey.trim() || optKey.includes(":")}><Icon name="plus" size={14} />{t("sync.addOption")}</button>
      </form>
    </div>
  </section>

  <section>
    <div class="row-card">
      <span class="grow">
        <span class="row-title">{t("sync.delete")}</span>
        <span class="hint">{t("sync.deleteText")}</span>
      </span>
      <button class="danger" disabled={busy} onclick={() => (confirmDelete = true)}>{t("sync.delete")}</button>
    </div>
  </section>
</div>

{#if fromProfile}
  <FromProfileDialog
    {profiles}
    onClose={() => (fromProfile = false)}
    onPick={(profileId, keys) => {
      fromProfile = false;
      run(async () => api.syncpackAddFromProfile(await saved(), profileId, keys));
    }}
  />
{/if}

{#if confirmDelete}
  <ConfirmDialog
    title={t("sync.deleteTitle", { name: draft.name })}
    confirmLabel={t("sync.delete")}
    onCancel={() => (confirmDelete = false)}
    onConfirm={async () => {
      confirmDelete = false;
      try {
        await api.syncpackDelete(draft.id);
        onChanged(null);
      } catch (e) {
        error = errorText(e);
      }
    }}
  >
    <p>{t("sync.deleteText")}</p>
  </ConfirmDialog>
{/if}

<style>
  .header {
    position: relative;
    padding: 24px 28px 20px;
    display: flex;
    align-items: flex-end;
    gap: 20px;
    background: var(--stage);
    border-bottom: 1px solid var(--line);
  }

  .scales-right {
    position: absolute;
    inset: 0 0 0 50%;
  }

  .big-icon {
    position: relative;
    width: 96px;
    height: 96px;
    flex: none;
    display: grid;
    place-items: center;
    background: var(--surface-2);
    color: var(--accent);
    clip-path: polygon(16px 0, 100% 0, 100% calc(100% - 16px), calc(100% - 16px) 100%, 0 100%, 0 16px);
  }

  .title {
    position: relative;
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .kind {
    font-family: var(--font-mono);
    font-size: 11px;
    letter-spacing: 1.5px;
    text-transform: uppercase;
    color: var(--text-dim);
  }

  .name-input {
    padding: 0;
    min-height: 0;
    background: transparent;
    font-family: var(--font-pixel);
    font-weight: 700;
    font-size: 40px;
    line-height: 1;
  }

  .chips {
    display: flex;
    gap: 8px;
  }

  .chips .chip {
    padding: 4px 8px;
  }

  .actions {
    position: relative;
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .unsaved {
    font-size: 12px;
    color: var(--accent-fg);
  }

  .body {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 18px 28px 24px;
    display: flex;
    flex-direction: column;
    gap: 22px;
  }

  section {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  h3 {
    margin: 0;
    font-weight: 400;
  }

  .mode {
    display: flex;
    align-items: center;
    gap: 14px;
    flex-wrap: wrap;
  }

  .hint {
    margin: 0;
    font-size: 12px;
    color: var(--text-muted);
  }

  .row-card,
  .entry {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 12px;
    background: var(--surface);
    clip-path: var(--cut-1);
  }

  .grow {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .row-title {
    font-weight: 700;
    font-size: 14px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .toggle-chip {
    min-height: 28px;
    padding: 0 10px;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-muted);
  }

  .toggle-chip.on {
    background: var(--segment-active);
    color: var(--text);
    box-shadow: inset 0 -2px 0 var(--accent);
  }

  .entries {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .letter {
    width: 34px;
    height: 34px;
    flex: none;
    display: grid;
    place-items: center;
    background: var(--surface-2);
    font-family: var(--font-pixel);
    font-weight: 700;
    color: var(--text-soft);
  }

  .src {
    font-size: 12px;
    color: var(--text-muted);
  }

  .tiny {
    width: 30px;
    min-height: 30px;
    padding: 0;
  }

  .small {
    min-height: 32px;
    font-size: 13px;
  }

  .search {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }

  .search select {
    width: 150px;
  }

  .search input {
    flex: 1;
    min-width: 180px;
  }

  .options {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .option {
    display: grid;
    grid-template-columns: 200px 1fr auto;
    gap: 8px;
    align-items: center;
  }

  code {
    font-size: 13px;
    color: var(--text-soft);
  }
</style>
