<script lang="ts">
  import { onMount, untrack } from "svelte";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { api, errorText } from "../lib/api";
  import { formatAgo, locale, t } from "../lib/i18n.svelte";
  import type { Category, ContentKind, ImportResult, ModrinthVersion, Platform, Profile, ProjectType, SearchHit, SortIndex, VersionInfo } from "../lib/types";
  import Icon from "./Icon.svelte";
  import ProfileIcon from "./ProfileIcon.svelte";
  import TwoggPacks from "./TwoggPacks.svelte";

  interface Props {
    profiles: Profile[];
    initialProfileId: string | null;
    initialType: ProjectType;
    curseforge: boolean;
    onPackInstalled: (result: ImportResult) => void;
  }
  let { profiles, initialProfileId, initialType, curseforge, onPackInstalled }: Props = $props();

  let source = $state<Platform>("modrinth");
  const CF_PATHS: Record<ProjectType, string> = { mod: "mc-mods", resourcepack: "texture-packs", shader: "shaders", modpack: "modpacks" };

  const TYPES: ProjectType[] = ["mod", "resourcepack", "shader", "modpack"];
  const SORTS: SortIndex[] = ["relevance", "downloads", "follows", "newest", "updated"];
  const PAGE = 20;

  // Startwerte einmalig übernehmen; die Ansicht wird für jedes Öffnen neu erzeugt.
  const start = untrack(() => ({ profileId: initialProfileId, type: initialType }));
  let type = $state<ProjectType>(start.type);
  let profileId = $state<string | null>(start.profileId);
  let query = $state("");
  let sort = $state<SortIndex>("relevance");
  let selectedCategories = $state<string[]>([]);
  let loader = $state<string>("");
  let gameVersion = $state<string>("");

  let hits = $state<SearchHit[]>([]);
  let total = $state(0);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let message = $state<string | null>(null);
  let categories = $state<Category[]>([]);
  let gameVersions = $state<VersionInfo[]>([]);
  let installed = $state<Set<string>>(new Set());
  let installing = $state<Set<string>>(new Set());
  let versionMenu = $state<{ hit: SearchHit; versions: ModrinthVersion[] | null } | null>(null);

  const profile = $derived(profiles.find((p) => p.id === profileId) ?? null);
  const contentKind = $derived(type === "modpack" ? null : (type as ContentKind));
  const blockedByVanilla = $derived(type === "mod" && !!profile && !profile.loader);
  const typeCategories = $derived(
    Object.entries(
      categories
        .filter((c) => c.projectType === type)
        .reduce<Record<string, string[]>>((groups, c) => {
          (groups[c.header] ??= []).push(c.name);
          return groups;
        }, {}),
    ),
  );
  const compact = new Intl.NumberFormat(locale(), { notation: "compact", maximumFractionDigits: 1 });

  onMount(async () => {
    [categories, gameVersions] = await Promise.all([api.modrinthCategories().catch(() => []), api.versions(false).catch(() => [])]);
  });

  // Installierte Projekte des Zielprofils für „Installiert“-Markierung.
  $effect(() => {
    const id = profileId;
    if (!id) {
      installed = new Set();
      return;
    }
    api
      .contentList(id)
      .then((list) => {
        installed = new Set(
          list.flatMap((c) =>
            c.source.type === "modrinth" ? [c.source.project_id] : c.source.type === "curseforge" ? [`cf:${c.source.project_id}`] : [],
          ),
        );
      })
      .catch(() => {});
  });

  // Neue Suche bei jeder Filteränderung, Texteingabe entprellt.
  let request = 0;
  let timer: ReturnType<typeof setTimeout> | undefined;
  $effect(() => {
    // TwoGG-Packs lädt die eigene Ansicht.
    if (source === "twogg") return;
    const args = {
      query,
      projectType: type,
      profileId: blockedByVanilla ? null : profileId,
      loaders: !profileId && loader ? [loader] : [],
      gameVersion: !profileId && gameVersion ? gameVersion : null,
      categories: [...selectedCategories],
      sort,
      offset: 0,
      limit: PAGE,
    };
    clearTimeout(timer);
    timer = setTimeout(() => search(args, false), query ? 300 : 0);
  });

  async function search(args: Parameters<typeof api.modrinthSearch>[0], append: boolean) {
    const id = ++request;
    loading = true;
    error = null;
    try {
      const result = source === "curseforge" ? await api.curseforgeSearch(args) : await api.modrinthSearch(args);
      if (id !== request) return;
      hits = append ? [...hits, ...result.hits] : result.hits;
      total = result.totalHits;
    } catch (e) {
      if (id === request) error = errorText(e);
    } finally {
      if (id === request) loading = false;
    }
  }

  function loadMore() {
    search(
      {
        query,
        projectType: type,
        profileId: blockedByVanilla ? null : profileId,
        loaders: !profileId && loader ? [loader] : [],
        gameVersion: !profileId && gameVersion ? gameVersion : null,
        categories: [...selectedCategories],
        sort,
        offset: hits.length,
        limit: PAGE,
      },
      true,
    );
  }

  function setType(next: ProjectType) {
    if (next === type) return;
    type = next;
    selectedCategories = [];
    message = null;
  }

  function toggleCategory(name: string) {
    selectedCategories = selectedCategories.includes(name) ? selectedCategories.filter((c) => c !== name) : [...selectedCategories, name];
  }

  async function install(hit: SearchHit, version: string | null = null) {
    if (!profileId || !contentKind) return;
    versionMenu = null;
    error = message = null;
    installing = new Set([...installing, hit.projectId]);
    try {
      const r =
        source === "curseforge"
          ? await api.contentInstallCurseforge(profileId, contentKind, Number(hit.projectId), version ? Number(version) : null)
          : await api.contentInstall(profileId, contentKind, hit.projectId, version);
      installed = new Set([...installed, installedKey(hit)]);
      const parts = [];
      if (r.installed.length) parts.push(t("content.installedMsg", { names: r.installed.map(([n, v]) => `${n} ${v}`).join(", ") }));
      if (r.alreadyPresent.length && !r.installed.length) parts.push(t("content.alreadyMsg", { names: r.alreadyPresent.join(", ") }));
      parts.push(...r.warnings);
      message = parts.join(" · ");
    } catch (e) {
      error = errorText(e);
    } finally {
      installing = new Set([...installing].filter((p) => p !== hit.projectId));
    }
  }

  async function installPack(hit: SearchHit, version: string | null = null) {
    versionMenu = null;
    error = message = null;
    installing = new Set([...installing, hit.projectId]);
    try {
      onPackInstalled(
        source === "curseforge"
          ? await api.packInstallCurseforge(Number(hit.projectId), version ? Number(version) : null)
          : await api.packInstallModrinth(hit.projectId, version),
      );
    } catch (e) {
      error = errorText(e);
    } finally {
      installing = new Set([...installing].filter((p) => p !== hit.projectId));
    }
  }

  async function chooseVersion(hit: SearchHit) {
    versionMenu = { hit, versions: null };
    try {
      const target = type === "modpack" ? null : profileId;
      const versions =
        source === "curseforge"
          ? await api.curseforgeFiles(Number(hit.projectId), target, type)
          : await api.modrinthVersions(hit.projectId, target, type);
      if (versionMenu?.hit === hit) versionMenu = { hit, versions };
    } catch (e) {
      error = errorText(e);
      versionMenu = null;
    }
  }

  function installedKey(hit: SearchHit) {
    return source === "curseforge" ? `cf:${hit.projectId}` : hit.projectId;
  }

  function setSource(next: Platform) {
    if (next === source) return;
    source = next;
    hits = [];
    total = 0;
    selectedCategories = [];
    message = null;
  }

  function webUrl(hit: SearchHit) {
    return source === "curseforge"
      ? `https://www.curseforge.com/minecraft/${CF_PATHS[type]}/${hit.slug}`
      : `https://modrinth.com/${hit.projectType}/${hit.slug}`;
  }

  function categoryLabel(header: string) {
    const key = `category.${header}` as const;
    return ["categories", "features", "resolutions", "performance impact"].includes(header) ? t(key as "category.categories") : header;
  }
</script>

<svelte:window
  onkeydown={(e) => e.key === "Escape" && (versionMenu = null)}
  onclick={(e) => versionMenu && !(e.target as HTMLElement).closest(".actions") && (versionMenu = null)}
/>

<section class="discover">
  <header>
    <h1>{t("discover.title")}</h1>
    {#if type === "modpack" || source === "twogg"}
      <span class="muted">{t("pack.newProfileHint")}</span>
    {:else}
    <label class="target">
      {t("discover.for")}
      <span class="target-select">
        {#if profile}<ProfileIcon {profile} size={24} />{/if}
        <select bind:value={profileId}>
          <option value={null}>{t("discover.noProfile")}</option>
          {#each profiles as p (p.id)}
            <option value={p.id}>{p.name} · {p.minecraft}{p.loader ? ` · ${p.loader.kind === "fabric" ? "Fabric" : "Quilt"}` : ""}</option>
          {/each}
        </select>
      </span>
    </label>
    {/if}
  </header>

  <div class="tabs-row">
  {#if source !== "twogg"}
  <div class="types" role="tablist">
    {#each TYPES as ty (ty)}
      <button role="tab" aria-selected={type === ty} class:active={type === ty} onclick={() => setType(ty)}>
        {ty === "modpack" ? t("discover.modpack") : t(`content.${ty}`)}
      </button>
    {/each}
  </div>
  {:else}
    <div></div>
  {/if}
  <div class="sources" role="radiogroup" aria-label={t("discover.source")}>
    <button role="radio" aria-checked={source === "twogg"} class:selected={source === "twogg"} onclick={() => setSource("twogg")}>TwoGG</button>
    <button role="radio" aria-checked={source === "modrinth"} class:selected={source === "modrinth"} onclick={() => setSource("modrinth")}>Modrinth</button>
    <button
      role="radio"
      aria-checked={source === "curseforge"}
      class:selected={source === "curseforge"}
      disabled={!curseforge}
      title={curseforge ? undefined : t("discover.cfMissing")}
      onclick={() => setSource("curseforge")}>CurseForge</button
    >
  </div>
  </div>

  {#if source === "twogg"}
    <TwoggPacks onInstalled={onPackInstalled} />
  {:else}
  <div class="layout">
    <aside class="filters">
      <label class="search">
        <Icon name="search" size={16} />
        <input bind:value={query} placeholder={t("discover.search")} aria-label={t("discover.search")} />
      </label>
      <label>
        {t("discover.sortBy")}
        <select bind:value={sort}>
          {#each SORTS as s (s)}<option value={s}>{t(`discover.sort.${s}`)}</option>{/each}
        </select>
      </label>
      {#if !profileId}
        {#if type === "mod" || type === "modpack"}
          <label>
            {t("discover.loader")}
            <select bind:value={loader}>
              <option value="">{t("discover.any")}</option>
              <option value="fabric">Fabric</option>
              <option value="quilt">Quilt</option>
            </select>
          </label>
        {/if}
        <label>
          {t("discover.version")}
          <select bind:value={gameVersion}>
            <option value="">{t("discover.any")}</option>
            {#each gameVersions as v (v.id)}<option value={v.id}>{v.id}</option>{/each}
          </select>
        </label>
      {/if}
      {#each source === "modrinth" ? typeCategories : [] as [header, names] (header)}
        <fieldset>
          <legend>{categoryLabel(header)}</legend>
          {#each names as name (name)}
            <label class="check">
              <input type="checkbox" checked={selectedCategories.includes(name)} onchange={() => toggleCategory(name)} />
              {name.replaceAll("-", " ")}
            </label>
          {/each}
        </fieldset>
      {/each}
      {#if selectedCategories.length > 0}
        <button class="ghost" onclick={() => (selectedCategories = [])}>{t("discover.clearFilters")}</button>
      {/if}
    </aside>

    <div class="results">
      {#if blockedByVanilla && profile}
        <p class="hint">{t("content.vanillaHint")}</p>
      {/if}
      {#if !profileId && type !== "modpack"}<p class="muted small">{t("discover.needProfile")}</p>{/if}
      {#if error}<p class="error">{error}</p>{/if}
      {#if message}<p class="notice">{message}</p>{/if}
      <p class="muted small">{t("discover.results", { count: compact.format(total) })}</p>

      <ul class="hits">
        {#each hits as hit (hit.projectId)}
          {@const isInstalled = installed.has(installedKey(hit))}
          {@const isInstalling = installing.has(hit.projectId)}
          <li>
            {#if hit.iconUrl}
              <img class="icon" src={hit.iconUrl} alt="" width="64" height="64" loading="lazy" />
            {:else}
              <span class="icon placeholder"><Icon name="box" size={26} /></span>
            {/if}
            <div class="body">
              <div class="head">
                <span class="title">{hit.title}</span>
                <span class="muted small">{t("discover.by", { author: hit.author })}</span>
              </div>
              <p class="desc">{hit.description}</p>
              <div class="meta muted small">
                <span><Icon name="download" size={13} /> {compact.format(hit.downloads)}</span>
                {#if hit.dateModified}<span>{t("discover.updated", { ago: formatAgo(Date.parse(hit.dateModified) / 1000) })}</span>{/if}
                {#each hit.displayCategories.slice(0, 4) as c (c)}<span class="badge">{c}</span>{/each}
              </div>
            </div>
            <div class="actions">
              {#if type === "modpack"}
                <div class="split">
                  <button class="primary" disabled={isInstalling} onclick={() => installPack(hit)}>
                    {isInstalling ? t("pack.importing") : t("pack.installNew")}
                  </button>
                  <button class="primary chevron" disabled={isInstalling} aria-label={t("discover.chooseVersion")} onclick={() => chooseVersion(hit)}>
                    <Icon name="chevron" size={16} />
                  </button>
                </div>
              {:else if isInstalled}
                <span class="installed"><Icon name="check" size={16} />{t("discover.installed")}</span>
              {:else}
                <div class="split">
                  <button
                    class="primary"
                    disabled={!profileId || !contentKind || blockedByVanilla || isInstalling}
                    onclick={() => install(hit)}
                  >{isInstalling ? t("discover.installing") : t("discover.install")}</button>
                  <button
                    class="primary chevron"
                    disabled={!profileId || !contentKind || blockedByVanilla || isInstalling}
                    aria-label={t("discover.chooseVersion")}
                    onclick={() => chooseVersion(hit)}><Icon name="chevron" size={16} /></button
                  >
                </div>
              {/if}
              <button class="ghost small link" onclick={() => openUrl(webUrl(hit))}>
                <Icon name="external" size={14} />{source === "curseforge" ? t("discover.openCf") : t("discover.openWeb")}
              </button>
              {#if versionMenu?.hit === hit}
                <div class="versions" role="menu">
                  {#if versionMenu.versions === null}
                    <span class="muted small">{t("common.loading")}</span>
                  {:else if versionMenu.versions.length === 0}
                    <span class="muted small">{t("discover.noVersions")}</span>
                  {:else}
                    {#each versionMenu.versions.slice(0, 15) as v (v.id)}
                      <button role="menuitem" onclick={() => (type === "modpack" ? installPack(hit, v.id) : install(hit, v.id))}>
                        <span>{v.versionNumber}</span>
                        {#if v.versionType !== "release"}<span class="badge">{v.versionType}</span>{/if}
                      </button>
                    {/each}
                  {/if}
                </div>
              {/if}
            </div>
          </li>
        {:else}
          {#if !loading}<p class="muted">{t("discover.noResults")}</p>{/if}
        {/each}
      </ul>
      {#if loading}<p class="muted">{t("common.loading")}</p>{/if}
      {#if !loading && hits.length < total}
        <button class="more" onclick={loadMore}>{t("discover.loadMore")}</button>
      {/if}
    </div>
  </div>
  {/if}
</section>

<style>
  .discover {
    display: flex;
    flex-direction: column;
    gap: 14px;
    min-height: 0;
    flex: 1;
  }

  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
    flex-wrap: wrap;
  }

  h1 {
    font-size: 24px;
  }

  .target {
    flex-direction: row;
    align-items: center;
    gap: 10px;
  }

  .target-select {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .target-select select {
    min-width: 240px;
  }

  .tabs-row {
    display: flex;
    justify-content: space-between;
    align-items: flex-end;
    gap: 12px;
    border-bottom: 1px solid var(--border);
  }

  .sources {
    display: flex;
    background: var(--surface-2);
    padding: 3px;
    border-radius: 10px;
    margin-bottom: 6px;
  }

  .sources button {
    border: none;
    background: transparent;
    color: var(--muted);
    border-radius: 8px;
    padding: 5px 12px;
    font-size: 13px;
  }

  .sources button.selected {
    background: var(--surface);
    color: var(--text);
    box-shadow: 0 1px 3px rgb(0 0 0 / 0.25);
  }

  .types {
    display: flex;
    gap: 4px;
  }

  .types button {
    border: none;
    border-radius: 8px 8px 0 0;
    background: transparent;
    color: var(--muted);
    padding: 8px 16px;
    border-bottom: 2px solid transparent;
  }

  .types button.active {
    color: var(--text);
    border-bottom-color: var(--accent);
  }

  .layout {
    display: grid;
    grid-template-columns: 220px 1fr;
    gap: 18px;
    min-height: 0;
    flex: 1;
  }

  .filters {
    display: flex;
    flex-direction: column;
    gap: 12px;
    overflow-y: auto;
    padding-right: 4px;
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
  }

  .search input:focus-visible {
    outline: none;
  }

  fieldset {
    border: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  legend {
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--muted);
    margin-bottom: 4px;
  }

  .check {
    flex-direction: row;
    align-items: center;
    gap: 8px;
    color: var(--text);
    font-size: 13px;
    text-transform: capitalize;
    padding: 2px 0;
  }

  .check input {
    width: auto;
    accent-color: var(--accent);
  }

  .results {
    overflow-y: auto;
    min-height: 0;
    padding-right: 4px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .hint {
    margin: 0;
    color: var(--warning);
  }

  .notice {
    margin: 0;
    color: var(--accent);
    user-select: text;
  }

  .small {
    font-size: 12px;
  }

  .hits {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .hits li {
    display: flex;
    gap: 14px;
    padding: 12px;
    border-radius: 10px;
    border: 1px solid var(--border);
    background: color-mix(in srgb, var(--surface) 92%, transparent);
  }

  .icon {
    width: 64px;
    height: 64px;
    border-radius: 12px;
    flex-shrink: 0;
    object-fit: cover;
  }

  .placeholder {
    display: grid;
    place-items: center;
    background: var(--surface-3);
    color: var(--muted);
  }

  .body {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .head {
    display: flex;
    gap: 8px;
    align-items: baseline;
    flex-wrap: wrap;
  }

  .title {
    font-weight: 700;
    font-size: 15px;
  }

  .desc {
    margin: 0;
    color: var(--muted);
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .meta {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: center;
  }

  .meta span {
    display: inline-flex;
    align-items: center;
    gap: 3px;
  }

  .actions {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 6px;
    flex-shrink: 0;
  }

  .split {
    display: flex;
  }

  .split button:first-child {
    border-radius: var(--radius) 0 0 var(--radius);
    min-width: 120px;
  }

  .split .chevron {
    border-radius: 0 var(--radius) var(--radius) 0;
    border-left: 1px solid color-mix(in srgb, var(--accent-text) 25%, transparent);
    padding: 7px 8px;
  }

  .installed {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--accent);
    font-weight: 600;
    padding: 7px 4px;
  }

  .link {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 3px 8px;
  }

  .versions {
    position: absolute;
    top: 40px;
    right: 0;
    z-index: 5;
    min-width: 240px;
    max-height: 300px;
    overflow-y: auto;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 10px;
    box-shadow: var(--shadow);
    padding: 6px;
    display: flex;
    flex-direction: column;
  }

  .versions button {
    display: flex;
    justify-content: space-between;
    gap: 8px;
    border: none;
    background: transparent;
    text-align: left;
    font-size: 13px;
  }

  .versions button:hover {
    background: var(--surface-2);
  }

  .more {
    align-self: center;
    margin: 6px 0 12px;
  }
</style>
