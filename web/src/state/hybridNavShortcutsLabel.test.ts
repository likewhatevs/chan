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
    // The graph chord moved onto the `web` field (Mod+. V) and its note
    // now carries the native-retirement meaning, so it spot-checks the
    // title-case form in the note + the capital-V web chord rather than
    // the old `Mod+. v (Hybrid Nav)` parenthetical.
    expect(shortcuts).toContain('web: "Mod+. V"');
    expect(shortcuts).toContain(
      'note: "Hybrid Nav; native Cmd+Shift+M pending retirement"',
    );
  });

  test("`Enter Hybrid Nav` label reads title-case", () => {
    expect(shortcuts).toContain('"Enter Hybrid Nav"');
  });
});
