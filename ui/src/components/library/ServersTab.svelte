<script lang="ts">
  // Tab „Server“: gespeicherte Server der Instanz (servers.dat) mit Status und Schnellbeitritt.
  import { onMount } from "svelte";
  import { api, errorText } from "../../lib/api";
  import { t } from "../../lib/i18n.svelte";
  import type { Profile, SavedServer, ServerStatus } from "../../lib/types";
  import Icon from "../Icon.svelte";

  let { profile, onJoin }: { profile: Profile; onJoin: (address: string) => void } = $props();

  let servers = $state<SavedServer[] | null>(null);
  let status = $state<Record<string, ServerStatus | "offline" | undefined>>({});
  let error = $state<string | null>(null);

  onMount(async () => {
    try {
      servers = await api.savedServers(profile.id);
    } catch (e) {
      error = errorText(e);
      servers = [];
    }
    for (const s of servers) {
      api
        .pingServer(s.address)
        .then((st) => (status[s.address] = st))
        .catch(() => (status[s.address] = "offline"));
    }
  });
</script>

<div class="tab">
  {#if error}<p class="error">{error}</p>{/if}
  {#if servers === null}
    <p class="empty">{t("common.loading")}</p>
  {:else if servers.length === 0}
    <p class="empty">{t("servers.empty")}</p>
  {:else}
    <div class="list">
      {#each servers as s (s.address + s.name)}
        {@const st = status[s.address]}
        <div class="row">
          {#if s.icon}
            <img class="icon" src={`data:image/png;base64,${s.icon}`} alt="" width="48" height="48" />
          {:else}
            <span class="icon placeholder"><Icon name="server" size={22} /></span>
          {/if}
          <span class="text">
            <span class="name">{s.name}</span>
            <span class="address mono">{s.address}</span>
          </span>
          <span class="state">
            {#if st && st !== "offline"}
              <span class="ok"><span class="square"></span>{t("servers.players", { online: st.online, max: st.max })}</span>
              <span class="ping mono">{st.pingMs} ms</span>
            {:else if st === "offline"}
              <span class="off"><span class="square off"></span>{t("home.serverOffline")}</span>
            {:else}
              <span class="off">{t("home.serverChecking")}</span>
            {/if}
          </span>
          <button class="primary small" onclick={() => onJoin(s.address)}>{t("servers.join")}</button>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .tab {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 16px 28px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .list {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 10px 12px;
    background: var(--surface);
    clip-path: var(--cut-1);
  }

  .icon {
    width: 48px;
    height: 48px;
    flex: none;
    image-rendering: pixelated;
  }

  .placeholder {
    display: grid;
    place-items: center;
    background: #1e2a1c;
    color: var(--nadel);
  }

  .text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .name {
    font-size: 15px;
    font-weight: 700;
  }

  .address {
    font-size: 12px;
    color: var(--text-muted);
  }

  .state {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 2px;
    font-size: 12px;
    min-width: 120px;
  }

  .ok,
  .off {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .ok {
    color: var(--nadel);
  }

  .off {
    color: var(--text-dim);
  }

  .square {
    width: 7px;
    height: 7px;
    background: var(--nadel);
  }

  .square.off {
    background: var(--text-dim);
  }

  .ping {
    font-size: 11px;
    color: var(--text-dim);
  }

  .small {
    min-height: 36px;
    font-size: 13px;
  }

  .empty {
    color: var(--text-dim);
  }
</style>
