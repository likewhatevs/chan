import { describe, expect, test } from "vitest";
import app from "./App.svelte?raw";

// The Cmd+, keymap handler in App.svelte's onWindowKey is layout-
// independent (matches `e.code === "Comma"` ahead of the `e.key === ","`
// form) and calls stopImmediatePropagation so no downstream listener can
// re-trigger flipHybrid in the same tick. It requires meta + !shift +
// !alt so Cmd+Shift+, / Cmd+Alt+, stay free for future bindings.

describe("Cmd+, matcher is layout-independent + stops propagation", () => {
  test("matcher uses e.code === Comma ahead of e.key === ',' fallback", () => {
    expect(app).toMatch(
      /if \(\s*meta &&\s*!e\.shiftKey &&\s*!e\.altKey &&\s*\(e\.code === "Comma" \|\| e\.key === ","\)\s*\) \{/,
    );
  });

  test("matcher preventDefaults AND stopImmediatePropagation before flipping", () => {
    expect(app).toMatch(
      /\(e\.code === "Comma" \|\| e\.key === ","\)\s*\) \{[\s\S]{1,300}e\.preventDefault\(\);[\s\S]{1,200}e\.stopImmediatePropagation\(\);[\s\S]{1,200}flipHybrid\(layout\.activePaneId\);/,
    );
  });
});
