import { describe, expect, test } from "vitest";
import graphCanvas from "./GraphCanvas.svelte?raw";

// Phase-13 round-1 closing (B12): the carousel's indexing slide
// mounts GraphCanvas before slide 2 is visible. The host's first
// `resize()` therefore clamps the canvas to 0x0, the initial
// `fitToContent(24)` returns null, and the view stays at the
// origin/zoom-1 placeholder until the user manually zooms.
// `pendingInitialFit` defers the fit until the host reports
// nonzero dimensions so the spine ends up framed in the viewport
// automatically.

describe("phase-13 round-1 closing B12: GraphCanvas pending-initial-fit", () => {
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
