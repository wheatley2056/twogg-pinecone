<script lang="ts">
  // Steuerung und Optionen aus der options.txt einer Instanz ins Sync-Pack übernehmen.
  import { onMount } from "svelte";
  import { api, errorText } from "../../lib/api";
  import { t } from "../../lib/i18n.svelte";
  import { controlName, isControl, keyLabel } from "../../lib/sync";
  import type { Profile } from "../../lib/types";
  import SearchField from "../ui/SearchField.svelte";
  import Segmented from "../ui/Segmented.svelte";

  interface Props {
    profiles: Profile[];
    onClose: () => void;
    onPick: (options: Record<string, string>) => void;
  }
  let { profiles, onClose, onPick }: Props = $props();

  type Group = "controls" | "options";

  let dialog: HTMLDialogElement;
  let profileId = $state("");
  let group = $state<Group>("controls");
  let query = $state("");
  let all = $state<Record<string, string>>({});
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
      .syncpackProfileOptions(id)
      .then((o) => id === profileId && (all = o))
      .catch((e) => (error = errorText(e)))
      .finally(() => (loading = false));
  });

  const q = $derived(query.trim().toLowerCase());
  const shown = $derived(
    Object.entries(all).filter(([k]) => isControl(k) === (group === "controls") && (!q || k.toLowerCase().includes(q))),
  );
  const allShownPicked = $derived(shown.length > 0 && shown.every(([k]) => picked.has(k)));

  function toggle(key: string) {
    const next = new Set(picked);
    if (next.has(key)) next.delete(key);
    else next.add(key);
    picked = next;
  }

  function toggleShown() {
    const next = new Set(picked);
    for (const [k] of shown) {
      if (allShownPicked) next.delete(k);
      else next.add(k);
    }
    picked = next;
  }

  const take = () => onPick(Object.fromEntries([...picked].map((k) => [k, all[k]])));
</script>

<dialog bind:this={dialog} oncancel={(e) => (e.preventDefault(), onClose())}>
  <div class="dialog-body">
    <h2>{t("sync.optionsFromTitle")}</h2>
    <p class="muted">{t("sync.optionsFromHint")}</p>
    <label>
      {t("sync.profile")}
      <select bind:value={profileId}>
        {#each profiles as p (p.id)}<option value={p.id}>{p.name}</option>{/each}
      </select>
    </label>
    <div class="bar">
      <Segmented
        items={[
          { id: "controls", label: t("sync.controls") },
          { id: "options", label: t("sync.generalOptions") },
        ]}
        value={group}
        label={t("sync.options")}
        variant="soft"
        onchange={(g) => (group = g as Group)}
      />
      <SearchField bind:value={query} placeholder={t("content.search")} width="100%" />
    </div>
    {#if loading}
      <p class="muted">{t("common.loading")}</p>
    {:else if Object.keys(all).length === 0}
      <p class="muted">{t("sync.optionsFromEmpty")}</p>
    {:else}
      <label class="check all">
        <input type="checkbox" checked={allShownPicked} onchange={toggleShown} disabled={shown.length === 0} />
        <span class="name">{t("sync.pickAll", { count: shown.length })}</span>
      </label>
      <ul>
        {#each shown as [k, v] (k)}
          <li>
            <label class="check">
              <input type="checkbox" checked={picked.has(k)} onchange={() => toggle(k)} />
              <span class="name mono">{isControl(k) ? controlName(k) : k}</span>
              <span class="value mono">{isControl(k) ? keyLabel(v) : v}</span>
            </label>
          </li>
        {/each}
      </ul>
    {/if}
    {#if error}<p class="error">{error}</p>{/if}
  </div>
  <div class="dialog-actions">
    <button onclick={onClose}>{t("common.cancel")}</button>
    <button class="primary" disabled={picked.size === 0} onclick={take}>
      {t("sync.takeOver", { count: picked.size })}
    </button>
  </div>
</dialog>

<style>
  p {
    margin: 0;
  }

  .bar {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 8px;
    align-items: center;
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
    background: var(--surface-2);
    font-size: 13px;
    color: var(--text);
  }

  .check.all {
    background: transparent;
    padding: 0 10px;
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

  .value {
    max-width: 40%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text-muted);
  }
</style>
