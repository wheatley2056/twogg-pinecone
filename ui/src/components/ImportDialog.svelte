<script lang="ts">
  import { onMount, untrack } from "svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { api, errorText } from "../lib/api";
  import { t } from "../lib/i18n.svelte";
  import type { ImportResult, OpenRequest, PreviewResult } from "../lib/types";
  import Icon from "./Icon.svelte";

  interface Props {
    request: OpenRequest;
    onClose: () => void;
    onImported: (result: ImportResult) => void;
  }
  let { request, onClose, onImported }: Props = $props();

  let dialog: HTMLDialogElement;
  // Ohne vorgegebenen Link fragt der Dialog zuerst danach. Pro Anfrage wird der Dialog neu erzeugt.
  let linkInput = $state("");
  let asking = $state(untrack(() => request.kind === "link" && !request.link));
  let loading = $state(false);
  let installing = $state(false);
  let error = $state<string | null>(null);
  let result = $state<PreviewResult | null>(null);
  let name = $state("");

  onMount(() => {
    dialog.showModal();
    if (!asking) load(request);
  });

  async function chooseFile() {
    const path = await open({ multiple: false, filters: [{ name: t("pack.filterAny"), extensions: ["mrpack", "zip"] }] });
    if (typeof path === "string") load({ kind: "file", path });
  }

  async function load(r: OpenRequest) {
    loading = true;
    error = null;
    try {
      result = r.kind === "file" ? await api.packPreviewFile(r.path) : await api.packPreviewLink(r.link);
      name = result.preview.name;
      asking = false;
    } catch (e) {
      error = errorText(e);
    } finally {
      loading = false;
    }
  }

  async function install() {
    if (!result) return;
    installing = true;
    error = null;
    try {
      const imported = await api.packImportPending(result.token, name.trim() || null);
      result = null;
      onImported(imported);
      onClose();
    } catch (e) {
      error = errorText(e);
    } finally {
      installing = false;
    }
  }

  function close() {
    if (installing) return;
    if (result) api.packPreviewCancel(result.token).catch(() => {});
    onClose();
  }

  function size(bytes: number) {
    return bytes >= 1e6 ? `${(bytes / 1e6).toFixed(1)} MB` : `${Math.max(1, Math.round(bytes / 1e3))} kB`;
  }

  const loaderName = (kind: string) => (kind === "fabric" ? "Fabric" : "Quilt");
</script>

<dialog bind:this={dialog} oncancel={(e) => (e.preventDefault(), close())}>
  <div class="dialog-body">
    <h2>{t("import.title")}</h2>

    {#if asking}
      <form class="link-form" onsubmit={(e) => (e.preventDefault(), linkInput.trim() && load({ kind: "link", link: linkInput.trim() }))}>
        <label>
          {t("import.linkLabel")}
          <!-- svelte-ignore a11y_autofocus -->
          <input bind:value={linkInput} placeholder="pinecone://modrinth/…" spellcheck="false" autofocus />
        </label>
        <p class="muted small">{t("import.linkHint")}</p>
        <button type="button" class="dashed" onclick={chooseFile} disabled={loading}><Icon name="file" size={16} />{t("import.chooseFile")}</button>
      </form>
    {/if}

    {#if loading}
      <p class="muted">{t("import.loading")}</p>
    {/if}

    {#if result}
      {@const p = result.preview}
      <div class="head">
        <div>
          <strong class="pack-name">{p.name}</strong>
          <span class="muted">{p.version}</span>
        </div>
        <span class="badge">{t(`import.source.${result.source}`)}</span>
      </div>
      {#if p.summary}<p class="summary">{p.summary}</p>{/if}

      <dl>
        <dt>{t("import.game")}</dt>
        <dd>Minecraft {p.minecraft}{p.loader ? ` · ${loaderName(p.loader.kind)} ${p.loader.version}` : ""}</dd>
        <dt>{t("import.contents")}</dt>
        <dd>{t("import.counts", { mods: p.mods, resourcepacks: p.resourcepacks, shaders: p.shaders, other: p.otherFiles })}</dd>
        <dt>{t("import.size")}</dt>
        <dd>{size(p.downloadSize + p.archiveSize)}</dd>
      </dl>

      {#if result.source === "file"}
        <p class="warn"><Icon name="file" size={16} />{t("import.thirdParty")}</p>
      {/if}
      {#if p.missing.length > 0}
        <p class="warn">{t("import.missing", { count: p.missing.length })}</p>
      {/if}
      {#if p.blockedHosts.length > 0}
        <p class="error">{t("import.blocked", { hosts: p.blockedHosts.join(", ") })}</p>
      {/if}

      <label>
        {t("import.profileName")}
        <input bind:value={name} maxlength="64" />
      </label>
    {/if}

    {#if error}<p class="error">{error}</p>{/if}
  </div>
  <div class="dialog-actions">
    <button onclick={close} disabled={installing}>{t("common.cancel")}</button>
    {#if asking}
      <button class="primary" disabled={loading || !linkInput.trim()} onclick={() => load({ kind: "link", link: linkInput.trim() })}>{t("import.show")}</button>
    {:else}
      <button class="primary" disabled={!result || installing || result.preview.blockedHosts.length > 0} onclick={install}>
        {installing ? t("pack.importing") : t("import.install")}
      </button>
    {/if}
  </div>
</dialog>

<style>
  p {
    margin: 0;
  }

  .link-form {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .link-form input {
    font-family: ui-monospace, monospace;
  }

  .head {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 10px;
  }

  .head > div {
    display: flex;
    align-items: baseline;
    gap: 8px;
    min-width: 0;
  }

  .pack-name {
    font-size: 17px;
    overflow-wrap: anywhere;
  }

  .summary {
    color: var(--muted);
  }

  dl {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 6px 14px;
    margin: 0;
    padding: 10px 12px;
    border-radius: var(--radius);
    background: var(--surface-2);
  }

  dt {
    color: var(--muted);
    font-size: 13px;
  }

  dd {
    margin: 0;
  }

  .warn {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    padding: 8px 12px;
    border-radius: var(--radius);
    border: 1px solid color-mix(in srgb, var(--accent) 40%, transparent);
    background: color-mix(in srgb, var(--accent) 10%, transparent);
    font-size: 13px;
  }

  .warn :global(svg) {
    flex: none;
    margin-top: 1px;
  }

  .small {
    font-size: 12px;
  }
</style>
