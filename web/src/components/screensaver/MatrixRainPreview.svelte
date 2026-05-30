<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { drawStaticMatrix, gridDimensions } from "./matrixRain";

  // Small, self-contained matrix preview for config panels (the Dashboard
  // About-slot back). Unlike MatrixRain.svelte this never reads the window
  // size: it renders into a fixed preview box so it can sit inside a card.
  //
  // The static frame is the safe default for an always-mounted back face: no
  // timers, no leaked rAF. The animated path re-rolls the same shared static
  // frame on a throttled cadence so it never forks the live screensaver's
  // column state machine, which is intentionally not extracted.

  let {
    width = 320,
    height = 180,
    animated = false,
  }: { width?: number; height?: number; animated?: boolean } = $props();

  // Re-roll cadence for the animated path. Slower than the live screensaver's
  // 40ms tick because a previewbox-sized full re-roll every frame reads as
  // noise; this gives a calmer shimmer.
  const FRAME_INTERVAL_MS = 90;

  let canvas = $state<HTMLCanvasElement | undefined>();
  let ctx: CanvasRenderingContext2D | null = null;
  let fontsReady = false;
  let rafId: number | null = null;
  let lastFrameAt = 0;

  // Device pixel ratio matters here: the box is small, so glyph blur on retina
  // is visible. We scale the backing store and map drawing units back to CSS
  // pixels so the shared grid math stays in CSS-pixel space.
  function dpr(): number {
    return typeof window === "undefined" ? 1 : window.devicePixelRatio || 1;
  }

  function sizeCanvas(): void {
    if (!canvas || !ctx) return;
    const ratio = dpr();
    canvas.width = Math.round(width * ratio);
    canvas.height = Math.round(height * ratio);
    ctx.setTransform(ratio, 0, 0, ratio, 0, 0);
  }

  function renderStatic(): void {
    if (!ctx) return;
    const { numCols, numChars } = gridDimensions(width, height);
    drawStaticMatrix(ctx, numCols, numChars);
  }

  function stopLoop(): void {
    if (rafId !== null) {
      cancelAnimationFrame(rafId);
      rafId = null;
    }
  }

  function tick(now: number): void {
    // Bail if the prop flipped off mid-flight so a stale rAF cannot re-arm.
    if (!animated || !ctx) {
      rafId = null;
      return;
    }
    if (now - lastFrameAt >= FRAME_INTERVAL_MS) {
      lastFrameAt = now;
      renderStatic();
    }
    rafId = requestAnimationFrame(tick);
  }

  function startLoop(): void {
    if (rafId !== null) return;
    lastFrameAt = 0;
    rafId = requestAnimationFrame(tick);
  }

  async function loadMatrixFont(): Promise<void> {
    if (typeof document === "undefined" || !document.fonts) {
      fontsReady = true;
      return;
    }
    try {
      await document.fonts.load("20px matrix_code");
    } catch {
      // A missing font falls back to the canvas default; still legible.
    }
    fontsReady = true;
  }

  onMount(() => {
    if (!canvas) return;
    ctx = canvas.getContext("2d");
    if (!ctx) return;
    sizeCanvas();
    void loadMatrixFont().then(() => {
      if (!ctx) return;
      sizeCanvas();
      renderStatic();
      if (animated) startLoop();
    });
  });

  onDestroy(() => {
    stopLoop();
  });

  // React to prop changes after mount: resize and re-render on dimension
  // changes, and start/stop the loop strictly with the animated flag. The
  // guard on fontsReady avoids a flash of fallback-font glyphs before the
  // matrix_code face resolves on first mount.
  $effect(() => {
    // Touch the reactive inputs so the effect re-runs when any of them change.
    void width;
    void height;
    const wantAnimated = animated;
    if (!ctx || !fontsReady) return;
    sizeCanvas();
    if (wantAnimated) {
      startLoop();
    } else {
      stopLoop();
      renderStatic();
    }
  });
</script>

<canvas
  bind:this={canvas}
  class="matrix-rain-preview"
  style="width: {width}px; height: {height}px;"
  aria-hidden="true"
></canvas>

<style>
  @font-face {
    font-family: "matrix_code";
    src: url("/static/matrix/matrix_code.woff2") format("woff2");
    font-weight: normal;
    font-style: normal;
  }

  .matrix-rain-preview {
    display: block;
    background: #000;
    border-radius: 6px;
  }
</style>
