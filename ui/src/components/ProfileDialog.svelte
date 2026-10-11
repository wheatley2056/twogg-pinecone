<script lang="ts">
  import { onMount, untrack } from "svelte";
  import { api, errorText } from "../lib/api";
  import { t } from "../lib/i18n.svelte";
  import ProfileIcon, { MOTIFS, motifOf, type Motif } from "./ProfileIcon.svelte";
  import type { JavaInstallation, LoaderKind, LoaderVersion, Profile, VersionInfo } from "../lib/types";

  interface Props {
    /** Ohne Profil: neues anlegen. */
    profile?: Profile;
    onClose: () => void;
    onSaved: (p: Profile) => void;
  }
  let { profile, onClose, onSaved }: Props = $props();

  let dialog: HTMLDialogElement;
  // Startwerte einmalig übernehmen; der Dialog wird für jedes Öffnen neu erzeugt.
  const initial = untrack(() => profile);
  let name = $state(initial?.name ?? "");
  let minecraft = $state(initial?.minecraft ?? "");
  let snapshots = $state(false);
  let loader: LoaderKind | "none" = $state(initial?.loader?.kind ?? "none");
  // Leer = empfohlene Version beim Speichern wählen.
  let loaderVersion = $state(initial?.loader?.version ?? "");
  let memoryMb = $state(initial?.memoryMb ?? 4096);
  let javaMode: "auto" | "custom" = $state(initial?.java ? "custom" : "auto");
  let javaPath = $state(initial?.java ?? "");
  let jvmArgs = $state(initial?.jvmArgs.join(" ") ?? "");
  // Motiv (`motif:<name>`) oder leer für Initialen; alte Emoji-Symbole werden nicht mehr angeboten.
  let icon = $state(motifOf(initial?.icon) ? initial!.icon! : "");

  let versions: VersionInfo[] = $state([]);
  let loaderVersions: LoaderVersion[] = $state([]);
  let loaderLoading = $state(false);
  let javas: JavaInstallation[] = $state([]);
  let saving = $state(false);
  let error: string | null = $state(null);

  const recommended = $derived(loaderVersions.find((v) => v.stable) ?? loaderVersions[0]);
  const loaderName = $derived(loader === "fabric" ? "Fabric" : "Quilt");

  onMount(async () => {
    dialog.showModal();
    try {
      versions = await api.versions(false);
      if (!minecraft) minecraft = versions[0]?.id ?? "";
    } catch (e) {
      error = errorText(e);
    }
    javas = await api.javaList().catch(() => []);
  });

  async function toggleSnapshots() {
    try {
      versions = await api.versions(snapshots);
    } catch (e) {
      error = errorText(e);
    }
  }

  // Loader-Versionen neu laden, wenn sich Version oder Loader ändern.
  let loaderRequest = 0;
  $effect(() => {
    const kind = loader;
    const mc = minecraft;
    if (kind === "none" || !mc) {
      loaderVersions = [];
      return;
    }
    const request = ++loaderRequest;
    loaderLoading = true;
    api
      .loaderVersions(kind, mc)
      .then((list) => {
        if (request !== loaderRequest) return;
        loaderVersions = list;
        if (loaderVersion && !list.some((v) => v.version === loaderVersion)) loaderVersion = "";
      })
      .catch((e) => request === loaderRequest && (error = errorText(e)))
      .finally(() => request === loaderRequest && (loaderLoading = false));
  });

  // Die aktuelle Version des Profils auch dann anbieten, wenn sie nicht in der Liste steht (z. B. Snapshot).
  const versionOptions = $derived(
    minecraft && !versions.some((v) => v.id === minecraft) ? [{ id: minecraft, kind: "", releaseTime: "" }, ...versions] : versions,
  );

  const canSave = $derived(
    name.trim().length > 0 && minecraft !== "" && !saving && (loader === "none" || (!loaderLoading && loaderVersions.length > 0)),
  );

  async function save(event: SubmitEvent) {
    event.preventDefault();
    if (!canSave) return;
    saving = true;
    error = null;
    const input = {
      name: name.trim(),
      minecraft,
      loader: loader === "none" ? null : { kind: loader, version: loaderVersion },
      memoryMb,
      jvmArgs: jvmArgs.split(/\s+/).filter(Boolean),
      java: javaMode === "custom" && javaPath.trim() ? javaPath.trim() : null,
      icon: icon || null,
      color: initial?.color ?? null,
    };
    try {
      const saved = profile ? await api.updateProfile(profile.id, input) : await api.createProfile(input);
      onSaved(saved);
    } catch (e) {
      error = errorText(e);
      saving = false;
    }
  }
</script>

<dialog bind:this={dialog} oncancel={(e) => (e.preventDefault(), onClose())}>
  <form onsubmit={save}>
    <div class="dialog-body">
      <h2>{profile ? t("dialog.editProfile") : t("dialog.newProfile")}</h2>

      <label>
        {t("dialog.name")}
        <!-- svelte-ignore a11y_autofocus (Erstes Feld im modalen Dialog) -->
        <input bind:value={name} maxlength="64" placeholder={t("dialog.namePlaceholder")} autofocus />
      </label>

      <div class="row">
        <label class="grow">
          {t("dialog.version")}
          <select bind:value={minecraft} disabled={versions.length === 0}>
            {#each versionOptions as v (v.id)}
              <option value={v.id}>{v.id}{v.kind === "snapshot" ? ` (${t("dialog.snapshot")})` : ""}</option>
            {/each}
          </select>
        </label>
        <label class="check">
          <input type="checkbox" bind:checked={snapshots} onchange={toggleSnapshots} />
          {t("dialog.snapshots")}
        </label>
      </div>

      <div class="field">
        <span class="field-label">{t("dialog.loader")}</span>
        <div class="segmented" role="radiogroup">
          {#each [["none", t("common.vanilla")], ["fabric", "Fabric"], ["quilt", "Quilt"]] as [value, label] (value)}
            <button
              type="button"
              role="radio"
              aria-checked={loader === value}
              class:selected={loader === value}
              onclick={() => {
                if (loader !== value) loaderVersion = "";
                loader = value as LoaderKind | "none";
              }}>{label}</button
            >
          {/each}
        </div>
      </div>

      {#if loader !== "none"}
        <label>
          {t("dialog.loaderVersion", { loader: loaderName })}
          {#if loaderLoading}
            <select disabled><option>{t("common.loading")}</option></select>
          {:else if loaderVersions.length === 0}
            <p class="warning">{t("dialog.loaderUnsupported", { loader: loaderName, version: minecraft })}</p>
          {:else}
            <select bind:value={loaderVersion}>
              <option value="">
                {t(recommended?.stable ? "dialog.recommended" : "dialog.recommendedPre", { version: recommended?.version ?? "" })}
              </option>
              {#each loaderVersions as v (v.version)}
                <option value={v.version}>{v.version}{v.stable ? "" : ` (${t("dialog.prerelease")})`}</option>
              {/each}
            </select>
          {/if}
        </label>
      {/if}

      <label>
        {t("dialog.memory", { gb: (memoryMb / 1024).toFixed(1).replace(".0", "") })}
        <input type="range" min="1024" max="16384" step="512" bind:value={memoryMb} />
      </label>

      <div class="field">
        <span class="field-label">{t("dialog.look")}</span>
        <div class="look">
          <div class="motifs" role="radiogroup" aria-label={t("dialog.icon")}>
            <button type="button" class="motif" class:selected={icon === ""} role="radio" aria-checked={icon === ""}
              title={t("dialog.initials")} onclick={() => (icon = "")}>
              <ProfileIcon profile={{ name: name || "?", icon: null }} size={36} />
            </button>
            {#each Object.keys(MOTIFS) as Motif[] as m (m)}
              <button type="button" class="motif" class:selected={icon === `motif:${m}`} role="radio"
                aria-checked={icon === `motif:${m}`} title={t(`motif.${m}`)} aria-label={t(`motif.${m}`)}
                onclick={() => (icon = `motif:${m}`)}>
                <ProfileIcon profile={{ name, icon: `motif:${m}` }} size={36} />
              </button>
            {/each}
          </div>
        </div>
      </div>

      <details open={javaMode === "custom" || jvmArgs !== ""}>
        <summary>{t("dialog.advanced")}</summary>
        <div class="advanced">
          <label>
            {t("dialog.java")}
            <select bind:value={javaMode}>
              <option value="auto">{t("dialog.javaAuto")}</option>
              <option value="custom">{t("dialog.javaCustom")}</option>
            </select>
          </label>
          {#if javaMode === "custom"}
            <label>
              {t("dialog.javaPath")}
              <input bind:value={javaPath} list="java-list" placeholder="/usr/lib/jvm/…/bin/java" />
              <datalist id="java-list">
                {#each javas as j (j.path)}
                  <option value={j.path}>Java {j.major} ({j.version}){j.managed ? ` · ${t("dialog.managed")}` : ""}</option>
                {/each}
              </datalist>
            </label>
          {/if}
          <label>
            {t("dialog.jvmArgs")}
            <input bind:value={jvmArgs} placeholder={t("dialog.jvmArgsPlaceholder")} spellcheck="false" />
          </label>
        </div>
      </details>

      {#if error}<p class="error">{error}</p>{/if}
    </div>
    <div class="dialog-actions">
      <button type="button" onclick={onClose}>{t("common.cancel")}</button>
      <button type="submit" class="primary" disabled={!canSave}>
        {saving ? t("dialog.saving") : profile ? t("common.save") : t("dialog.create")}
      </button>
    </div>
  </form>
</dialog>

<style>
  .row {
    display: flex;
    gap: 12px;
    align-items: flex-end;
  }

  .grow {
    flex: 1;
  }

  .check {
    flex-direction: row;
    align-items: center;
    gap: 6px;
    padding-bottom: 9px;
    white-space: nowrap;
  }

  .check input {
    width: auto;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .field-label {
    color: var(--muted);
    font-size: 12px;
  }

  .segmented {
    display: flex;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    overflow: hidden;
  }

  .segmented button {
    flex: 1;
    border: none;
    border-radius: 0;
    background: var(--bg);
  }

  .segmented button + button {
    border-left: 1px solid var(--border);
  }

  .segmented button.selected {
    background: var(--accent);
    color: var(--accent-text);
    font-weight: 600;
  }

  input[type="range"] {
    padding: 0;
    accent-color: var(--accent);
  }

  details summary {
    cursor: pointer;
    color: var(--muted);
    font-size: 13px;
  }

  .advanced {
    display: flex;
    flex-direction: column;
    gap: 12px;
    margin-top: 10px;
  }

  .look {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .motifs {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .motif {
    padding: 3px;
    min-height: 0;
    background: transparent;
  }

  .motif.selected {
    box-shadow: inset 0 -3px 0 var(--accent);
    background: var(--surface-active);
  }

  .warning {
    margin: 0;
    color: var(--warning);
    font-size: 13px;
  }
</style>
