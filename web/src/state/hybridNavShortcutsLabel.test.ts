import { describe, expect, test } from "vitest";
import shortcuts from "./shortcuts.ts?raw";

// Pins the title-case "Hybrid Nav" label in shortcuts.ts: no
// "NAV" (all-caps) variant should remain anywhere in the registry.

describe("shortcuts.ts Hybrid Nav label casing", () => {
  test("no `Hybrid NAV` literal remains in shortcuts.ts", () => {
    expect(shortcuts).not.toContain("Hybrid NAV");
  });

  test("no `Hybrid NaV` (intermediate case) literal remains", () => {
    expect(shortcuts).not.toContain("Hybrid NaV");
  });

  test("registry notes carry the title-case `Hybrid Nav` form", () => {
    // Spot-check entries that confirm the rename is complete.
    expect(shortcuts).toContain("Mod+. p (Hybrid Nav)");
    expect(shortcuts).toContain("Mod+. o (Hybrid Nav)");
    expect(shortcuts).toContain("Mod+. t (Hybrid Nav)");
    // The graph chord lives on the `web` field (Mod+. V) with a title-case
    // `Hybrid Nav` note (Cmd+Shift+M is fully retired, web + native), so it
    // spot-checks the capital-V web chord + the note rather than the old
    // `Mod+. v (Hybrid Nav)` parenthetical.
    expect(shortcuts).toContain('web: "Mod+. V"');
    expect(shortcuts).toContain('note: "Hybrid Nav"');
  });

  test("`Enter Hybrid Nav` label reads title-case", () => {
    expect(shortcuts).toContain('"Enter Hybrid Nav"');
  });
});
