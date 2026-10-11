<script lang="ts" module>
  // Einfache Strich-Symbole (24×24, eigene Zeichnung).
  const PATHS = {
    home: "M3 11 12 4l9 7M5 10v10h5v-6h4v6h5V10",
    library: "M4 4h6v6H4zM14 4h6v6h-6zM4 14h6v6H4zM14 14h6v6h-6z",
    discover: "M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18zM15.5 8.5l-2 5-5 2 2-5z",
    settings: "M12 9a3 3 0 1 0 0 6 3 3 0 0 0 0-6zM12 2v3M12 19v3M4.2 4.2l2.1 2.1M17.7 17.7l2.1 2.1M2 12h3M19 12h3M4.2 19.8l2.1-2.1M17.7 6.3l2.1-2.1",
    play: "M7 4v16l13-8z",
    stop: "M6 6h12v12H6z",
    more: "M5 12h.01M12 12h.01M19 12h.01",
    folder: "M3 6h6l2 2h10v11H3z",
    edit: "M4 20h4L19 9l-4-4L4 16zM14 6l4 4",
    trash: "M4 7h16M9 7V4h6v3M6 7l1 13h10l1-13",
    plus: "M12 5v14M5 12h14",
    file: "M6 3h8l4 4v14H6zM14 3v4h4",
    refresh: "M20 11a8 8 0 0 0-14-5L4 8M4 4v4h4M4 13a8 8 0 0 0 14 5l2-2M20 20v-4h-4",
    search: "M11 4a7 7 0 1 0 0 14 7 7 0 0 0 0-14zM20 20l-4-4",
    external: "M14 4h6v6M20 4l-9 9M18 14v6H4V6h6",
    download: "M12 4v12M7 11l5 5 5-5M5 20h14",
    shirt: "M8 3 3 6l2 4 2-1v12h10V9l2 1 2-4-5-3a4 4 0 0 1-8 0z",
    layers: "M12 3 3 8l9 5 9-5zM3 13l9 5 9-5M3 17.5l9 5 9-5",
    share: "M18 3a3 3 0 1 0 0 6 3 3 0 0 0 0-6zM6 9a3 3 0 1 0 0 6 3 3 0 0 0 0-6zM18 15a3 3 0 1 0 0 6 3 3 0 0 0 0-6zM8.6 10.7l6.8-3.4M8.6 13.3l6.8 3.4",
    link: "M10 14a4 4 0 0 0 5.7 0l3-3a4 4 0 0 0-5.7-5.7l-1 1M14 10a4 4 0 0 0-5.7 0l-3 3a4 4 0 0 0 5.7 5.7l1-1",
    check: "M5 12l5 5 9-10",
    chevron: "M6 9l6 6 6-6",
    log: "M4 5h16M4 10h16M4 15h10M4 20h7",
    box: "M12 3l8 4.5v9L12 21l-8-4.5v-9zM4 7.5l8 4.5 8-4.5M12 12v9",
    close: "M6 6l12 12M18 6 6 18",
  } as const;

  export type IconName = keyof typeof PATHS;
</script>

<script lang="ts">
  let { name, size = 20, filled = false }: { name: IconName; size?: number; filled?: boolean } = $props();
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill={filled ? "currentColor" : "none"}
  stroke="currentColor"
  stroke-width={name === "more" ? 3.5 : 1.8}
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"><path d={PATHS[name]} /></svg
>
