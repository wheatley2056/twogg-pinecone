<script lang="ts" module>
  // Instanz-Motive mit Farbcodes aus dem Design-System (Abschnitt 3 und 7).
  export const MOTIFS = {
    shield: { bg: "#2a1f14", fg: "#e9a23b" },
    block: { bg: "#1e2a1c", fg: "#8ccb7e" },
    house: { bg: "#1b2232", fg: "#9fc3e8" },
    sliders: { bg: "#2b2817", fg: "#e6d27a" },
    waves: { bg: "#142329", fg: "#7fb8c9" },
    snowflake: { bg: "#1b2232", fg: "#9fc3e8" },
    server: { bg: "#1e2a1c", fg: "#8ccb7e" },
    bolt: { bg: "#1e2a1c", fg: "#8ccb7e" },
  } as const;

  export type Motif = keyof typeof MOTIFS;

  /** `motif:<name>` im Feld `icon` eines Profils wählt ein Motiv. */
  export function motifOf(icon: string | null | undefined): Motif | null {
    const name = icon?.startsWith("motif:") ? icon.slice(6) : null;
    return name && name in MOTIFS ? (name as Motif) : null;
  }
</script>

<script lang="ts">
  import Icon from "./Icon.svelte";
  import type { Profile } from "../lib/types";

  interface Props {
    profile: Pick<Profile, "name" | "icon">;
    size?: number;
    /** Größe des Schuppen-Schnitts; 0 = gerade Ecken. */
    cut?: number;
  }
  let { profile, size = 36, cut = 0 }: Props = $props();

  const motif = $derived(motifOf(profile.icon));
  // Ohne Motiv: Anfangsbuchstaben der ersten beiden Wörter.
  const initials = $derived(
    profile.name
      .split(/\s+/)
      .filter(Boolean)
      .slice(0, 2)
      .map((w) => [...w][0]?.toUpperCase() ?? "")
      .join("") || "?",
  );
  const glyph = $derived(Math.round(size * 0.52));
</script>

<span
  class="icon"
  style:width="{size}px"
  style:height="{size}px"
  style:background={motif ? MOTIFS[motif].bg : null}
  style:clip-path={cut ? `polygon(${cut}px 0, 100% 0, 100% 100%, 0 100%, 0 ${cut}px)` : null}
  aria-hidden="true"
>
  {#if motif === "shield"}
    <svg width={glyph} height={glyph} viewBox="0 0 24 24">
      <path d="M12 3l7 3v5c0 5-3.5 8-7 10-3.5-2-7-5-7-10V6z" fill={MOTIFS.shield.fg} />
      <path d="M12 7v10M8.5 11h7" stroke={MOTIFS.shield.bg} stroke-width="2" />
    </svg>
  {:else if motif === "block"}
    <svg width={glyph} height={glyph} viewBox="0 0 24 24">
      <path d="M12 3l8 4.5-8 4.5-8-4.5z" fill="#8CCB7E" />
      <path d="M4 7.5l8 4.5v9l-8-4.5z" fill="#7A5230" />
      <path d="M20 7.5l-8 4.5v9l8-4.5z" fill="#5E3E22" />
    </svg>
  {:else if motif}
    <span style:color={MOTIFS[motif].fg} style:display="contents"><Icon name={motif} size={glyph} /></span>
  {:else}
    <span class="initials" style:font-size="{Math.round(size * 0.36)}px">{initials}</span>
  {/if}
</span>

<style>
  .icon {
    display: inline-grid;
    place-items: center;
    flex-shrink: 0;
    background: var(--surface-2);
    line-height: 1;
  }

  .initials {
    font-family: var(--font-pixel);
    font-weight: 600;
    color: var(--text-soft);
  }
</style>
