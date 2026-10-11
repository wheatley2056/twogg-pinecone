<script lang="ts">
  // 3D-Vorschau eines Skins (mit Cape) über skinview3d (MIT, github.com/bs-community/skinview3d).
  // Bewegung kommt aus lib/skinAnimations.ts: weiche Übergänge, Kopf folgt dem Mauszeiger.
  import { onDestroy, onMount } from "svelte";
  import type { SkinViewer } from "skinview3d";
  import { SkinAnimator, type AnimationId } from "../lib/skinAnimations";
  import type { SkinModel } from "../lib/types";

  interface Props {
    skin: string | null;
    model: SkinModel;
    cape?: string | null;
    width?: number;
    height?: number;
    animation?: AnimationId;
    /** Kamera langsam um die Figur drehen. */
    rotate?: boolean;
    /** Kleinere Ausschläge (Einstellung „Animationen reduzieren“). */
    calm?: boolean;
  }
  let { skin, model, cape = null, width = 260, height = 340, animation = "idle", rotate = false, calm = false }: Props = $props();

  let canvas: HTMLCanvasElement;
  let viewer: SkinViewer | null = $state(null);
  const animator = new SkinAnimator();

  onMount(async () => {
    // Erst hier laden: skinview3d bringt eine eigene three.js-Version mit.
    const { SkinViewer, FunctionAnimation } = await import("skinview3d");
    viewer = new SkinViewer({ canvas, width, height, zoom: 0.85 });
    viewer.animation = new FunctionAnimation((player, _progress, delta) => animator.apply(player, delta));
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

  $effect(() => {
    animator.play(animation);
  });

  $effect(() => {
    animator.intensity = calm ? 0.35 : 1;
  });

  $effect(() => {
    if (viewer) viewer.autoRotate = rotate && !calm;
  });

  function onPointerMove(e: PointerEvent) {
    // Bei gedrückter Taste dreht man die Kamera; dann schaut die Figur geradeaus.
    if (e.buttons) return animator.lookAt(null);
    const r = canvas.getBoundingClientRect();
    const x = ((e.clientX - r.left) / r.width) * 2 - 1;
    // Augenhöhe liegt im oberen Drittel der Vorschau.
    const y = ((e.clientY - r.top) / r.height - 0.3) * 2;
    animator.lookAt({ x: Math.max(-1, Math.min(1, x)), y: Math.max(-1, Math.min(1, y)) });
  }
</script>

<canvas
  bind:this={canvas}
  {width}
  {height}
  aria-label="3D-Vorschau"
  onpointermove={onPointerMove}
  onpointerleave={() => animator.lookAt(null)}
></canvas>

<style>
  canvas {
    display: block;
    cursor: grab;
  }

  canvas:active {
    cursor: grabbing;
  }
</style>
