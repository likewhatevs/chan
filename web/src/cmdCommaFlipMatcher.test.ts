import { describe, expect, test } from "vitest";
import app from "./App.svelte?raw";

// Phase-13 round-1 closing (B2): the Cmd+, keymap handler
// in App.svelte's onWindowKey was rewritten to be layout-
// independent (matches `e.code === "Comma"` ahead of the
// `e.key === ","` legacy form) and to stopImmediatePropagation
// so no downstream listener can re-trigger flipHybrid in the
// same tick. The matcher also continues to require meta +
// !shift + !alt so Cmd+Shift+, / Cmd+Alt+, stay free for
// future bindings.

describe("phase-13 round-1 closing B2: Cmd+, matcher is layout-independent + stops propagation", () => {
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

  test("rationale comment cites the second-press-no-op regression", () => {
    expect(app).toMatch(/Phase-13 round-1 closing \(B2\)/);
    expect(app).toMatch(/second press is a no-op/);
  });
});
