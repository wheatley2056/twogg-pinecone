<script lang="ts">
  // Einstellungen › Konten: Minecraft-Konten (Microsoft oder offline) verwalten.
  import { api, errorText } from "../../lib/api";
  import { t } from "../../lib/i18n.svelte";
  import type { Account, AppInfo } from "../../lib/types";
  import ConfirmDialog from "../ConfirmDialog.svelte";
  import LoginDialog from "../LoginDialog.svelte";
  import PixelAvatar from "../ui/PixelAvatar.svelte";

  let { accounts, info, onChange }: { accounts: Account[]; info: AppInfo | null; onChange: () => void } = $props();

  let offlineName = $state("");
  let error = $state<string | null>(null);
  let loggingIn = $state(false);
  let removing = $state<Account | null>(null);
  let deletingFriends = $state(false);
  let friendsNotice = $state<string | null>(null);
  const valid = $derived(/^[A-Za-z0-9_]{1,16}$/.test(offlineName));

  async function run(action: () => Promise<unknown>) {
    error = null;
    try {
      await action();
      onChange();
    } catch (e) {
      error = errorText(e);
    }
  }
</script>

<div class="head">
  <h1>{t("settings.accounts")}</h1>
  <p class="lead">{t("settings.accountsLead")}</p>
</div>

<section>
  <h3 class="section-label">{t("settings.accountsList")}</h3>
  <div class="rows">
    {#each accounts as a (a.uuid)}
      <div class="account" class:active={a.active}>
        <PixelAvatar size={40} />
        <span class="who">
          <span class="name">{a.name}</span>
          <span class="kind">{a.kind === "microsoft" ? t("account.microsoftLong") : t("account.offlineLong")}</span>
        </span>
        {#if a.active}
          <span class="chip ok">{t("settings.accountActive")}</span>
        {:else}
          <button class="small" onclick={() => run(() => api.setActiveAccount(a.uuid))}>{t("settings.accountUse")}</button>
        {/if}
        <button class="ghost small danger" onclick={() => (removing = a)}>{t("account.remove")}</button>
      </div>
    {:else}
      <p class="hint">{t("account.none")}</p>
    {/each}
  </div>
</section>

<section>
  <h3 class="section-label">{t("settings.accountsAdd")}</h3>
  <div class="add">
    <button class="primary cut" disabled={!info?.msaConfigured} onclick={() => (loggingIn = true)}>{t("account.loginMicrosoft")}</button>
    <form class="offline" onsubmit={(e) => (e.preventDefault(), valid && run(async () => (await api.addOfflineAccount(offlineName), (offlineName = ""))))}>
      <input bind:value={offlineName} placeholder={t("account.offlineName")} maxlength="16" spellcheck="false" aria-label={t("account.offlineName")} />
      <button disabled={!valid}>{t("account.add")}</button>
    </form>
  </div>
  {#if !info?.msaConfigured}<p class="hint">{t("account.msaMissing")}</p>{/if}
  {#if offlineName && !valid}<p class="hint">{t("account.nameRule")}</p>{/if}
  {#if error}<p class="error">{error}</p>{/if}
</section>

<section>
  <h3 class="section-label">{t("friends.dataTitle")}</h3>
  <p class="hint">{t("friends.dataHint")}</p>
  <div class="add">
    <button class="ghost danger" onclick={() => (deletingFriends = true)}>{t("friends.dataDelete")}</button>
  </div>
  {#if friendsNotice}<p class="hint">{friendsNotice}</p>{/if}
</section>

{#if deletingFriends}
  <ConfirmDialog
    title={t("friends.dataDeleteTitle")}
    confirmLabel={t("friends.dataDelete")}
    onCancel={() => (deletingFriends = false)}
    onConfirm={async () => {
      deletingFriends = false;
      friendsNotice = null;
      await run(async () => {
        await api.friendsDeleteData();
        friendsNotice = t("friends.dataDeleted");
      });
    }}
  >
    <p>{t("friends.dataDeleteText")}</p>
  </ConfirmDialog>
{/if}

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

  .account {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 10px 14px;
    background: var(--surface);
  }

  .account.active {
    box-shadow: inset 0 -3px 0 var(--accent);
  }

  .who {
    flex: 1;
    display: flex;
    flex-direction: column;
  }

  .name {
    font-weight: 700;
  }

  .kind {
    font-size: 12px;
    color: var(--text-dim);
  }

  .small {
    min-height: 32px;
    font-size: 13px;
  }

  .add {
    display: flex;
    gap: 12px;
    align-items: center;
    flex-wrap: wrap;
  }

  .add .primary {
    height: 40px;
  }

  .offline {
    display: flex;
    gap: 6px;
  }

  .offline input {
    width: 220px;
  }

  .hint {
    margin: 0;
    font-size: 12px;
    color: var(--text-dim);
  }
</style>
