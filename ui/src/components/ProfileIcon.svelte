<script lang="ts">
  import type { Profile } from "../lib/types";

  let { profile, size = 30 }: { profile: Pick<Profile, "name" | "icon" | "color">; size?: number } = $props();

  // Ohne Symbol: Anfangsbuchstaben der ersten beiden Wörter.
  const label = $derived(
    profile.icon ||
      profile.name
        .split(/\s+/)
        .filter(Boolean)
        .slice(0, 2)
        .map((w) => [...w][0]?.toUpperCase() ?? "")
        .join("") ||
      "?",
  );
</script>

<span
  class="icon"
  class:colored={!!profile.color}
  style:width="{size}px"
  style:height="{size}px"
  style:font-size="{Math.round(size * (profile.icon ? 0.55 : 0.4))}px"
  style:--profile-color={profile.color}
  aria-hidden="true">{label}</span
>

<style>
  .icon {
    display: inline-grid;
    place-items: center;
    flex-shrink: 0;
    border-radius: 8px;
    background: var(--surface-3);
    color: var(--text);
    font-weight: 700;
    line-height: 1;
  }

  .colored {
    background: var(--profile-color);
    color: #fff;
    text-shadow: 0 1px 2px rgb(0 0 0 / 0.35);
  }
</style>
