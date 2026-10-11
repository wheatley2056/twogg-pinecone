<script lang="ts" module>
  // Standard-Gesicht (8 × 8), wenn kein Skin bekannt ist: Haare, Haut, Augen.
  const H = "#4a3020";
  const S = "#c69c6d";
  const D = "#a57d52";
  const E = "#f1e8da";
  const P = "#3a5a8c";
  const M = "#7a4a2e";
  export const DEFAULT_FACE = [
    [H, H, H, H, H, H, H, H],
    [H, H, H, H, H, H, H, H],
    [H, S, S, S, S, S, S, H],
    [S, S, S, S, S, S, S, S],
    [S, E, P, S, S, P, E, S],
    [S, S, S, D, D, S, S, S],
    [S, S, M, S, S, M, S, S],
    [S, S, M, M, M, M, S, S],
  ];
</script>

<script lang="ts">
  import SkinHead from "../SkinHead.svelte";

  // Pixel-Avatar: Kopf des Skins, sonst das Standard-Gesicht.
  let { skin = null, size = 40 }: { skin?: string | null; size?: number } = $props();
</script>

{#if skin}
  <span class="avatar"><SkinHead src={skin} {size} /></span>
{:else}
  <svg class="avatar" width={size} height={size} viewBox="0 0 8 8" shape-rendering="crispEdges" aria-hidden="true">
    {#each DEFAULT_FACE as row, y (y)}
      {#each row as fill, x (x)}
        <rect {x} {y} width="1" height="1" {fill} />
      {/each}
    {/each}
  </svg>
{/if}

<style>
  .avatar {
    display: inline-flex;
    flex: none;
  }

  .avatar :global(canvas) {
    border-radius: 0;
  }
</style>
