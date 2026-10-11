<script lang="ts">
  // Pixel-Kiefernzapfen auf 9 × 13 Raster (Design-System Abschnitt 6).
  interface Props {
    /** Höhe in Pixeln; Breite folgt dem Raster. */
    size?: number;
    /** Wortmarke „Pinecone“ daneben. */
    wordmark?: boolean;
    /** Einfarbige Silhouette (z. B. auf Akzent). */
    mono?: boolean;
  }
  let { size = 26, wordmark = false, mono = false }: Props = $props();
  const width = $derived(Math.round((size * 9) / 13));
</script>

<span class="logo" style:--h="{size}px">
  <svg width={width} height={size} viewBox="0 0 9 13" shape-rendering="crispEdges" aria-hidden="true">
    {#if mono}
      <path
        fill="var(--ink)"
        d="M3 0h1v1H3z M5 0h1v1H5z M4 1h1v1H4z M3 2h3v1H3z M2 3h5v1H2z M1 4h7v1H1z M1 5h7v4H1z M0 6h1v2H0z M8 6h1v2H8z M2 9h5v1H2z M3 10h3v1H3z M3 11h3v1H3z M4 12h1v1H4z"
      />
    {:else}
      <path fill="#8CCB7E" d="M3 0h1v1H3z M5 0h1v1H5z M4 1h1v1H4z" />
      <path
        fill="#5A3416"
        d="M3 2h3v1H3z M2 3h1v1H2z M6 3h1v1H6z M1 4h1v1H1z M7 4h1v1H7z M0 6h1v2H0z M8 6h1v2H8z M1 9h1v1H1z M7 9h1v1H7z M2 10h1v1H2z M6 10h1v1H6z M3 11h1v1H3z M5 11h1v1H5z M4 12h1v1H4z"
      />
      <path fill="#E9A23B" d="M3 3h3v1H3z M2 4h5v1H2z M1 5h7v4H1z M2 9h5v1H2z M3 10h3v1H3z M4 11h1v1H4z" />
      <path
        fill="#A8641F"
        d="M4 3h1v1H4z M3 4h1v1H3z M5 4h1v1H5z M2 5h1v1H2z M6 5h1v1H6z M3 6h1v1H3z M5 6h1v1H5z M2 7h1v1H2z M6 7h1v1H6z M3 8h1v1H3z M5 8h1v1H5z M4 9h1v1H4z M4 10h1v1H4z"
      />
      <path fill="#F6C77A" d="M4 5h1v1H4z M4 7h1v1H4z" />
    {/if}
  </svg>
  {#if wordmark}<span class="word">Pinecone</span>{/if}
</span>

<style>
  .logo {
    display: inline-flex;
    align-items: center;
    gap: 10px;
  }

  .word {
    font-family: var(--font-pixel);
    font-weight: 600;
    font-size: calc(var(--h) * 0.77);
    letter-spacing: 0.5px;
    color: var(--text);
  }
</style>
