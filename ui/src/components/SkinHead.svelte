<script lang="ts">
  // Kopf eines Skins (Gesicht + Hut-Ebene) als kleines Vorschaubild.
  let { src, size = 56 }: { src: string | null; size?: number } = $props();
  let canvas: HTMLCanvasElement;

  $effect(() => {
    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    ctx.clearRect(0, 0, size, size);
    if (!src) return;
    const img = new Image();
    img.onload = () => {
      ctx.imageSmoothingEnabled = false;
      // Gesicht bei (8,8), Hut-Ebene bei (40,8), jeweils 8×8 (bei 64 Pixel breiten Skins).
      const s = img.width / 64;
      ctx.drawImage(img, 8 * s, 8 * s, 8 * s, 8 * s, 0, 0, size, size);
      ctx.drawImage(img, 40 * s, 8 * s, 8 * s, 8 * s, 0, 0, size, size);
    };
    img.src = src;
  });
</script>

<canvas bind:this={canvas} width={size} height={size} aria-hidden="true"></canvas>

<style>
  canvas {
    image-rendering: pixelated;
    border-radius: 6px;
  }
</style>
