<script lang="ts">
  import { onMount } from "svelte";

  // High-fidelity Svelte adaptation of the MIT-licensed
  // dcragusa/MatrixScreensaver project:
  // https://github.com/dcragusa/MatrixScreensaver
  // License notice: /static/matrix/LICENSE-MatrixScreensaver.txt
  const MATRIX_ALPHABET =
    "abcdefghijklmnopqrstuvwxyz123456789890~!#$%^&*()-_=+[]{};:'\",.<>/?\\|".split("");
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
  const INTRO_COLOR = "#7bff8d";
  const HEAD_COLOR = "#f6f6f4";
  const LEAD_COLOR = "#c9cfb9";
  const MID_COLOR = "#95a297";
  const BODY_COLOR = "#2cb231";

  type Column = {
    chars: string[];
    delay: number;
    speed: number;
    position: number;
  };

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
    let columns: Column[] = [];
    let drawTimer: ReturnType<typeof setInterval> | null = null;
    const timeouts: ReturnType<typeof setTimeout>[] = [];

    function randInt(max: number): number {
      return Math.floor(Math.random() * max);
    }

    function sleep(milliseconds: number): Promise<void> {
      return new Promise((resolve) => {
        const timeout = setTimeout(resolve, milliseconds);
        timeouts.push(timeout);
      });
    }

    function randomChar(): string {
      return MATRIX_ALPHABET[randInt(MATRIX_ALPHABET.length)] ?? "0";
    }

    function randomChars(count: number): string[] {
      const chars: string[] = [];
      for (let index = 0; index < count; index += 1) {
        chars.push(randomChar());
      }
      return chars;
    }

    function newColumn(initialCharCount: number, delayColumnCount: number): Column {
      return {
        chars: randomChars(initialCharCount),
        delay: randInt(delayColumnCount * RAIN_DENSITY * 2),
        speed: !randInt(4) ? 1 : 0,
        position: 0,
      };
    }

    function outputChar(char: string, horpos: number, verpos: number): void {
      ctx.fillText(char, horpos, verpos);
    }

    function clearChar(horpos: number, verpos: number): void {
      ctx.clearRect(horpos, verpos, COLUMN_SPACING_PX, ROW_SPACING_PX);
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
      columns = [];
      for (let index = 0; index < numCols; index += 1) {
        columns.push(newColumn(numCols, numChars));
      }
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

    function drawScreen(): void {
      ctx.font = `${RAIN_FONT_SIZE_PX}px matrix_code`;

      for (let colIndex = 0; colIndex < columns.length; colIndex += 1) {
        const col = columns[colIndex]!;

        if (col.delay) {
          col.delay -= 1;
          continue;
        }

        for (let rowIndex = 0; rowIndex < col.chars.length; rowIndex += 1) {
          const char = col.chars[rowIndex] ?? randomChar();
          const horpos = colIndex * COLUMN_SPACING_PX;
          const verpos = rowIndex * ROW_SPACING_PX;
          const verout = verpos + ROW_SPACING_PX;

          if (rowIndex > col.position) {
            break;
          } else if (rowIndex === col.position) {
            clearChar(horpos, verpos);
            ctx.fillStyle = HEAD_COLOR;
            outputChar(char, horpos, verout);
          } else if (rowIndex === col.position - 1) {
            clearChar(horpos, verpos);
            ctx.fillStyle = LEAD_COLOR;
            outputChar(char, horpos, verout);
          } else if (rowIndex === col.position - 2) {
            clearChar(horpos, verpos);
            ctx.fillStyle = MID_COLOR;
            outputChar(char, horpos, verout);
          } else if (rowIndex === col.position - 3) {
            clearChar(horpos, verpos);
            ctx.fillStyle = BODY_COLOR;
            outputChar(char, horpos, verout);
          } else if (
            rowIndex < col.position - 3 &&
            rowIndex >= col.position - numChars + 10 &&
            !randInt(15)
          ) {
            const newChar = randomChar();
            clearChar(horpos, verpos);
            ctx.fillStyle = BODY_COLOR;
            outputChar(newChar, horpos, verout);
          } else if (
            rowIndex < col.position - numChars + 10 &&
            rowIndex > col.position - numChars - 10
          ) {
            ctx.fillStyle = !randInt(5)
              ? "rgba(0, 0, 0, 0.30)"
              : "rgba(0, 0, 0, 0.05)";
            ctx.fillRect(horpos, verpos, COLUMN_SPACING_PX, ROW_SPACING_PX);
          } else if (rowIndex === col.position - numChars - 10) {
            clearChar(horpos, verpos);
          }
        }

        col.delay = col.speed;
        col.position += 1;

        if (col.position > numChars * 2 + 10) {
          col.chars = randomChars(numChars);
          col.position = 0;
          col.delay = randInt((numCols * RAIN_DENSITY) / 2);
        }
      }
    }

    function drawStaticMatrix(): void {
      clearScreen();
      ctx.font = `${RAIN_FONT_SIZE_PX}px matrix_code`;
      for (let colIndex = 0; colIndex < numCols; colIndex += 1) {
        for (let rowIndex = 0; rowIndex < numChars; rowIndex += 1) {
          const roll = Math.random();
          if (roll < 0.04) {
            ctx.fillStyle = HEAD_COLOR;
          } else if (roll < 0.08) {
            ctx.fillStyle = LEAD_COLOR;
          } else if (roll < 0.12) {
            ctx.fillStyle = MID_COLOR;
          } else {
            ctx.fillStyle = BODY_COLOR;
          }
          outputChar(
            randomChar(),
            colIndex * COLUMN_SPACING_PX,
            rowIndex * ROW_SPACING_PX + ROW_SPACING_PX,
          );
        }
      }
    }

    function startRain(): void {
      clearScreen();
      if (reduced.matches) {
        drawStaticMatrix();
        return;
      }
      drawTimer = setInterval(drawScreen, DRAW_INTERVAL_MS);
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
