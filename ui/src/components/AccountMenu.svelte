<script lang="ts">
  import { api, errorText } from "../lib/api";
  import { t } from "../lib/i18n.svelte";
  import type { Account } from "../lib/types";
  import ConfirmDialog from "./ConfirmDialog.svelte";
  import LoginDialog from "./LoginDialog.svelte";

  interface Props {
    accounts: Account[];
    msaConfigured: boolean;
    onChange: () => void;
    /** Nur Avatar (Navigationsleiste); Menü öffnet nach rechts. */
    compact?: boolean;
  }
  let { accounts, msaConfigured, onChange, compact = false }: Props = $props();

  let open = $state(false);
  let offlineName = $state("");
  let error: string | null = $state(null);
  let loggingIn = $state(false);
  let removing: Account | null = $state(null);
  let menu: HTMLElement | undefined = $state();

  const active = $derived(accounts.find((a) => a.active) ?? null);
  const validName = $derived(/^[A-Za-z0-9_]{1,16}$/.test(offlineName));

  function onWindowClick(e: MouseEvent) {
    if (open && menu && !menu.contains(e.target as Node)) open = false;
  }

  async function run(action: () => Promise<unknown>) {
    error = null;
    try {
      await action();
      onChange();
    } catch (e) {
      error = errorText(e);
    }
  }

  async function addOffline(e: SubmitEvent) {
    e.preventDefault();
    if (!validName) return;
    await run(() => api.addOfflineAccount(offlineName));
    if (!error) offlineName = "";
  }
</script>

<svelte:window onclick={onWindowClick} />

<div class="account-menu" class:compact bind:this={menu}>
  {#if compact}
    <button
      class="trigger-compact"
      onclick={() => (open = !open)}
      aria-expanded={open}
      title={active ? `${active.name} (${active.kind === "microsoft" ? t("account.microsoft") : t("account.offline")})` : t("account.none")}
    >
      <span class="avatar big" class:empty={!active}>{active ? active.name[0].toUpperCase() : "?"}</span>
    </button>
  {:else}
  <button class="trigger" onclick={() => (open = !open)} aria-expanded={open}>
    {#if active}
      <span class="avatar">{active.name[0].toUpperCase()}</span>
      <span>{active.name}</span>
      <span class="badge">{active.kind === "microsoft" ? t("account.microsoft") : t("account.offline")}</span>
    {:else}
      <span>{t("account.none")}</span>
    {/if}
    <span class="chevron">▾</span>
  </button>
  {/if}

  {#if open}
    <div class="popover">
      {#if accounts.length > 0}
        <ul>
          {#each accounts as a (a.uuid)}
            <li class:active={a.active}>
              <button class="pick" onclick={() => run(() => api.setActiveAccount(a.uuid))}>
                <span class="avatar">{a.name[0].toUpperCase()}</span>
                <span class="name">{a.name}</span>
                <span class="badge">{a.kind === "microsoft" ? t("account.microsoft") : t("account.offline")}</span>
              </button>
              <button class="ghost remove" title={t("account.remove")} onclick={() => (removing = a)}>✕</button>
            </li>
          {/each}
        </ul>
      {/if}

      <button
        class="primary ms"
        disabled={!msaConfigured}
        onclick={() => {
          open = false;
          loggingIn = true;
        }}>{t("account.loginMicrosoft")}</button
      >
      {#if !msaConfigured}
        <p class="hint">{t("account.msaMissing")}</p>
      {/if}

      <form onsubmit={addOffline}>
        <input bind:value={offlineName} placeholder={t("account.offlineName")} maxlength="16" spellcheck="false" />
        <button type="submit" disabled={!validName}>{t("account.add")}</button>
      </form>
      {#if offlineName && !validName}
        <p class="hint">{t("account.nameRule")}</p>
      {/if}
      {#if error}<p class="error">{error}</p>{/if}
    </div>
  {/if}
</div>

{#if loggingIn}
  <LoginDialog
    onClose={() => (loggingIn = false)}
    onDone={() => {
      loggingIn = false;
      onChange();
    }}
  />
{/if}

{#if removing}
  {@const account = removing}
  <ConfirmDialog
    title={t("account.removeTitle")}
    confirmLabel={t("account.removeConfirm")}
    onCancel={() => (removing = null)}
    onConfirm={() => {
      removing = null;
      run(() => api.removeAccount(account.uuid));
    }}
  >
    <p>{t(account.kind === "microsoft" ? "account.removeTextMs" : "account.removeText", { name: account.name })}</p>
  </ConfirmDialog>
{/if}

<style>
  .account-menu {
    position: relative;
  }

  .trigger {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .chevron {
    color: var(--muted);
    font-size: 11px;
  }

  .avatar {
    width: 24px;
    height: 24px;
    border-radius: 6px;
    background: var(--accent);
    color: var(--accent-text);
    display: inline-grid;
    place-items: center;
    font-weight: 700;
    font-size: 12px;
    flex-shrink: 0;
  }

  .trigger-compact {
    border: none;
    background: transparent;
    padding: 6px;
    margin-top: 6px;
  }

  .avatar.big {
    width: 38px;
    height: 38px;
    font-size: 16px;
    border-radius: 10px;
  }

  .avatar.empty {
    background: var(--surface-3);
    color: var(--muted);
  }

  .compact .popover {
    left: calc(100% + 12px);
    right: auto;
    top: auto;
    bottom: 0;
  }

  .popover {
    position: absolute;
    right: 0;
    top: calc(100% + 6px);
    z-index: 10;
    width: 300px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 10px;
    box-shadow: var(--shadow);
    padding: 10px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  li {
    display: flex;
    align-items: center;
    border-radius: var(--radius);
  }

  li.active {
    background: var(--surface-3);
  }

  .pick {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 8px;
    border: none;
    background: transparent;
    text-align: left;
    min-width: 0;
  }

  .name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .remove {
    padding: 4px 8px;
    color: var(--muted);
  }

  form {
    display: flex;
    gap: 6px;
  }

  .hint {
    margin: 0;
    color: var(--muted);
    font-size: 12px;
  }
</style>
