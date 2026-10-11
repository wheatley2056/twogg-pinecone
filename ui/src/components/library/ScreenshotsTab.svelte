<script lang="ts">
  // Tab „Screenshots“: Galerie der Instanz nach Tagen, Vollbild mit Kopieren, Speichern,
  // Ordner, extern öffnen, Löschen. Neue Bilder erscheinen von selbst.
  import { onDestroy, onMount } from "svelte";
  import { save } from "@tauri-apps/plugin-dialog";
  import { api, errorText, fileSrc } from "../../lib/api";
  import { locale, t } from "../../lib/i18n.svelte";
  import type { Profile, ScreenshotItem } from "../../lib/types";
  import ConfirmDialog from "../ConfirmDialog.svelte";
  import Icon from "../Icon.svelte";

  let { profile }: { profile: Profile } = $props();

  let shots = $state<ScreenshotItem[] | null>(null);
  let thumbs = $state<Record<string, string>>({});
  let open = $state<ScreenshotItem | null>(null);
  let error = $state<string | null>(null);
  let notice = $state<string | null>(null);
  let confirmDelete = $state<ScreenshotItem | null>(null);
  let unlisten: (() => void) | null = null;

  const list = $derived(shots ?? []);
  const groups = $derived.by(() => {
    const out: { day: string; items: ScreenshotItem[] }[] = [];
    for (const s of list) {
      const day = new Date(s.modified * 1000).toLocaleDateString(locale(), { weekday: "long", day: "numeric", month: "long" });
      const last = out.at(-1);
      if (last?.day === day) last.items.push(s);
      else out.push({ day, items: [s] });
    }
    return out;
  });
  const index = $derived(open ? list.findIndex((s) => s.path === open!.path) : -1);

  async function load() {
    try {
      shots = (await api.screenshotsList()).filter((s) => s.profileId === profile.id);
    } catch (e) {
      error = errorText(e);
      shots = [];
    }
  }

  async function thumb(path: string) {
    if (thumbs[path]) return;
    try {
      thumbs[path] = fileSrc(await api.screenshotThumb(path));
    } catch {
      thumbs[path] = fileSrc(path);
    }
  }

  /** Vorschaubilder erst erzeugen, wenn die Kachel sichtbar wird. */
  function lazyThumb(node: HTMLElement, path: string) {
    const io = new IntersectionObserver((entries) => {
      if (entries.some((e) => e.isIntersecting)) {
        thumb(path);
        io.disconnect();
      }
    }, { rootMargin: "200px" });
    io.observe(node);
    return { destroy: () => io.disconnect() };
  }

  onMount(async () => {
    await load();
    unlisten = await api.onScreenshotsChanged(load).catch(() => null);
  });
  onDestroy(() => unlisten?.());

  function step(delta: number) {
    const next = list[index + delta];
    if (index >= 0 && next) open = next;
  }

  function onKey(e: KeyboardEvent) {
    if (!open || confirmDelete) return;
    if (e.key === "ArrowRight") step(1);
    else if (e.key === "ArrowLeft") step(-1);
    else if (e.key === "Escape") open = null;
  }

  async function act(fn: () => Promise<unknown>, message?: string) {
    error = notice = null;
    try {
      await fn();
      if (message) notice = message;
    } catch (e) {
      error = errorText(e);
    }
  }

  async function copy(s: ScreenshotItem) {
    await act(async () => {
      const blob = await (await fetch(fileSrc(s.path))).blob();
      await navigator.clipboard.write([new ClipboardItem({ "image/png": blob })]);
    }, t("shots.copied"));
  }

  async function saveAs(s: ScreenshotItem) {
    const target = await save({ defaultPath: s.path.split(/[\\/]/).pop(), filters: [{ name: "PNG", extensions: ["png"] }] });
    if (target) await act(() => api.screenshotSaveAs(s.path, target), t("shots.saved"));
  }

  async function remove(s: ScreenshotItem) {
    confirmDelete = null;
    const next = list[index + 1] ?? list[index - 1] ?? null;
    await act(() => api.screenshotDelete(s.path));
    if (open?.path === s.path) open = next && next.path !== s.path ? next : null;
    await load();
  }

  function time(s: ScreenshotItem) {
    return new Date(s.modified * 1000).toLocaleTimeString(locale(), { hour: "2-digit", minute: "2-digit" });
  }
</script>

<svelte:window onkeydown={onKey} />

<div class="tab">
  {#if error}<p class="error">{error}</p>{/if}
  {#if notice}<p class="notice">{notice}</p>{/if}

  {#if shots === null}
    <p class="empty">{t("common.loading")}</p>
  {:else if list.length === 0}
    <p class="empty">{t("shots.emptyInstance")}</p>
  {/if}

  {#each groups as g (g.day)}
    <h3 class="section-label">{g.day}</h3>
    <div class="grid">
      {#each g.items as s (s.path)}
        <button class="tile" onclick={() => (open = s)} use:lazyThumb={s.path} title={time(s)}>
          {#if thumbs[s.path]}<img src={thumbs[s.path]} alt="" loading="lazy" />{:else}<span class="placeholder"></span>{/if}
          <span class="caption mono">{time(s)}</span>
        </button>
      {/each}
    </div>
  {/each}
</div>

{#if open}
  <div class="lightbox" role="dialog" aria-modal="true" aria-label={t("shots.title")}>
    <button class="backdrop" aria-label={t("common.close")} onclick={() => (open = null)}></button>
    <img src={fileSrc(open.path)} alt="" />
    <button class="nav prev" onclick={() => step(-1)} disabled={index <= 0} aria-label={t("shots.prev")}><Icon name="chevron" size={22} /></button>
    <button class="nav next" onclick={() => step(1)} disabled={index >= list.length - 1} aria-label={t("shots.next")}><Icon name="chevron" size={22} /></button>
    <div class="bar">
      <span class="info mono">{new Date(open.modified * 1000).toLocaleString(locale())}</span>
      <span class="spacer"></span>
      <button onclick={() => copy(open!)}>{t("shots.copy")}</button>
      <button onclick={() => saveAs(open!)}>{t("shots.saveAs")}</button>
      <button class="icon-btn" aria-label={t("shots.reveal")} title={t("shots.reveal")} onclick={() => act(() => api.screenshotReveal(open!.path))}><Icon name="folder" size={17} /></button>
      <button class="icon-btn" aria-label={t("shots.openExternal")} title={t("shots.openExternal")} onclick={() => act(() => api.openScreenshot(open!.path))}><Icon name="external" size={17} /></button>
      <button class="icon-btn danger" aria-label={t("shots.delete")} title={t("shots.delete")} onclick={() => (confirmDelete = open)}><Icon name="trash" size={17} /></button>
      <button class="icon-btn" aria-label={t("common.close")} onclick={() => (open = null)}><Icon name="close" size={17} /></button>
    </div>
  </div>
{/if}

{#if confirmDelete}
  <ConfirmDialog title={t("shots.deleteTitle")} confirmLabel={t("shots.delete")} onCancel={() => (confirmDelete = null)} onConfirm={() => remove(confirmDelete!)}>
    <p>{t("shots.deleteText")}</p>
  </ConfirmDialog>
{/if}

<style>
  .tab {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 16px 28px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  h3 {
    margin: 6px 0 0;
    font-weight: 400;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: 12px;
  }

  .tile {
    position: relative;
    padding: 0;
    min-height: 0;
    aspect-ratio: 16 / 9;
    overflow: hidden;
    background: var(--surface);
    clip-path: var(--cut-2);
  }

  .tile img,
  .placeholder {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  .caption {
    position: absolute;
    right: 0;
    bottom: 0;
    padding: 3px 8px;
    font-size: 11px;
    font-weight: 400;
    background: var(--bar-glass);
    color: var(--text-soft);
  }

  .empty {
    color: var(--text-dim);
  }

  .notice {
    margin: 0;
    font-size: 13px;
    color: var(--nadel);
  }

  .lightbox {
    position: fixed;
    inset: 0;
    z-index: 50;
    display: grid;
    place-items: center;
    background: rgb(8 6 4 / 0.92);
  }

  .backdrop {
    position: absolute;
    inset: 0;
    background: transparent;
  }

  .backdrop:hover:not(:disabled) {
    background: transparent;
  }

  .lightbox img {
    position: relative;
    max-width: calc(100% - 140px);
    max-height: calc(100% - 140px);
    object-fit: contain;
  }

  .nav {
    position: absolute;
    top: 50%;
    width: 44px;
    height: 44px;
    min-height: 0;
    padding: 0;
    translate: 0 -50%;
    background: var(--surface-2);
  }

  .prev {
    left: 20px;
    rotate: 90deg;
  }

  .next {
    right: 20px;
    rotate: -90deg;
  }

  .bar {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 20px;
    background: var(--bar-glass);
    border-top: 1px solid var(--line);
  }

  .info {
    font-size: 12px;
    color: var(--text-muted);
  }

  .spacer {
    flex: 1;
  }

  .icon-btn {
    width: 38px;
    padding: 0;
  }
</style>
