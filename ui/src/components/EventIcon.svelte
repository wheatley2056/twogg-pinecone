<script lang="ts">
  // Event-Symbol: Logo von twogg.app, sonst ein Motiv aus dem Design-System.
  import { eventMotif } from "../lib/events.svelte";
  import type { AppEvent } from "../lib/types";
  import ProfileIcon from "./ProfileIcon.svelte";

  let { event, size = 38 }: { event: AppEvent; size?: number } = $props();
  let failed = $state(false);
</script>

{#if event.logoUrl && !failed}
  <span class="logo" style:width="{size}px" style:height="{size}px">
    <img src={event.logoUrl} alt="" width={size - 6} height={size - 6} onerror={() => (failed = true)} />
  </span>
{:else}
  <ProfileIcon profile={{ name: event.title, icon: `motif:${eventMotif(event)}` }} {size} />
{/if}

<style>
  .logo {
    display: inline-grid;
    place-items: center;
    flex: none;
    background: var(--surface-2);
  }

  img {
    object-fit: contain;
  }
</style>
