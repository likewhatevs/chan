import { describe, expect, test } from "vitest";
import app from "./App.svelte?raw";

// The Cmd+, default chord was dropped in the no-defaults round: "Flip focused
// Hybrid" (app.settings.toggle) is reachable via the launcher and the
// chan:command -> runCommand path, not an onWindowKey keydown matcher. This
// pins that the Cmd+, matcher is gone so the default is not silently re-added.

describe("Cmd+, chord is removed (no-defaults)", () => {
  test("onWindowKey has no e.code === Comma flip matcher", () => {
    expect(app).not.toMatch(/\(e\.code === "Comma" \|\| e\.key === ","\)/);
  });

  test("the flip action survives only on the command path", () => {
    // flipHybrid(layout.activePaneId) remains in the runCommand
    // app.settings.toggle case, but no longer behind a Cmd+, keydown branch.
    expect(app).toMatch(/case "app\.settings\.toggle":/);
    expect(app).not.toMatch(
      /"Comma"[\s\S]{1,200}flipHybrid\(layout\.activePaneId\)/,
    );
  });
});
