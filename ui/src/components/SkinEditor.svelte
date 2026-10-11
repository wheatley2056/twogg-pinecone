<script lang="ts">
  // Hülle um den eingebetteten MC Skin Editor (ui/vendor/mc-skin-editor, Apache-2.0).
  // Der Editor läuft unverändert im iframe; Laden und Speichern gehen über postMessage.
  import { onDestroy, onMount, untrack } from "svelte";
  import { api, errorText } from "../lib/api";
  import { t } from "../lib/i18n.svelte";
  import type { SavedSkin, SkinModel } from "../lib/types";
  import { SOURCE, type FromEditor, type ToEditor, type Unsourced } from "../../skin-editor/protocol";

  interface Props {
    /** Bestehender Skin oder `null` für einen neuen. */
    skin: SavedSkin | null;
    onClose: () => void;
    onSaved: (skin: SavedSkin) => void;
  }
  let { skin, onClose, onSaved }: Props = $props();

  let frame: HTMLIFrameElement;
  // Startname einmal übernehmen; der Editor wird pro Skin neu geöffnet.
  let name = $state(untrack(() => skin?.name ?? t("skins.newName")));
  let ready = $state(false);
  let busy = $state(false);
  let error = $state<string | null>(null);
  let pending: ((m: Extract<FromEditor, { type: "png" }>) => void) | null = null;

  function send(msg: Unsourced<ToEditor>) {
    frame.contentWindow?.postMessage({ source: SOURCE, ...msg }, window.location.origin);
  }

  async function onMessage(e: MessageEvent) {
    if (e.origin !== window.location.origin || e.source !== frame?.contentWindow) return;
    const msg = e.data as FromEditor;
    if (msg?.source !== SOURCE) return;
    if (msg.type === "ready") {
      ready = true;
      try {
        if (skin) send({ type: "load", name: skin.name, model: skin.model, dataUrl: await api.skinData(skin.id) });
        else send({ type: "new", name, model: "classic" });
      } catch (err) {
        error = errorText(err);
      }
    } else if (msg.type === "png") {
      pending?.(msg);
      pending = null;
    } else if (msg.type === "error") {
      error = msg.message;
      busy = false;
    }
  }

  onMount(() => window.addEventListener("message", onMessage));
  onDestroy(() => window.removeEventListener("message", onMessage));

  async function save(asNew: boolean) {
    busy = true;
    error = null;
    const png = await new Promise<Extract<FromEditor, { type: "png" }>>((resolve) => {
      pending = resolve;
      send({ type: "export" });
    });
    try {
      const model: SkinModel = png.model;
      const saved = await api.skinSave(asNew ? null : (skin?.id ?? null), name.trim() || t("skins.newName"), model, png.dataUrl);
      onSaved(saved);
    } catch (err) {
      error = errorText(err);
    } finally {
      busy = false;
    }
  }
</script>

<div class="editor-shell">
  <header>
    <span class="title">{t("skins.editor")}</span>
    <input bind:value={name} maxlength="48" aria-label={t("skins.name")} />
    <span class="muted small">{t("skins.editorHint")}</span>
    <div class="spacer"></div>
    {#if error}<span class="error small">{error}</span>{/if}
    <button onclick={onClose} disabled={busy}>{t("common.close")}</button>
    {#if skin}<button onclick={() => save(true)} disabled={!ready || busy}>{t("skins.saveAsNew")}</button>{/if}
    <button class="primary" onclick={() => save(false)} disabled={!ready || busy}>{busy ? t("common.loading") : t("common.save")}</button>
  </header>
  <iframe bind:this={frame} src="/skin-editor.html" title={t("skins.editor")}></iframe>
  <footer class="muted small">
    {t("skins.credit")}
    <a href="https://github.com/eheikes/mc-skin-editor" target="_blank" rel="noreferrer">MC Skin Editor</a> (Apache-2.0)
  </footer>
</div>

<style>
  .editor-shell {
    position: fixed;
    inset: 0;
    z-index: 50;
    display: flex;
    flex-direction: column;
    background: var(--bg);
  }

  header {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 14px;
    border-bottom: 1px solid var(--line);
    background: var(--bar);
  }

  .title {
    font-family: var(--font-pixel);
    font-weight: 600;
    font-size: 18px;
    margin-right: 6px;
  }

  header input {
    width: 220px;
  }

  .spacer {
    flex: 1;
  }

  .error {
    padding: 4px 10px;
  }

  iframe {
    flex: 1;
    border: none;
    width: 100%;
  }

  footer {
    padding: 4px 14px;
    border-top: 1px solid var(--line-strong);
  }

  .small {
    font-size: 12px;
  }
</style>
