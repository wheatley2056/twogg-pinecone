<script lang="ts">
  import { onMount } from 'svelte';
  import { skinStore } from '../lib/stores/skin';
  import { applyToolAt, applyToolAlongLine, beginStroke, endStroke } from '../lib/paintController';

  const CELL = 10; // device px per skin pixel

  let canvasEl: HTMLCanvasElement;
  let ctx: CanvasRenderingContext2D | null = null;
  let painting = false;
  let lastPixel: { x: number; y: number } | null = null;

  function draw() {
    if (!ctx) return;
    const { width, height, pixels } = $skinStore;
    const w = width * CELL;
    const h = height * CELL;
    if (canvasEl.width !== w) canvasEl.width = w;
    if (canvasEl.height !== h) canvasEl.height = h;

    const checkSize = CELL / 2;
    for (let y = 0; y < h; y += checkSize) {
      for (let x = 0; x < w; x += checkSize) {
        const even = (Math.floor(x / checkSize) + Math.floor(y / checkSize)) % 2 === 0;
        ctx.fillStyle = even ? '#d8d8d8' : '#aeaeae';
        ctx.fillRect(x, y, checkSize, checkSize);
      }
    }

    for (let y = 0; y < height; y++) {
      for (let x = 0; x < width; x++) {
        const i = (y * width + x) * 4;
        const a = pixels[i + 3];
        if (a === 0) continue;
        ctx.fillStyle = `rgba(${pixels[i]},${pixels[i + 1]},${pixels[i + 2]},${a / 255})`;
        ctx.fillRect(x * CELL, y * CELL, CELL, CELL);
      }
    }

    ctx.strokeStyle = 'rgba(110,110,110,0.6)';
    ctx.lineWidth = 1;
    ctx.beginPath();
    for (let x = 0; x <= width; x++) {
      ctx.moveTo(x * CELL + 0.5, 0);
      ctx.lineTo(x * CELL + 0.5, h);
    }
    for (let y = 0; y <= height; y++) {
      ctx.moveTo(0, y * CELL + 0.5);
      ctx.lineTo(w, y * CELL + 0.5);
    }
    ctx.stroke();
  }

  $: if (ctx && $skinStore) draw();

  onMount(() => {
    ctx = canvasEl.getContext('2d');
    draw();
  });

  function eventToPixel(e: PointerEvent): { x: number; y: number } | null {
    const rect = canvasEl.getBoundingClientRect();
    const scaleX = canvasEl.width / rect.width;
    const scaleY = canvasEl.height / rect.height;
    const px = Math.floor(((e.clientX - rect.left) * scaleX) / CELL);
    const py = Math.floor(((e.clientY - rect.top) * scaleY) / CELL);
    if (px < 0 || py < 0 || px >= $skinStore.width || py >= $skinStore.height) return null;
    return { x: px, y: py };
  }

  function onPointerDown(e: PointerEvent) {
    const p = eventToPixel(e);
    if (!p) return;
    canvasEl.setPointerCapture(e.pointerId);
    painting = true;
    beginStroke();
    applyToolAt(p.x, p.y);
    lastPixel = p;
  }

  function onPointerMove(e: PointerEvent) {
    if (!painting) return;
    const p = eventToPixel(e);
    if (!p) return;
    if (lastPixel) applyToolAlongLine(lastPixel.x, lastPixel.y, p.x, p.y);
    else applyToolAt(p.x, p.y);
    lastPixel = p;
  }

  function onPointerUp(e: PointerEvent) {
    if (!painting) return;
    painting = false;
    lastPixel = null;
    endStroke();
    try {
      canvasEl.releasePointerCapture(e.pointerId);
    } catch {
      // already released
    }
  }
</script>

<div class="canvas2d-wrap">
  <canvas
    bind:this={canvasEl}
    style="touch-action: none;"
    on:pointerdown={onPointerDown}
    on:pointermove={onPointerMove}
    on:pointerup={onPointerUp}
    on:pointercancel={onPointerUp}
    on:pointerleave={onPointerUp}
  ></canvas>
</div>

<style>
  .canvas2d-wrap {
    overflow: auto;
    width: 100%;
    height: 100%;
    background: var(--panel-bg-dark, #1e1e1e);
    border-radius: 6px;
    padding: 8px;
    box-sizing: border-box;
  }
  canvas {
    image-rendering: pixelated;
    display: block;
    cursor: crosshair;
  }
</style>
