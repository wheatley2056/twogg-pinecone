<script lang="ts">
  // TwoGG-Modpacks vom Pinecone-Server (nur über den Launcher herunterladbar).
  import { onMount } from "svelte";
  import { api, errorText } from "../lib/api";
  import { t, formatAgo } from "../lib/i18n.svelte";
  import { externalLinks } from "../lib/links";
  import type { ImportResult, TwoggPack } from "../lib/types";
  import Icon from "./Icon.svelte";

  let { onInstalled }: { onInstalled: (r: ImportResult) => void } = $props();

  let packs = $state<TwoggPack[] | null>(null);
  let error = $state<string | null>(null);
  let open = $state<TwoggPack | null>(null);
  let installing = $state<string | null>(null);

  onMount(async () => {
    try {
      packs = await api.twoggPacks();
    } catch (e) {
      error = errorText(e);
      packs = [];
    }
  });

  async function details(p: TwoggPack) {
    open = p;
    try {
      open = await api.twoggPack(p.id);
    } catch (e) {
      error = errorText(e);
    }
  }

  async function install(p: TwoggPack, version: string | null = null) {
    installing = p.id;
    error = null;
    try {
      onInstalled(await api.packInstallTwogg(p.id, version));
    } catch (e) {
      error = errorText(e);
    } finally {
      installing = null;
    }
  }

  const loaderName = (l: string | null) => (l ? l[0].toUpperCase() + l.slice(1) : t("common.vanilla"));
</script>

<div class="twogg">
  <p class="muted intro">{t("twogg.intro")}</p>
  {#if error}<p class="error">{error}</p>{/if}
  {#if packs === null}
    <p class="muted">{t("common.loading")}</p>
  {:else if packs.length === 0 && !error}
    <p class="muted">{t("twogg.empty")}</p>
  {/if}

  <div class="grid">
    {#each packs ?? [] as p (p.id)}
      <article class="card" class:featured={p.featured}>
        <button class="head" onclick={() => details(p)}>
          {#if p.iconUrl}<img src={p.iconUrl} alt="" width="56" height="56" />{:else}<span class="icon"><Icon name="box" size={26} /></span>{/if}
          <div class="text">
            <strong>{p.title}</strong>
            <span class="muted small">{p.summary}</span>
          </div>
        </button>
        {#if p.latest}
          <div class="meta">
            <span class="badge">{p.latest.version}</span>
            <span class="muted small">Minecraft {p.latest.minecraft} · {loaderName(p.latest.loader)}</span>
            {#if p.featured}<span class="badge accent">{t("twogg.featured")}</span>{/if}
          </div>
        {/if}
        <div class="actions">
          <button class="ghost" onclick={() => details(p)}>{t("twogg.details")}</button>
          <button class="primary" onclick={() => install(p)} disabled={installing !== null}>
            <Icon name="download" size={15} />{installing === p.id ? t("pack.importing") : t("pack.installNew")}
          </button>
        </div>
      </article>
    {/each}
  </div>
</div>

{#if open}
  <div class="overlay" role="dialog" aria-modal="true" aria-label={open.title}>
    <button class="backdrop" aria-label={t("common.close")} onclick={() => (open = null)}></button>
    <div class="panel">
      <header>
        {#if open.iconUrl}<img src={open.iconUrl} alt="" width="48" height="48" />{/if}
        <div>
          <h2>{open.title}</h2>
          <span class="muted small">{t("twogg.downloads", { count: open.downloads })}</span>
        </div>
        <button class="ghost" onclick={() => (open = null)} aria-label={t("common.close")}><Icon name="close" size={16} /></button>
      </header>
      <div class="body">
        {#if open.descriptionHtml}
          <div class="html" use:externalLinks>{@html open.descriptionHtml}</div>
        {:else}
          <p class="muted">{open.summary}</p>
        {/if}
        {#if open.versions}
          <h3>{t("twogg.versions")}</h3>
          <ul class="versions">
            {#each open.versions as v (v.id)}
              <li>
                <div class="vhead">
                  <strong>{v.version}</strong>
                  <span class="muted small">Minecraft {v.minecraft} · {loaderName(v.loader)} · {formatAgo(v.publishedAt)}</span>
                  {#if v.channel !== "release"}<span class="badge">{v.channel}</span>{/if}
                  <button class="small" onclick={() => install(open!, v.id)} disabled={installing !== null}>{t("twogg.installVersion")}</button>
                </div>
                {#if v.changelogHtml}<div class="html small" use:externalLinks>{@html v.changelogHtml}</div>{/if}
              </li>
            {/each}
          </ul>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .twogg {
    display: flex;
    flex-direction: column;
    gap: 12px;
    min-height: 0;
    overflow-y: auto;
    padding-right: 4px;
  }

  .intro {
    margin: 0;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
    gap: 12px;
  }

  .card {
    border: 1px solid var(--border);
    border-radius: 12px;
    background: color-mix(in srgb, var(--surface) 92%, transparent);
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .card.featured {
    border-color: color-mix(in srgb, var(--accent) 55%, var(--border));
  }

  .head {
    display: flex;
    gap: 12px;
    align-items: center;
    text-align: left;
    border: none;
    background: transparent;
    padding: 0;
  }

  .head:hover:not(:disabled) {
    background: transparent;
  }

  .head img,
  .icon {
    border-radius: 10px;
    flex: none;
  }

  .icon {
    width: 56px;
    height: 56px;
    display: grid;
    place-items: center;
    background: var(--surface-2);
    color: var(--muted);
  }

  .text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .meta {
    display: flex;
    gap: 8px;
    align-items: center;
    flex-wrap: wrap;
  }

  .badge.accent {
    background: var(--accent);
    color: var(--accent-text);
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  .actions button {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .small {
    font-size: 12px;
  }

  .overlay {
    position: fixed;
    inset: 0;
    z-index: 40;
    display: grid;
    place-items: center;
  }

  .backdrop {
    position: absolute;
    inset: 0;
    border: none;
    border-radius: 0;
    background: rgb(0 0 0 / 0.6);
  }

  .backdrop:hover:not(:disabled) {
    background: rgb(0 0 0 / 0.6);
  }

  .panel {
    position: relative;
    width: min(680px, calc(100vw - 48px));
    max-height: calc(100vh - 80px);
    display: flex;
    flex-direction: column;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 12px;
    box-shadow: var(--shadow);
  }

  .panel header {
    display: flex;
    gap: 12px;
    align-items: center;
    padding: 16px 18px;
    border-bottom: 1px solid var(--border);
  }

  .panel header > div {
    flex: 1;
  }

  .panel h2 {
    margin: 0;
    font-size: 18px;
  }

  .panel header img {
    border-radius: 10px;
  }

  .body {
    padding: 14px 18px 18px;
    overflow-y: auto;
  }

  .body h3 {
    font-size: 14px;
    margin: 16px 0 8px;
  }

  .html :global(p) {
    margin: 0 0 8px;
  }

  .html :global(a) {
    color: var(--accent);
  }

  .versions {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .versions li {
    padding: 8px 10px;
    border-radius: var(--radius);
    background: var(--surface-2);
  }

  .vhead {
    display: flex;
    gap: 8px;
    align-items: center;
    flex-wrap: wrap;
  }

  .vhead button {
    margin-left: auto;
  }

  button.small {
    padding: 3px 10px;
    font-size: 12px;
  }
</style>
