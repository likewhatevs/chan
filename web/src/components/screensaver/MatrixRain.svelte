<script lang="ts">
  import { onMount } from "svelte";
  import {
    COLUMN_SPACING_PX,
    createRainColumns,
    DRAW_INTERVAL_MS,
    drawStaticMatrix as drawStaticFrame,
    RAIN_FONT_SIZE_PX,
    randInt,
    type RainColumn,
    ROW_SPACING_PX,
    stepRain,
  } from "./matrixRain";

  // High-fidelity Svelte adaptation of the MIT-licensed
  // dcragusa/MatrixScreensaver project:
  // https://github.com/dcragusa/MatrixScreensaver
  // License notice: /static/matrix/LICENSE-MatrixScreensaver.txt
  const INTRO_MESSAGES = ["Wake up, Neo...", "The Matrix has you..."] as const;
  const INTRO_START_DELAY_MS = 500;
  const INTRO_HOLD_MS = 2000;
  const TYPE_DELAY_SLOW_MS = 300;
  const TYPE_DELAY_FAST_MS = 100;
  const INTRO_FONT_SIZE_PX = 22;
  const INTRO_COLOR = "#7bff8d";

  let canvas = $state<HTMLCanvasElement | undefined>();

  onMount(() => {
    if (!canvas) return;
    const context = canvas.getContext("2d");
    if (!context) return;
    const ctx = context;
    const reduced = window.matchMedia("(prefers-reduced-motion: reduce)");

    let active = true;
    let numCols = 0;
    let numChars = 0;
    let columns: RainColumn[] = [];
    let drawTimer: ReturnType<typeof setInterval> | null = null;
    const timeouts: ReturnType<typeof setTimeout>[] = [];

    function sleep(milliseconds: number): Promise<void> {
      return new Promise((resolve) => {
        const timeout = setTimeout(resolve, milliseconds);
        timeouts.push(timeout);
      });
    }

    function outputChar(char: string, horpos: number, verpos: number): void {
      ctx.fillText(char, horpos, verpos);
    }

    function clearScreen(): void {
      if (!canvas) return;
      ctx.clearRect(0, 0, canvas.width, canvas.height);
    }

    function setupCanvas(): void {
      if (!canvas) return;
      canvas.width = window.innerWidth;
      canvas.height = window.innerHeight;
      numCols = Math.floor(canvas.width / COLUMN_SPACING_PX) + 1;
      numChars = Math.floor(canvas.height / ROW_SPACING_PX) + 1;
      columns = createRainColumns(numCols, numChars);
    }

    async function loadMatrixFonts(): Promise<void> {
      if (!document.fonts) return;
      await Promise.all([
        document.fonts.load(`${INTRO_FONT_SIZE_PX}px matrix_courier`),
        document.fonts.load(`${RAIN_FONT_SIZE_PX}px matrix_code`),
      ]);
    }

    async function renderIntro(chars: string[], index: number): Promise<boolean> {
      await sleep(!randInt(3) ? TYPE_DELAY_FAST_MS : TYPE_DELAY_SLOW_MS);
      if (!active) return false;
      outputChar(
        chars[index] ?? "",
        index * (COLUMN_SPACING_PX + 2) + 30,
        40,
      );
      if (index === chars.length - 1) return true;
      return renderIntro(chars, index + 1);
    }

    async function intro(): Promise<void> {
      ctx.font = `${INTRO_FONT_SIZE_PX}px matrix_courier`;
      ctx.fillStyle = INTRO_COLOR;
      await sleep(INTRO_START_DELAY_MS);
      if (!active) return;

      await renderIntro(INTRO_MESSAGES[0].split(""), 0);
      await sleep(INTRO_HOLD_MS);
      clearScreen();
      if (!active) return;

      await renderIntro(INTRO_MESSAGES[1].split(""), 0);
      await sleep(INTRO_HOLD_MS);
      clearScreen();
    }

    // Delegates to the shared static-frame renderer so the screensaver and the
    // config-panel preview never drift. The shared helper clears its own grid
    // extent before drawing, so no separate clearScreen() call is needed.
    function drawStaticMatrix(): void {
      drawStaticFrame(ctx, numCols, numChars);
    }

    function startRain(): void {
      clearScreen();
      if (reduced.matches) {
        drawStaticMatrix();
        return;
      }
      // One rain frame per tick via the shared engine (same logic the preview
      // runs), so the two surfaces stay identical.
      drawTimer = setInterval(
        () => stepRain(ctx, columns, numCols, numChars),
        DRAW_INTERVAL_MS,
      );
    }

    function onResize(): void {
      setupCanvas();
      if (reduced.matches) drawStaticMatrix();
    }

    async function run(): Promise<void> {
      setupCanvas();
      await loadMatrixFonts();
      if (!active) return;
      if (reduced.matches) {
        drawStaticMatrix();
        return;
      }
      await intro();
      if (!active) return;
      setupCanvas();
      startRain();
    }

    window.addEventListener("resize", onResize, false);
    void run();

    return () => {
      active = false;
      if (drawTimer !== null) clearInterval(drawTimer);
      for (const timeout of timeouts) clearTimeout(timeout);
      window.removeEventListener("resize", onResize, false);
    };
  });
</script>

<canvas bind:this={canvas} class="matrix-rain" aria-hidden="true"></canvas>

<style>
  @font-face {
    font-family: "matrix_code";
    src: url("/static/matrix/matrix_code.woff2") format("woff2");
    font-weight: normal;
    font-style: normal;
  }

  @font-face {
    font-family: "matrix_courier";
    src: url("/static/matrix/matrix_courier.woff2") format("woff2");
    font-weight: bold;
    font-style: normal;
  }

  .matrix-rain {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    background: #000;
  }
</style>
