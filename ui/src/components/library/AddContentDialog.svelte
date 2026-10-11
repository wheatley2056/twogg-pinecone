<script lang="ts">
  // „Inhalt hinzufügen“ direkt in der Instanz: Modrinth durchsuchen (passend zu Version und
  // Loader), mit einem Klick hinzufügen, eigene Dateien wählen. Das große „Entdecken“ bleibt für
  // Filter, CurseForge und Details.
  import { onMount, untrack } from "svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { api, errorText } from "../../lib/api";
  import { locale, t } from "../../lib/i18n.svelte";
  import type { ContentKind, InstalledContent, Profile, SearchHit } from "../../lib/types";
  import Icon from "../Icon.svelte";
  import SearchField from "../ui/SearchField.svelte";
  import Segmented from "../ui/Segmented.svelte";

  interface Props {
    profile: Profile;
    kind: ContentKind;
    installed: InstalledContent[];
    onClose: () => void;
    onAdded: () => void;
    onDiscover: (kind: ContentKind) => void;
  }
  let { profile, kind: initialKind, installed, onClose, onAdded, onDiscover }: Props = $props();

  let dialog: HTMLDialogElement;
  let kind = $state<ContentKind>(untrack(() => initialKind));
  let query = $state("");
  let hits = $state<SearchHit[]>([]);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let status = $state<Record<string, "busy" | "done" | string>>({});
  let added = $state(0);

  const kinds = $derived<{ id: ContentKind; label: string }[]>([
    { id: "mod", label: t("content.mod") },
    { id: "resourcepack", label: t("content.resourcepack") },
    { id: "shader", label: t("content.shader") },
  ]);
  const have = $derived(
    new Set(installed.flatMap((i) => (i.source.type === "modrinth" ? [i.source.project_id] : []))),
  );
  const compact = new Intl.NumberFormat(locale(), { notation: "compact", maximumFractionDigits: 1 });
  const vanilla = $derived(!profile.loader && kind === "mod");

  onMount(() => dialog.showModal());

  // Suche mit kurzer Verzögerung; ohne Suchbegriff die beliebtesten passenden Projekte.
  let seq = 0;
  $effect(() => {
    const q = query.trim();
    const k = kind;
    const n = ++seq;
    loading = true;
    error = null;
    const timer = setTimeout(async () => {
      try {
        const r = await api.modrinthSearch({
          query: q,
          projectType: k,
          profileId: profile.id,
          loaders: [],
          gameVersion: null,
          categories: [],
          sort: q ? "relevance" : "downloads",
          offset: 0,
          limit: 20,
        });
        if (n === seq) hits = r.hits;
      } catch (e) {
        if (n === seq) error = errorText(e);
      } finally {
        if (n === seq) loading = false;
      }
    }, q ? 300 : 0);
    return () => clearTimeout(timer);
  });

  async function add(hit: SearchHit) {
    status[hit.projectId] = "busy";
    try {
      await api.contentInstall(profile.id, kind, hit.projectId);
      status[hit.projectId] = "done";
      added++;
      onAdded();
    } catch (e) {
      status[hit.projectId] = errorText(e);
    }
  }

  async function chooseFiles() {
    const picked = await open({
      multiple: true,
      filters: [{ name: t("content.fileFilter"), extensions: ["jar", "zip"] }],
    });
    const paths = Array.isArray(picked) ? picked : picked ? [picked] : [];
    error = null;
    for (const path of paths) {
      try {
        await api.contentAddDropped(profile.id, path);
        added++;
      } catch (e) {
        error = errorText(e);
      }
    }
    if (paths.length) onAdded();
  }

  function close() {
    dialog.close();
    onClose();
  }
</script>

<dialog bind:this={dialog} class="add" oncancel={(e) => (e.preventDefault(), close())} aria-labelledby="add-title">
  <header>
    <div class="title-row">
      <h2 id="add-title">{t("addContent.title", { name: profile.name })}</h2>
      <button class="icon ghost" onclick={close} aria-label={t("common.close")}><Icon name="close" size={18} /></button>
    </div>
    <div class="controls">
      <Segmented items={kinds} value={kind} label={t("content.filterLabel")} onchange={(k) => (kind = k)} />
      <SearchField bind:value={query} placeholder={t("addContent.search")} width="100%" />
    </div>
  </header>

  <div class="results" aria-busy={loading}>
    {#if vanilla}
      <p class="hint">{t("content.vanillaHint")}</p>
    {:else if error && hits.length === 0}
      <p class="error">{error}</p>
    {:else if !loading && hits.length === 0}
      <p class="hint">{t("addContent.none")}</p>
    {/if}
    {#if !vanilla}
      {#each hits as hit (hit.projectId)}
        {@const s = status[hit.projectId]}
        {@const done = s === "done" || have.has(hit.projectId)}
        <div class="hit">
          {#if hit.iconUrl}
            <img class="ico" src={hit.iconUrl} alt="" width="40" height="40" loading="lazy" />
          {:else}
            <span class="ico letter">{hit.title.slice(0, 2).toUpperCase()}</span>
          {/if}
          <span class="text">
            <span class="name">{hit.title}<span class="by"> · {hit.author}</span></span>
            <span class="desc">{hit.description}</span>
            {#if s && s !== "busy" && s !== "done"}<span class="err">{s}</span>{/if}
          </span>
          <span class="dl mono" title={t("addContent.downloads")}><Icon name="download" size={13} />{compact.format(hit.downloads)}</span>
          {#if done}
            <span class="done"><Icon name="check" size={15} stroke={2.4} />{t("addContent.added")}</span>
          {:else}
            <button class="primary cut small" onclick={() => add(hit)} disabled={s === "busy"}>
              {#if s === "busy"}{t("addContent.adding")}{:else}<Icon name="plus" size={14} stroke={2.6} />{t("addContent.add")}{/if}
            </button>
          {/if}
        </div>
      {/each}
    {/if}
  </div>

  <footer>
    <button class="dashed" onclick={chooseFiles}><Icon name="file" size={16} />{t("addContent.files")}</button>
    <span class="drop-hint">{t("addContent.dropHint")}</span>
    <span class="grow"></span>
    {#if error && hits.length > 0}<span class="err">{error}</span>{/if}
    <button class="link" onclick={() => (close(), onDiscover(kind))}>{t("addContent.discover")}<Icon name="external" size={14} stroke={2} /></button>
    <button class="primary" onclick={close}>{added > 0 ? t("addContent.doneCount", { count: added }) : t("common.close")}</button>
  </footer>
</dialog>

<style>
  .add {
    width: min(760px, calc(100vw - 64px));
    height: min(640px, calc(100vh - 80px));
    padding: 0;
    display: flex;
    flex-direction: column;
  }

  .add:not([open]) {
    display: none;
  }

  header {
    padding: 20px 22px 14px;
    display: flex;
    flex-direction: column;
    gap: 14px;
    border-bottom: 1px solid var(--line);
  }

  .title-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  h2 {
    font-size: 22px;
  }

  .controls {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .controls :global(.search) {
    flex: 1;
    background: var(--surface-2);
  }

  .results {
    flex: 1;
    overflow: auto;
    padding: 10px 22px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .hit {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 10px 12px;
    background: var(--surface);
    clip-path: var(--cut-1);
  }

  .ico {
    width: 40px;
    height: 40px;
    flex: none;
    object-fit: cover;
    image-rendering: auto;
  }

  .letter {
    display: grid;
    place-items: center;
    background: var(--surface-2);
    font-family: var(--font-mono);
    font-size: 13px;
    color: var(--text-muted);
  }

  .text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .name {
    font-size: 14px;
    font-weight: 700;
  }

  .by {
    font-weight: 400;
    color: var(--text-dim);
    font-size: 12.5px;
  }

  .desc {
    font-size: 12.5px;
    color: var(--text-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .dl {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 11.5px;
    color: var(--text-dim);
    flex: none;
  }

  .small {
    height: 32px;
    padding: 0 12px;
    gap: 6px;
    flex: none;
    min-width: 112px;
  }

  .done {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    min-width: 112px;
    font-size: 13px;
    font-weight: 600;
    color: var(--nadel);
  }

  .hint {
    margin: 18px 0;
    color: var(--text-muted);
    font-size: 14px;
  }

  .err {
    font-size: 12px;
    color: var(--danger, #e58a8a);
  }

  footer {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 14px 22px;
    border-top: 1px solid var(--line);
  }

  .drop-hint {
    font-size: 12.5px;
    color: var(--text-dim);
  }

  .grow {
    flex: 1;
  }

  footer .link {
    gap: 6px;
  }
</style>
