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
  import ConfirmDialog from "../ConfirmDialog.svelte";
  import Dropdown from "../ui/Dropdown.svelte";
  import Segmented from "../ui/Segmented.svelte";
  import Switch from "../ui/Switch.svelte";
  import SettingRow from "./SettingRow.svelte";

  let { info }: { info: AppInfo | null } = $props();

  let error = $state<string | null>(null);
  let notice = $state<string | null>(null);
  let diagBusy = $state(false);
  let wiping = $state(false);

  async function wipe() {
    error = null;
    try {
      try {
        localStorage.clear();
      } catch {
        // Ohne Zugriff bleibt nur, was der Launcher selbst löscht.
      }
      await api.wipeLocalData();
    } catch (e) {
      error = errorText(e);
      wiping = false;
    }
  }
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

  function leadLabel(m: number) {
    return m >= 60 ? t(m === 60 ? "reminder.hour" : "reminder.hours", { count: m / 60 }) : t("reminder.minutes", { count: m });
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
      {#if info?.discordConfigured}
        <SettingRow title={t("settings.discord")} hint={t("settings.discordHint")}>
          <Switch checked={s.discordPresence} label={t("settings.discord")} onchange={(on) => change((x) => (x.discordPresence = on))} />
        </SettingRow>
      {/if}
      <SettingRow title={t("settings.dataDir")} hint={info?.dataDir ?? "?"} />
      <SettingRow title={t("settings.wipe")} hint={t("settings.wipeHint")}>
        <button onclick={() => (wiping = true)}>{t("settings.wipeButton")}</button>
      </SettingRow>
    </div>
  </section>

  <!-- Benachrichtigungen (F-14, F-37) -->
  {@const n = s.notifications}
  <section>
    <h3 class="section-label">{t("settings.notifications")}</h3>
    <div class="rows">
      <SettingRow title={t("settings.eventReminders")} hint={t("settings.eventRemindersHint")}>
        <Switch checked={n.eventReminders} label={t("settings.eventReminders")} onchange={(on) => change((x) => (x.notifications.eventReminders = on))} />
      </SettingRow>
      {#if n.eventReminders}
        <SettingRow title={t("settings.reminderTimes")}>
          <div class="leads">
            {#each [0, 1] as i (i)}
              <Dropdown
                label=""
                value={String(n.leadMinutes[i] ?? 0)}
                options={(i === 0 ? [180, 120, 60, 30] : [30, 15, 10, 5, 0]).map((m) => ({ value: String(m), label: m ? leadLabel(m) : t("settings.reminderOff") }))}
                align="right"
                onchange={(v) =>
                  change((x) => {
                    const leads = [x.notifications.leadMinutes[0] ?? 60, x.notifications.leadMinutes[1] ?? 0];
                    leads[i] = Number(v);
                    x.notifications.leadMinutes = leads.filter((m) => m > 0);
                  })}
              />
            {/each}
          </div>
        </SettingRow>
        <SettingRow title={t("settings.allEvents")} hint={t("settings.allEventsHint")}>
          <Switch checked={n.allEvents} label={t("settings.allEvents")} onchange={(on) => change((x) => (x.notifications.allEvents = on))} />
        </SettingRow>
      {/if}
      <SettingRow title={t("settings.osCrash")}>
        <Switch checked={n.osCrash} label={t("settings.osCrash")} onchange={(on) => change((x) => (x.notifications.osCrash = on))} />
      </SettingRow>
      <SettingRow title={t("settings.osFriends")} hint={t("settings.osHint")}>
        <Switch checked={n.osFriends} label={t("settings.osFriends")} onchange={(on) => change((x) => (x.notifications.osFriends = on))} />
      </SettingRow>
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

{#if wiping}
  <ConfirmDialog title={t("settings.wipeTitle")} confirmLabel={t("settings.wipeConfirm")} onConfirm={wipe} onCancel={() => (wiping = false)}>
    <p>{t("settings.wipeText")}</p>
    <ul class="wipe-list">
      <li>{t("settings.wipeAccounts")}</li>
      <li>{t("settings.wipeInstances")}</li>
      <li>{t("settings.wipeRest")}</li>
    </ul>
    <p class="muted">{t("settings.wipeServer")}</p>
  </ConfirmDialog>
{/if}

<style>
  .wipe-list {
    margin: 0;
    padding-left: 20px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .muted {
    color: var(--text-muted);
    font-size: 13px;
  }

  .leads {
    display: flex;
    gap: 8px;
  }

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
