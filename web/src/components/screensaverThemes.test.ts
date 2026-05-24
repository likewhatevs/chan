import { describe, expect, test } from "vitest";
import matrix from "./screensaver/MatrixRain.svelte?raw";

describe("fullstack-a-99: screensaver canvas themes", () => {
  test("MatrixRain is a full-bleed canvas with reduced-motion handling", () => {
    expect(matrix).toMatch(/<canvas bind:this=\{canvas\} class="matrix-rain"/);
    expect(matrix).toMatch(/prefers-reduced-motion: reduce/);
    expect(matrix).toMatch(/drawStaticMatrix\(\)/);
    expect(matrix).toMatch(/abcdefghijklmnopqrstuvwxyz123456789890/);
  });

  test("Matrix intro matches the reference message sequence and timing", () => {
    expect(matrix).toMatch(/INTRO_MESSAGES = \["Wake up, Neo\.\.\.", "The Matrix has you\.\.\."\]/);
    expect(matrix).toMatch(/INTRO_START_DELAY_MS = 500/);
    expect(matrix).toMatch(/INTRO_HOLD_MS = 2000/);
    expect(matrix).toMatch(/TYPE_DELAY_SLOW_MS = 300/);
    expect(matrix).toMatch(/TYPE_DELAY_FAST_MS = 100/);
    expect(matrix).toMatch(/INTRO_FONT_SIZE_PX = 22/);
    expect(matrix).toMatch(/ctx\.fillText\(text, 30, 40\)/);
  });

  test("Matrix rain follows the dcragusa reference spacing, cadence, and color tiers", () => {
    expect(matrix).toMatch(/DRAW_INTERVAL_MS = 40/);
    expect(matrix).toMatch(/COLUMN_SPACING_PX = 11/);
    expect(matrix).toMatch(/ROW_SPACING_PX = 19/);
    expect(matrix).toMatch(/RAIN_FONT_SIZE_PX = 20/);
    expect(matrix).toMatch(/RAIN_DENSITY = 4/);
    expect(matrix).toMatch(/HEAD_COLOR = "#f6f6f4"/);
    expect(matrix).toMatch(/LEAD_COLOR = "#c9cfb9"/);
    expect(matrix).toMatch(/MID_COLOR = "#95a297"/);
    expect(matrix).toMatch(/BODY_COLOR = "#2cb231"/);
    expect(matrix).toMatch(/Math\.random\(\) < 1 \/ 15/);
    expect(matrix).toMatch(/rgba\(0, 0, 0, 0\.30\)/);
    expect(matrix).toMatch(/rgba\(0, 0, 0, 0\.05\)/);
  });
});
