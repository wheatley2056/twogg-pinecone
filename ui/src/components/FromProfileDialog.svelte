<script lang="ts">
  import { onMount } from "svelte";
  import { api, errorText } from "../lib/api";
  import { t } from "../lib/i18n.svelte";
  import type { InstalledContent, Profile } from "../lib/types";

  interface Props {
    profiles: Profile[];
    onClose: () => void;
    onPick: (profileId: string, keys: string[]) => void;
  }
  let { profiles, onClose, onPick }: Props = $props();

  let dialog: HTMLDialogElement;
  let profileId = $state("");
  let items = $state<InstalledContent[]>([]);
  let picked = $state<Set<string>>(new Set());
  let loading = $state(false);
  let error = $state<string | null>(null);

  onMount(() => {
    dialog.showModal();
    profileId = profiles[0]?.id ?? "";
  });

  $effect(() => {
    const id = profileId;
    if (!id) return;
    loading = true;
    error = null;
    picked = new Set();
    api
      .contentList(id)
      .then((list) => id === profileId && (items = list))
      .catch((e) => (error = errorText(e)))
      .finally(() => (loading = false));
  });

  function toggle(key: string) {
    const next = new Set(picked);
    if (next.has(key)) next.delete(key);
    else next.add(key);
    picked = next;
  }
</script>

<dialog bind:this={dialog} oncancel={(e) => (e.preventDefault(), onClose())}>
  <div class="dialog-body">
    <h2>{t("sync.fromProfileTitle")}</h2>
    <p class="muted">{t("sync.fromProfileHint")}</p>
    <label>
      {t("sync.profile")}
      <select bind:value={profileId}>
        {#each profiles as p (p.id)}<option value={p.id}>{p.name}</option>{/each}
      </select>
    </label>
    {#if loading}
      <p class="muted">{t("common.loading")}</p>
    {:else if items.length === 0}
      <p class="muted">{t("content.empty")}</p>
    {:else}
      <ul>
        {#each items as item (item.key)}
          <li>
            <label class="check">
              <input type="checkbox" checked={picked.has(item.key)} onchange={() => toggle(item.key)} />
              <span class="name">{item.name}</span>
              <span class="badge">{t(`content.${item.kind}`)}</span>
            </label>
          </li>
        {/each}
      </ul>
    {/if}
    {#if error}<p class="error">{error}</p>{/if}
  </div>
  <div class="dialog-actions">
    <button onclick={onClose}>{t("common.cancel")}</button>
    <button class="primary" disabled={picked.size === 0} onclick={() => onPick(profileId, [...picked])}>
      {t("sync.takeOver", { count: picked.size })}
    </button>
  </div>
</dialog>

<style>
  p {
    margin: 0;
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    max-height: 320px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .check {
    flex-direction: row;
    align-items: center;
    gap: 8px;
    padding: 6px 10px;
    border-radius: var(--radius);
    background: var(--surface-2);
    font-size: 13px;
    color: var(--text);
  }

  .check input {
    width: auto;
  }

  .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
