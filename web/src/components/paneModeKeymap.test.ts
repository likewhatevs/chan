// Raw-source assertions on the Cmd+K (pane mode) keymap wired in
// App.svelte. The dispatcher lives inline inside the App component
// and is hard to mount in isolation, so this gate reads the source
// directly and checks that the arrow → move-focus and WASD → swap
// arrangement from `fullstack-40` is preserved. Catches accidental
// regressions if someone "fixes" the mapping back to the
// `fullstack-16` defaults.

import { describe, expect, test } from "vitest";
import app from "../App.svelte?raw";

describe("Cmd+K pane mode keymap (fullstack-40 inversion)", () => {
  test("arrow keys move focus", () => {
    expect(app).toContain('case "ArrowUp":\n        paneModeMoveFocus("up");');
    expect(app).toContain('case "ArrowLeft":\n        paneModeMoveFocus("left");');
    expect(app).toContain('case "ArrowDown":\n        paneModeMoveFocus("down");');
    expect(app).toContain('case "ArrowRight":\n        paneModeMoveFocus("right");');
  });

  test("WASD swaps tiles", () => {
    expect(app).toContain('case "w":\n      case "W":\n        paneModeSwap("up");');
    expect(app).toContain('case "a":\n      case "A":\n        paneModeSwap("left");');
    expect(app).toContain('case "s":\n      case "S":\n        paneModeSwap("down");');
    expect(app).toContain('case "d":\n      case "D":\n        paneModeSwap("right");');
  });
});
