import { describe, expect, test } from "vitest";
import pane from "./Pane.svelte?raw";
import paneModeHelp from "./PaneModeHelp.svelte?raw";

// fullstack-62: user-facing copy renamed from "Pane Mode" to
// "Hybrid NAV" (NAV uppercase per the locked wording). Internal
// symbols (paneMode, paneModeKeymap, etc.) stay; CSS class names
// stay; comments stay. This sentinel pins the visible-text rename
// so a future refactor can't silently regress to old copy.

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

describe("fullstack-62: Pane Mode → Hybrid NAV user-facing rename", () => {
  test("Pane.svelte hamburger entry reads Enter Hybrid NAV", () => {
    expect(pane).toContain(">Enter Hybrid NAV<");
  });

  test("Pane.svelte Hybrid NAV preview aria-label uses the new copy", () => {
    expect(pane).toContain('aria-label="Hybrid NAV preview"');
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

  test("PaneModeHelp.svelte title + aria-label use Hybrid NAV", () => {
    expect(paneModeHelp).toContain('aria-label="Hybrid NAV help"');
    // `fullstack-a-19` extended the title to include the entry chord
    // (`(Cmd+.)`) so the cheatsheet's header doubles as a docs-side
    // pin of the entry binding. Still asserts the Hybrid NAV brand,
    // just with the chord suffix.
    expect(paneModeHelp).toContain(">Hybrid NAV (Cmd+.)<");
  });

  test("PaneModeHelp.svelte renders no user-facing 'Pane Mode' string", () => {
    const visible = stripCommentsAndCss(paneModeHelp);
    expect(visible).not.toContain(">Pane Mode<");
    expect(visible).not.toMatch(/aria-label="[^"]*Pane Mode[^"]*"/);
  });
});
