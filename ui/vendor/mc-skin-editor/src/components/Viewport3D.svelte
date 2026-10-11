<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { Color, PerspectiveCamera, Raycaster, Scene, Vector2, Vector3, WebGLRenderer, type Mesh } from 'three';
  import { OrbitControls } from 'three/addons/controls/OrbitControls.js';
  import { skinStore } from '../lib/stores/skin';
  import { layerVisibility, partVisibility } from '../lib/stores/visibility';
  import { buildSkinModel, type SkinModel } from '../lib/three/buildSkinModel';
  import { pickPixel, type PixelHit } from '../lib/three/raycastPixel';
  import { applyToolAt, applyToolAlongLine, beginStroke, endStroke } from '../lib/paintController';

  let container: HTMLDivElement;
  let renderer: WebGLRenderer;
  let scene: Scene;
  let camera: PerspectiveCamera;
  let controls: OrbitControls;
  let sourceCanvas: HTMLCanvasElement;
  let sourceCtx: CanvasRenderingContext2D;
  let model: SkinModel | null = null;
  let raycaster: Raycaster;
  let painting = false;
  let lastPixel: PixelHit | null = null;
  let resizeObserver: ResizeObserver;
  let frameId = 0;
  let tickFallback: ReturnType<typeof setInterval> | undefined;
  let webglError = false;

  const MIN_DIST = 20;
  const MAX_DIST = 220;

  /** Renders a frame immediately, instead of waiting for the next rAF tick — keeps the
   *  canvas from visibly lagging a change when the tab is backgrounded/throttled. */
  function renderNow() {
    if (renderer && scene && camera) renderer.render(scene, camera);
  }

  function applyVisibility() {
    if (!model) return;
    const layers = $layerVisibility;
    const parts = $partVisibility;
    for (const entry of model.parts) {
      const visible = parts[entry.part];
      if (entry.base) entry.base.visible = visible && layers.base;
      if (entry.overlay) entry.overlay.visible = visible && layers.overlay;
      if (entry.outline) entry.outline.visible = visible && layers.base;
      if (entry.baseGrid) entry.baseGrid.visible = visible && layers.base;
      if (entry.overlayGrid) entry.overlayGrid.visible = visible && layers.overlay;
      if (entry.baseOccluder) entry.baseOccluder.visible = visible && layers.base;
      // Only self-occludes when the base layer isn't shown to backstop it —
      // otherwise it would block the base layer from showing through the
      // overlay's legitimate transparent areas.
      if (entry.overlayOccluder) entry.overlayOccluder.visible = visible && layers.overlay && !layers.base;
    }
    renderNow();
  }

  function visibleMeshes(): Mesh[] {
    if (!model) return [];
    return model.meshes.filter((m) => m.visible);
  }

  function syncTexture() {
    if (!model) return;
    const { width, height, pixels } = $skinStore;
    if (sourceCanvas.width !== width) sourceCanvas.width = width;
    if (sourceCanvas.height !== height) sourceCanvas.height = height;
    sourceCtx.putImageData(new ImageData(new Uint8ClampedArray(pixels), width, height), 0, 0);
    model.texture.needsUpdate = true;
    renderNow();
  }

  function rebuildModel() {
    if (model) {
      scene.remove(model.group);
      model.dispose();
      model = null;
    }
    const { width, height, pixels, model: modelType, resolution } = $skinStore;
    if (sourceCanvas.width !== width) sourceCanvas.width = width;
    if (sourceCanvas.height !== height) sourceCanvas.height = height;
    sourceCtx.putImageData(new ImageData(new Uint8ClampedArray(pixels), width, height), 0, 0);
    model = buildSkinModel(sourceCanvas, modelType, resolution);
    scene.add(model.group);
    applyVisibility();
    renderNow();
  }

  let lastModel = '';
  $: if (scene) {
    const key = `${$skinStore.model}:${$skinStore.resolution}`;
    if (key !== lastModel) {
      lastModel = key;
      rebuildModel();
    } else {
      syncTexture();
    }
  }
  $: if (model) {
    $layerVisibility;
    $partVisibility;
    applyVisibility();
  }

  function eventToNDC(e: PointerEvent): Vector2 {
    const rect = renderer.domElement.getBoundingClientRect();
    return new Vector2(
      ((e.clientX - rect.left) / rect.width) * 2 - 1,
      -((e.clientY - rect.top) / rect.height) * 2 + 1
    );
  }

  function hitTest(e: PointerEvent): PixelHit | null {
    raycaster.setFromCamera(eventToNDC(e), camera);
    const { width, height } = $skinStore;
    return pickPixel(raycaster, visibleMeshes(), width, height);
  }

  function onContainerPointerDownCapture(e: PointerEvent) {
    const hit = hitTest(e);
    if (!hit) return;
    controls.enabled = false;
    painting = true;
    lastPixel = hit;
    beginStroke();
    applyToolAt(hit.x, hit.y);
  }

  function onWindowPointerMove(e: PointerEvent) {
    if (!painting) return;
    const hit = hitTest(e);
    if (!hit) return;
    // The atlas regions for different body parts aren't adjacent, so a
    // straight texture-space line between a hit on one part and a hit on
    // another would cut through unrelated pixels instead of following what
    // the cursor actually crossed on the model. Only interpolate within the
    // same part; a jump to a new part just paints at the new hit.
    if (lastPixel && lastPixel.part === hit.part) applyToolAlongLine(lastPixel.x, lastPixel.y, hit.x, hit.y);
    else applyToolAt(hit.x, hit.y);
    lastPixel = hit;
  }

  function onWindowPointerUp() {
    if (!painting) return;
    painting = false;
    lastPixel = null;
    controls.enabled = true;
    endStroke();
  }

  export function zoomIn() {
    dolly(0.85);
  }
  export function zoomOut() {
    dolly(1.18);
  }
  function dolly(factor: number) {
    const dir = new Vector3().subVectors(camera.position, controls.target);
    const dist = Math.max(MIN_DIST, Math.min(MAX_DIST, dir.length() * factor));
    dir.setLength(dist);
    camera.position.copy(controls.target).add(dir);
    renderNow();
  }

  function animate() {
    frameId = requestAnimationFrame(animate);
    controls.update();
    renderer.render(scene, camera);
  }

  function applySize(width: number, height: number) {
    if (width < 2 || height < 2) return false;
    camera.aspect = width / height;
    camera.updateProjectionMatrix();
    renderer.setSize(width, height);
    renderNow();
    return true;
  }

  function resizeToContainer() {
    const r = container.getBoundingClientRect();
    return applySize(r.width, r.height);
  }

  onMount(() => {
    try {
      sourceCanvas = document.createElement('canvas');
      sourceCtx = sourceCanvas.getContext('2d')!;

      scene = new Scene();
      scene.background = new Color(0x2b2f36);

      camera = new PerspectiveCamera(35, 16 / 9, 1, 1000);
      camera.position.set(45, 30, 70);

      renderer = new WebGLRenderer({ antialias: true, failIfMajorPerformanceCaveat: false });
      renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
      renderer.setSize(300, 169);
      container.appendChild(renderer.domElement);

      controls = new OrbitControls(camera, renderer.domElement);
      controls.target.set(0, 16, 0);
      controls.enableDamping = true;
      controls.dampingFactor = 0.12;
      controls.minDistance = MIN_DIST;
      controls.maxDistance = MAX_DIST;
      controls.update();

      raycaster = new Raycaster();

      rebuildModel();
      lastModel = `${$skinStore.model}:${$skinStore.resolution}`;

      container.addEventListener('pointerdown', onContainerPointerDownCapture, { capture: true });
      window.addEventListener('pointermove', onWindowPointerMove);
      window.addEventListener('pointerup', onWindowPointerUp);
      window.addEventListener('pointercancel', onWindowPointerUp);

      // The container's layout box may not be fully settled at this exact
      // synchronous point (e.g. right after a conditional {#if} mount, or
      // during dev-mode CSS injection). Retry via both rAF and a plain timer
      // — rAF alone isn't enough on a throttled/backgrounded tab, since it's
      // tied to the compositor and can stall for longer than is reasonable
      // here. ResizeObserver takes over from there for any later resizes.
      if (!resizeToContainer()) {
        requestAnimationFrame(resizeToContainer);
        setTimeout(resizeToContainer, 60);
        setTimeout(resizeToContainer, 250);
      }

      resizeObserver = new ResizeObserver(() => resizeToContainer());
      resizeObserver.observe(container);

      animate();
      // Supplements the rAF loop above with a plain-timer tick, so orbit
      // damping/rendering keeps progressing even on a throttled/backgrounded
      // tab where rAF can stall far longer than a user would expect.
      tickFallback = setInterval(() => {
        if (controls) controls.update();
        renderNow();
      }, 100);
    } catch (err) {
      // WebGL can be unavailable (old hardware, disabled GPU
      // acceleration, some sandboxed/virtualized environments). Fail
      // soft instead of throwing out of onMount, which would otherwise
      // leave the rest of the app's mount cycle in an inconsistent
      // state.
      console.error('3D viewport unavailable:', err);
      webglError = true;
    }
  });

  onDestroy(() => {
    cancelAnimationFrame(frameId);
    if (tickFallback) clearInterval(tickFallback);
    resizeObserver?.disconnect();
    container?.removeEventListener('pointerdown', onContainerPointerDownCapture, { capture: true } as EventListenerOptions);
    window.removeEventListener('pointermove', onWindowPointerMove);
    window.removeEventListener('pointerup', onWindowPointerUp);
    window.removeEventListener('pointercancel', onWindowPointerUp);
    model?.dispose();
    renderer?.dispose();
    controls?.dispose();
  });
</script>

<div class="viewport" bind:this={container} style="touch-action: none;">
  {#if webglError}
    <div class="webgl-fallback">
      <p>3D preview unavailable — your browser or device doesn't support WebGL.</p>
      <p class="hint">You can still edit the skin using the 2D texture map.</p>
    </div>
  {:else}
    <div class="zoom-controls">
      <button type="button" on:click={zoomIn} title="Zoom in" aria-label="Zoom in">+</button>
      <button type="button" on:click={zoomOut} title="Zoom out" aria-label="Zoom out">−</button>
    </div>
  {/if}
</div>

<style>
  .viewport {
    position: relative;
    width: 100%;
    height: 100%;
    border-radius: 6px;
    overflow: hidden;
    cursor: grab;
  }
  .viewport :global(canvas) {
    display: block;
  }
  .webgl-fallback {
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    gap: 6px;
    padding: 24px;
    color: var(--text-dim, #9a9ba3);
    cursor: default;
  }
  .webgl-fallback .hint {
    font-size: 12px;
    opacity: 0.8;
  }
  .zoom-controls {
    position: absolute;
    right: 10px;
    bottom: 10px;
    display: flex;
    flex-direction: column;
    gap: 4px;
    z-index: 2;
  }
  .zoom-controls button {
    width: 32px;
    height: 32px;
    font-size: 18px;
    line-height: 1;
    border-radius: 6px;
    border: 1px solid var(--border, #555);
    background: var(--panel-bg, #333);
    color: var(--text, #eee);
    cursor: pointer;
  }
  .zoom-controls button:hover {
    background: var(--panel-bg-hover, #444);
  }
</style>
