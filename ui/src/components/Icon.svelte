<script lang="ts" module>
  // Strich-Symbole (24×24) nach dem Pinecone-Design-System; neuere Formen aus dem Prototyp.
  const PATHS = {
    home: "M3 11 12 4l9 7M5 10v10h5v-6h4v6h5V10",
    library: "M4 4h6v6H4zM14 4h6v6h-6zM4 14h6v6H4zM14 14h6v6h-6z",
    discover: "M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18zM15.5 8.5l-2 5-5 2 2-5z",
    settings: "M18.7 9.5L21.4 9.9L21.4 14.1L18.7 14.5L18.6 15L20.1 17.1L17.1 20.1L15 18.6L14.5 18.7L14.1 21.4L9.9 21.4L9.5 18.7L9 18.6L6.9 20.1L3.9 17.1L5.4 15L5.3 14.5L2.6 14.1L2.6 9.9L5.3 9.5L5.4 9L3.9 6.9L6.9 3.9L9 5.4L9.5 5.3L9.9 2.6L14.1 2.6L14.5 5.3L15 5.4L17.1 3.9L20.1 6.9L18.6 9zM15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0z",
    play: "M7 4v16l13-8z",
    stop: "M6 6h12v12H6z",
    more: "M5 12h.01M12 12h.01M19 12h.01",
    folder: "M3.5 6.5a1 1 0 0 1 1-1h5l2 2h8a1 1 0 0 1 1 1v10a1 1 0 0 1-1 1h-15a1 1 0 0 1-1-1z",
    edit: "M4 20h4L19 9l-4-4L4 16zM14 6l4 4",
    trash: "M4 7h16M9 7V4h6v3M6 7l1 13h10l1-13",
    plus: "M12 5v14M5 12h14",
    file: "M6 3h8l4 4v14H6zM14 3v4h4",
    refresh: "M20 11a8 8 0 0 0-14-5L4 8M4 4v4h4M4 13a8 8 0 0 0 14 5l2-2M20 20v-4h-4",
    search: "M17.5 11a6.5 6.5 0 1 1-13 0 6.5 6.5 0 0 1 13 0zM16 16l4 4",
    external: "M14 4h6v6M20 4l-9 9M18 14v6H4V6h6",
    download: "M12 4v11M7 10l5 5 5-5M5 20h14",
    image: "M4 5h16v14H4zM4 16l5-5 4 4 3-3 4 4M15 9h.01",
    shirt: "M8 3 3 6l2 4 2-1v12h10V9l2 1 2-4-5-3a4 4 0 0 1-8 0z",
    layers: "M12 3 3 8l9 5 9-5zM3 13l9 5 9-5M3 17.5l9 5 9-5",
    share: "M18 3a3 3 0 1 0 0 6 3 3 0 0 0 0-6zM6 9a3 3 0 1 0 0 6 3 3 0 0 0 0-6zM18 15a3 3 0 1 0 0 6 3 3 0 0 0 0-6zM8.6 10.7l6.8-3.4M8.6 13.3l6.8 3.4",
    link: "M10 14a4 4 0 0 0 5.7 0l3-3a4 4 0 0 0-5.7-5.7l-1 1M14 10a4 4 0 0 0-5.7 0l-3 3a4 4 0 0 0 5.7 5.7l1-1",
    check: "M5 12.5l4.5 4.5L19 7.5",
    sort: "M4 7h16M7 12h10M10 17h4",
    chevron: "M6 9l6 6 6-6",
    log: "M4 5h16M4 10h16M4 15h10M4 20h7",
    box: "M12 3l8 4.5v9L12 21l-8-4.5v-9zM4 7.5l8 4.5 8-4.5M12 12v9",
    close: "M6 6l12 12M18 6 6 18",
    bell: "M6 16V11a6 6 0 0 1 12 0v5l1.5 2h-15zM10 20.5h4",
    minimize: "M5 12h14",
    maximize: "M5 5h14v14H5z",
    restore: "M8 8h11v11H8zM5 16V5h11",
    lock: "M6 11h12a1 1 0 0 1 1 1v7a1 1 0 0 1-1 1H6a1 1 0 0 1-1-1v-7a1 1 0 0 1 1-1zM8 11V8a4 4 0 0 1 8 0v3",
    "arrow-up": "M12 19V7M7 12l5-5 5 5",
    "chevron-up": "M6 15l6-6 6 6",
    bolt: "M13 3 5 14h6l-1 7 8-11h-6z",
    chat: "M4 5h16v11H9l-5 4z",
    users: "M9 11a3.5 3.5 0 1 0 0-7 3.5 3.5 0 0 0 0 7zM3 20c0-3.3 2.7-6 6-6s6 2.7 6 6M16 4.5a3.5 3.5 0 0 1 0 6.5M18 14c2 .7 3 2.8 3 6",
    "user-plus": "M10 11a3.5 3.5 0 1 0 0-7 3.5 3.5 0 0 0 0 7zM3.5 20c0-3.3 2.9-6 6.5-6 1.2 0 2.3.3 3.2.8M19 14v6M16 17h6",
    calendar: "M4 6h16v14H4zM4 10h16M8 3v4M16 3v4",
    globe: "M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18zM3 12h18M12 3c2.5 2.5 3.5 5.5 3.5 9s-1 6.5-3.5 9c-2.5-2.5-3.5-5.5-3.5-9s1-6.5 3.5-9z",
    clock: "M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18zM12 7v5l3 2",
    logout: "M15 4h4v16h-4M10 8l-4 4 4 4M6 12h10",
    shield: "M12 3l7 3v5c0 5-3.5 8-7 10-3.5-2-7-5-7-10V6z",
    snowflake: "M12 3v18M4.2 7.5l15.6 9M4.2 16.5l15.6-9M9.5 4.5 12 7l2.5-2.5M9.5 19.5 12 17l2.5 2.5",
    waves: "M3 15c2 0 2-1.5 4.5-1.5S10 15 12 15s2.5-1.5 4.5-1.5S19 15 21 15M3 19c2 0 2-1.5 4.5-1.5S10 19 12 19s2.5-1.5 4.5-1.5S19 19 21 19M12 3v8M8 7l4-4 4 4",
    house: "M4 20h16M6 20V10l6-5 6 5v10M10 20v-5h4v5",
    sliders: "M4 7h10M18 7h2M4 17h4M12 17h8M18 7a2 2 0 1 1-4 0 2 2 0 0 1 4 0zM12 17a2 2 0 1 1-4 0 2 2 0 0 1 4 0z",
    copy: "M9 9h11v11H9zM5 15V4h11",
    cut: "M6 9a3 3 0 1 0 0-6 3 3 0 0 0 0 6zM6 21a3 3 0 1 0 0-6 3 3 0 0 0 0 6zM8.2 7.8 20 19.5M8.2 16.2 20 4.5",
    paste: "M9 3h6v3H9zM8 4.5H5V21h14V4.5h-3",
    server: "M5 4h14a1 1 0 0 1 1 1v5a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V5a1 1 0 0 1 1-1zM5 13h14a1 1 0 0 1 1 1v5a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1v-5a1 1 0 0 1 1-1zM8 7.5h.01M8 16.5h.01",
  } as const;

  export type IconName = keyof typeof PATHS;
</script>

<script lang="ts">
  let {
    name,
    size = 20,
    filled = false,
    stroke = 1.8,
  }: { name: IconName; size?: number; filled?: boolean; stroke?: number } = $props();
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill={filled ? "currentColor" : "none"}
  stroke="currentColor"
  stroke-width={name === "more" ? 3.5 : stroke}
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"><path d={PATHS[name]} /></svg
>
