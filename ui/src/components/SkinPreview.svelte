<script lang="ts">
  // 3D-Vorschau eines Skins (mit Cape) über skinview3d (MIT, github.com/bs-community/skinview3d).
  import { onDestroy, onMount } from "svelte";
  import type { SkinViewer } from "skinview3d";
  import type { SkinModel } from "../lib/types";

  interface Props {
    skin: string | null;
    model: SkinModel;
    cape?: string | null;
    width?: number;
    height?: number;
  }
  let { skin, model, cape = null, width = 260, height = 340 }: Props = $props();

  let canvas: HTMLCanvasElement;
  let viewer: SkinViewer | null = $state(null);

  onMount(async () => {
    // Erst hier laden: skinview3d bringt eine eigene three.js-Version mit.
    const { SkinViewer, WalkingAnimation } = await import("skinview3d");
    viewer = new SkinViewer({ canvas, width, height, zoom: 0.85 });
    const walk = new WalkingAnimation();
    walk.speed = 0.6;
    viewer.animation = walk;
    viewer.autoRotate = true;
    viewer.autoRotateSpeed = 0.6;
  });

  onDestroy(() => viewer?.dispose());

  $effect(() => {
    if (!viewer) return;
    if (skin) viewer.loadSkin(skin, { model: model === "slim" ? "slim" : "default" });
    else viewer.loadSkin(null);
  });

  $effect(() => {
    if (!viewer) return;
    if (cape) viewer.loadCape(cape);
    else viewer.loadCape(null);
  });

  $effect(() => {
    viewer?.setSize(width, height);
  });
</script>

<canvas bind:this={canvas} {width} {height} aria-label="3D-Vorschau"></canvas>

<style>
  canvas {
    display: block;
    cursor: grab;
  }
</style>
