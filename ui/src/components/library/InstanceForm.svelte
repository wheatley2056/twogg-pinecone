<script lang="ts">
  import { LOADERS } from "../../lib/profiles";
  // Felder einer Instanz: Name, Symbol, Minecraft-Version, Loader, Arbeitsspeicher, Java.
  // Genutzt vom Tab „Einstellungen“ und vom Dialog „Neue Instanz“.
  import type { Snippet } from "svelte";
  import { onMount, untrack } from "svelte";
  import { api, errorText } from "../../lib/api";
  import { t } from "../../lib/i18n.svelte";
  import { NO_PRESET, mergePresets } from "../../lib/jvm";
  import { settings } from "../../lib/settings.svelte";
  import type { JvmPreset, JavaInstallation, LoaderKind, LoaderVersion, Profile, VersionInfo } from "../../lib/types";
  import ProfileIcon, { MOTIFS, imageOf, keptIcon, type Motif } from "../ProfileIcon.svelte";
  import Segmented from "../ui/Segmented.svelte";

  interface Props {
    /** Ohne Profil: neue Instanz. */
    profile?: Profile;
    /** Spiel läuft: Version und Loader nicht ändern. */
    locked?: boolean;
    onSaved: (p: Profile) => void;
    /** Buttons unter dem Formular (Speichern ist immer dabei). */
    actions?: Snippet<[{ saving: boolean; canSave: boolean; dirty: boolean }]>;
    id?: string;
  }
  let { profile, locked = false, onSaved, actions, id = "instance-form" }: Props = $props();

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
  // Leer = globaler Standard aus Einstellungen › Java.
  let jvmPreset = $state(initial?.jvmPreset ?? "");
  let builtinPresets: JvmPreset[] = $state([]);
  const jvmPresets = $derived(settings.current ? mergePresets(builtinPresets, settings.current.jvm) : builtinPresets);
  const defaultPresetName = $derived.by(() => {
    const id = settings.current?.jvm.defaultPreset ?? NO_PRESET;
    return jvmPresets.find((p) => p.id === id)?.name ?? t("jvm.none");
  });
  // Motiv (`motif:<name>`), Pack-Logo (`data:`-URL) oder leer für Initialen.
  let icon = $state(keptIcon(initial?.icon) ?? "");
  const packLogo = imageOf(initial?.icon);

  let versions: VersionInfo[] = $state([]);
  let loaderVersions: LoaderVersion[] = $state([]);
  let loaderLoading = $state(false);
  let javas: JavaInstallation[] = $state([]);
  let saving = $state(false);
  let error: string | null = $state(null);
  let saved = $state(false);

  const recommended = $derived(loaderVersions.find((v) => v.stable) ?? loaderVersions[0]);
  const loaderName = $derived(loader === "none" ? "" : LOADERS[loader]);

  onMount(async () => {
    try {
      versions = await api.versions(false);
      if (!minecraft) minecraft = versions[0]?.id ?? "";
    } catch (e) {
      error = errorText(e);
    }
    javas = await api.javaList().catch(() => []);
    builtinPresets = await api.jvmBuiltinPresets().catch(() => []);
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

  // Die aktuelle Version auch dann anbieten, wenn sie nicht in der Liste steht (z. B. Snapshot).
  const versionOptions = $derived(
    minecraft && !versions.some((v) => v.id === minecraft) ? [{ id: minecraft, kind: "", releaseTime: "" }, ...versions] : versions,
  );

  const input = $derived({
    name: name.trim(),
    minecraft,
    loader: loader === "none" ? null : { kind: loader, version: loaderVersion },
    memoryMb,
    jvmArgs: jvmArgs.split(/\s+/).filter(Boolean),
    jvmPreset: jvmPreset || null,
    java: javaMode === "custom" && javaPath.trim() ? javaPath.trim() : null,
    icon: icon || null,
    color: initial?.color ?? null,
  });

  const dirty = $derived(
    !initial ||
      JSON.stringify(input) !==
        JSON.stringify({
          name: initial.name,
          minecraft: initial.minecraft,
          loader: initial.loader,
          memoryMb: initial.memoryMb ?? 4096,
          jvmArgs: initial.jvmArgs,
          jvmPreset: initial.jvmPreset ?? null,
          java: initial.java,
          icon: keptIcon(initial.icon),
          color: initial.color,
        }),
  );

  const canSave = $derived(
    dirty && name.trim().length > 0 && minecraft !== "" && !saving && (loader === "none" || (!loaderLoading && loaderVersions.length > 0)),
  );

  async function save(event: SubmitEvent) {
    event.preventDefault();
    if (!canSave) return;
    saving = true;
    error = null;
    saved = false;
    try {
      const result = profile ? await api.updateProfile(profile.id, input) : await api.createProfile(input);
      saved = !!profile;
      onSaved(result);
    } catch (e) {
      error = errorText(e);
    } finally {
      saving = false;
    }
  }
</script>

<form {id} class="form" onsubmit={save}>
  <div class="grid">
    <label>
      {t("dialog.name")}
      <input bind:value={name} maxlength="64" placeholder={t("dialog.namePlaceholder")} />
    </label>

    <div class="field">
      <span class="field-label">{t("dialog.icon")}</span>
      <div class="motifs" role="radiogroup" aria-label={t("dialog.icon")}>
        <button type="button" class="motif" class:selected={icon === ""} role="radio" aria-checked={icon === ""}
          title={t("dialog.initials")} aria-label={t("dialog.initials")} onclick={() => (icon = "")}>
          <ProfileIcon profile={{ name: name || "?", icon: null }} size={34} />
        </button>
        {#if packLogo}
          <button type="button" class="motif" class:selected={icon === packLogo} role="radio" aria-checked={icon === packLogo}
            title={t("dialog.packLogo")} aria-label={t("dialog.packLogo")} onclick={() => (icon = packLogo)}>
            <ProfileIcon profile={{ name, icon: packLogo }} size={34} />
          </button>
        {/if}
        {#each Object.keys(MOTIFS) as Motif[] as m (m)}
          <button type="button" class="motif" class:selected={icon === `motif:${m}`} role="radio"
            aria-checked={icon === `motif:${m}`} title={t(`motif.${m}`)} aria-label={t(`motif.${m}`)}
            onclick={() => (icon = `motif:${m}`)}>
            <ProfileIcon profile={{ name, icon: `motif:${m}` }} size={34} />
          </button>
        {/each}
      </div>
    </div>

    <label>
      {t("dialog.version")}
      <select bind:value={minecraft} disabled={versions.length === 0 || locked}>
        {#each versionOptions as v (v.id)}
          <option value={v.id}>{v.id}{v.kind === "snapshot" ? ` (${t("dialog.snapshot")})` : ""}</option>
        {/each}
      </select>
      <span class="inline-check">
        <input type="checkbox" bind:checked={snapshots} onchange={toggleSnapshots} id="{id}-snapshots" />
        <label for="{id}-snapshots">{t("dialog.snapshots")}</label>
      </span>
    </label>

    <div class="field">
      <span class="field-label">{t("dialog.loader")}</span>
      <Segmented
        items={[
          { id: "none", label: t("common.vanilla") },
          { id: "fabric", label: "Fabric" },
          { id: "quilt", label: "Quilt" },
          { id: "forge", label: "Forge" },
          { id: "neoforge", label: "NeoForge" },
        ]}
        value={loader}
        label={t("dialog.loader")}
        variant="soft"
        onchange={(v) => {
          if (locked) return;
          if (loader !== v) loaderVersion = "";
          loader = v as LoaderKind | "none";
        }}
      />
      {#if loader !== "none"}
        {#if loaderLoading}
          <select disabled><option>{t("common.loading")}</option></select>
        {:else if loaderVersions.length === 0}
          <p class="warning">{t("dialog.loaderUnsupported", { loader: loaderName, version: minecraft })}</p>
        {:else}
          <select bind:value={loaderVersion} aria-label={t("dialog.loaderVersion", { loader: loaderName })} disabled={locked}>
            <option value="">
              {t(recommended?.stable ? "dialog.recommended" : "dialog.recommendedPre", { version: recommended?.version ?? "" })}
            </option>
            {#each loaderVersions as v (v.version)}
              <option value={v.version}>{v.version}{v.stable ? "" : ` (${t("dialog.prerelease")})`}</option>
            {/each}
          </select>
        {/if}
      {/if}
    </div>

    <label>
      <span class="memory">{t("dialog.memoryLabel")} <span class="mono value">{(memoryMb / 1024).toFixed(1).replace(".0", "")} GB</span></span>
      <input type="range" min="1024" max="16384" step="512" bind:value={memoryMb} />
    </label>

    <label>
      {t("dialog.java")}
      <select bind:value={javaMode}>
        <option value="auto">{t("dialog.javaAuto")}</option>
        <option value="custom">{t("dialog.javaCustom")}</option>
      </select>
      {#if javaMode === "custom"}
        <input bind:value={javaPath} list="{id}-java" placeholder="/usr/lib/jvm/…/bin/java" aria-label={t("dialog.javaPath")} />
        <datalist id="{id}-java">
          {#each javas as j (j.path)}
            <option value={j.path}>Java {j.major} ({j.version}){j.managed ? ` · ${t("dialog.managed")}` : ""}</option>
          {/each}
        </datalist>
      {/if}
    </label>

    <label class="wide">
      {t("dialog.jvmPreset")}
      <select bind:value={jvmPreset}>
        <option value="">{t("dialog.jvmPresetDefault", { name: defaultPresetName })}</option>
        <option value={NO_PRESET}>{t("jvm.none")}</option>
        {#each jvmPresets as p (p.id)}
          <option value={p.id}>{p.name}{p.minJava > 8 ? ` · Java ${p.minJava}+` : ""}</option>
        {/each}
      </select>
    </label>

    <label class="wide">
      {t("dialog.jvmArgs")}
      <input class="mono" bind:value={jvmArgs} placeholder={t("dialog.jvmArgsPlaceholder")} spellcheck="false" />
    </label>
  </div>

  {#if error}<p class="error">{error}</p>{/if}
  {#if saved && !dirty}<p class="saved">{t("instance.saved")}</p>{/if}
  {@render actions?.({ saving, canSave, dirty })}
</form>

<style>
  .form {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 16px 20px;
  }

  .wide {
    grid-column: 1 / -1;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .field-label {
    color: var(--text-dim);
    font-size: 12px;
  }

  .motifs {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }

  .motif {
    padding: 2px;
    min-height: 0;
    background: transparent;
  }

  .motif.selected {
    background: var(--surface-active);
    box-shadow: inset 0 -3px 0 var(--accent);
  }

  .inline-check {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-top: 2px;
  }

  .inline-check input {
    width: auto;
  }

  .inline-check label {
    flex-direction: row;
    font-size: 12px;
  }

  .memory {
    display: flex;
    justify-content: space-between;
  }

  .value {
    color: var(--text);
  }

  input[type="range"] {
    padding: 0;
    background: transparent;
  }

  .warning {
    margin: 0;
    font-size: 13px;
    color: var(--accent-fg);
  }

  .saved {
    margin: 0;
    font-size: 13px;
    color: var(--nadel);
  }
</style>
