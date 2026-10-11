<script lang="ts">
  import { onMount, untrack } from "svelte";
  import { api, errorText } from "../lib/api";
  import { t } from "../lib/i18n.svelte";
  import type { ImportResult, LoaderKind, LoaderSpec, MigrationPlan, MissingChoice, Profile, VersionInfo } from "../lib/types";

  interface Props {
    profile: Profile;
    onClose: () => void;
    /** Neues Profil angelegt (Umzug oder Pack-Version). */
    onCreated: (result: ImportResult) => void;
  }
  let { profile, onClose, onCreated }: Props = $props();

  let dialog: HTMLDialogElement;
  const initial = untrack(() => profile);
  let versions = $state<VersionInfo[]>([]);
  let snapshots = $state(false);
  let minecraft = $state(initial.minecraft);
  let loader = $state<LoaderKind | "none">(initial.loader?.kind ?? "none");

  let plan = $state<MigrationPlan | null>(null);
  let name = $state("");
  let includeWorlds = $state(true);
  let missing = $state<Record<string, MissingChoice>>({});
  let busy = $state(false);
  let error = $state<string | null>(null);
  let warnings = $state<string[] | null>(null);
  let created = $state<ImportResult | null>(null);

  onMount(async () => {
    dialog.showModal();
    await loadVersions();
  });

  async function loadVersions() {
    try {
      versions = await api.versions(snapshots);
    } catch (e) {
      error = errorText(e);
    }
  }

  const sameAsNow = $derived(minecraft === initial.minecraft && (loader === "none" ? null : loader) === (initial.loader?.kind ?? null));

  async function check() {
    busy = true;
    error = null;
    plan = null;
    try {
      plan = await api.migratePlan(initial.id, { minecraft, loader: loader === "none" ? null : loader });
      name = `${initial.name} (${plan.minecraft})`;
      missing = Object.fromEntries(plan.items.filter((i) => i.status === "missing").map((i) => [i.key, "disable" as MissingChoice]));
    } catch (e) {
      error = errorText(e);
    } finally {
      busy = false;
    }
  }

  async function apply() {
    if (!plan) return;
    busy = true;
    error = null;
    try {
      const r = await api.migrateApply(initial.id, {
        target: { minecraft: plan.minecraft, loader: plan.loader?.kind ?? null },
        name: name.trim() || null,
        includeWorlds,
        missing,
      });
      created = { profile: r.profile, missing: [] };
      if (r.warnings.length > 0) warnings = r.warnings;
      else finish();
    } catch (e) {
      error = errorText(e);
    } finally {
      busy = false;
    }
  }

  async function installPack() {
    const offer = plan?.packOffer;
    if (!offer) return;
    busy = true;
    error = null;
    try {
      const r =
        offer.platform === "curseforge"
          ? await api.packInstallCurseforge(Number(offer.projectId), Number(offer.versionId))
          : await api.packInstallModrinth(offer.projectId, offer.versionId);
      onCreated(r);
      onClose();
    } catch (e) {
      error = errorText(e);
    } finally {
      busy = false;
    }
  }

  function finish() {
    if (created) onCreated(created);
    onClose();
  }

  function close() {
    if (busy) return;
    if (created) finish();
    else onClose();
  }

  function loaderLabel(l: LoaderSpec | null) {
    return l ? `${l.kind === "fabric" ? "Fabric" : "Quilt"} ${l.version}` : t("common.vanilla");
  }

  const groups = $derived(
    plan
      ? {
          missing: plan.items.filter((i) => i.status === "missing"),
          substitute: plan.items.filter((i) => i.status === "substitute"),
          update: plan.items.filter((i) => i.status === "update"),
          compatible: plan.items.filter((i) => i.status === "compatible"),
          kept: plan.items.filter((i) => i.status === "kept"),
        }
      : null,
  );
</script>

<dialog bind:this={dialog} class="wide" oncancel={(e) => (e.preventDefault(), close())}>
  <div class="dialog-body">
    <h2>{t("migrate.title", { name: initial.name })}</h2>

    {#if warnings}
      <p>{t("migrate.doneWarnings")}</p>
      <ul class="warnings">
        {#each warnings as w (w)}<li>{w}</li>{/each}
      </ul>
    {:else if !plan}
      <p class="muted">{t("migrate.hint")}</p>
      <div class="row">
        <label class="grow">
          {t("dialog.version")}
          <select bind:value={minecraft}>
            {#if !versions.some((v) => v.id === minecraft)}<option value={minecraft}>{minecraft}</option>{/if}
            {#each versions as v (v.id)}<option value={v.id}>{v.id}</option>{/each}
          </select>
        </label>
        <label class="grow">
          {t("dialog.loader")}
          <select bind:value={loader}>
            <option value="none">{t("common.vanilla")}</option>
            <option value="fabric">Fabric</option>
            <option value="quilt">Quilt</option>
          </select>
        </label>
      </div>
      <label class="check">
        <input type="checkbox" bind:checked={snapshots} onchange={loadVersions} />
        {t("dialog.snapshots")}
      </label>
    {:else if groups}
      <p class="route">
        <span>{plan.fromMinecraft} · {loaderLabel(plan.fromLoader)}</span>
        <span aria-hidden="true">→</span>
        <strong>{plan.minecraft} · {loaderLabel(plan.loader)}</strong>
      </p>

      {#if plan.packOffer}
        <div class="offer">
          <span>{t("migrate.packOffer", { name: plan.packOffer.name, version: plan.packOffer.version })}</span>
          <button onclick={installPack} disabled={busy}>{t("migrate.packOfferButton")}</button>
        </div>
      {/if}

      <div class="summary">
        <span class="ok">✓ {t("migrate.found", { count: groups.update.length + groups.compatible.length })}</span>
        {#if groups.substitute.length}<span class="ok">↺ {t("migrate.substituted", { count: groups.substitute.length })}</span>{/if}
        {#if groups.missing.length}<span class="bad">✗ {t("migrate.missingCount", { count: groups.missing.length })}</span>{/if}
        {#if groups.kept.length}<span class="muted">· {t("migrate.keptCount", { count: groups.kept.length })}</span>{/if}
      </div>

      <div class="items">
        {#if groups.missing.length}
          <h3>{t("migrate.missingTitle")}</h3>
          {#each groups.missing as item (item.key)}
            <div class="item">
              <span class="name" title={item.key}>{item.name}</span>
              <select bind:value={missing[item.key]} aria-label={item.name}>
                <option value="disable">{t("migrate.disable")}</option>
                <option value="drop">{t("migrate.drop")}</option>
              </select>
            </div>
          {/each}
        {/if}
        {#if groups.substitute.length}
          <h3>{t("migrate.substituteTitle")}</h3>
          {#each groups.substitute as item (item.key)}
            <div class="item"><span class="name">{item.name}</span><span class="muted">→ {item.substitute}</span></div>
          {/each}
        {/if}
        {#if groups.update.length}
          <details>
            <summary>{t("migrate.updateTitle", { count: groups.update.length })}</summary>
            {#each groups.update as item (item.key)}
              <div class="item"><span class="name">{item.name}</span><span class="muted ver">{item.fromVersion ?? "?"} → {item.toVersion}</span></div>
            {/each}
          </details>
        {/if}
        {#if groups.compatible.length || groups.kept.length}
          <details>
            <summary>{t("migrate.unchangedTitle", { count: groups.compatible.length + groups.kept.length })}</summary>
            {#each [...groups.compatible, ...groups.kept] as item (item.key)}
              <div class="item">
                <span class="name">{item.name}</span>
                <span class="muted">{item.status === "kept" ? t("migrate.keptHint") : t("migrate.compatibleHint")}</span>
              </div>
            {/each}
          </details>
        {/if}
      </div>

      <label>
        {t("import.profileName")}
        <input bind:value={name} maxlength="64" />
      </label>
      {#if plan.worlds > 0}
        <label class="check">
          <input type="checkbox" bind:checked={includeWorlds} />
          {t("migrate.worlds", { count: plan.worlds })}
        </label>
        {#if includeWorlds && plan.downgrade}
          <p class="error">{t("migrate.downgrade")}</p>
        {/if}
      {/if}
      <p class="muted small">{t("migrate.copyHint")}</p>
    {/if}

    {#if error}<p class="error">{error}</p>{/if}
  </div>
  <div class="dialog-actions">
    {#if warnings}
      <button class="primary" onclick={finish}>{t("migrate.openProfile")}</button>
    {:else if !plan}
      <button onclick={onClose}>{t("common.cancel")}</button>
      <button class="primary" onclick={check} disabled={busy || !minecraft || sameAsNow}>{busy ? t("migrate.checking") : t("migrate.check")}</button>
    {:else}
      <button onclick={() => (plan = null)} disabled={busy}>{t("migrate.back")}</button>
      <button class="primary" onclick={apply} disabled={busy}>{busy ? t("migrate.working") : t("migrate.apply")}</button>
    {/if}
  </div>
</dialog>

<style>
  p {
    margin: 0;
  }

  dialog.wide {
    width: min(600px, calc(100vw - 32px));
  }

  .row {
    display: flex;
    gap: 10px;
  }

  .grow {
    flex: 1;
  }

  .check {
    flex-direction: row;
    align-items: center;
    gap: 6px;
    font-size: 13px;
    color: var(--text);
  }

  .check input {
    width: auto;
  }

  .route {
    display: flex;
    gap: 8px;
    align-items: baseline;
    flex-wrap: wrap;
  }

  .offer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 10px 12px;
    border-radius: var(--radius);
    border: 1px solid color-mix(in srgb, var(--accent) 40%, transparent);
    background: color-mix(in srgb, var(--accent) 10%, transparent);
    font-size: 13px;
  }

  .offer button {
    flex: none;
  }

  .summary {
    display: flex;
    gap: 14px;
    flex-wrap: wrap;
    font-size: 13px;
  }

  .ok {
    color: var(--accent);
  }

  .bad {
    color: var(--danger);
  }

  .items {
    max-height: 260px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding-right: 4px;
  }

  h3 {
    font-size: 13px;
    margin: 6px 0 2px;
  }

  summary {
    cursor: pointer;
    font-size: 13px;
    font-weight: 600;
    margin: 6px 0 4px;
  }

  .item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 10px;
    padding: 5px 10px;
    border-radius: var(--radius);
    background: var(--surface-2);
    font-size: 13px;
  }

  details .item + .item {
    margin-top: 4px;
  }

  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
  }

  .ver {
    font-size: 12px;
    text-align: right;
    overflow-wrap: anywhere;
  }

  .item select {
    width: auto;
    flex: none;
  }

  .warnings {
    margin: 0;
    padding-left: 18px;
    font-size: 13px;
    user-select: text;
  }

  .small {
    font-size: 12px;
  }
</style>
