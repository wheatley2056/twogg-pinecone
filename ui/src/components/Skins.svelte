<script lang="ts">
  import { onMount } from "svelte";
  import { open, save } from "@tauri-apps/plugin-dialog";
  import { api, errorText } from "../lib/api";
  import { t } from "../lib/i18n.svelte";
  import type { Account, AccountSkin, SavedSkin, SkinModel } from "../lib/types";
  import ConfirmDialog from "./ConfirmDialog.svelte";
  import Icon from "./Icon.svelte";
  import SkinEditor from "./SkinEditor.svelte";
  import SkinHead from "./SkinHead.svelte";
  import SkinPreview from "./SkinPreview.svelte";

  let { accounts }: { accounts: Account[] } = $props();

  let skins = $state<SavedSkin[]>([]);
  let images = $state<Record<string, string>>({});
  let selectedId = $state<string | null>(null);
  let account = $state<AccountSkin | null>(null);
  let accountError = $state<string | null>(null);
  let accountLoading = $state(false);
  let busy = $state(false);
  let error = $state<string | null>(null);
  let notice = $state<string | null>(null);
  let player = $state("");
  let editing = $state<{ skin: SavedSkin | null } | null>(null);
  let confirmDelete = $state(false);
  let renaming = $state<string | null>(null);

  const active = $derived(accounts.find((a) => a.active) ?? null);
  const online = $derived(active?.kind === "microsoft");
  const selected = $derived(skins.find((s) => s.id === selectedId) ?? null);
  const activeCape = $derived(account?.capes.find((c) => c.active)?.dataUrl ?? null);

  onMount(async () => {
    await reload();
    if (online) await loadAccount();
  });

  async function reload() {
    try {
      skins = await api.skinsList();
      for (const s of skins) {
        if (!images[s.id]) images[s.id] = await api.skinData(s.id);
      }
      if (!selectedId || !skins.some((s) => s.id === selectedId)) selectedId = skins[0]?.id ?? null;
    } catch (e) {
      error = errorText(e);
    }
  }

  async function loadAccount() {
    accountLoading = true;
    accountError = null;
    try {
      account = await api.skinAccount();
    } catch (e) {
      accountError = errorText(e);
    } finally {
      accountLoading = false;
    }
  }

  async function run<T>(action: () => Promise<T>, done?: (r: T) => void | Promise<void>) {
    busy = true;
    error = notice = null;
    try {
      const r = await action();
      await done?.(r);
    } catch (e) {
      error = errorText(e);
    } finally {
      busy = false;
    }
  }

  async function added(s: SavedSkin) {
    delete images[s.id];
    await reload();
    selectedId = s.id;
  }

  const importPlayer = () =>
    run(() => api.skinImportPlayer(player.trim()), async (s) => {
      player = "";
      await added(s);
    });

  async function importFile() {
    const path = await open({ multiple: false, filters: [{ name: "PNG", extensions: ["png"] }] });
    if (typeof path === "string") await run(() => api.skinImportFile(path, "classic"), added);
  }

  async function exportSkin() {
    if (!selected) return;
    const path = await save({ defaultPath: `${selected.name.replace(/[^\w.-]+/g, "_")}.png`, filters: [{ name: "PNG", extensions: ["png"] }] });
    if (path) await run(() => api.skinExport(selected.id, path), () => void (notice = t("skins.exported")));
  }

  const apply = () =>
    run(() => api.skinApply(selected!.id), (a) => {
      account = a;
      notice = t("skins.applied", { name: selected!.name });
    });

  const setCape = (id: string | null) => run(() => api.skinSetCape(id), (a) => void (account = a));

  const setModel = (model: SkinModel) => run(() => api.skinSetModel(selected!.id, model), reloadKeep);

  async function reloadKeep() {
    await reload();
  }

  function commitRename() {
    const name = renaming?.trim();
    renaming = null;
    if (name && selected && name !== selected.name) run(() => api.skinRename(selected.id, name), reloadKeep);
  }
</script>

<section class="skins">
  <header>
    <div>
      <h1>{t("skins.title")}</h1>
      <span class="muted">{t("skins.subtitle")}</span>
    </div>
    <div class="tools">
      <form class="player" onsubmit={(e) => (e.preventDefault(), player.trim() && importPlayer())}>
        <input bind:value={player} placeholder={t("skins.playerPlaceholder")} aria-label={t("skins.playerPlaceholder")} maxlength="16" />
        <button disabled={busy || !player.trim()}><Icon name="download" size={15} />{t("skins.fromPlayer")}</button>
      </form>
      <button onclick={importFile} disabled={busy}><Icon name="file" size={15} />{t("skins.importFile")}</button>
      <button class="primary" onclick={() => (editing = { skin: null })}><Icon name="plus" size={15} />{t("skins.create")}</button>
    </div>
  </header>
  {#if error}<p class="error">{error}</p>{/if}
  {#if notice}<p class="notice">{notice}</p>{/if}

  <div class="layout">
    <div class="stage">
      {#if selected}
        <SkinPreview skin={images[selected.id] ?? null} model={selected.model} cape={activeCape} />
        <div class="selected">
          {#if renaming !== null}
            <!-- svelte-ignore a11y_autofocus -->
            <input class="rename" bind:value={renaming} onblur={commitRename} onkeydown={(e) => e.key === "Enter" && commitRename()} autofocus maxlength="48" />
          {:else}
            <button class="name-btn" title={t("skins.rename")} onclick={() => (renaming = selected.name)}>{selected.name}</button>
          {/if}
          <div class="model" role="radiogroup" aria-label={t("skins.model")}>
            {#each [["classic", t("skins.classic")], ["slim", t("skins.slim")]] as [m, label] (m)}
              <button role="radio" aria-checked={selected.model === m} class:active={selected.model === m} onclick={() => setModel(m as SkinModel)} disabled={busy}>{label}</button>
            {/each}
          </div>
          <div class="actions">
            <button class="primary" onclick={apply} disabled={busy || !online} title={online ? "" : t("skins.needsMicrosoft")}><Icon name="check" size={15} />{t("skins.apply")}</button>
            <button onclick={() => (editing = { skin: selected })} disabled={busy}><Icon name="edit" size={15} />{t("skins.edit")}</button>
            <button onclick={exportSkin} disabled={busy}><Icon name="download" size={15} />{t("skins.export")}</button>
            <button class="danger" onclick={() => (confirmDelete = true)} disabled={busy} aria-label={t("skins.delete")}><Icon name="trash" size={15} /></button>
          </div>
          {#if !online}<p class="muted small">{t("skins.needsMicrosoft")}</p>{/if}
        </div>
      {:else}
        <p class="muted empty">{t("skins.empty")}</p>
      {/if}
    </div>

    <div class="side">
      {#if online}
        <div class="card">
          <h3>{t("skins.account", { name: account?.name ?? active?.name ?? "" })}</h3>
          {#if accountLoading}
            <p class="muted small">{t("common.loading")}</p>
          {:else if accountError}
            <p class="error small">{accountError}</p>
            <button onclick={loadAccount}>{t("skins.retry")}</button>
          {:else if account}
            <div class="current">
              <SkinHead src={account.skinDataUrl} size={40} />
              <span class="muted small">{t("skins.current")}</span>
              <button class="small" onclick={() => run(() => api.skinImportAccount(), added)} disabled={busy || !account.skinUrl}>{t("skins.toLibrary")}</button>
            </div>
            <h4>{t("skins.capes")}</h4>
            {#if account.capes.length === 0}
              <p class="muted small">{t("skins.noCapes")}</p>
            {:else}
              <div class="capes">
                <button class="cape" class:active={!account.capes.some((c) => c.active)} onclick={() => setCape(null)} disabled={busy}>{t("skins.noCape")}</button>
                {#each account.capes as c (c.id)}
                  <button class="cape" class:active={c.active} onclick={() => setCape(c.id)} disabled={busy || c.active} title={c.alias ?? c.id}>
                    {#if c.dataUrl}<span class="cape-img" style:background-image={`url(${c.dataUrl})`}></span>{/if}
                    <span>{c.alias ?? t("skins.cape")}</span>
                  </button>
                {/each}
              </div>
            {/if}
          {/if}
        </div>
      {/if}

      <div class="card">
        <h3>{t("skins.library")}</h3>
        {#if skins.length === 0}
          <p class="muted small">{t("skins.empty")}</p>
        {/if}
        <div class="grid">
          {#each skins as s (s.id)}
            <button class="tile" class:active={s.id === selectedId} onclick={() => (selectedId = s.id)} title={s.name}>
              <SkinHead src={images[s.id] ?? null} size={44} />
              <span class="tile-name">{s.name}</span>
            </button>
          {/each}
        </div>
      </div>
    </div>
  </div>
</section>

{#if editing}
  <SkinEditor
    skin={editing.skin}
    onClose={() => (editing = null)}
    onSaved={async (s) => {
      editing = null;
      await added(s);
      notice = t("skins.saved", { name: s.name });
    }}
  />
{/if}

{#if confirmDelete && selected}
  <ConfirmDialog
    title={t("skins.deleteTitle", { name: selected.name })}
    confirmLabel={t("skins.delete")}
    onCancel={() => (confirmDelete = false)}
    onConfirm={() => {
      const id = selected.id;
      confirmDelete = false;
      run(() => api.skinDelete(id), reloadKeep);
    }}
  >
    <p>{t("skins.deleteText")}</p>
  </ConfirmDialog>
{/if}

<style>
  .skins {
    display: flex;
    flex-direction: column;
    gap: 14px;
    min-height: 0;
    overflow-y: auto;
    padding-right: 4px;
  }

  header {
    display: flex;
    justify-content: space-between;
    align-items: flex-end;
    gap: 16px;
    flex-wrap: wrap;
  }

  h1 {
    font-size: 24px;
  }

  h3 {
    font-size: 14px;
    margin: 0;
  }

  h4 {
    font-size: 13px;
    margin: 4px 0 0;
  }

  p {
    margin: 0;
  }

  .tools,
  .player,
  .actions,
  .current {
    display: flex;
    gap: 8px;
    align-items: center;
    flex-wrap: wrap;
  }

  .player input {
    width: 170px;
  }

  .tools button,
  .actions button {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .notice {
    color: var(--accent);
  }

  .layout {
    display: grid;
    grid-template-columns: minmax(300px, 1fr) 340px;
    gap: 16px;
    align-items: start;
  }

  .stage {
    border: 1px solid var(--border);
    border-radius: 12px;
    background: radial-gradient(circle at 50% 35%, color-mix(in srgb, var(--accent) 12%, var(--surface-2)), var(--surface));
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    padding: 16px;
    min-height: 420px;
  }

  .empty {
    margin: auto;
    text-align: center;
    max-width: 280px;
  }

  .selected {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
    width: 100%;
  }

  .name-btn {
    border: none;
    background: transparent;
    font-size: 18px;
    font-weight: 600;
    padding: 2px 8px;
  }

  .rename {
    max-width: 260px;
    text-align: center;
  }

  .model {
    display: inline-flex;
    border: 1px solid var(--border);
    border-radius: 999px;
    overflow: hidden;
  }

  .model button {
    border: none;
    border-radius: 0;
    padding: 5px 14px;
    background: transparent;
  }

  .model button.active {
    background: var(--accent);
    color: var(--accent-text);
  }

  .side {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .card {
    border: 1px solid var(--border);
    border-radius: 12px;
    background: color-mix(in srgb, var(--surface) 92%, transparent);
    padding: 12px 14px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(72px, 1fr));
    gap: 8px;
  }

  .tile {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    padding: 8px 4px;
    min-width: 0;
  }

  .tile.active,
  .cape.active {
    border-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 12%, var(--surface));
  }

  .tile-name {
    font-size: 11px;
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .capes {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .cape {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 4px 10px 4px 6px;
    font-size: 12px;
  }

  /* Vorderseite des Capes: 10×16 Pixel ab (1,1) in der 64×32-Textur. */
  .cape-img {
    width: 15px;
    height: 24px;
    background-size: 96px 48px;
    background-position: -1.5px -1.5px;
    image-rendering: pixelated;
    border-radius: 2px;
  }

  button.small {
    padding: 3px 10px;
    font-size: 12px;
  }

  .small {
    font-size: 12px;
  }

  @media (max-width: 960px) {
    .layout {
      grid-template-columns: 1fr;
    }
  }
</style>
