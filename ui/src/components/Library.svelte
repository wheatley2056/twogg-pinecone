<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { api, errorText } from "../lib/api";
  import { t } from "../lib/i18n.svelte";
  import { runOf } from "../lib/runs.svelte";
  import type { ImportResult, Profile } from "../lib/types";
  import Icon from "./Icon.svelte";
  import ProfileIcon from "./ProfileIcon.svelte";

  interface Props {
    profiles: Profile[];
    hasAccount: boolean;
    onOpen: (id: string) => void;
    onPlay: (id: string) => void;
    onCreate: () => void;
    onImported: (result: ImportResult) => void;
  }
  let { profiles, hasAccount, onOpen, onPlay, onCreate, onImported }: Props = $props();

  let importing = $state(false);
  let error = $state<string | null>(null);

  async function importPack() {
    const path = await open({ multiple: false, filters: [{ name: t("pack.filter"), extensions: ["mrpack"] }] });
    if (typeof path !== "string") return;
    importing = true;
    error = null;
    try {
      onImported(await api.packImportFile(path));
    } catch (e) {
      error = errorText(e);
    } finally {
      importing = false;
    }
  }

  let filter = $state("");
  const shown = $derived(
    profiles.filter((p) => {
      const q = filter.trim().toLowerCase();
      return !q || p.name.toLowerCase().includes(q) || p.minecraft.includes(q) || (p.loader?.kind ?? "vanilla").includes(q);
    }),
  );

  function loaderLabel(p: Profile) {
    return p.loader ? (p.loader.kind === "fabric" ? "Fabric" : "Quilt") : t("common.vanilla");
  }
</script>

<section class="library">
  <header>
    <div>
      <h1>{t("library.title")}</h1>
      <span class="muted">{t("library.count", { count: profiles.length })}</span>
    </div>
    <div class="tools">
      <label class="search">
        <Icon name="search" size={16} />
        <input bind:value={filter} placeholder={t("library.search")} aria-label={t("library.search")} />
      </label>
      <button onclick={importPack} disabled={importing}><Icon name="file" size={16} />{importing ? t("pack.importing") : t("pack.import")}</button>
      <button class="primary" onclick={onCreate}><Icon name="plus" size={16} />{t("nav.newProfile").replace("+ ", "")}</button>
    </div>
  </header>
  {#if error}<p class="error">{error}</p>{/if}

  {#if shown.length === 0}
    <p class="muted">{profiles.length === 0 ? t("nav.noProfiles") : t("library.noMatch")}</p>
  {/if}

  <div class="grid">
    {#each shown as p (p.id)}
      {@const run = runOf(p.id)}
      <div class="card" style:--profile-color={p.color ?? "var(--surface-3)"}>
        <button class="card-open" onclick={() => onOpen(p.id)}>
          <div class="banner">
            <ProfileIcon profile={p} size={56} />
            {#if run.status !== "idle"}<span class="running">{t("nav.running")}</span>{/if}
          </div>
          <div class="card-body">
            <span class="name">{p.name}</span>
            <span class="meta">{p.minecraft} · {loaderLabel(p)}</span>
          </div>
        </button>
        <button
          class="primary play"
          title={t("home.play")}
          aria-label={`${t("home.play")}: ${p.name}`}
          disabled={!hasAccount || run.status !== "idle"}
          onclick={() => onPlay(p.id)}><Icon name="play" size={18} filled /></button
        >
      </div>
    {/each}
  </div>
</section>

<style>
  .library {
    display: flex;
    flex-direction: column;
    gap: 18px;
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

  .tools {
    display: flex;
    gap: 8px;
  }

  .tools button {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .search {
    flex-direction: row;
    align-items: center;
    gap: 6px;
    padding: 0 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg);
    color: var(--muted);
  }

  .search input {
    border: none;
    background: transparent;
    padding: 7px 0;
    width: 200px;
  }

  .search input:focus-visible {
    outline: none;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(190px, 1fr));
    gap: 14px;
  }

  .card {
    position: relative;
    border: 1px solid var(--border);
    border-radius: 12px;
    background: color-mix(in srgb, var(--surface) 92%, transparent);
    overflow: hidden;
    transition: transform 0.12s, border-color 0.12s;
  }

  .card:hover {
    transform: translateY(-2px);
    border-color: color-mix(in srgb, var(--accent) 50%, var(--border));
  }

  .card-open {
    width: 100%;
    border: none;
    border-radius: 0;
    padding: 0;
    background: transparent;
    text-align: left;
    display: flex;
    flex-direction: column;
  }

  .card-open:hover:not(:disabled) {
    background: transparent;
  }

  .banner {
    position: relative;
    height: 96px;
    display: grid;
    place-items: center;
    background: linear-gradient(135deg, color-mix(in srgb, var(--profile-color) 45%, var(--surface-2)), var(--surface-2));
  }

  .running {
    position: absolute;
    top: 8px;
    left: 8px;
    font-size: 11px;
    padding: 2px 8px;
    border-radius: 999px;
    background: var(--accent);
    color: var(--accent-text);
    font-weight: 600;
  }

  .card-body {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 10px 12px 12px;
  }

  .name {
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .meta {
    font-size: 12px;
    color: var(--muted);
  }

  /* Play-Button erscheint beim Hovern (bzw. Tastaturfokus) über dem Banner. */
  .play {
    position: absolute;
    top: 58px;
    right: 10px;
    width: 42px;
    height: 42px;
    padding: 0;
    border-radius: 50%;
    display: grid;
    place-items: center;
    opacity: 0;
    transform: translateY(6px);
    transition: opacity 0.12s, transform 0.12s;
    box-shadow: 0 4px 14px rgb(0 0 0 / 0.35);
  }

  .card:hover .play,
  .play:focus-visible {
    opacity: 1;
    transform: none;
  }
</style>
