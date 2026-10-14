<script lang="ts">
  // Geführter Erststart (F-06, AK-06.4/06.5): Konto → TwoGG verknüpfen (überspringbar) →
  // Instanz oder Event. Der Schritt bleibt über Neustarts erhalten.
  import { onDestroy, onMount } from "svelte";
  import { api, errorText } from "../lib/api";
  import { loadTwogg, twogg } from "../lib/events.svelte";
  import { t } from "../lib/i18n.svelte";
  import type { Account, AppInfo } from "../lib/types";
  import Icon from "./Icon.svelte";
  import LoginDialog from "./LoginDialog.svelte";
  import Logo from "./ui/Logo.svelte";

  interface Props {
    info: AppInfo | null;
    accounts: Account[];
    onAccountsChange: () => void;
    onCreate: () => void;
    onEvents: () => void;
    onDiscover: () => void;
  }
  let { info, accounts, onAccountsChange, onCreate, onEvents, onDiscover }: Props = $props();

  const STORE_KEY = "pinecone.onboarding.step";
  function stored(): number {
    try {
      const n = Number(localStorage.getItem(STORE_KEY));
      return n >= 1 && n <= 3 ? n : 1;
    } catch {
      return 1;
    }
  }
  let step = $state(stored());
  function go(n: number) {
    step = n;
    try {
      localStorage.setItem(STORE_KEY, String(n));
    } catch {
      // Ohne Speicher beginnt der nächste Start wieder bei Schritt 1.
    }
  }

  const account = $derived(accounts.find((a) => a.active) ?? accounts[0] ?? null);
  let loggingIn = $state(false);
  let offlineName = $state("");
  const validName = $derived(/^[A-Za-z0-9_]{1,16}$/.test(offlineName));
  let error = $state<string | null>(null);
  let busy = $state(false);

  // Schritt 2: twogg.app (Discord öffnet ein eigenes Fenster und meldet sich per Ereignis zurück).
  let waiting = $state(false);
  let emailMode = $state(false);
  let email = $state("");
  let password = $state("");
  let unlisten: (() => void) | null = null;

  onMount(async () => {
    // Wer schon ein Konto hat, startet bei Schritt 2.
    if (step === 1 && accounts.length > 0) go(2);
    unlisten = await api
      .onAppLogin((r) => {
        waiting = false;
        if (r.error) error = r.error;
        else loadTwogg(true).then(() => go(3));
      })
      .catch(() => null);
  });
  onDestroy(() => unlisten?.());

  async function run(action: () => Promise<unknown>) {
    error = null;
    busy = true;
    try {
      await action();
    } catch (e) {
      error = errorText(e);
    } finally {
      busy = false;
    }
  }

  function addOffline(e: SubmitEvent) {
    e.preventDefault();
    if (!validName) return;
    run(async () => {
      await api.addOfflineAccount(offlineName);
      onAccountsChange();
      go(2);
    });
  }

  function discord() {
    waiting = true;
    run(() => api.appLoginDiscord()).then(() => error && (waiting = false));
  }

  function loginEmail(e: SubmitEvent) {
    e.preventDefault();
    run(async () => {
      await api.appLoginPassword(email, password);
      password = "";
      await loadTwogg(true);
      go(3);
    });
  }

  const steps = $derived([t("onboarding.stepAccount"), t("onboarding.stepTwogg"), t("onboarding.stepStart")]);
</script>

<div class="onboarding">
  <div class="intro">
    <Logo size={56} />
    <h1>{t("welcome.title")}</h1>
    <p>{t("onboarding.intro")}</p>
  </div>

  <ol class="steps" aria-label={t("onboarding.progress")}>
    {#each steps as label, i (i)}
      <li class:done={step > i + 1} class:current={step === i + 1} aria-current={step === i + 1 ? "step" : undefined}>
        <span class="num mono">{step > i + 1 ? "✓" : i + 1}</span>{label}
      </li>
    {/each}
  </ol>

  <section class="card">
    {#if step === 1}
      <h2>{t("onboarding.accountTitle")}</h2>
      <p class="hint">{t("onboarding.accountText")}</p>
      <div class="actions">
        <button class="primary cut" disabled={!info?.msaConfigured} title={info?.msaConfigured ? undefined : t("account.msaMissing")} onclick={() => (loggingIn = true)}>
          {t("account.loginMicrosoft")}
        </button>
      </div>
      <form class="offline" onsubmit={addOffline}>
        <label for="onb-offline">{t("onboarding.offline")}</label>
        <div class="row">
          <input id="onb-offline" bind:value={offlineName} placeholder={t("account.offlineName")} maxlength="16" spellcheck="false" />
          <button type="submit" disabled={!validName || busy}>{t("onboarding.offlineAdd")}</button>
        </div>
        {#if offlineName && !validName}<p class="hint">{t("account.nameRule")}</p>{/if}
      </form>
      <button class="link skip" onclick={() => go(2)}>{t("onboarding.later")}</button>
    {:else if step === 2}
      <h2>{t("onboarding.twoggTitle")}</h2>
      <p class="hint">{t("twoggLink.connectHint")}</p>
      {#if twogg.user}
        <p class="ok"><Icon name="check" size={16} stroke={2.4} />{t("onboarding.twoggDone", { name: twogg.user.username })}</p>
        <div class="actions"><button class="primary cut" onclick={() => go(3)}>{t("onboarding.next")}</button></div>
      {:else if emailMode}
        <form class="email" onsubmit={loginEmail}>
          <input type="email" bind:value={email} placeholder={t("twoggLink.emailPlaceholder")} aria-label={t("twoggLink.emailPlaceholder")} autocomplete="username" required />
          <input type="password" bind:value={password} placeholder={t("twoggLink.password")} aria-label={t("twoggLink.password")} autocomplete="current-password" required />
          <div class="actions">
            <button type="submit" class="primary cut" disabled={busy}>{busy ? t("common.loading") : t("twoggLink.login")}</button>
            <button type="button" onclick={() => (emailMode = false)}>{t("common.cancel")}</button>
          </div>
        </form>
      {:else}
        <div class="actions">
          <button class="primary cut" disabled={waiting} onclick={discord}>{waiting ? t("twoggLink.waiting") : t("twoggLink.discord")}</button>
          <button onclick={() => (emailMode = true)}>{t("twoggLink.email")}</button>
        </div>
      {/if}
      <div class="nav">
        <button class="link" onclick={() => go(1)}>{t("onboarding.back")}</button>
        {#if !twogg.user}<button class="link skip" onclick={() => go(3)}>{t("onboarding.skip")}</button>{/if}
      </div>
    {:else}
      <h2>{t("onboarding.startTitle")}</h2>
      <p class="hint">{account ? t("onboarding.startText", { name: account.name }) : t("onboarding.startNoAccount")}</p>
      <div class="choices">
        <button class="choice" onclick={onEvents}>
          <Icon name="calendar" size={22} />
          <span><strong>{t("onboarding.event")}</strong><span>{t("onboarding.eventText")}</span></span>
        </button>
        <button class="choice" onclick={onCreate}>
          <Icon name="plus" size={22} />
          <span><strong>{t("welcome.create")}</strong><span>{t("onboarding.createText")}</span></span>
        </button>
        <button class="choice" onclick={onDiscover}>
          <Icon name="discover" size={22} />
          <span><strong>{t("onboarding.discover")}</strong><span>{t("onboarding.discoverText")}</span></span>
        </button>
      </div>
      <div class="nav"><button class="link" onclick={() => go(2)}>{t("onboarding.back")}</button></div>
    {/if}
    {#if error}<p class="error">{error}</p>{/if}
  </section>
</div>

{#if loggingIn}
  <LoginDialog
    onClose={() => (loggingIn = false)}
    onDone={() => {
      loggingIn = false;
      onAccountsChange();
      go(2);
    }}
  />
{/if}

<style>
  .onboarding {
    box-sizing: border-box;
    width: 100%;
    max-width: 640px;
    margin: 0 auto;
    padding: 40px 24px;
    display: flex;
    flex-direction: column;
    gap: 22px;
  }

  .intro {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
    text-align: center;
  }

  .intro h1 {
    margin: 0;
    font-family: var(--font-pixel);
    font-size: 30px;
  }

  .intro p {
    margin: 0;
    color: var(--text-soft);
  }

  .steps {
    display: flex;
    justify-content: center;
    gap: 22px;
    margin: 0;
    padding: 0;
    list-style: none;
    font-size: 13px;
    color: var(--text-muted);
  }

  .steps li {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .num {
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    font-size: 12px;
    background: var(--surface-2);
    color: var(--text-muted);
  }

  .steps .current {
    color: var(--text);
    font-weight: 600;
  }

  .steps .current .num {
    background: var(--accent);
    color: var(--ink);
  }

  .steps .done .num {
    color: var(--nadel);
  }

  .card {
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 24px;
    background: var(--surface);
  }

  .card h2 {
    margin: 0;
    font-size: 18px;
  }

  .hint {
    margin: 0;
    font-size: 13.5px;
    color: var(--text-muted);
  }

  .actions,
  .row,
  .nav {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: center;
  }

  .nav {
    justify-content: space-between;
  }

  .offline,
  .email {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .offline label {
    font-size: 13px;
    color: var(--text-soft);
  }

  .row input {
    flex: 1;
    min-width: 180px;
  }

  .skip {
    align-self: flex-end;
  }

  .ok {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0;
    color: var(--nadel);
  }

  .choices {
    display: grid;
    gap: 8px;
  }

  .choice {
    display: flex;
    align-items: center;
    justify-content: flex-start;
    width: 100%;
    gap: 14px;
    padding: 14px 16px;
    text-align: left;
    background: var(--surface-2);
    min-height: 64px;
  }

  .choice :global(svg) {
    flex: none;
    color: var(--accent-fg);
  }

  .choice span {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .choice span span {
    font-size: 13px;
    font-weight: 400;
    color: var(--text-muted);
  }

  .error {
    margin: 0;
    color: var(--danger);
  }
</style>
