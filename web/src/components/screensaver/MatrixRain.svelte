<script lang="ts">
  import { onMount } from "svelte";

  const GLYPHS = "ｱｲｳｴｵｶｷｸｹｺｻｼｽｾｿﾀﾁﾂﾃﾄﾅﾆﾇﾈﾉ0123456789#$%&*+-";

  let canvas = $state<HTMLCanvasElement | undefined>();

  onMount(() => {
    if (!canvas) return;
    const context = canvas.getContext("2d");
    if (!context) return;
    const ctx = context;

    const reduced = window.matchMedia("(prefers-reduced-motion: reduce)");
    const columns: Array<{ y: number; speed: number; ticks: number }> = [];
    let width = 0;
    let height = 0;
    let columnCount = 0;
    let frame = 0;
    let raf = 0;
    let last = 0;
    const fontSize = 18;

    function resize(): void {
      if (!canvas) return;
      const dpr = window.devicePixelRatio || 1;
      width = canvas.clientWidth;
      height = canvas.clientHeight;
      canvas.width = Math.max(1, Math.floor(width * dpr));
      canvas.height = Math.max(1, Math.floor(height * dpr));
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      columnCount = Math.ceil(width / fontSize);
      columns.length = 0;
      for (let i = 0; i < columnCount; i += 1) {
        columns.push({
          y: Math.floor(Math.random() * -height),
          speed: 0.7 + Math.random() * 1.8,
          ticks: Math.floor(Math.random() * 30),
        });
      }
    }

    function glyph(): string {
      return GLYPHS[Math.floor(Math.random() * GLYPHS.length)] ?? "0";
    }

    function drawStatic(): void {
      const grad = ctx.createLinearGradient(0, 0, 0, height);
      grad.addColorStop(0, "#031407");
      grad.addColorStop(1, "#000000");
      ctx.fillStyle = grad;
      ctx.fillRect(0, 0, width, height);
      ctx.font = `${fontSize}px ui-monospace, SFMono-Regular, Menlo, monospace`;
      for (let x = 0; x < width; x += fontSize) {
        for (let y = 0; y < height; y += fontSize * 2) {
          ctx.fillStyle = `rgba(80, 255, 140, ${0.08 + Math.random() * 0.22})`;
          ctx.fillText(glyph(), x, y);
        }
      }
    }

    function draw(t: number): void {
      if (reduced.matches) {
        if (frame === 0 || t - last > 1000) {
          drawStatic();
          last = t;
          frame += 1;
        }
        raf = requestAnimationFrame(draw);
        return;
      }

      if (t - last < 33) {
        raf = requestAnimationFrame(draw);
        return;
      }
      last = t;
      ctx.fillStyle = "rgba(0, 0, 0, 0.18)";
      ctx.fillRect(0, 0, width, height);
      ctx.font = `${fontSize}px ui-monospace, SFMono-Regular, Menlo, monospace`;
      ctx.textBaseline = "top";

      for (let i = 0; i < columnCount; i += 1) {
        const col = columns[i]!;
        const x = i * fontSize;
        const head = col.y;
        for (let trail = 14; trail >= 0; trail -= 1) {
          const y = head - trail * fontSize;
          if (y < -fontSize || y > height + fontSize) continue;
          const alpha = trail === 0 ? 1 : Math.max(0.06, (14 - trail) / 22);
          ctx.fillStyle = trail === 0
            ? "#d8ffe2"
            : `rgba(58, 255, 112, ${alpha})`;
          ctx.fillText(glyph(), x, y);
        }
        col.ticks += 1;
        col.y += fontSize * col.speed;
        if (col.y > height + fontSize * 18) {
          col.y = Math.floor(Math.random() * -height);
          col.speed = 0.7 + Math.random() * 1.8;
        }
      }
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

<canvas bind:this={canvas} class="matrix-rain" aria-hidden="true"></canvas>

<style>
  .matrix-rain {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    background: #000;
  }
</style>
