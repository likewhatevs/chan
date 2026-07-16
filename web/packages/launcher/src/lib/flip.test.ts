// The flip-axis algorithm's behavior, plus source pins mirroring the
// workspace-app Pane.test.ts "flip axis follows pane dimensions" pins: the
// launcher's flip is a deliberate copy of the pane side flip (extraction is
// forbidden by Pane's own ?raw pins), so the same load-bearing strings are
// pinned HERE to keep the two copies from drifting apart silently.

import { describe, expect, test, vi } from "vitest";
import { flipAxisForElement, flipTransforms } from "./flip";
import flipSource from "./flip.ts?raw";
import shellSource from "../components/ScreenFlip.svelte?raw";

function fakeEl(width: number, height: number): HTMLElement {
  const el = document.createElement("div");
  el.getBoundingClientRect = () =>
    ({
      x: 0,
      y: 0,
      top: 0,
      left: 0,
      right: width,
      bottom: height,
      width,
      height,
      toJSON: () => ({}),
    }) as DOMRect;
  return el;
}

describe("flip axis", () => {
  test("tall areas turn vertically, wide areas horizontally", () => {
    expect(flipAxisForElement(fakeEl(120, 320))).toBe("vertical");
    expect(flipAxisForElement(fakeEl(320, 120))).toBe("horizontal");
  });

  test("a square area chooses either axis", () => {
    const random = vi.spyOn(Math, "random");
    try {
      random.mockReturnValue(0.2);
      expect(flipAxisForElement(fakeEl(200, 200))).toBe("vertical");
      random.mockReturnValue(0.8);
      expect(flipAxisForElement(fakeEl(200, 200))).toBe("horizontal");
    } finally {
      random.mockRestore();
    }
  });

  test("a missing element reads as square", () => {
    const random = vi.spyOn(Math, "random");
    try {
      random.mockReturnValue(0.2);
      expect(flipAxisForElement(null)).toBe("vertical");
    } finally {
      random.mockRestore();
    }
  });

  test("transforms follow the axis", () => {
    expect(flipTransforms("vertical")).toEqual({
      start: "rotateY(-180deg)",
      back: "rotateY(-180deg)",
    });
    expect(flipTransforms("horizontal")).toEqual({
      start: "rotateX(-180deg)",
      back: "rotateX(-180deg)",
    });
  });
});

describe("pane-flip copy pins", () => {
  test("the axis algorithm matches the pane original", () => {
    expect(flipSource).toMatch(/if \(height > width\) return "vertical"/);
    expect(flipSource).toMatch(/if \(width > height\) return "horizontal"/);
    expect(flipSource).toMatch(/return Math\.random\(\) < 0\.5 \? "vertical" : "horizontal"/);
    expect(flipSource).toMatch(/axis === "vertical" \? "rotateY" : "rotateX"/);
    expect(flipSource).toContain("start: `${rotate}(-180deg)`,");
    expect(flipSource).toContain("back: `${rotate}(-180deg)`,");
  });

  test("the flip shell keeps the pane's card mechanics", () => {
    expect(shellSource).toMatch(/class:flipActive=\{flipActive\}/);
    expect(shellSource).toContain('class="screen-flip-inner"');
    expect(shellSource).toMatch(/backface-visibility: hidden/);
    expect(shellSource).toMatch(/@keyframes launcher-screen-flip/);
    expect(shellSource).toMatch(/transform: var\(--screen-flip-start\)/);
    expect(shellSource).toMatch(/rotateX\(0deg\) rotateY\(0deg\)/);
    expect(shellSource).toContain("520ms cubic-bezier(0.2, 0.7, 0.2, 1)");
    expect(shellSource).toMatch(/prefers-reduced-motion/);
  });
});
