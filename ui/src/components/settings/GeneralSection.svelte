<script lang="ts">
  // Einstellungen › Allgemein: Sprache, Fensterrahmen, App-Updates, Diagnose, Datenordner,
  // Über Pinecone (Pflichthinweis zu Minecraft, Impressum, Datenschutz, Lizenz).
  import { onMount } from "svelte";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { api, errorText } from "../../lib/api";
  import { exportDiagnostics } from "../../lib/diagnostics";
  import { t } from "../../lib/i18n.svelte";
  import { settings, updateSettings } from "../../lib/settings.svelte";
  import { checkUpdates, installUpdate, updates } from "../../lib/updates.svelte";
  import type { AppInfo, ServerInfo, Settings } from "../../lib/types";
  import Dropdown from "../ui/Dropdown.svelte";
  import Segmented from "../ui/Segmented.svelte";
  import Switch from "../ui/Switch.svelte";
  import SettingRow from "./SettingRow.svelte";

  let { info }: { info: AppInfo | null } = $props();

  let error = $state<string | null>(null);
  let notice = $state<string | null>(null);
  let diagBusy = $state(false);
  const s = $derived(settings.current);
  let legal = $state<ServerInfo["legal"] | null>(null);

  onMount(() => {
    api
      .serverInfo()
      .then((i) => (legal = i.legal ?? null))
      .catch(() => {});
  });

  const open = (url: string) => openUrl(url).catch((e: unknown) => (error = errorText(e)));

  async function change(fn: (x: Settings) => void) {
    error = null;
    await updateSettings(fn).catch((e) => (error = errorText(e)));
  }

  async function diagnose() {
    diagBusy = true;
    error = notice = null;
    try {
      notice = await exportDiagnostics();
    } catch (e) {
      error = errorText(e);
    } finally {
      diagBusy = false;
    }
  }
</script>

<div class="head">
  <h1>{t("settings.general")}</h1>
</div>

{#if s}
  <section>
    <h3 class="section-label">{t("settings.launcher")}</h3>
    <div class="rows">
      <SettingRow title={t("settings.language")}>
        <Dropdown
          label=""
          value={s.language}
          options={[
            { value: "de", label: "Deutsch" },
            { value: "en", label: "English" },
          ]}
          align="right"
          onchange={(v) => change((x) => (x.language = v as "de" | "en"))}
        />
      </SettingRow>
      <SettingRow title={t("settings.splash")} hint={t("settings.splashHint")}>
        <Switch checked={s.splash} label={t("settings.splash")} onchange={(on) => change((x) => (x.splash = on))} />
      </SettingRow>
      <SettingRow title={t("settings.systemFrame")} hint={t("settings.systemFrameHint")}>
        <Switch checked={s.systemFrame} label={t("settings.systemFrame")} onchange={(on) => change((x) => (x.systemFrame = on))} />
      </SettingRow>
      <SettingRow title={t("settings.launchBehavior")} hint={t(`settings.launchBehaviorHint.${s.launchBehavior}`)}>
        <Dropdown
          label=""
          value={s.launchBehavior}
          options={[
            { value: "keep", label: t("settings.launchKeep") },
            { value: "minimize", label: t("settings.launchMinimize") },
            { value: "hide", label: t("settings.launchHide") },
            { value: "close", label: t("settings.launchClose") },
          ]}
          align="right"
          onchange={(v) => change((x) => (x.launchBehavior = v))}
        />
      </SettingRow>
      {#if s.launchBehavior === "minimize"}
        <SettingRow title={t("settings.showAfterExit")} hint={t("settings.showAfterExitHint")}>
          <Switch checked={s.showAfterExit} label={t("settings.showAfterExit")} onchange={(on) => change((x) => (x.showAfterExit = on))} />
        </SettingRow>
      {/if}
      <SettingRow title={t("settings.dataDir")} hint={info?.dataDir ?? "?"} />
    </div>
  </section>
{/if}

<section>
  <h3 class="section-label">{t("appUpdate.title")}</h3>
  <div class="rows">
    <SettingRow
      title={t("appUpdate.current", { version: info?.version ?? "?" })}
      hint={!info?.updatesConfigured
        ? t("appUpdate.notConfigured")
        : updates.info
          ? t("appUpdate.available", { version: updates.info.version })
          : updates.checked
            ? t("appUpdate.upToDate")
            : undefined}
    >
      {#if info?.updatesConfigured && updates.progress === null}
        {#if updates.info}<button class="primary cut" onclick={installUpdate}>{t("appUpdate.install")}</button>{/if}
        <button onclick={() => checkUpdates()} disabled={updates.checking}>{updates.checking ? t("appUpdate.checking") : t("appUpdate.check")}</button>
      {:else if updates.progress !== null}
        <progress max="1" value={updates.progress}></progress>
      {/if}
    </SettingRow>
    {#if s && info?.updatesConfigured}
      <SettingRow title={t("appUpdate.channel")} hint={s.updateChannel === "beta" ? t("appUpdate.channelBetaHint") : t("appUpdate.channelReleaseHint")}>
        <Segmented
          items={[
            { id: "release", label: t("appUpdate.channelRelease") },
            { id: "beta", label: t("appUpdate.channelBeta") },
          ]}
          value={s.updateChannel}
          label={t("appUpdate.channel")}
          variant="soft"
          onchange={async (c) => {
            await change((x) => (x.updateChannel = c));
            updates.info = null;
            checkUpdates();
          }}
        />
      </SettingRow>
    {/if}
    {#if updates.info?.notes}<pre class="notes">{updates.info.notes}</pre>{/if}
    {#if updates.error}<p class="error">{updates.error}</p>{/if}
  </div>
</section>

<section>
  <h3 class="section-label">{t("diag.title")}</h3>
  <div class="rows">
    <SettingRow title={t("diag.export")} hint={t("diag.hint")}>
      <button onclick={diagnose} disabled={diagBusy}>{diagBusy ? t("common.loading") : t("diag.exportShort")}</button>
      <button class="ghost" onclick={() => api.openLogsFolder().catch((e) => (error = errorText(e)))}>{t("diag.openLogs")}</button>
    </SettingRow>
  </div>
  {#if notice}<p class="notice">{notice}</p>{/if}
</section>

<section>
  <h3 class="section-label">{t("about.title")}</h3>
  <div class="rows">
    <SettingRow title={t("about.name", { version: info?.version ?? "?" })} hint={t("about.license")} />
    <p class="disclaimer">{t("about.disclaimer")}</p>
    {#if legal?.imprintUrl || legal?.privacyUrl || legal?.contactEmail}
      <div class="links">
        {#if legal.imprintUrl}<button class="link" onclick={() => open(legal!.imprintUrl!)}>{t("about.imprint")}</button>{/if}
        {#if legal.privacyUrl}<button class="link" onclick={() => open(legal!.privacyUrl!)}>{t("about.privacy")}</button>{/if}
        {#if legal.contactEmail}<button class="link" onclick={() => open(`mailto:${legal!.contactEmail}`)}>{legal.contactEmail}</button>{/if}
      </div>
    {/if}
  </div>
</section>

{#if error}<p class="error">{error}</p>{/if}

<style>
  .disclaimer {
    margin: 0;
    font-size: 12px;
    line-height: 1.5;
    color: var(--text-muted);
  }

  .links {
    display: flex;
    gap: 16px;
    flex-wrap: wrap;
  }

  .link {
    min-height: 0;
    padding: 0;
    background: none;
    box-shadow: none;
    color: var(--accent);
    font-weight: 700;
  }

  .head h1 {
    font-weight: 600;
    font-size: 30px;
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

  .notes {
    margin: 0;
    padding: 10px 14px;
    background: var(--bar);
    font-size: 12px;
    white-space: pre-wrap;
    user-select: text;
  }

  progress {
    width: 160px;
  }

  .notice {
    margin: 0;
    font-size: 13px;
    color: var(--nadel);
    user-select: text;
  }
</style>
