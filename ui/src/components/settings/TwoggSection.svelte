<script lang="ts">
  // Einstellungen › TwoGG-Verknüpfung: twogg.app-Konto (Discord oder E-Mail), eigene Anmeldungen.
  import { onDestroy, onMount } from "svelte";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { api, errorText } from "../../lib/api";
  import { loadTwogg, twogg } from "../../lib/events.svelte";
  import { t } from "../../lib/i18n.svelte";
  import Icon from "../Icon.svelte";
  import SettingRow from "./SettingRow.svelte";

  let waiting = $state(false);
  let emailMode = $state(false);
  let email = $state("");
  let password = $state("");
  let busy = $state(false);
  let error = $state<string | null>(null);
  let unlisten: (() => void) | null = null;

  onMount(async () => {
    loadTwogg(true);
    unlisten = await api
      .onAppLogin((r) => {
        waiting = false;
        if (r.error) error = r.error;
        else loadTwogg(true);
      })
      .catch(() => null);
  });
  onDestroy(() => unlisten?.());

  async function discord() {
    error = null;
    waiting = true;
    try {
      await api.appLoginDiscord();
    } catch (e) {
      waiting = false;
      error = errorText(e);
    }
  }

  async function loginEmail(e: SubmitEvent) {
    e.preventDefault();
    busy = true;
    error = null;
    try {
      await api.appLoginPassword(email, password);
      password = "";
      emailMode = false;
      await loadTwogg(true);
    } catch (err) {
      error = errorText(err);
    } finally {
      busy = false;
    }
  }

  async function logout() {
    busy = true;
    try {
      await api.appLogout();
      await loadTwogg(true);
    } catch (e) {
      error = errorText(e);
    } finally {
      busy = false;
    }
  }

  const eventTitle = (id: string) => twogg.events?.find((e) => e.id === id)?.title ?? id;
  const kindLabel = (k: string) => t(`twoggLink.kind.${k as "registered"}`);
</script>

<div class="head">
  <h1>{t("settings.twogg")}</h1>
  <p class="lead">{t("twoggLink.lead")}</p>
</div>

{#if twogg.user}
  {@const user = twogg.user}
  <section>
    <h3 class="section-label">{t("twoggLink.account")}</h3>
    <div class="account">
      {#if user.avatarUrl?.startsWith("https://")}
        <img class="avatar" src={user.avatarUrl} alt="" width="56" height="56" />
      {:else}
        <span class="avatar letter">{user.username[0]?.toUpperCase()}</span>
      {/if}
      <span class="who">
        <span class="name">{user.username}</span>
        <span class="chips">
          <span class="chip ok">{t("twoggLink.connected")}</span>
          {#if user.mcUsername}<span class="chip">Minecraft: {user.mcUsername}</span>{/if}
        </span>
      </span>
      <button class="link" onclick={() => openUrl("https://twogg.app/profile").catch(() => {})}>{t("twoggLink.profile")} <Icon name="external" size={14} /></button>
      <button disabled={busy} onclick={logout}>{t("twoggLink.logout")}</button>
    </div>
  </section>

  <section>
    <h3 class="section-label">{t("twoggLink.myEvents")}</h3>
    <div class="rows">
      {#each twogg.participations as p (p.eventId + p.kind)}
        <SettingRow title={eventTitle(p.eventId)} hint={[kindLabel(p.kind), p.teamName, p.status].filter(Boolean).join(" · ")} />
      {:else}
        <p class="hint">{t("twoggLink.noEvents")}</p>
      {/each}
    </div>
  </section>
{:else}
  <section>
    <h3 class="section-label">{t("twoggLink.account")}</h3>
    <div class="connect scales">
      <span class="connect-text">
        <span class="connect-title">{t("twoggLink.connectTitle")}</span>
        <span class="hint">{t("twoggLink.connectHint")}</span>
      </span>
      <div class="connect-actions">
        <button class="primary cut" disabled={waiting} onclick={discord}>
          {waiting ? t("twoggLink.waiting") : t("twoggLink.discord")}
        </button>
        <button class="ghost" onclick={() => (emailMode = !emailMode)}>{t("twoggLink.email")}</button>
      </div>
    </div>
    {#if waiting}<p class="hint">{t("twoggLink.waitingHint")}</p>{/if}
    {#if emailMode}
      <form class="email" onsubmit={loginEmail}>
        <input type="email" bind:value={email} placeholder={t("twoggLink.emailPlaceholder")} autocomplete="username" aria-label={t("twoggLink.emailPlaceholder")} />
        <input type="password" bind:value={password} placeholder={t("twoggLink.password")} autocomplete="current-password" aria-label={t("twoggLink.password")} />
        <button class="primary" disabled={busy || !email.trim() || !password}>{t("twoggLink.login")}</button>
      </form>
    {/if}
  </section>
{/if}

<section>
  <h3 class="section-label">{t("twoggLink.what")}</h3>
  <div class="rows">
    <SettingRow title={t("twoggLink.whatEvents")} hint={t("twoggLink.whatEventsHint")} />
    <SettingRow title={t("twoggLink.whatServer")} hint={t("twoggLink.whatServerHint")} />
    <SettingRow title={t("twoggLink.whatFriends")} hint={t("twoggLink.whatFriendsHint")} />
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
    max-width: 640px;
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

  .account,
  .connect {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 16px;
    background: var(--surface);
    clip-path: var(--cut-1);
  }

  .avatar {
    width: 56px;
    height: 56px;
    flex: none;
    object-fit: cover;
  }

  .letter {
    display: grid;
    place-items: center;
    background: var(--surface-2);
    font-family: var(--font-pixel);
    font-size: 24px;
  }

  .who {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .name {
    font-family: var(--font-pixel);
    font-size: 22px;
    font-weight: 700;
  }

  .chips {
    display: flex;
    gap: 6px;
  }

  .chips .chip {
    padding: 4px 8px;
  }

  .link {
    font-size: 13px;
  }

  .connect {
    background: color-mix(in srgb, var(--accent) 7%, var(--stage));
    --scales-opacity: 0.08;
  }

  .connect-text,
  .connect-actions {
    position: relative;
  }

  .connect-text {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .connect-title {
    font-family: var(--font-pixel);
    font-size: 18px;
    font-weight: 600;
  }

  .connect-actions {
    display: flex;
    gap: 8px;
  }

  .connect-actions .primary {
    height: 42px;
  }

  .email {
    display: grid;
    grid-template-columns: 1fr 1fr auto;
    gap: 8px;
  }

  .rows {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .hint {
    margin: 0;
    font-size: 12px;
    color: var(--text-muted);
  }
</style>
