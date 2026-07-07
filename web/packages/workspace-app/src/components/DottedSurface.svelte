<script lang="ts">
  import { onMount } from "svelte";

  const TAU = Math.PI * 2;
  const MAX_DPR = 2;
  const REDUCED_MOTION = "(prefers-reduced-motion: reduce)";
  // Wave/grid constants follow the 21st.dev dotted-surface reference.
  // Source: https://21st.dev/@efferd/components/dotted-surface
  const AMOUNT_X = 40;
  const AMOUNT_Y = 60;
  const SEPARATION = 150;
  const CAMERA_Y = 355;
  const CAMERA_Z = 1220;
  const FOV_RAD = (60 * Math.PI) / 180;
  const POINT_SIZE = 6.4;
  const WAVE_SPEED = 1.45;
  const FRAME_INTERVAL_MS = 1000 / 24;

  let canvas = $state<HTMLCanvasElement | undefined>();

  function cssValue(name: string, fallback: string): string {
    if (!canvas) return fallback;
    const host = canvas.parentElement ?? canvas;
    return getComputedStyle(host).getPropertyValue(name).trim() || fallback;
  }

  function cssNumber(name: string, fallback: number): number {
    const raw = Number.parseFloat(cssValue(name, String(fallback)));
    return Number.isFinite(raw) ? raw : fallback;
  }

  onMount(() => {
    if (!canvas) return;
    const context = canvas.getContext("2d");
    if (!context || typeof context.clearRect !== "function") return;
    const ctx = context;
    const reduced = window.matchMedia?.(REDUCED_MOTION) ?? null;

    let width = 0;
    let height = 0;
    let rafId: number | null = null;
    let lastDrawMs = 0;

    function draw(timeMs: number): void {
      if (!canvas || width <= 0 || height <= 0) return;

      const count = timeMs * 0.001 * WAVE_SPEED;
      const dotColor = cssValue("--dotted-surface-dot-rgb", "200, 200, 200");
      const alphaBase = cssNumber("--dotted-surface-alpha-base", 0.16);
      const alphaRange = cssNumber("--dotted-surface-alpha-range", 0.30);
      const sizeScale = cssNumber("--dotted-surface-size-scale", 1);
      const focal = (height * 1.28) / (2 * Math.tan(FOV_RAD / 2));
      const horizon = height * 0.34;

      ctx.clearRect(0, 0, width, height);
      ctx.fillStyle = `rgb(${dotColor})`;

      for (let iy = 0; iy < AMOUNT_Y; iy += 1) {
        const worldZ = iy * SEPARATION - (AMOUNT_Y * SEPARATION) / 2;
        const zView = CAMERA_Z - worldZ;
        if (zView <= 0) continue;

        const perspective = focal / zView;
        const depth = iy / (AMOUNT_Y - 1);

        for (let ix = 0; ix < AMOUNT_X; ix += 1) {
          const worldX = ix * SEPARATION - (AMOUNT_X * SEPARATION) / 2;
          const worldY =
            Math.sin((ix + count) * 0.3) * 50 +
            Math.sin((iy + count) * 0.5) * 50;
          const x = width / 2 + worldX * perspective;
          const y = horizon + (CAMERA_Y - worldY) * perspective;
          if (x < -10 || x > width + 10 || y < -10 || y > height + 10) continue;

          const radius = Math.min(
            3.8,
            Math.max(0.85, POINT_SIZE * perspective * sizeScale),
          );
          ctx.globalAlpha = alphaBase + depth * alphaRange;
          ctx.beginPath();
          ctx.arc(x, y, radius, 0, TAU);
          ctx.fill();
        }
      }
      ctx.globalAlpha = 1;
    }

    function resize(): void {
      if (!canvas) return;
      width = Math.max(1, Math.floor(canvas.clientWidth));
      height = Math.max(1, Math.floor(canvas.clientHeight));
      const dpr = Math.min(window.devicePixelRatio || 1, MAX_DPR);
      canvas.width = Math.floor(width * dpr);
      canvas.height = Math.floor(height * dpr);
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      draw(performance.now());
    }

    function stop(): void {
      if (rafId === null) return;
      cancelAnimationFrame(rafId);
      rafId = null;
    }

    function loop(timeMs: number): void {
      if (timeMs - lastDrawMs >= FRAME_INTERVAL_MS) {
        draw(timeMs);
        lastDrawMs = timeMs;
      }
      rafId = requestAnimationFrame(loop);
    }

    function start(): void {
      stop();
      if (document.hidden) return;
      if (reduced?.matches) {
        draw(performance.now());
        return;
      }
      lastDrawMs = 0;
      rafId = requestAnimationFrame(loop);
    }

    function onVisibilityChange(): void {
      if (document.hidden) stop();
      else start();
    }

    const observer =
      typeof ResizeObserver !== "undefined" ? new ResizeObserver(resize) : null;
    observer?.observe(canvas);
    window.addEventListener("resize", resize);
    document.addEventListener("visibilitychange", onVisibilityChange);
    reduced?.addEventListener?.("change", start);

    resize();
    start();

    return () => {
      stop();
      observer?.disconnect();
      window.removeEventListener("resize", resize);
      document.removeEventListener("visibilitychange", onVisibilityChange);
      reduced?.removeEventListener?.("change", start);
    };
  });
</script>

<div class="dotted-surface" aria-hidden="true">
  <canvas bind:this={canvas}></canvas>
</div>

<style>
  .dotted-surface {
    position: absolute;
    top: var(--dotted-surface-top, auto);
    left: 0;
    right: 0;
    bottom: var(--dotted-surface-bottom, 0);
    height: var(--dotted-surface-height, clamp(260px, 40%, 475px));
    z-index: 0;
    --dotted-surface-dot-rgb: 200, 200, 200;
    --dotted-surface-alpha-base: 0.18;
    --dotted-surface-alpha-range: 0.42;
    --dotted-surface-size-scale: 0.94;
    pointer-events: none;
    overflow: hidden;
    opacity: 1;
    -webkit-mask-image: linear-gradient(to bottom, transparent, black 38%);
            mask-image: linear-gradient(to bottom, transparent, black 38%);
  }
  canvas {
    width: 100%;
    height: 100%;
    display: block;
  }
  :global([data-theme="light"]) .dotted-surface {
    --dotted-surface-dot-rgb: 0, 0, 0;
    --dotted-surface-alpha-base: 0.13;
    --dotted-surface-alpha-range: 0.32;
    --dotted-surface-size-scale: 0.9;
  }
  :global([data-theme="dark"]) .dotted-surface {
    --dotted-surface-dot-rgb: 218, 218, 218;
    --dotted-surface-alpha-base: 0.18;
    --dotted-surface-alpha-range: 0.42;
    --dotted-surface-size-scale: 0.94;
  }
  @media (prefers-reduced-motion: reduce) {
    .dotted-surface {
      opacity: 0.8;
    }
  }
</style>
