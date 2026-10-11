<script lang="ts">
  // Einstellungen › Downloads & Quellen: welche Dienste angebunden sind.
  import { onMount } from "svelte";
  import { api } from "../../lib/api";
  import { t } from "../../lib/i18n.svelte";
  import type { AppInfo, ServerInfo } from "../../lib/types";
  import SettingRow from "./SettingRow.svelte";

  let { info }: { info: AppInfo | null } = $props();

  let server = $state<ServerInfo | null | "offline">(null);

  onMount(() => {
    api
      .serverInfo()
      .then((s) => (server = s))
      .catch(() => (server = "offline"));
  });
</script>

<div class="head">
  <h1>{t("settings.sources")}</h1>
  <p class="lead">{t("settings.sourcesLead")}</p>
</div>

<section>
  <h3 class="section-label">{t("settings.sourcesContent")}</h3>
  <div class="rows">
    <SettingRow title="Modrinth" hint={t("settings.modrinthHint")}><span class="chip ok">{t("settings.connected")}</span></SettingRow>
    <SettingRow title="CurseForge" hint={info?.curseforgeConfigured ? t("settings.cfOk") : t("settings.cfHint")}>
      <span class="chip" class:ok={info?.curseforgeConfigured}>{info?.curseforgeConfigured ? t("settings.connected") : t("settings.notConfigured")}</span>
    </SettingRow>
    <SettingRow title={t("settings.pineconeServer")} hint={t("settings.pineconeServerHint")}>
      <span class="chip" class:ok={server !== null && server !== "offline"}>
        {server === null ? t("common.loading") : server === "offline" ? t("settings.unreachable") : t("settings.connected")}
      </span>
    </SettingRow>
  </div>
</section>

<section>
  <h3 class="section-label">{t("settings.sourcesAccounts")}</h3>
  <div class="rows">
    <SettingRow title={t("settings.msa")} hint={info?.msaConfigured ? t("settings.msaOkHint") : t("settings.msaHint")}>
      <span class="chip" class:ok={info?.msaConfigured}>{info?.msaConfigured ? t("settings.connected") : t("settings.notConfigured")}</span>
    </SettingRow>
  </div>
</section>

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

  .rows {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
</style>
