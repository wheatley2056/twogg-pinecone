<script lang="ts">
  // Einstellungen › Java & Arbeitsspeicher: gefundene Java-Versionen; Speicher gilt je Instanz.
  import { onMount } from "svelte";
  import { api, errorText } from "../../lib/api";
  import { t } from "../../lib/i18n.svelte";
  import type { JavaInstallation } from "../../lib/types";
  import JvmPresets from "./JvmPresets.svelte";
  import SettingRow from "./SettingRow.svelte";

  let javas = $state<JavaInstallation[] | null>(null);
  let error = $state<string | null>(null);

  async function load() {
    javas = null;
    try {
      javas = await api.javaList();
    } catch (e) {
      error = errorText(e);
      javas = [];
    }
  }

  onMount(load);
</script>

<div class="head">
  <h1>{t("settings.javaTitle")}</h1>
  <p class="lead">{t("settings.javaLead")}</p>
</div>

<section>
  <div class="label-row">
    <h3 class="section-label">{t("settings.javaFound")}</h3>
    <button class="ghost small" onclick={load} disabled={javas === null}>{t("settings.rescan")}</button>
  </div>
  <div class="rows">
    {#if javas === null}
      <p class="hint">{t("settings.searching")}</p>
    {:else if javas.length === 0}
      <p class="hint">{t("settings.noJava")}</p>
    {:else}
      {#each javas as j (j.path)}
        <SettingRow title={`Java ${j.major}`} hint={j.path}>
          <span class="chip">{j.version}</span>
          <span class="chip" class:ok={j.managed}>{j.managed ? t("settings.byPinecone") : t("settings.system")}</span>
        </SettingRow>
      {/each}
    {/if}
  </div>
</section>

<section>
  <h3 class="section-label">{t("jvm.title")}</h3>
  <p class="hint">{t("jvm.lead")}</p>
  <JvmPresets />
</section>

<section>
  <h3 class="section-label">{t("settings.memory")}</h3>
  <div class="rows">
    <SettingRow title={t("settings.memoryPerInstance")} hint={t("settings.memoryHint")} />
  </div>
</section>

{#if error}<p class="error">{error}</p>{/if}

<style>
  .head h1 {
    font-weight: 600;
    font-size: 30px;
  }

  .lead {
    margin: 4px 0 0;
    font-size: 14px;
    color: var(--text-muted);
  }

  section {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  h3 {
    margin: 0;
    font-weight: 500;
  }

  .label-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .small {
    min-height: 28px;
    font-size: 12px;
  }

  .rows {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .hint {
    margin: 0;
    font-size: 13px;
    color: var(--text-dim);
  }
</style>
