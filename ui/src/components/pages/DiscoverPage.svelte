<script lang="ts">
  // Entdecken nach docs/design/referenz/03-entdecken.html: Inhaltsart, Ziel-Instanz, Suche mit
  // Filtern, TwoGG-Auswahl und Ergebnisraster. Quelle „Alle“ fragt Modrinth, CurseForge und
  // TwoGG gleichzeitig ab.
  import { onMount, untrack } from "svelte";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { api, errorText } from "../../lib/api";
  import { locale, t } from "../../lib/i18n.svelte";
  import { versionLabel } from "../../lib/profiles";
  import type {
    Category,
    ContentKind,
    ImportResult,
    ModrinthVersion,
    Profile,
    ProjectType,
    Recommendation,
    SearchArgs,
    SearchHit,
    SortIndex,
    TwoggPack,
    VersionInfo,
    World,
  } from "../../lib/types";
  import Icon from "../Icon.svelte";
  import ProfileIcon from "../ProfileIcon.svelte";
  import Dropdown from "../ui/Dropdown.svelte";
  import SearchField from "../ui/SearchField.svelte";
  import Segmented from "../ui/Segmented.svelte";

  interface Props {
    profiles: Profile[];
    initialProfileId: string | null;
    initialType: ProjectType;
    curseforge: boolean;
    onPackInstalled: (result: ImportResult) => void;
  }
  let { profiles, initialProfileId, initialType, curseforge, onPackInstalled }: Props = $props();

  type Source = "all" | "modrinth" | "curseforge" | "twogg";
  interface Card {
    key: string;
    source: "modrinth" | "curseforge" | "twogg";
    projectId: string;
    title: string;
    author: string;
    description: string;
    iconUrl: string | null;
    tags: string[];
    url: string;
    twogg?: TwoggPack;
    motif?: string | null;
  }

  const CF_PATHS: Record<ProjectType, string> = { mod: "mc-mods", resourcepack: "texture-packs", shader: "shaders", modpack: "modpacks", datapack: "data-packs" };
  const PAGE = 18;

  const start = untrack(() => ({ profileId: initialProfileId, type: initialType }));
  let type = $state<ProjectType>(start.type);
  let targetId = $state<string>(start.profileId ?? "");
  let query = $state("");
  let source = $state<Source>("all");
  let sort = $state<SortIndex>("relevance");
  let category = $state("");
  let loader = $state("");
  let gameVersion = $state("");

  let cards = $state<Card[]>([]);
  let more = $state(false);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let message = $state<string | null>(null);
  let categories = $state<Category[]>([]);
  let gameVersions = $state<VersionInfo[]>([]);
  let twoggPacks = $state<TwoggPack[]>([]);
  let installed = $state<Set<string>>(new Set());
  let installing = $state<Set<string>>(new Set());
  let versionMenu = $state<{ card: Card; versions: { id: string; label: string }[] | null } | null>(null);
  // Datenpakete: erst die Welt wählen.
  let worldMenu = $state<{ card: Card; version: string | null; worlds: World[] | null } | null>(null);
  let offsets = { modrinth: 0, curseforge: 0 };

  const target = $derived(profiles.find((p) => p.id === targetId) ?? null);
  const contentKind = $derived(type === "modpack" || type === "datapack" ? null : (type as ContentKind));
  const vanillaBlocked = $derived(type === "mod" && !!target && !target.loader);
  // TwoGG-Auswahl vom Server; für Modpacks ersatzweise die hervorgehobenen TwoGG-Packs.
  let recommendations = $state<Recommendation[]>([]);
  $effect(() => {
    const ty = type;
    api
      .recommendations(ty)
      .then((list) => ty === type && (recommendations = list))
      .catch(() => (recommendations = []));
  });
  // Datenpakete brauchen eine Weltauswahl, die gibt es nur in der Ergebnisliste.
  const picks = $derived<Card[]>(
    type === "datapack"
      ? []
      : recommendations.length > 0
      ? recommendations.slice(0, 3).map(fromRecommendation)
      : type === "modpack"
        ? twoggPacks
            .filter((p) => p.featured)
            .slice(0, 3)
            .map(fromTwogg)
        : [],
  );

  onMount(async () => {
    [categories, gameVersions, twoggPacks] = await Promise.all([
      api.modrinthCategories().catch(() => []),
      api.versions(false).catch(() => []),
      api.twoggPacks().catch(() => []),
    ]);
  });

  // Installierte Projekte der Ziel-Instanz für „Installiert“.
  $effect(() => {
    const id = targetId;
    if (!id) {
      installed = new Set();
      return;
    }
    api
      .contentList(id)
      .then((list) => {
        installed = new Set(
          list.flatMap((c) => (c.source.type === "modrinth" ? [`modrinth:${c.source.project_id}`] : c.source.type === "curseforge" ? [`curseforge:${c.source.project_id}`] : [])),
        );
      })
      .catch(() => {});
  });

  function args(offset: number): SearchArgs {
    const useTarget = type !== "modpack" && !!target && !vanillaBlocked;
    return {
      query,
      projectType: type,
      profileId: useTarget ? targetId : null,
      loaders: !useTarget && loader ? [loader] : [],
      gameVersion: !useTarget && gameVersion ? gameVersion : null,
      categories: category ? [category] : [],
      sort,
      offset,
      limit: PAGE,
    };
  }

  function fromHit(hit: SearchHit, src: "modrinth" | "curseforge"): Card {
    return {
      key: `${src}:${hit.projectId}`,
      source: src,
      projectId: hit.projectId,
      title: hit.title,
      author: hit.author,
      description: hit.description,
      iconUrl: hit.iconUrl,
      tags: hit.displayCategories.slice(0, 2).map((c) => c.replaceAll("-", " ")),
      url: src === "curseforge" ? `https://www.curseforge.com/minecraft/${CF_PATHS[type]}/${hit.slug}` : `https://modrinth.com/${hit.projectType}/${hit.slug}`,
    };
  }

  function fromTwogg(p: TwoggPack): Card {
    return {
      key: `twogg:${p.id}`,
      source: "twogg",
      projectId: p.id,
      title: p.title,
      author: "TwoGG",
      description: p.summary,
      iconUrl: p.iconUrl,
      tags: p.latest ? [`${p.latest.loader ? p.latest.loader[0].toUpperCase() + p.latest.loader.slice(1) : "Vanilla"} ${p.latest.minecraft}`] : [],
      url: "",
      twogg: p,
    };
  }

  function fromRecommendation(r: Recommendation): Card {
    const pack = r.source === "twogg" ? twoggPacks.find((p) => p.id === r.projectId) : undefined;
    return {
      key: `${r.source}:${r.projectId}`,
      source: r.source,
      projectId: r.projectId,
      title: r.title,
      author: "TwoGG",
      description: r.subtitle,
      iconUrl: r.iconUrl ?? pack?.iconUrl ?? null,
      tags: [],
      url: r.source === "modrinth" ? `https://modrinth.com/project/${r.projectId}` : "",
      twogg: pack,
      motif: r.motif,
    };
  }

  /** Abwechselnd mischen, damit keine Quelle die erste Seite füllt. */
  function interleave(lists: Card[][]): Card[] {
    const out: Card[] = [];
    for (let i = 0; lists.some((l) => i < l.length); i++) for (const l of lists) if (i < l.length) out.push(l[i]);
    return out;
  }

  let request = 0;
  let timer: ReturnType<typeof setTimeout> | undefined;
  $effect(() => {
    void [type, targetId, query, source, sort, category, loader, gameVersion, twoggPacks];
    clearTimeout(timer);
    timer = setTimeout(() => untrack(() => search(false)), query ? 300 : 0);
  });

  async function search(append: boolean) {
    const id = ++request;
    loading = true;
    error = null;
    if (!append) offsets = { modrinth: 0, curseforge: 0 };
    try {
      const wantModrinth = source === "all" || source === "modrinth";
      const wantCf = (source === "all" || source === "curseforge") && curseforge && type !== "datapack";
      const [m, c] = await Promise.all([
        wantModrinth ? api.modrinthSearch(args(offsets.modrinth)) : null,
        wantCf ? api.curseforgeSearch(args(offsets.curseforge)).catch((e) => (source === "curseforge" ? Promise.reject(e) : null)) : null,
      ]);
      if (id !== request) return;
      const lists = [m?.hits.map((h) => fromHit(h, "modrinth")) ?? [], c?.hits.map((h) => fromHit(h, "curseforge")) ?? []];
      offsets.modrinth += m?.hits.length ?? 0;
      offsets.curseforge += c?.hits.length ?? 0;
      more = (m ? offsets.modrinth < m.totalHits : false) || (c ? offsets.curseforge < c.totalHits : false);
      const q = query.trim().toLowerCase();
      // TwoGG-Packs gibt es nur als Modpacks.
      const own =
        !append && type === "modpack" && (source === "all" || source === "twogg")
          ? twoggPacks.filter((p) => !q || p.title.toLowerCase().includes(q)).map(fromTwogg)
          : [];
      const merged = [...own, ...interleave(lists)];
      cards = append ? [...cards, ...merged] : merged;
    } catch (e) {
      if (id === request) error = errorText(e);
    } finally {
      if (id === request) loading = false;
    }
  }

  async function pickWorld(card: Card, version: string | null = null) {
    versionMenu = null;
    worldMenu = { card, version, worlds: null };
    const worlds = await api.worlds(targetId).catch(() => []);
    if (worldMenu?.card === card) worldMenu = { card, version, worlds };
  }

  async function installDatapack(card: Card, world: World, version: string | null) {
    worldMenu = null;
    error = message = null;
    installing = new Set([...installing, card.key]);
    try {
      const file = await api.datapackInstall(targetId, world.folder, card.projectId, version);
      message = t("discover.datapackInstalled", { file, world: world.name });
    } catch (e) {
      error = errorText(e);
    } finally {
      installing = new Set([...installing].filter((k) => k !== card.key));
    }
  }

  async function install(card: Card, version: string | null = null) {
    if (type === "datapack") return pickWorld(card, version);
    versionMenu = null;
    error = message = null;
    installing = new Set([...installing, card.key]);
    try {
      if (card.source === "twogg") {
        onPackInstalled(await api.packInstallTwogg(card.projectId, version));
      } else if (type === "modpack") {
        onPackInstalled(
          card.source === "curseforge"
            ? await api.packInstallCurseforge(Number(card.projectId), version ? Number(version) : null)
            : await api.packInstallModrinth(card.projectId, version),
        );
      } else if (targetId && contentKind) {
        const r =
          card.source === "curseforge"
            ? await api.contentInstallCurseforge(targetId, contentKind, Number(card.projectId), version ? Number(version) : null)
            : await api.contentInstall(targetId, contentKind, card.projectId, version);
        installed = new Set([...installed, card.key]);
        const parts = [];
        if (r.installed.length) parts.push(t("content.installedMsg", { names: r.installed.map(([n, v]) => `${n} ${v}`).join(", ") }));
        if (r.alreadyPresent.length && !r.installed.length) parts.push(t("content.alreadyMsg", { names: r.alreadyPresent.join(", ") }));
        parts.push(...r.warnings);
        message = parts.join(" · ");
      }
    } catch (e) {
      error = errorText(e);
    } finally {
      installing = new Set([...installing].filter((k) => k !== card.key));
    }
  }

  async function chooseVersion(card: Card) {
    versionMenu = { card, versions: null };
    try {
      let versions: { id: string; label: string }[];
      if (card.source === "twogg") {
        const pack = await api.twoggPack(card.projectId);
        versions = (pack.versions ?? []).map((v) => ({ id: v.id, label: `${v.version} · ${v.minecraft}` }));
      } else {
        const tgt = type === "modpack" ? null : targetId || null;
        const list: ModrinthVersion[] =
          card.source === "curseforge"
            ? await api.curseforgeFiles(Number(card.projectId), tgt, type)
            : await api.modrinthVersions(card.projectId, tgt, type);
        versions = list.map((v) => ({ id: v.id, label: `${v.versionNumber} · ${v.gameVersions.slice(0, 2).join(", ")}` }));
      }
      if (versionMenu?.card === card) versionMenu = { card, versions };
    } catch (e) {
      error = errorText(e);
      versionMenu = null;
    }
  }

  function open(url: string) {
    if (url) openUrl(url).catch(() => window.open(url, "_blank"));
  }

  const initials = (s: string) =>
    s
      .split(/\s+/)
      .filter(Boolean)
      .slice(0, 2)
      .map((w) => w[0]?.toUpperCase())
      .join("");

  const typeItems = $derived<{ id: ProjectType; label: string }[]>([
    { id: "modpack", label: t("discover.modpacks") },
    { id: "mod", label: t("discover.mods") },
    { id: "resourcepack", label: t("discover.resourcepacks") },
    { id: "shader", label: t("discover.shaders") },
    { id: "datapack", label: t("discover.datapacks") },
  ]);
  const categoryOptions = $derived([
    { value: "", label: t("discover.all") },
    ...categories.filter((c) => c.projectType === type && c.header === "categories").map((c) => ({ value: c.name, label: c.name.replaceAll("-", " ") })),
  ]);
  const sortOptions = $derived(
    (["relevance", "downloads", "follows", "newest", "updated"] as SortIndex[]).map((s) => ({ value: s, label: t(`discover.sort.${s}`) })),
  );
  const sourceOptions = $derived<{ value: Source; label: string }[]>([
    { value: "all", label: t("discover.all") },
    { value: "modrinth", label: "Modrinth" },
    ...(curseforge ? [{ value: "curseforge" as const, label: "CurseForge" }] : []),
    ...(type === "modpack" ? [{ value: "twogg" as const, label: "TwoGG" }] : []),
  ]);
  const targetOptions = $derived([
    ...(type === "modpack" ? [{ value: "", label: t("discover.newInstance") }] : [{ value: "", label: t("discover.noTarget") }]),
    ...profiles.map((p) => ({ value: p.id, label: p.name })),
  ]);
  const usesTarget = $derived(type !== "modpack" && !!target);
  const compact = new Intl.NumberFormat(locale(), { notation: "compact", maximumFractionDigits: 1 });
</script>

<svelte:window
  onkeydown={(e) => e.key === "Escape" && (versionMenu = null)}
  onclick={(e) => (versionMenu || worldMenu) && !(e.target as HTMLElement).closest(".get") && ((versionMenu = null), (worldMenu = null))}
/>

<div class="page">
  <div class="top">
    <h1>{t("tab.discover")}</h1>
    <Segmented items={typeItems} value={type} label={t("discover.type")} onchange={(v) => ((type = v), (category = ""), (message = null), source === "twogg" && v !== "modpack" && (source = "all"))} />
    <span class="spacer"></span>
    {#if type !== "modpack"}
      <span class="install-label">{t("discover.installIn")}</span>
      <Dropdown label="" value={targetId} options={targetOptions} align="right" onchange={(v) => (targetId = v)}>
        {#snippet leading()}{#if target}<ProfileIcon profile={target} size={22} />{/if}{/snippet}
      </Dropdown>
    {:else}
      <span class="install-label">{t("discover.packHint")}</span>
    {/if}
  </div>

  <div class="filters">
    <div class="grow"><SearchField bind:value={query} placeholder={t(`discover.searchIn.${type}`)} height={40} /></div>
    {#if type === "mod" || type === "modpack"}
      <Dropdown
        label={t("discover.loader")}
        value={usesTarget ? (target!.loader?.kind ?? "vanilla") : loader}
        options={[{ value: "", label: t("discover.any") }, { value: "fabric", label: "Fabric" }, { value: "quilt", label: "Quilt" }, { value: "forge", label: "Forge" }, { value: "neoforge", label: "NeoForge" }, { value: "vanilla", label: "Vanilla" }]}
        disabled={usesTarget}
        title={usesTarget ? t("discover.fromTarget") : undefined}
        onchange={(v) => (loader = v === "vanilla" ? "" : v)}
      />
    {/if}
    <Dropdown
      label={t("discover.version")}
      value={usesTarget ? target!.minecraft : gameVersion}
      options={[{ value: "", label: t("discover.any") }, ...gameVersions.map((v) => ({ value: v.id, label: v.id })), ...(usesTarget ? [{ value: target!.minecraft, label: target!.minecraft }] : [])]}
      disabled={usesTarget}
      title={usesTarget ? t("discover.fromTarget") : undefined}
      onchange={(v) => (gameVersion = v)}
    />
    <Dropdown label={t("discover.category")} value={category} options={categoryOptions} onchange={(v) => (category = v)} />
    <Dropdown label={t("discover.source")} value={source} options={sourceOptions} onchange={(v) => (source = v)} />
    <Dropdown label={t("discover.sortBy")} value={sort} options={sortOptions} align="right" onchange={(v) => (sort = v)} />
  </div>

  {#if picks.length > 0}
    <div class="picks">
      <div class="picks-label scales">
        <span class="picks-title">{t("discover.picks")}</span>
        <span class="picks-sub">{t("discover.picksHint")}</span>
      </div>
      {#each picks as card (card.key)}
        <button
          class="pick"
          onclick={() => install(card)}
          disabled={installing.has(card.key) || (type !== "modpack" && !targetId)}
          title={type !== "modpack" && !targetId ? t("discover.chooseTarget") : card.title}
        >
          {#if card.iconUrl}
            <img src={card.iconUrl} alt="" width="38" height="38" />
          {:else}
            <ProfileIcon profile={{ name: card.title, icon: `motif:${card.motif ?? "shield"}` }} size={38} />
          {/if}
          <span class="pick-text">
            <span class="pick-name">{card.title}</span>
            <span class="pick-sub">{installing.has(card.key) ? t("discover.loading") : card.description}</span>
          </span>
        </button>
      {/each}
    </div>
  {/if}

  {#if vanillaBlocked}<p class="hint">{t("discover.vanillaHint")}</p>{/if}
  {#if error}<p class="error">{error}</p>{/if}
  {#if message}<p class="message">{message}</p>{/if}

  <div class="results">
    <div class="grid">
      {#each cards as card (card.key)}
        {@const busy = installing.has(card.key)}
        {@const done = type !== "datapack" && installed.has(card.key)}
        <article class="card">
          <div class="head">
            {#if card.iconUrl}
              <img class="icon" src={card.iconUrl} alt="" width="50" height="50" loading="lazy" />
            {:else}
              <span class="icon letters">{initials(card.title)}</span>
            {/if}
            <span class="title-block">
              <button class="title" onclick={() => (card.url ? open(card.url) : null)} title={card.url || undefined}>{card.title}</button>
              <span class="by">{card.author} · <span class="mono" class:twogg={card.source === "twogg"}>{card.source === "twogg" ? "TwoGG" : card.source === "curseforge" ? "CurseForge" : "Modrinth"}</span></span>
            </span>
          </div>
          <p class="desc">{card.description}</p>
          <div class="foot">
            {#each card.tags as tag (tag)}<span class="tag">{tag}</span>{/each}
            <span class="spacer"></span>
            {#if done}
              <span class="installed"><Icon name="check" size={15} stroke={2.4} />{t("discover.installed")}</span>
            {:else if busy}
              <span class="progress" role="status"><span class="bar"><span class="fill"></span></span><span class="mono">{t("discover.loading")}</span></span>
            {:else}
              <span class="get">
                <button class="primary get-main" disabled={(type !== "modpack" && card.source !== "twogg" && !targetId) || vanillaBlocked} title={type !== "modpack" && !targetId ? t("discover.chooseTarget") : undefined} onclick={() => install(card)}>
                  <Icon name="download" size={15} stroke={2.2} />{t("discover.get")}
                </button>
                <button class="primary get-more" aria-label={t("discover.chooseVersion")} title={t("discover.chooseVersion")} disabled={(type !== "modpack" && card.source !== "twogg" && !targetId) || vanillaBlocked} onclick={() => chooseVersion(card)}>
                  <Icon name="chevron" size={14} stroke={2.2} />
                </button>
                {#if worldMenu?.card === card}
                  {@const menu = worldMenu}
                  <div class="versions" role="listbox" aria-label={t("discover.chooseWorld")}>
                    <span class="menu-title">{t("discover.chooseWorld")}</span>
                    {#if menu.worlds === null}
                      <span class="hint">{t("common.loading")}</span>
                    {:else if menu.worlds.length === 0}
                      <span class="hint">{t("discover.noWorlds")}</span>
                    {:else}
                      {#each menu.worlds as w (w.folder)}
                        <button role="option" aria-selected="false" onclick={() => installDatapack(card, w, menu.version)}>{w.name}</button>
                      {/each}
                    {/if}
                  </div>
                {/if}
                {#if versionMenu?.card === card}
                  <div class="versions" role="listbox">
                    {#if versionMenu.versions === null}
                      <span class="hint">{t("common.loading")}</span>
                    {:else if versionMenu.versions.length === 0}
                      <span class="hint">{t("discover.noVersions")}</span>
                    {:else}
                      {#each versionMenu.versions.slice(0, 30) as v (v.id)}
                        <button role="option" aria-selected="false" onclick={() => install(card, v.id)}>{v.label}</button>
                      {/each}
                    {/if}
                  </div>
                {/if}
              </span>
            {/if}
          </div>
        </article>
      {/each}
    </div>
    {#if !loading && cards.length === 0 && !error}
      <p class="hint center">{t("discover.noResults")}</p>
    {/if}
    {#if more}
      <button class="dashed more" disabled={loading} onclick={() => search(true)}>{loading ? t("common.loading") : t("discover.more")}</button>
    {:else if loading && cards.length === 0}
      <p class="hint center">{t("common.loading")}</p>
    {/if}
  </div>
</div>

<style>
  .page {
    height: 100%;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 22px 32px 0;
  }

  .top {
    display: flex;
    align-items: center;
    gap: 22px;
  }

  h1 {
    font-weight: 700;
    font-size: 28px;
  }

  .spacer {
    flex: 1;
  }

  .install-label {
    font-size: 13px;
    color: var(--text-muted);
  }

  .filters {
    display: flex;
    gap: 8px;
    align-items: center;
  }

  .grow {
    flex: 1;
    min-width: 200px;
  }

  .picks {
    display: grid;
    grid-template-columns: 230px repeat(3, minmax(0, 1fr));
    background: color-mix(in srgb, var(--accent) 7%, var(--stage));
    clip-path: polygon(0 0, 100% 0, 100% calc(100% - 10px), calc(100% - 10px) 100%, 0 100%);
  }

  .picks-label {
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 4px;
    padding: 16px 20px;
    border-right: 1px solid var(--line-strong);
    --scales-opacity: 0.12;
  }

  .picks-title,
  .picks-sub {
    position: relative;
  }

  .picks-title {
    font-family: var(--font-pixel);
    font-weight: 600;
    font-size: 16px;
    color: var(--accent-fg);
  }

  .picks-sub {
    font-size: 12px;
    color: var(--text-soft);
  }

  .pick {
    justify-content: flex-start;
    gap: 14px;
    padding: 16px 18px;
    min-height: 0;
    background: transparent;
    text-align: left;
    border-right: 1px solid var(--line-strong);
  }

  .pick:last-child {
    border-right: 0;
  }

  .pick:hover:not(:disabled) {
    background: color-mix(in srgb, var(--accent) 6%, transparent);
  }

  .pick-text {
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .pick-name {
    font-size: 14px;
    font-weight: 700;
  }

  .pick-sub {
    font-size: 12px;
    font-weight: 400;
    color: var(--text-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .results {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding-bottom: 20px;
    margin-right: -10px;
    padding-right: 10px;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 14px;
  }

  /* Der Schnitt liegt auf einer eigenen Fläche, damit Auswahllisten nicht abgeschnitten werden. */
  .card {
    position: relative;
    isolation: isolate;
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 14px;
  }

  .card::before {
    content: "";
    position: absolute;
    inset: 0;
    z-index: -1;
    background: var(--surface);
    clip-path: polygon(10px 0, 100% 0, 100% 100%, 0 100%, 0 10px);
  }

  .card:has(.versions) {
    z-index: 5;
  }

  .head {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .icon {
    width: 50px;
    height: 50px;
    flex: none;
    object-fit: cover;
  }

  .letters {
    display: grid;
    place-items: center;
    background: var(--surface-2);
    font-family: var(--font-pixel);
    font-weight: 700;
    font-size: 17px;
    color: var(--text-soft);
  }

  .title-block {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .title {
    justify-content: flex-start;
    min-height: 0;
    padding: 0;
    background: transparent;
    font-size: 15px;
    font-weight: 700;
    text-align: left;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .title:hover:not(:disabled) {
    background: transparent;
    color: var(--accent-hover);
  }

  .by {
    font-size: 12px;
    color: var(--text-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .by .twogg {
    color: var(--accent-fg);
  }

  .desc {
    margin: 0;
    font-size: 13px;
    color: var(--text-soft);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .foot {
    display: flex;
    align-items: center;
    gap: 6px;
    min-height: 32px;
  }

  .get {
    position: relative;
    display: flex;
  }

  .get-main {
    min-height: 32px;
    padding: 0 12px;
    font-size: 13px;
    gap: 6px;
  }

  .get-more {
    width: 30px;
    min-height: 32px;
    padding: 0;
    background: var(--accent-split);
    box-shadow:
      inset 1px 0 0 var(--accent-edge),
      var(--edge-accent);
  }

  .versions {
    position: absolute;
    right: 0;
    top: calc(100% + 4px);
    z-index: 25;
    width: 260px;
    max-height: 260px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    padding: 4px;
    background: var(--bar);
    border: 1px solid var(--line-strong);
  }

  .versions button {
    justify-content: flex-start;
    min-height: 30px;
    background: transparent;
    font-size: 12px;
    font-weight: 500;
    font-family: var(--font-mono);
  }

  .menu-title {
    padding: 4px 8px 6px;
    font-size: 11px;
    font-family: var(--font-mono);
    letter-spacing: 1px;
    text-transform: uppercase;
    color: var(--text-dim);
  }

  .versions button:hover:not(:disabled) {
    background: var(--surface);
  }

  .installed {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 8px;
    font-size: 13px;
    font-weight: 700;
    color: var(--nadel);
  }

  .progress {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 8px;
    font-size: 12px;
    color: var(--accent-fg);
  }

  .bar {
    position: relative;
    width: 48px;
    height: 4px;
    overflow: hidden;
    background: var(--line-strong);
  }

  .fill {
    position: absolute;
    inset: 0 auto 0 0;
    width: 40%;
    background: var(--accent);
    animation: slide 1.1s ease-in-out infinite;
  }

  @keyframes slide {
    from {
      left: -40%;
    }
    to {
      left: 100%;
    }
  }

  .hint {
    margin: 0;
    font-size: 13px;
    color: var(--text-dim);
  }

  .center {
    text-align: center;
    padding: 30px 0;
  }

  .message {
    margin: 0;
    font-size: 13px;
    color: var(--nadel);
  }

  .more {
    width: 100%;
    margin-top: 14px;
    height: 40px;
  }

  @media (max-width: 1180px) {
    .grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }

    .filters {
      flex-wrap: wrap;
    }
  }
</style>
