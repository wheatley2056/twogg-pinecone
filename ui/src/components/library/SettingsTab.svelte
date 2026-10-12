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
</style>
