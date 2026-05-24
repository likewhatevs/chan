<script lang="ts">
  import { onMount } from "svelte";

  // Matrix rain cadence, color tiers, and cell geometry are adapted
  // from the MIT-licensed dcragusa/MatrixScreensaver project:
  // https://github.com/dcragusa/MatrixScreensaver
  const GLYPHS = "abcdefghijklmnopqrstuvwxyz123456789890~!#$%^&*()-_=+[]{};:'\",.<>/?\\|";
  const INTRO_MESSAGES = ["Wake up, Neo...", "The Matrix has you..."] as const;
  const INTRO_START_DELAY_MS = 500;
  const INTRO_HOLD_MS = 2000;
  const TYPE_DELAY_SLOW_MS = 300;
  const TYPE_DELAY_FAST_MS = 100;
  const DRAW_INTERVAL_MS = 40;
  const COLUMN_SPACING_PX = 11;
  const ROW_SPACING_PX = 19;
  const INTRO_FONT_SIZE_PX = 22;
  const RAIN_FONT_SIZE_PX = 20;
  const RAIN_DENSITY = 4;
  const HEAD_COLOR = "#f6f6f4";
  const LEAD_COLOR = "#c9cfb9";
  const MID_COLOR = "#95a297";
  const BODY_COLOR = "#2cb231";

  type Column = {
    position: number;
    delay: number;
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
    let columnCount = 0;
    let columns: Column[] = [];
    let drawTimer: ReturnType<typeof setInterval> | null = null;
    const timeouts: ReturnType<typeof setTimeout>[] = [];

    function glyph(): string {
      return GLYPHS[Math.floor(Math.random() * GLYPHS.length)] ?? "0";
    }

    function randomColumn(): Column {
      const glyphCount = Math.max(rowCount + 20, columnCount);
      return {
        position: 0,
        delay: Math.floor(Math.random() * Math.max(1, rowCount * RAIN_DENSITY * 2)),
        speed: Math.random() < 0.25 ? 1 : 0,
        glyphs: Array.from({ length: glyphCount }, glyph),
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
      columnCount = Math.ceil(width / COLUMN_SPACING_PX) + 1;
      columns = Array.from({ length: columnCount }, randomColumn);
      clear();
      if (reduced.matches) drawStaticMatrix();
    }

    function introDelay(index: number): number {
      return index % 7 === 5 || Math.random() < 1 / 3
        ? TYPE_DELAY_FAST_MS
        : TYPE_DELAY_SLOW_MS;
    }

    function drawIntroText(text: string): void {
      clear();
      ctx.font = `${INTRO_FONT_SIZE_PX}px "Courier New", ui-monospace, monospace`;
      ctx.fillStyle = "#7bff8d";
      ctx.shadowBlur = 0;
      ctx.fillText(text, 30, 40);
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

    function clearCell(x: number, y: number): void {
      ctx.clearRect(x, y, COLUMN_SPACING_PX, ROW_SPACING_PX);
    }

    function drawRainFrame(): void {
      ctx.font = `${RAIN_FONT_SIZE_PX}px "Courier New", ui-monospace, monospace`;

      for (let xIndex = 0; xIndex < columns.length; xIndex += 1) {
        const column = columns[xIndex]!;
        const x = xIndex * COLUMN_SPACING_PX;
        if (column.delay > 0) {
          column.delay -= 1;
          continue;
        }
        for (let row = 0; row < column.glyphs.length; row += 1) {
          if (row > column.position) break;
          const y = row * ROW_SPACING_PX;
          const outY = y + ROW_SPACING_PX;
          const g = column.glyphs[row] ?? glyph();
          if (row === column.position) {
            clearCell(x, y);
            ctx.fillStyle = HEAD_COLOR;
            ctx.fillText(g, x, outY);
          } else if (row === column.position - 1) {
            clearCell(x, y);
            ctx.fillStyle = LEAD_COLOR;
            ctx.fillText(g, x, outY);
          } else if (row === column.position - 2) {
            clearCell(x, y);
            ctx.fillStyle = MID_COLOR;
            ctx.fillText(g, x, outY);
          } else if (row === column.position - 3) {
            clearCell(x, y);
            ctx.fillStyle = BODY_COLOR;
            ctx.fillText(g, x, outY);
          } else if (
            row < column.position - 3 &&
            row >= column.position - rowCount + 10 &&
            Math.random() < 1 / 15
          ) {
            clearCell(x, y);
            ctx.fillStyle = BODY_COLOR;
            ctx.fillText(glyph(), x, outY);
          } else if (
            row < column.position - rowCount + 10 &&
            row > column.position - rowCount - 10
          ) {
            ctx.fillStyle =
              Math.random() < 0.2 ? "rgba(0, 0, 0, 0.30)" : "rgba(0, 0, 0, 0.05)";
            ctx.fillRect(x, y, COLUMN_SPACING_PX, ROW_SPACING_PX);
          } else if (row === column.position - rowCount - 10) {
            clearCell(x, y);
          }
        }
        column.delay = column.speed;
        column.position += 1;
        if (column.position > rowCount * 2 + 10) {
          column.glyphs = Array.from(
            { length: Math.max(rowCount + 20, columnCount) },
            glyph,
          );
          column.position = 0;
          column.delay = Math.floor(
            Math.random() * Math.max(1, (columnCount * RAIN_DENSITY) / 2),
          );
          column.speed = Math.random() < 0.25 ? 1 : 0;
        }
      }
    }

    function drawStaticMatrix(): void {
      clear();
      ctx.font = `${RAIN_FONT_SIZE_PX}px "Courier New", ui-monospace, monospace`;
      for (let x = 0; x < width; x += COLUMN_SPACING_PX) {
        for (let y = 0; y < height; y += ROW_SPACING_PX) {
          const r = Math.random();
          if (r < 0.04) {
            ctx.fillStyle = HEAD_COLOR;
          } else if (r < 0.08) {
            ctx.fillStyle = LEAD_COLOR;
          } else if (r < 0.12) {
            ctx.fillStyle = MID_COLOR;
          } else {
            ctx.fillStyle = BODY_COLOR;
          }
          ctx.fillText(glyph(), x, y + ROW_SPACING_PX);
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
