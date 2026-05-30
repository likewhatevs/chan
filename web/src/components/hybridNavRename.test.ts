import { describe, expect, test } from "vitest";
import pane from "./Pane.svelte?raw";
import paneModeHelp from "./PaneModeHelp.svelte?raw";

// fullstack-62 + fullstack-a-68 slice 1: user-facing copy
// settled on "Hybrid Nav" (Nav title-case). fullstack-62
// renamed Pane Mode → Hybrid NAV (all-caps); fullstack-a-68
// slice 1 demotes the all-caps to title-case per @@Alex's
// addendum-a flag ("NAame/moV -> Nav"). Internal symbols
// (paneMode, paneModeKeymap, etc.) stay; CSS class names
// stay; comments stay. This sentinel pins the visible-text
// rename + guards against regressions to either older form.

function stripCommentsAndCss(src: string): string {
  // Strip multi-line comments (/* ... */) so JSDoc explanations
  // about "Pane Mode" semantics don't trip the negative match.
  const noBlock = src.replace(/\/\*[\s\S]*?\*\//g, "");
  // Strip line comments (// ...).
  const noLine = noBlock.replace(/\/\/.*$/gm, "");
  // Drop HTML / Svelte comments (<!-- ... -->) which discuss
  // historical phase work, also not user-facing.
  const noHtml = noLine.replace(/<!--[\s\S]*?-->/g, "");
  // Strip the <style> block — class names like `pane-mode-flash`
  // are internal CSS hooks, not user copy.
  const noStyle = noHtml.replace(/<style[\s\S]*?<\/style>/g, "");
  return noStyle;
}

describe("Pane Mode → Hybrid Nav user-facing rename", () => {
  test("Pane.svelte hamburger entry reads Enter Hybrid Nav", () => {
    expect(pane).toContain(">Enter Hybrid Nav<");
  });

  test("Pane.svelte Hybrid Nav preview aria-label uses the new copy", () => {
    expect(pane).toContain('aria-label="Hybrid Nav preview"');
  });

  test("Pane.svelte renders no user-facing 'Pane Mode' string", () => {
    const visible = stripCommentsAndCss(pane);
    // Visible text in Svelte templates lives between tags
    // (`>Pane Mode<`) or inside attribute values
    // (`aria-label="Pane Mode …"`). Internal references like
    // `paneMode.active` (variable access) survive the strip and
    // are intentional; they're not rendered to the user.
    expect(visible).not.toContain(">Pane Mode<");
    expect(visible).not.toMatch(/aria-label="[^"]*Pane Mode[^"]*"/);
    expect(visible).not.toMatch(/title="[^"]*Pane Mode[^"]*"/);
  });

  test("Pane.svelte no longer renders all-caps 'Hybrid NAV' in visible copy", () => {
    // `fullstack-a-68` slice 1: title-case "Nav" replaces the
    // all-caps form @@Alex flagged. Internal CSS classes like
    // `.pane-mode-flash` aren't user-facing; the strip helper
    // covers the relevant surfaces.
    const visible = stripCommentsAndCss(pane);
    expect(visible).not.toContain(">Enter Hybrid NAV<");
    expect(visible).not.toMatch(/aria-label="[^"]*Hybrid NAV[^"]*"/);
  });

  test("PaneModeHelp.svelte title + aria-label use Hybrid Nav", () => {
    expect(paneModeHelp).toContain('aria-label="Hybrid Nav help"');
    // `fullstack-a-19` extended the title to include the entry chord
    // (`(Cmd+.)`) so the cheatsheet's header doubles as a docs-side
    // pin of the entry binding. Still asserts the Hybrid Nav brand,
    // just with the chord suffix.
    expect(paneModeHelp).toContain(">Hybrid Nav (Cmd+.)<");
  });

  test("PaneModeHelp.svelte renders no user-facing 'Pane Mode' string", () => {
    const visible = stripCommentsAndCss(paneModeHelp);
    expect(visible).not.toContain(">Pane Mode<");
    expect(visible).not.toMatch(/aria-label="[^"]*Pane Mode[^"]*"/);
  });

  test("PaneModeHelp.svelte no longer renders all-caps 'Hybrid NAV' in visible copy", () => {
    const visible = stripCommentsAndCss(paneModeHelp);
    expect(visible).not.toContain(">Hybrid NAV (Cmd+.)<");
    expect(visible).not.toMatch(/aria-label="[^"]*Hybrid NAV[^"]*"/);
  });
});
