<script lang="ts" module>
  export type Tab = "home" | "library" | "discover" | "events" | "skins";
</script>

<script lang="ts">
  // Kopfleiste (52 px): Logo, Haupt-Tabs, Aktivität, Benachrichtigungen, Einstellungen, Fenster-Buttons.
  import { t } from "../../lib/i18n.svelte";
  import { appWindow, inTauri } from "../../lib/window";
  import Icon from "../Icon.svelte";
  import Logo from "../ui/Logo.svelte";

  interface Props {
    /** Aktiver Tab; null, wenn gerade die Einstellungen offen sind. */
    tab: Tab | null;
    settingsOpen: boolean;
    /** Laufende Downloads/Installationen. */
    activity: number;
    unread: boolean;
    customFrame: boolean;
    panel: "activity" | "notifications" | null;
    onTab: (tab: Tab) => void;
    onSettings: () => void;
    onPanel: (panel: "activity" | "notifications") => void;
  }
  let { tab, settingsOpen, activity, unread, customFrame, panel, onTab, onSettings, onPanel }: Props = $props();

  const tabs: Tab[] = ["home", "library", "discover", "events", "skins"];
  const showWindowButtons = $derived(customFrame && inTauri);

  function drag(e: MouseEvent) {
    // Nur auf freier Fläche ziehen, nicht auf Buttons.
    if (customFrame && e.target === e.currentTarget) appWindow.drag(e);
  }
</script>

<header class="bar" onmousedown={drag} role="presentation">
  <div class="brand" onmousedown={drag} role="presentation">
    <Logo size={26} wordmark />
  </div>
  <nav aria-label={t("nav.main")}>
    {#each tabs as id (id)}
      <button class="tab" class:active={tab === id} aria-current={tab === id ? "page" : undefined} onclick={() => onTab(id)}>
        {t(`tab.${id}`)}
      </button>
    {/each}
  </nav>
  <div class="spacer" onmousedown={drag} role="presentation"></div>
  <div class="tools">
    <button
      class="tool"
      class:open={panel === "activity"}
      aria-label={t("shell.activity")}
      title={t("shell.activity")}
      aria-expanded={panel === "activity"}
      onclick={() => onPanel("activity")}
    >
      <Icon name="download" size={19} />
      {#if activity > 0}<span class="count">{activity}</span>{/if}
    </button>
    <button
      class="tool"
      class:open={panel === "notifications"}
      aria-label={t("shell.notifications")}
      title={t("shell.notifications")}
      aria-expanded={panel === "notifications"}
      onclick={() => onPanel("notifications")}
    >
      <Icon name="bell" size={19} />
      {#if unread}<span class="dot"></span>{/if}
    </button>
    <button class="tool" class:open={settingsOpen} aria-label={t("nav.settings")} title={t("nav.settings")} onclick={onSettings}>
      <Icon name="settings" size={19} />
    </button>
  </div>
  {#if showWindowButtons}
    <div class="sep"></div>
    <div class="window">
      <button aria-label={t("shell.minimize")} onclick={appWindow.minimize}><Icon name="minimize" size={14} stroke={2} /></button>
      <button aria-label={t("shell.maximize")} onclick={appWindow.toggleMaximize}><Icon name="maximize" size={13} stroke={2} /></button>
      <button class="close" aria-label={t("shell.close")} onclick={appWindow.close}><Icon name="close" size={14} stroke={2} /></button>
    </div>
  {/if}
</header>

<style>
  .bar {
    display: flex;
    align-items: stretch;
    padding-left: 20px;
    background: var(--bar);
    border-bottom: 1px solid var(--line);
    min-width: 0;
  }

  .brand {
    display: flex;
    align-items: center;
    padding-right: 30px;
  }

  nav {
    display: flex;
    align-items: stretch;
    gap: 2px;
  }

  .tab {
    position: relative;
    min-height: 0;
    padding: 0 16px;
    background: transparent;
    font-family: var(--font-pixel);
    font-weight: 600;
    font-size: 17px;
    color: var(--text-muted);
  }

  .tab:hover:not(:disabled) {
    background: transparent;
    color: var(--text);
  }

  .tab.active {
    color: var(--text);
  }

  .tab.active::after {
    content: "";
    position: absolute;
    left: 10px;
    right: 10px;
    bottom: 0;
    height: 4px;
    background: var(--accent);
    clip-path: polygon(4px 0, calc(100% - 4px) 0, 100% 100%, 0 100%);
  }

  .spacer {
    flex: 1;
  }

  .tools {
    display: flex;
    align-items: center;
    gap: 2px;
    padding-right: 8px;
  }

  .tool {
    position: relative;
    width: 40px;
    height: 40px;
    min-height: 0;
    padding: 0;
    background: transparent;
    color: var(--text-muted);
  }

  .tool:hover:not(:disabled),
  .tool.open {
    background: var(--surface-2);
    color: var(--accent);
  }

  .dot {
    position: absolute;
    top: 9px;
    right: 9px;
    width: 7px;
    height: 7px;
    background: var(--accent);
  }

  .count {
    position: absolute;
    top: 5px;
    right: 4px;
    min-width: 14px;
    padding: 0 3px;
    font-family: var(--font-mono);
    font-size: 10px;
    line-height: 14px;
    background: var(--accent);
    color: var(--ink);
  }

  .sep {
    width: 1px;
    background: var(--line);
    margin: 14px 0;
  }

  .window {
    display: flex;
  }

  .window button {
    width: 46px;
    min-height: 0;
    padding: 0;
    background: transparent;
    color: var(--text-muted);
  }

  .window button:hover:not(:disabled) {
    background: var(--surface-2);
    color: var(--text);
  }

  .window .close:hover:not(:disabled) {
    background: #c42b1c;
    color: #fff;
  }
</style>
