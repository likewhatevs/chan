<script lang="ts">
  import { onMount } from "svelte";

  const GLYPHS = "ｱｲｳｴｵｶｷｸｹｺｻｼｽｾｿﾀﾁﾂﾃﾄﾅﾆﾇﾈﾉ0123456789#$%&*+-";
  const INTRO_MESSAGES = ["Wake up, Neo...", "The Matrix has you..."] as const;
  const INTRO_START_DELAY_MS = 500;
  const INTRO_HOLD_MS = 2000;
  const TYPE_DELAY_SLOW_MS = 300;
  const TYPE_DELAY_FAST_MS = 100;
  const DRAW_INTERVAL_MS = 40;
  const COLUMN_SPACING_PX = 11;
  const ROW_SPACING_PX = 19;
  const FONT_SIZE_PX = 18;

  type Column = {
    head: number;
    length: number;
    speed: number;
    glyphs: string[];
  };

  let canvas = $state<HTMLCanvasElement | undefined>();

  onMount(() => {
    if (!canvas) return;
    const context = canvas.getContext("2d");
    if (!context) return;
    const ctx = context;
    const reduced = window.matchMedia("(prefers-reduced-motion: reduce)");

    let width = 0;
    let height = 0;
    let rowCount = 0;
    let columns: Column[] = [];
    let drawTimer: ReturnType<typeof setInterval> | null = null;
    const timeouts: ReturnType<typeof setTimeout>[] = [];

    function glyph(): string {
      return GLYPHS[Math.floor(Math.random() * GLYPHS.length)] ?? "0";
    }

    function randomColumn(): Column {
      const length = 8 + Math.floor(Math.random() * 24);
      return {
        head: -Math.floor(Math.random() * Math.max(1, rowCount)),
        length,
        speed: 0.35 + Math.random() * 0.9,
        glyphs: Array.from({ length: rowCount + length + 4 }, glyph),
      };
    }

    function clear(): void {
      ctx.fillStyle = "#000";
      ctx.fillRect(0, 0, width, height);
    }

    function resize(): void {
      if (!canvas) return;
      const dpr = window.devicePixelRatio || 1;
      width = canvas.clientWidth;
      height = canvas.clientHeight;
      canvas.width = Math.max(1, Math.floor(width * dpr));
      canvas.height = Math.max(1, Math.floor(height * dpr));
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      ctx.textBaseline = "top";
      rowCount = Math.ceil(height / ROW_SPACING_PX) + 2;
      const columnCount = Math.ceil(width / COLUMN_SPACING_PX);
      columns = Array.from({ length: columnCount }, randomColumn);
      clear();
      if (reduced.matches) drawStaticMatrix();
    }

    function introDelay(index: number): number {
      return index % 7 === 5 || Math.random() < 0.16
        ? TYPE_DELAY_FAST_MS
        : TYPE_DELAY_SLOW_MS;
    }

    function drawIntroText(text: string): void {
      clear();
      ctx.font = `${FONT_SIZE_PX}px "Courier New", ui-monospace, monospace`;
      ctx.fillStyle = "#b8f7c1";
      ctx.shadowColor = "rgba(128, 255, 160, 0.5)";
      ctx.shadowBlur = 4;
      ctx.fillText(text, 30, 40);
      ctx.shadowBlur = 0;
    }

    function scheduleTimeout(fn: () => void, ms: number): void {
      timeouts.push(setTimeout(fn, ms));
    }

    function typeMessage(messageIndex: number, charIndex = 0): void {
      const message = INTRO_MESSAGES[messageIndex];
      if (!message) {
        startRain();
        return;
      }
      drawIntroText(message.slice(0, charIndex));
      if (charIndex < message.length) {
        scheduleTimeout(
          () => typeMessage(messageIndex, charIndex + 1),
          introDelay(charIndex),
        );
        return;
      }
      scheduleTimeout(() => {
        clear();
        scheduleTimeout(() => typeMessage(messageIndex + 1, 0), 0);
      }, INTRO_HOLD_MS);
    }

    function mutateColumn(column: Column): void {
      for (let i = 0; i < column.glyphs.length; i += 1) {
        if (Math.random() < 0.025) column.glyphs[i] = glyph();
      }
    }

    function drawRainFrame(): void {
      ctx.fillStyle = "rgba(0, 0, 0, 0.16)";
      ctx.fillRect(0, 0, width, height);
      ctx.font = `${FONT_SIZE_PX}px "Courier New", ui-monospace, monospace`;

      for (let xIndex = 0; xIndex < columns.length; xIndex += 1) {
        const column = columns[xIndex]!;
        mutateColumn(column);
        const headRow = Math.floor(column.head);
        const x = xIndex * COLUMN_SPACING_PX;
        for (let trail = 0; trail < column.length; trail += 1) {
          const row = headRow - trail;
          if (row < 0 || row >= rowCount) continue;
          const y = row * ROW_SPACING_PX;
          const g = column.glyphs[(row + trail) % column.glyphs.length] ?? glyph();
          if (trail === 0) {
            ctx.fillStyle = "#f2fff2";
          } else if (trail < 3) {
            ctx.fillStyle = `rgba(150, 185, 150, ${0.82 - trail * 0.14})`;
          } else {
            const alpha = Math.max(0.08, 0.78 - trail / column.length);
            ctx.fillStyle = `rgba(0, 205, 55, ${alpha * (0.7 + Math.random() * 0.3)})`;
          }
          ctx.fillText(g, x, y);
        }
        column.head += column.speed;
        if (column.head - column.length > rowCount + 2) {
          columns[xIndex] = randomColumn();
        }
      }
    }

    function drawStaticMatrix(): void {
      clear();
      ctx.font = `${FONT_SIZE_PX}px "Courier New", ui-monospace, monospace`;
      for (let x = 0; x < width; x += COLUMN_SPACING_PX) {
        for (let y = 0; y < height; y += ROW_SPACING_PX) {
          const head = Math.random() < 0.08;
          ctx.fillStyle = head
            ? "rgba(242, 255, 242, 0.9)"
            : `rgba(0, 205, 55, ${0.12 + Math.random() * 0.28})`;
          ctx.fillText(glyph(), x, y);
        }
      }
    }

    function startRain(): void {
      clear();
      if (reduced.matches) {
        drawStaticMatrix();
        return;
      }
      drawTimer = setInterval(drawRainFrame, DRAW_INTERVAL_MS);
    }

    resize();
    if (reduced.matches) {
      drawStaticMatrix();
    } else {
      scheduleTimeout(() => typeMessage(0), INTRO_START_DELAY_MS);
    }
    window.addEventListener("resize", resize);
    return () => {
      if (drawTimer !== null) clearInterval(drawTimer);
      for (const timeout of timeouts) clearTimeout(timeout);
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
