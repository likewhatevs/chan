import { describe, expect, test } from "vitest";
import graphCanvas from "./GraphCanvas.svelte?raw";

// GraphCanvas mounts before the carousel slide that hosts it becomes
// visible, so the initial resize() clamps the canvas to 0x0 and
// fitToContent(24) returns null. `pendingInitialFit` defers the fit
// until the host reports nonzero dimensions.

describe("GraphCanvas pending-initial-fit", () => {
  test("pendingInitialFit flag is declared with the auto-fit state", () => {
    expect(graphCanvas).toMatch(/let pendingInitialFit = false;/);
  });

  test("start() defers the fit when the host is 0x0", () => {
    expect(graphCanvas).toMatch(
      /if \(cw > 0 && ch > 0\) \{\s*fitToContent\(24\);\s*\} else \{\s*pendingInitialFit = true;\s*\}/,
    );
  });

  test("resize() replays the fit + schedules a refit window on 0->nonzero transition", () => {
    expect(graphCanvas).toMatch(
      /if \(pendingInitialFit && r\.width > 0 && r\.height > 0\) \{[\s\S]{1,500}pendingInitialFit = false;[\s\S]{1,400}fitToContent\(24\);[\s\S]{1,200}scheduleRefit\(900\);/,
    );
  });
});
