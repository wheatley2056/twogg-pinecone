<script lang="ts" module>
  export type SettingsSection = "general" | "appearance" | "java" | "accounts" | "sources" | "twogg";
</script>

<script lang="ts">
  // Einstellungen nach docs/design/referenz/04-einstellungen-darstellung.html: Navigation links
  // (230 px), rechts der gewählte Bereich.
  import { settings } from "../../lib/settings.svelte";
  import { t } from "../../lib/i18n.svelte";
  import type { Account, AppInfo } from "../../lib/types";
  import AccountsSection from "../settings/AccountsSection.svelte";
  import AppearanceSection from "../settings/AppearanceSection.svelte";
  import GeneralSection from "../settings/GeneralSection.svelte";
  import JavaSection from "../settings/JavaSection.svelte";
  import SourcesSection from "../settings/SourcesSection.svelte";
  import TwoggSection from "../settings/TwoggSection.svelte";

  interface Props {
    info: AppInfo | null;
    accounts: Account[];
    section: SettingsSection;
    onAccountsChange: () => void;
  }
  let { info, accounts, section = $bindable(), onAccountsChange }: Props = $props();

  const sections: SettingsSection[] = ["general", "appearance", "java", "accounts", "sources", "twogg"];
</script>

<div class="settings">
  <aside aria-label={t("settings.areas")}>
    <h2>{t("settings.title")}</h2>
    {#each sections as id (id)}
      <button class="nav" class:active={section === id} aria-current={section === id ? "page" : undefined} onclick={() => (section = id)}>
        {t(`settings.nav.${id}`)}
      </button>
    {/each}
    <div class="version mono">
      Pinecone {info?.version ?? "?"}<br />{info?.updatesConfigured
        ? t("settings.channelAuto", { channel: settings.current?.updateChannel === "beta" ? t("appUpdate.channelBeta") : t("appUpdate.channelRelease") })
        : t("settings.channelManual")}
    </div>
  </aside>
  <main>
    {#if section === "general"}
      <GeneralSection {info} />
    {:else if section === "appearance"}
      <AppearanceSection />
    {:else if section === "java"}
      <JavaSection />
    {:else if section === "accounts"}
      <AccountsSection {accounts} {info} onChange={onAccountsChange} />
    {:else if section === "sources"}
      <SourcesSection {info} />
    {:else}
      <TwoggSection />
    {/if}
  </main>
</div>

<style>
  .settings {
    display: grid;
    grid-template-columns: 230px minmax(0, 1fr);
    height: 100%;
    min-height: 0;
  }

  aside {
    background: var(--side);
    border-right: 1px solid var(--line);
    padding: 18px 12px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  h2 {
    margin: 0 0 12px;
    padding: 0 10px;
    font-weight: 600;
    font-size: 22px;
  }

  .nav {
    justify-content: flex-start;
    height: 40px;
    padding: 0 12px;
    background: transparent;
    font-size: 14px;
    font-weight: 600;
    color: var(--text-muted);
    text-align: left;
  }

  .nav:hover:not(:disabled) {
    background: var(--surface);
    color: var(--text);
  }

  .nav.active,
  .nav.active:hover:not(:disabled) {
    background: var(--surface-active);
    font-weight: 700;
    color: var(--accent-hover);
    clip-path: polygon(7px 0, 100% 0, 100% 100%, 0 100%, 0 7px);
  }

  .version {
    margin-top: auto;
    padding: 12px;
    background: var(--surface);
    font-size: 11px;
    line-height: 1.6;
    color: var(--text-dim);
  }

  main {
    min-height: 0;
    overflow-y: auto;
    padding: 20px 30px 28px;
    display: flex;
    flex-direction: column;
    gap: 18px;
  }
</style>
