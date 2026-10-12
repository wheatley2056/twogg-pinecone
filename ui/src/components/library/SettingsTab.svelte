<script lang="ts">
  // Tab „Einstellungen“ der Instanz: Felder bearbeiten, Startkonto, Instanz löschen.
  import { onMount } from "svelte";
  import { api, errorText } from "../../lib/api";
  import { t } from "../../lib/i18n.svelte";
  import type { Account, Profile } from "../../lib/types";
  import ConfirmDialog from "../ConfirmDialog.svelte";
  import SettingRow from "../settings/SettingRow.svelte";
  import Dropdown from "../ui/Dropdown.svelte";
  import InstanceForm from "./InstanceForm.svelte";

  interface Props {
    profile: Profile;
    locked: boolean;
    onChanged: () => void;
    onDeleted: () => void;
  }
  let { profile, locked, onChanged, onDeleted }: Props = $props();

  let confirmDelete = $state(false);
  let error = $state<string | null>(null);
  let accounts = $state<Account[]>([]);
  // svelte-ignore state_referenced_locally
  let account = $state<string>(profile.account ?? "");

  onMount(() => {
    api.accounts().then((a) => (accounts = a)).catch(() => {});
  });

  // Gespeichertes Konto, das es nicht mehr gibt, bleibt als Eintrag sichtbar (AK-51.1).
  const accountOptions = $derived([
    { value: "", label: t("instance.accountActive") },
    ...accounts.map((a) => ({ value: a.uuid, label: `${a.name} · ${a.kind === "microsoft" ? "Microsoft" : t("account.offlineShort")}` })),
    ...(account && !accounts.some((a) => a.uuid === account) && accounts.length > 0 ? [{ value: account, label: t("instance.accountMissing") }] : []),
  ]);

  // svelte-ignore state_referenced_locally
  let hooks = $state({ pre: profile.preLaunch ?? "", wrapper: profile.wrapper ?? "", post: profile.postExit ?? "" });
  // svelte-ignore state_referenced_locally
  let savedHooks = $state(JSON.stringify(hooks));
  let hooksBusy = $state(false);
  let hooksError = $state<string | null>(null);
  const hooksDirty = $derived(JSON.stringify(hooks) !== savedHooks);
  let advanced = $state(!!(hooks.pre || hooks.wrapper || hooks.post));

  async function saveHooks(e: SubmitEvent) {
    e.preventDefault();
    hooksBusy = true;
    hooksError = null;
    try {
      await api.setProfileHooks(profile.id, hooks.pre, hooks.wrapper, hooks.post);
      savedHooks = JSON.stringify(hooks);
      onChanged();
    } catch (err) {
      hooksError = errorText(err);
    } finally {
      hooksBusy = false;
    }
  }

  async function setAccount(value: string) {
    const before = account;
    account = value;
    error = null;
    try {
      await api.setProfileAccount(profile.id, value || null);
      onChanged();
    } catch (e) {
      account = before;
      error = errorText(e);
    }
  }

  async function remove() {
    confirmDelete = false;
    try {
      await api.deleteProfile(profile.id);
      onDeleted();
    } catch (e) {
      error = errorText(e);
    }
  }
</script>

<div class="tab">
  <section class="block">
    <h3 class="section-label">{t("instance.general")}</h3>
    <InstanceForm {profile} {locked} id="instance-{profile.id}" onSaved={onChanged}>
      {#snippet actions({ saving, canSave })}
        <div class="actions">
          <button type="submit" class="primary cut" disabled={!canSave}>{saving ? t("dialog.saving") : t("common.save")}</button>
        </div>
      {/snippet}
    </InstanceForm>
  </section>

  <section class="block">
    <h3 class="section-label">{t("instance.launch")}</h3>
    <SettingRow title={t("instance.account")} hint={t("instance.accountHint")}>
      <Dropdown label="" value={account} options={accountOptions} align="right" disabled={locked} onchange={setAccount} />
    </SettingRow>
    <button type="button" class="ghost disclose" aria-expanded={advanced} onclick={() => (advanced = !advanced)}>
      {advanced ? "▾" : "▸"} {t("hooks.title")}
    </button>
    {#if advanced}
      <form class="hooks" onsubmit={saveHooks}>
        <p class="hint">{t("hooks.hint")}</p>
        <label>
          {t("hooks.pre")}
          <input class="mono" bind:value={hooks.pre} placeholder={t("hooks.prePlaceholder")} disabled={locked} spellcheck="false" />
        </label>
        <label>
          {t("hooks.wrapper")}
          <input class="mono" bind:value={hooks.wrapper} placeholder="gamemoderun" disabled={locked} spellcheck="false" />
        </label>
        <label>
          {t("hooks.post")}
          <input class="mono" bind:value={hooks.post} placeholder={t("hooks.postPlaceholder")} disabled={locked} spellcheck="false" />
        </label>
        <p class="hint">{t("hooks.placeholders")}</p>
        {#if hooksError}<p class="error">{hooksError}</p>{/if}
        <div class="actions">
          <button type="submit" class="primary cut" disabled={locked || hooksBusy || !hooksDirty}>{hooksBusy ? t("dialog.saving") : t("common.save")}</button>
        </div>
      </form>
    {/if}
  </section>

  <section class="block danger-zone">
    <h3 class="section-label">{t("instance.dangerZone")}</h3>
    <div class="danger-row">
      <span>
        <span class="title">{t("profile.delete")}</span>
        <span class="hint">{t("instance.deleteHint")}</span>
      </span>
      <button class="danger" disabled={locked} onclick={() => (confirmDelete = true)}>{t("profile.delete")}</button>
    </div>
    {#if error}<p class="error">{error}</p>{/if}
  </section>
</div>

{#if confirmDelete}
  <ConfirmDialog title={t("profile.deleteTitle")} confirmLabel={t("profile.delete")} onCancel={() => (confirmDelete = false)} onConfirm={remove}>
    <p>{t("profile.deleteText", { name: profile.name })}</p>
  </ConfirmDialog>
{/if}

<style>
  .tab {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 18px 28px 24px;
    display: flex;
    flex-direction: column;
    gap: 24px;
    max-width: 900px;
  }

  .block {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  h3 {
    margin: 0;
    font-weight: 400;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
  }

  .danger-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 14px;
    background: var(--surface);
    clip-path: var(--cut-1);
  }

  .danger-row > span {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .title {
    font-weight: 700;
  }

  .hint {
    font-size: 12px;
    color: var(--text-muted);
  }

  .disclose {
    align-self: flex-start;
    font-size: 13px;
    color: var(--text-soft);
  }

  .hooks {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 14px;
    background: var(--surface);
    clip-path: var(--cut-1);
  }

  .hooks p {
    margin: 0;
  }

  .hooks input {
    background: var(--surface-2);
    font-family: var(--font-mono);
    font-size: 13px;
  }
</style>
