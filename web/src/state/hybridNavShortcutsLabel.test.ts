import { describe, expect, test } from "vitest";
import shortcuts from "./shortcuts.ts?raw";

// `fullstack-a-68 slice 1b`: catch the missed `shortcuts.ts`
// "NAV" label remnants from slice 1's rename. @@WebtestA's
// walk (`3328d57`) flagged PARTIAL on the original sweep.

describe("fullstack-a-68 slice 1b: shortcuts.ts NAV label sweep", () => {
  test("no `Hybrid NAV` literal remains in shortcuts.ts", () => {
    expect(shortcuts).not.toContain("Hybrid NAV");
  });

  test("no `Hybrid NaV` (intermediate case) literal remains", () => {
    expect(shortcuts).not.toContain("Hybrid NaV");
  });

  test("registry notes carry the title-case `Hybrid Nav` form", () => {
    // Spot-check a few entries that the original slice 1
    // sweep should have caught but didn't (per webtest).
    expect(shortcuts).toContain("Mod+. p (Hybrid Nav)");
    expect(shortcuts).toContain("Mod+. o (Hybrid Nav)");
    expect(shortcuts).toContain("Mod+. v (Hybrid Nav)");
    expect(shortcuts).toContain("Mod+. t (Hybrid Nav)");
  });

  test("`Enter Hybrid Nav` label reads title-case", () => {
    expect(shortcuts).toContain('"Enter Hybrid Nav"');
  });
});
