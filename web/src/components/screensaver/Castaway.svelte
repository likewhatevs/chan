<script lang="ts">
  import { onMount } from "svelte";

  type SceneState = "idle" | "wave" | "sit" | "sleep" | "drink" | "walk" | "fish" | "ship";

  const STATES: SceneState[] = [
    "idle",
    "wave",
    "sit",
    "sleep",
    "drink",
    "walk",
    "fish",
    "ship",
  ];

  let canvas = $state<HTMLCanvasElement | undefined>();

  onMount(() => {
    if (!canvas) return;
    const context = canvas.getContext("2d");
    if (!context) return;
    const ctx = context;

    const reduced = window.matchMedia("(prefers-reduced-motion: reduce)");
    let width = 0;
    let height = 0;
    let scale = 1;
    let raf = 0;
    let last = 0;
    let state: SceneState = "idle";
    let stateUntil = 0;
    let walkX = 0;
    const logicalW = 320;
    const logicalH = 180;

    function resize(): void {
      if (!canvas) return;
      const dpr = window.devicePixelRatio || 1;
      width = canvas.clientWidth;
      height = canvas.clientHeight;
      canvas.width = Math.max(1, Math.floor(width * dpr));
      canvas.height = Math.max(1, Math.floor(height * dpr));
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      ctx.imageSmoothingEnabled = false;
      scale = Math.max(width / logicalW, height / logicalH);
    }

    function chooseState(t: number): void {
      if (t < stateUntil) return;
      state = reduced.matches ? "idle" : STATES[Math.floor(Math.random() * STATES.length)]!;
      stateUntil = t + 3500 + Math.random() * 4500;
      if (state === "walk") walkX = -18;
    }

    function px(x: number, y: number, w: number, h: number, color: string): void {
      ctx.fillStyle = color;
      ctx.fillRect(Math.floor(x), Math.floor(y), Math.floor(w), Math.floor(h));
    }

    function drawPalm(t: number): void {
      const sway = Math.sin(t / 900) * 2;
      px(217, 86, 9, 48, "#6b4328");
      px(212, 70, 9, 30, "#7c4d2c");
      px(207 + sway, 58, 56, 9, "#0f6f42");
      px(196 + sway, 66, 48, 8, "#15844d");
      px(219 + sway, 50, 10, 44, "#0d603a");
      px(229 + sway, 68, 47, 7, "#178c51");
      px(215, 65, 16, 15, "#6b3d22");
    }

    function drawCharacter(t: number): void {
      const bob = Math.floor(Math.sin(t / 250) * 2);
      const x = 141 + (state === "walk" ? walkX : 0);
      const y = 116 + bob;
      if (state === "sit" || state === "sleep") {
        px(x - 8, y + 14, 26, 8, "#304a61");
        px(x, y, 9, 10, "#d39a63");
        px(x + 8, y + 6, 16, 6, "#f2f0d0");
        if (state === "sleep") {
          px(x + 30, y - 14, 4, 4, "#eaf8ff");
          px(x + 36, y - 22, 5, 5, "#eaf8ff");
          px(x + 44, y - 32, 7, 7, "#eaf8ff");
        }
        return;
      }

      px(x, y, 9, 9, "#d39a63");
      px(x - 2, y + 9, 14, 18, "#f2f0d0");
      px(x - 3, y + 27, 6, 13, "#304a61");
      px(x + 8, y + 27, 6, 13, "#304a61");
      if (state === "wave") {
        px(x + 11, y + 6, 5, 6, "#d39a63");
        px(x + 16, y - 5, 5, 14, "#d39a63");
      } else if (state === "drink") {
        px(x + 11, y + 10, 10, 4, "#d39a63");
        px(x + 20, y + 9, 5, 5, "#6b3d22");
      } else {
        px(x - 7, y + 12, 7, 4, "#d39a63");
        px(x + 12, y + 12, 7, 4, "#d39a63");
      }
    }

    function drawEffects(t: number): void {
      if (state === "fish") {
        const x = 40 + ((t / 10) % 90);
        const y = 102 - Math.abs(Math.sin(t / 260)) * 16;
        px(x, y, 9, 4, "#f6d65b");
        px(x + 8, y + 1, 4, 2, "#ffef8a");
      }
      if (state === "ship") {
        const x = 290 - ((t / 18) % 340);
        px(x, 80, 38, 5, "#dedede");
        px(x + 7, 72, 18, 8, "#ffffff");
        px(x + 25, 76, 5, 4, "#d34b3f");
      }
    }

    function draw(t: number): void {
      if (t - last < (reduced.matches ? 1000 : 83)) {
        raf = requestAnimationFrame(draw);
        return;
      }
      last = t;
      chooseState(t);
      if (state === "walk") {
        walkX += 4;
        if (walkX > 26) walkX = -18;
      }

      const ox = (width - logicalW * scale) / 2;
      const oy = (height - logicalH * scale) / 2;
      ctx.save();
      ctx.clearRect(0, 0, width, height);
      ctx.translate(ox, oy);
      ctx.scale(scale, scale);

      const day = (Math.sin(t / 300000) + 1) / 2;
      const sky = ctx.createLinearGradient(0, 0, 0, 120);
      sky.addColorStop(0, day > 0.35 ? "#70c9ee" : "#172f5d");
      sky.addColorStop(1, day > 0.35 ? "#f6d9a4" : "#4d6292");
      ctx.fillStyle = sky;
      ctx.fillRect(0, 0, logicalW, logicalH);
      px(0, 86, logicalW, 94, "#1e7faf");
      px(0, 102, logicalW, 4, "#6ec6d5");
      px(0, 128, logicalW, 3, "#63b7c9");
      px(86, 124, 152, 24, "#d9b063");
      px(108, 111, 109, 24, "#efcc79");
      px(132, 101, 58, 20, "#f4d98b");
      px(96, 143, 136, 7, "#8a6d3f");
      drawPalm(t);
      drawEffects(t);
      drawCharacter(t);
      ctx.restore();

      raf = requestAnimationFrame(draw);
    }

    resize();
    raf = requestAnimationFrame(draw);
    window.addEventListener("resize", resize);
    return () => {
      cancelAnimationFrame(raf);
      window.removeEventListener("resize", resize);
    };
  });
</script>

<canvas bind:this={canvas} class="castaway" aria-hidden="true"></canvas>

<style>
  .castaway {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    background: #17385f;
  }
</style>
