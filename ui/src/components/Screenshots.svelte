<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { save } from "@tauri-apps/plugin-dialog";
  import { api, errorText, fileSrc } from "../lib/api";
  import { t, locale } from "../lib/i18n.svelte";
  import type { Profile, ScreenshotItem } from "../lib/types";
  import ConfirmDialog from "./ConfirmDialog.svelte";
  import Icon from "./Icon.svelte";

  let { profiles }: { profiles: Profile[] } = $props();

  let shots = $state<ScreenshotItem[]>([]);
  let thumbs = $state<Record<string, string>>({});
  let filter = $state<string>("all");
  let open = $state<ScreenshotItem | null>(null);
  let error = $state<string | null>(null);
  let notice = $state<string | null>(null);
  let confirmDelete = $state<ScreenshotItem | null>(null);
  let unlisten: (() => void) | null = null;

  const shown = $derived(filter === "all" ? shots : shots.filter((s) => s.profileId === filter));
  const groups = $derived.by(() => {
    const out: { day: string; items: ScreenshotItem[] }[] = [];
    for (const s of shown) {
      const day = new Date(s.modified * 1000).toLocaleDateString(locale(), { weekday: "long", day: "numeric", month: "long", year: "numeric" });
      const last = out.at(-1);
      if (last?.day === day) last.items.push(s);
      else out.push({ day, items: [s] });
    }
    return out;
  });
  const index = $derived(open ? shown.findIndex((s) => s.path === open!.path) : -1);

  async function load() {
    try {
      shots = await api.screenshotsList();
    } catch (e) {
      error = errorText(e);
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
    if (index < 0) return;
    const next = shown[index + delta];
    if (next) open = next;
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
    const next = shown[index + 1] ?? shown[index - 1] ?? null;
    await act(() => api.screenshotDelete(s.path));
    if (open?.path === s.path) open = next && next.path !== s.path ? next : null;
    await load();
  }

  function time(s: ScreenshotItem) {
    return new Date(s.modified * 1000).toLocaleTimeString(locale(), { hour: "2-digit", minute: "2-digit" });
  }
</script>

<svelte:window onkeydown={onKey} />

<section class="shots">
  <header>
    <div>
      <h1>{t("shots.title")}</h1>
      <span class="muted">{t("shots.count", { count: shown.length })}</span>
    </div>
    <label class="filter">
      {t("shots.profile")}
      <select bind:value={filter}>
        <option value="all">{t("shots.allProfiles")}</option>
        {#each profiles as p (p.id)}<option value={p.id}>{p.name}</option>{/each}
      </select>
    </label>
  </header>
  {#if error}<p class="error">{error}</p>{/if}
  {#if notice}<p class="notice">{notice}</p>{/if}

  {#if shown.length === 0}
    <div class="empty">
      <Icon name="image" size={40} />
      <p class="muted">{t("shots.empty")}</p>
    </div>
  {/if}

  {#each groups as g (g.day)}
    <h2>{g.day}</h2>
    <div class="grid">
      {#each g.items as s (s.path)}
        <button class="tile" onclick={() => (open = s)} use:lazyThumb={s.path} title={`${s.profileName} · ${time(s)}`}>
          {#if thumbs[s.path]}<img src={thumbs[s.path]} alt="" loading="lazy" />{:else}<span class="placeholder"></span>{/if}
          <span class="caption"><span>{s.profileName}</span><span>{time(s)}</span></span>
        </button>
      {/each}
    </div>
  {/each}
</section>

{#if open}
  <div class="lightbox" role="dialog" aria-modal="true" aria-label={t("shots.title")}>
    <button class="backdrop" aria-label={t("common.close")} onclick={() => (open = null)}></button>
    <img src={fileSrc(open.path)} alt="" />
    <button class="nav prev" onclick={() => step(-1)} disabled={index <= 0} aria-label={t("shots.prev")}>‹</button>
    <button class="nav next" onclick={() => step(1)} disabled={index >= shown.length - 1} aria-label={t("shots.next")}>›</button>
    <div class="bar">
      <span class="info">{open.profileName} · {new Date(open.modified * 1000).toLocaleString(locale())}</span>
      <div class="spacer"></div>
      <button onclick={() => copy(open!)}>{t("shots.copy")}</button>
      <button onclick={() => saveAs(open!)}>{t("shots.saveAs")}</button>
      <button onclick={() => act(() => api.screenshotReveal(open!.path))}><Icon name="folder" size={15} />{t("shots.reveal")}</button>
      <button onclick={() => act(() => api.openScreenshot(open!.path))}><Icon name="external" size={15} />{t("shots.openExternal")}</button>
      <button class="danger" onclick={() => (confirmDelete = open)}><Icon name="trash" size={15} /></button>
      <button onclick={() => (open = null)} aria-label={t("common.close")}><Icon name="close" size={15} /></button>
    </div>
  </div>
{/if}

{#if confirmDelete}
  <ConfirmDialog title={t("shots.deleteTitle")} confirmLabel={t("shots.delete")} onCancel={() => (confirmDelete = null)} onConfirm={() => remove(confirmDelete!)}>
    <p>{t("shots.deleteText")}</p>
  </ConfirmDialog>
{/if}

<style>
  .shots {
    display: flex;
    flex-direction: column;
    gap: 12px;
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

  h2 {
    font-size: 14px;
    margin: 8px 0 0;
    color: var(--muted);
    font-weight: 600;
  }

  .filter {
    min-width: 220px;
  }

  .notice {
    color: var(--accent);
    margin: 0;
  }

  .empty {
    margin: 60px auto;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
    color: var(--muted);
    text-align: center;
    max-width: 360px;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: 10px;
  }

  .tile {
    position: relative;
    padding: 0;
    border-radius: 10px;
    overflow: hidden;
    aspect-ratio: 16 / 9;
    background: var(--surface-2);
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
    left: 0;
    right: 0;
    bottom: 0;
    display: flex;
    justify-content: space-between;
    gap: 8px;
    padding: 14px 8px 5px;
    font-size: 11px;
    color: #fff;
    background: linear-gradient(transparent, rgb(0 0 0 / 0.65));
    opacity: 0;
    transition: opacity 0.12s;
  }

  .tile:hover .caption,
  .tile:focus-visible .caption {
    opacity: 1;
  }

  .lightbox {
    position: fixed;
    inset: 0;
    z-index: 40;
    display: grid;
    place-items: center;
    padding: 24px 24px 72px;
  }

  .backdrop {
    position: absolute;
    inset: 0;
    border: none;
    border-radius: 0;
    background: rgb(0 0 0 / 0.85);
  }

  .backdrop:hover:not(:disabled) {
    background: rgb(0 0 0 / 0.85);
  }

  .lightbox img {
    position: relative;
    max-width: 100%;
    max-height: 100%;
    border-radius: 8px;
    box-shadow: 0 10px 40px rgb(0 0 0 / 0.5);
  }

  .nav {
    position: absolute;
    top: 50%;
    transform: translateY(-50%);
    font-size: 32px;
    line-height: 1;
    width: 44px;
    height: 64px;
    padding: 0;
    background: rgb(0 0 0 / 0.4);
    color: #fff;
    border: none;
  }

  .prev {
    left: 12px;
  }

  .next {
    right: 12px;
  }

  .bar {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 16px;
    background: var(--surface);
    border-top: 1px solid var(--border);
  }

  .bar button {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .info {
    font-size: 13px;
    color: var(--muted);
  }

  .spacer {
    flex: 1;
  }
</style>
