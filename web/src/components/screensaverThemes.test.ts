import { describe, expect, test } from "vitest";
import matrix from "./screensaver/MatrixRain.svelte?raw";
import castaway from "./screensaver/Castaway.svelte?raw";

describe("fullstack-a-99: screensaver canvas themes", () => {
  test("MatrixRain is a full-bleed canvas with reduced-motion handling", () => {
    expect(matrix).toMatch(/<canvas bind:this=\{canvas\} class="matrix-rain"/);
    expect(matrix).toMatch(/prefers-reduced-motion: reduce/);
    expect(matrix).toMatch(/requestAnimationFrame\(draw\)/);
    expect(matrix).toMatch(/ｱｲｳｴｵ/);
  });

  test("Castaway is a full-bleed canvas with at least five animation states", () => {
    expect(castaway).toMatch(/<canvas bind:this=\{canvas\} class="castaway"/);
    expect(castaway).toMatch(/type SceneState = "idle" \| "wave" \| "sit" \| "sleep" \| "drink" \| "walk" \| "fish" \| "ship";/);
    expect(castaway).toMatch(/function drawPalm/);
    expect(castaway).toMatch(/function drawCharacter/);
  });
});
