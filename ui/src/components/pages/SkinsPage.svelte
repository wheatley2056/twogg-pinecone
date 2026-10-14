<script lang="ts">
  // Skins (im Prototyp noch nicht gestaltet, gebaut nach dem Design-System): Bibliothek links,
  // Bühne mit 3D-Vorschau, rechts Auswahl, Konto-Skin und Capes. Der Editor ist eingebunden
  // (mc-skin-editor), nicht selbst gebaut.
  import { onMount } from "svelte";
  import { open, save } from "@tauri-apps/plugin-dialog";
  import { api, errorText } from "../../lib/api";
  import { t } from "../../lib/i18n.svelte";
  import type { Account, AccountSkin, SavedSkin, SkinModel } from "../../lib/types";
  import ConfirmDialog from "../ConfirmDialog.svelte";
  import Icon from "../Icon.svelte";
  import SkinEditor from "../SkinEditor.svelte";
  import SkinHead from "../SkinHead.svelte";
  import SkinPreview from "../SkinPreview.svelte";
  import Segmented from "../ui/Segmented.svelte";
  import { ANIMATIONS, type AnimationId } from "../../lib/skinAnimations";
  import { settings } from "../../lib/settings.svelte";

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
  let animation = $state<AnimationId>("idle");
  let rotate = $state(false);

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

  async function importFile() {
    const path = await open({ multiple: false, filters: [{ name: "PNG", extensions: ["png"] }] });
    if (typeof path === "string") await run(() => api.skinImportFile(path, "classic"), added);
  }

  async function exportSkin() {
    if (!selected) return;
    const path = await save({ defaultPath: `${selected.name.replace(/[^\w.-]+/g, "_")}.png`, filters: [{ name: "PNG", extensions: ["png"] }] });
    if (path) await run(() => api.skinExport(selected.id, path), () => void (notice = t("skins.exported")));
  }

  function commitRename() {
    const name = renaming?.trim();
    renaming = null;
    if (name && selected && name !== selected.name) run(() => api.skinRename(selected.id, name), reload);
  }

  // Die Vorschau nutzt die ganze Bühne; unten bleibt Platz für die Animationsauswahl.
  let stageW = $state(0);
  let stageH = $state(0);
  const MOTION_SPACE = 96;
  const previewW = $derived(Math.max(240, Math.round(stageW)));
  const previewH = $derived(Math.max(300, Math.round(stageH - MOTION_SPACE)));
</script>

<div class="skins">
  <section class="side" aria-label={t("skins.library")}>
    <div class="head">
      <h2>{t("tab.skins")}</h2>
      <button class="add" aria-label={t("skins.create")} title={t("skins.create")} onclick={() => (editing = { skin: null })}>
        <Icon name="plus" size={16} stroke={2.6} />
      </button>
    </div>
    <form class="player" onsubmit={(e) => (e.preventDefault(), player.trim() && run(() => api.skinImportPlayer(player.trim()), async (s) => ((player = ""), await added(s))))}>
      <input bind:value={player} placeholder={t("skins.playerPlaceholder")} aria-label={t("skins.playerPlaceholder")} maxlength="16" spellcheck="false" />
      <button disabled={busy || !player.trim()} aria-label={t("skins.fromPlayer")} title={t("skins.fromPlayer")}><Icon name="download" size={16} /></button>
    </form>
    <span class="label">{t("skins.library")} · {skins.length}</span>
    <div class="grid">
      {#each skins as s (s.id)}
        <button class="tile" class:active={s.id === selectedId} onclick={() => (selectedId = s.id)} title={s.name}>
          <SkinHead src={images[s.id] ?? null} size={48} />
          <span class="tile-name">{s.name}</span>
        </button>
      {:else}
        <p class="empty">{t("skins.empty")}</p>
      {/each}
    </div>
    <button class="dashed" onclick={importFile} disabled={busy}><Icon name="file" size={16} />{t("skins.importFile")}</button>
  </section>

  <main class="stage scales" bind:clientWidth={stageW} bind:clientHeight={stageH}>
    {#if selected}
      <div class="preview">
        <SkinPreview
          skin={images[selected.id] ?? null}
          model={selected.model}
          cape={activeCape}
          width={previewW}
          height={previewH}
          {animation}
          {rotate}
          calm={settings.current?.theme.reduceMotion ?? false}
        />
      </div>
      <div class="motion">
        <Segmented
          items={ANIMATIONS.map((id) => ({ id, label: t(`skins.anim.${id}`) }))}
          value={animation}
          label={t("skins.animation")}
          variant="soft"
          onchange={(id) => (animation = id)}
        />
        <button class="icon-only" class:on={rotate} aria-pressed={rotate} onclick={() => (rotate = !rotate)} aria-label={t("skins.rotate")} title={t("skins.rotate")}>
          <Icon name="refresh" size={15} />
        </button>
      </div>
    {:else}
      <p class="empty center">{t("skins.empty")}</p>
    {/if}
  </main>

  <aside class="panel">
    {#if selected}
      <section>
        <span class="section-label">{t("skins.selected")}</span>
        {#if renaming !== null}
          <!-- svelte-ignore a11y_autofocus -->
          <input class="rename" bind:value={renaming} onblur={commitRename} onkeydown={(e) => e.key === "Enter" && commitRename()} autofocus maxlength="48" />
        {:else}
          <button class="name" title={t("skins.rename")} onclick={() => (renaming = selected.name)}>{selected.name}</button>
        {/if}
        <Segmented
          items={[
            { id: "classic", label: t("skins.classic") },
            { id: "slim", label: t("skins.slim") },
          ]}
          value={selected.model}
          label={t("skins.model")}
          variant="soft"
          onchange={(m) => run(() => api.skinSetModel(selected.id, m as SkinModel), reload)}
        />
        <button class="primary cut big" onclick={() => run(() => api.skinApply(selected.id), (a) => void ((account = a), (notice = t("skins.applied", { name: selected.name }))))} disabled={busy || !online} title={online ? undefined : t("skins.needsMicrosoft")}>
          <Icon name="check" size={16} stroke={2.2} />{t("skins.apply")}
        </button>
        <div class="row">
          <button onclick={() => (editing = { skin: selected })} disabled={busy}><Icon name="edit" size={15} />{t("skins.edit")}</button>
          <button onclick={exportSkin} disabled={busy}><Icon name="download" size={15} />{t("skins.export")}</button>
          <button class="danger icon-only" onclick={() => (confirmDelete = true)} disabled={busy} aria-label={t("skins.delete")} title={t("skins.delete")}><Icon name="trash" size={15} /></button>
        </div>
        {#if !online}<p class="hint">{t("skins.needsMicrosoft")}</p>{/if}
      </section>
    {/if}

    {#if online}
      <section>
        <span class="section-label">{t("skins.account", { name: account?.name ?? active?.name ?? "" })}</span>
        {#if accountLoading}
          <p class="hint">{t("common.loading")}</p>
        {:else if accountError}
          <p class="error">{accountError}</p>
          <button onclick={loadAccount}>{t("skins.retry")}</button>
        {:else if account}
          <div class="current">
            <SkinHead src={account.skinDataUrl} size={40} />
            <span class="grow">{t("skins.current")}</span>
            <button class="small" onclick={() => run(() => api.skinImportAccount(), added)} disabled={busy || !account.skinUrl}>{t("skins.toLibrary")}</button>
          </div>
          <span class="section-label">{t("skins.capes")}</span>
          {#if account.capes.length === 0}
            <p class="hint">{t("skins.noCapes")}</p>
          {:else}
            <div class="capes">
              <button class="cape" class:active={!account.capes.some((c) => c.active)} onclick={() => run(() => api.skinSetCape(null), (a) => void (account = a))} disabled={busy}>{t("skins.noCape")}</button>
              {#each account.capes as c (c.id)}
                <button class="cape" class:active={c.active} onclick={() => run(() => api.skinSetCape(c.id), (a) => void (account = a))} disabled={busy || c.active} title={c.alias ?? c.id}>
                  {#if c.dataUrl}<span class="cape-img" style:background-image={`url(${c.dataUrl})`}></span>{/if}
                  <span>{c.alias ?? t("skins.cape")}</span>
                </button>
              {/each}
            </div>
          {/if}
        {/if}
      </section>
    {/if}

    {#if error}<p class="error">{error}</p>{/if}
    {#if notice}<p class="notice">{notice}</p>{/if}
  </aside>
</div>

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
      run(() => api.skinDelete(id), reload);
    }}
  >
    <p>{t("skins.deleteText")}</p>
  </ConfirmDialog>
{/if}

<style>
  .skins {
    display: grid;
    grid-template-columns: 280px minmax(0, 1fr) 320px;
    height: 100%;
    min-height: 0;
  }

  .side {
    background: var(--side);
    border-right: 1px solid var(--line);
    padding: 18px 14px;
    display: flex;
    flex-direction: column;
    gap: 14px;
    min-height: 0;
  }

  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  h2 {
    font-weight: 600;
    font-size: 22px;
  }

  .add {
    width: 34px;
    height: 34px;
    min-height: 34px;
    padding: 0;
    background: var(--accent);
    color: var(--ink);
    box-shadow: var(--edge-accent);
    clip-path: polygon(6px 0, 100% 0, 100% calc(100% - 6px), calc(100% - 6px) 100%, 0 100%, 0 6px);
  }

  .add:hover:not(:disabled) {
    background: var(--accent-hover);
  }

  .player {
    display: flex;
    gap: 6px;
  }

  .player button {
    width: 40px;
    padding: 0;
  }

  .label {
    font-family: var(--font-mono);
    font-size: 11px;
    letter-spacing: 1px;
    text-transform: uppercase;
    color: var(--text-dim);
  }

  .grid {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    align-content: start;
    gap: 8px;
  }

  .tile {
    flex-direction: column;
    gap: 6px;
    padding: 10px 6px 8px;
    min-height: 0;
    background: var(--surface);
    clip-path: var(--cut-2);
  }

  .tile:hover:not(:disabled) {
    background: var(--surface-2);
  }

  .tile.active {
    background: var(--surface-active);
    box-shadow: inset 0 -3px 0 var(--accent);
  }

  .tile-name {
    max-width: 100%;
    font-size: 11px;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .stage {
    position: relative;
    background: var(--stage);
    display: grid;
    place-items: center;
    min-height: 0;
    overflow: hidden;
  }

  .preview {
    position: absolute;
    top: 0;
    left: 0;
  }

  .motion {
    position: absolute;
    left: 50%;
    bottom: 18px;
    transform: translateX(-50%);
    display: flex;
    align-items: center;
    gap: 8px;
    width: max-content;
    max-width: calc(100% - 32px);
  }

  /* Acht Animationen: bei schmaler Bühne in zwei Reihen statt abgeschnitten. */
  .motion :global(.segmented) {
    flex-wrap: wrap;
    justify-content: center;
    white-space: nowrap;
  }

  .motion :global(.segmented button) {
    padding: 0 11px;
  }

  .motion .on {
    color: var(--accent);
    border-color: var(--edge-accent);
  }

  .panel {
    background: var(--side);
    border-left: 1px solid var(--line);
    padding: 20px 18px;
    display: flex;
    flex-direction: column;
    gap: 22px;
    overflow-y: auto;
  }

  section {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .name {
    justify-content: flex-start;
    min-height: 0;
    padding: 0;
    background: transparent;
    font-family: var(--font-pixel);
    font-size: 26px;
    font-weight: 700;
    text-align: left;
  }

  .name:hover:not(:disabled) {
    background: transparent;
    color: var(--accent-hover);
  }

  .rename {
    font-family: var(--font-pixel);
    font-size: 22px;
  }

  .big {
    height: 44px;
  }

  .row {
    display: flex;
    gap: 6px;
  }

  .row button {
    flex: 1;
    font-size: 13px;
  }

  .row .icon-only {
    flex: none;
    width: 40px;
    padding: 0;
  }

  .current {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px;
    background: var(--surface);
    clip-path: var(--cut-1);
  }

  .grow {
    flex: 1;
    font-size: 13px;
    color: var(--text-muted);
  }

  .small {
    min-height: 30px;
    font-size: 12px;
  }

  .capes {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 6px;
  }

  .cape {
    flex-direction: column;
    gap: 6px;
    padding: 8px;
    min-height: 0;
    background: var(--surface);
    font-size: 12px;
  }

  .cape.active {
    background: var(--surface-active);
    box-shadow: inset 0 -3px 0 var(--accent);
  }

  .cape-img {
    width: 30px;
    height: 48px;
    background-size: 640% 200%;
    background-position: 1.6% 0;
    image-rendering: pixelated;
  }

  .hint {
    margin: 0;
    font-size: 12px;
    color: var(--text-muted);
  }

  .empty {
    color: var(--text-dim);
    font-size: 13px;
    grid-column: 1 / -1;
  }

  .center {
    position: relative;
  }

  .notice {
    margin: 0;
    font-size: 13px;
    color: var(--nadel);
  }
</style>
