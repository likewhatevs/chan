import { describe, expect, test } from "vitest";
import matrix from "./screensaver/MatrixRain.svelte?raw";
import matrixHelper from "./screensaver/matrixRain.ts?raw";

// The static-frame primitive, alphabet, spacing/size and color-tier
// constants were extracted to the shared `matrixRain.ts` helper so the
// fullscreen screensaver (MatrixRain.svelte) and the config-panel preview
// (MatrixRainPreview.svelte) render an identical static frame without
// forking. Definitions are asserted in the helper (`matrixHelper`); the
// live rain loop, intro, cadence/density, canvas markup and CSS stay in
// the screensaver component (`matrix`).

describe("screensaver canvas themes", () => {
  test("MatrixRain is a full-bleed canvas with reduced-motion handling", () => {
    expect(matrix).toMatch(/<canvas bind:this=\{canvas\} class="matrix-rain"/);
    expect(matrix).toMatch(/prefers-reduced-motion: reduce/);
    // Shared static-frame primitive + alphabet now live in the helper,
    // and MatrixRain imports them.
    expect(matrixHelper).toMatch(/export function drawStaticMatrix\(/);
    expect(matrixHelper).toMatch(/abcdefghijklmnopqrstuvwxyz123456789890/);
    expect(matrix).toMatch(/from "\.\/matrixRain"/);
    expect(matrix).toMatch(/font-family: "matrix_code"/);
    expect(matrix).toMatch(/font-family: "matrix_courier"/);
    expect(matrix).toMatch(/LICENSE-MatrixScreensaver\.txt/);
  });

  test("Matrix intro matches the reference message sequence and timing", () => {
    expect(matrix).toMatch(/INTRO_MESSAGES = \["Wake up, Neo\.\.\.", "The Matrix has you\.\.\."\]/);
    expect(matrix).toMatch(/INTRO_START_DELAY_MS = 500/);
    expect(matrix).toMatch(/INTRO_HOLD_MS = 2000/);
    expect(matrix).toMatch(/TYPE_DELAY_SLOW_MS = 300/);
    expect(matrix).toMatch(/TYPE_DELAY_FAST_MS = 100/);
    expect(matrix).toMatch(/INTRO_FONT_SIZE_PX = 22/);
    expect(matrix).toMatch(/document\.fonts\.load\(`\$\{INTRO_FONT_SIZE_PX\}px matrix_courier`\)/);
    expect(matrix).toMatch(/index \* \(COLUMN_SPACING_PX \+ 2\) \+ 30/);
    expect(matrix).toMatch(/outputChar\([\s\S]{1,120}40,/);
  });

  test("Matrix rain follows the dcragusa reference spacing, cadence, and color tiers", () => {
    // Cadence + density are the live-loop's own; the spacing/size and the
    // color tiers are shared constants in the helper.
    expect(matrix).toMatch(/DRAW_INTERVAL_MS = 40/);
    expect(matrixHelper).toMatch(/COLUMN_SPACING_PX = 11/);
    expect(matrixHelper).toMatch(/ROW_SPACING_PX = 19/);
    expect(matrixHelper).toMatch(/RAIN_FONT_SIZE_PX = 20/);
    expect(matrix).toMatch(/RAIN_DENSITY = 4/);
    expect(matrixHelper).toMatch(/HEAD_COLOR = "#f6f6f4"/);
    expect(matrixHelper).toMatch(/LEAD_COLOR = "#c9cfb9"/);
    expect(matrixHelper).toMatch(/MID_COLOR = "#95a297"/);
    expect(matrixHelper).toMatch(/BODY_COLOR = "#2cb231"/);
    expect(matrix).toMatch(/document\.fonts\.load\(`\$\{RAIN_FONT_SIZE_PX\}px matrix_code`\)/);
    expect(matrix).toMatch(/!randInt\(15\)/);
    expect(matrix).toMatch(/rgba\(0, 0, 0, 0\.30\)/);
    expect(matrix).toMatch(/rgba\(0, 0, 0, 0\.05\)/);
  });
});
