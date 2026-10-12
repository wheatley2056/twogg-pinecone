<script lang="ts">
  import { api, errorText } from "../lib/api";
  import { t } from "../lib/i18n.svelte";
  import type { Account } from "../lib/types";
  import ConfirmDialog from "./ConfirmDialog.svelte";
  import Icon from "./Icon.svelte";
  import LoginDialog from "./LoginDialog.svelte";
  import PixelAvatar from "./ui/PixelAvatar.svelte";

  interface Props {
    accounts: Account[];
    msaConfigured: boolean;
    onChange: () => void;
  }
  let { accounts, msaConfigured, onChange }: Props = $props();

  let open = $state(false);
  let offlineName = $state("");
  let error: string | null = $state(null);
  let loggingIn = $state(false);
  let removing: Account | null = $state(null);
  let menu: HTMLElement | undefined = $state();
  // Skin-Kopf des aktiven Microsoft-Kontos (einmal je Konto geladen).
  let skins = $state<Record<string, string | null>>({});

  const active = $derived(accounts.find((a) => a.active) ?? null);
  const validName = $derived(/^[A-Za-z0-9_]{1,16}$/.test(offlineName));

  $effect(() => {
    const a = active;
    if (!a || a.kind !== "microsoft" || a.uuid in skins) return;
    skins[a.uuid] = null;
    api
      .skinAccount()
      .then((s) => (skins[a.uuid] = s.skinDataUrl))
      .catch(() => {});
  });

  /** Von außen öffnen (z. B. SPIELEN ohne Konto). */
  export function show() {
    open = true;
  }

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

  const kindLabel = (a: Account) => (a.kind === "microsoft" ? t("account.microsoftLong") : t("account.offlineLong"));
</script>

<svelte:window onclick={onWindowClick} />

<div class="account-menu" bind:this={menu}>
  <button class="trigger" onclick={() => (open = !open)} aria-expanded={open} aria-haspopup="menu">
    <PixelAvatar skin={active ? (skins[active.uuid] ?? null) : null} size={40} />
    <span class="who">
      <span class="name">{active?.name ?? t("account.none")}</span>
      <span class="kind">{active ? kindLabel(active) : t("account.addHint")}</span>
    </span>
    <Icon name="chevron-up" size={16} />
  </button>

  {#if open}
    <div class="popover">
      {#if accounts.length > 0}
        <ul>
          {#each accounts as a (a.uuid)}
            <li class:active={a.active}>
              <button class="pick" onclick={() => run(() => api.setActiveAccount(a.uuid))}>
                <PixelAvatar skin={skins[a.uuid] ?? null} size={28} />
                <span class="name">{a.name}</span>
                <span class="tag">{a.kind === "microsoft" ? t("account.microsoft") : t("account.offline")}</span>
              </button>
              <button class="ghost remove" aria-label={t("account.remove")} title={t("account.remove")} onclick={() => (removing = a)}>
                <Icon name="close" size={14} />
              </button>
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
      // Erst die UUID merken: `account` hängt an `removing` und ist danach null.
      const uuid = account.uuid;
      removing = null;
      run(() => api.removeAccount(uuid));
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
    gap: 12px;
    min-height: 56px;
    padding: 0 8px 0 0;
    background: transparent;
    color: var(--text-muted);
    text-align: left;
  }

  .trigger:hover:not(:disabled) {
    background: transparent;
    color: var(--text);
  }

  .who {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .who .name {
    font-size: 14px;
    font-weight: 700;
    color: var(--text);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 140px;
  }

  .kind {
    font-size: 12px;
    font-weight: 500;
    color: var(--text-dim);
  }

  .popover {
    position: absolute;
    right: 0;
    bottom: calc(100% + 14px);
    z-index: 20;
    width: 320px;
    background: var(--bar);
    border: 1px solid var(--line-strong);
    padding: 12px;
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
  }

  li.active {
    background: var(--surface-active);
    clip-path: var(--cut-1);
  }

  .pick {
    flex: 1;
    justify-content: flex-start;
    gap: 10px;
    min-height: 40px;
    padding: 0 8px;
    background: transparent;
    text-align: left;
    min-width: 0;
  }

  .pick .name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .remove {
    width: 32px;
    padding: 0;
  }

  form {
    display: flex;
    gap: 6px;
  }

  .hint {
    margin: 0;
    color: var(--text-dim);
    font-size: 12px;
  }
</style>
