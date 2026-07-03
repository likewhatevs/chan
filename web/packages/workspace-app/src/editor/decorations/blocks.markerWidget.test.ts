// @vitest-environment jsdom

// Guards the WKWebView list-marker repaint fix: the `-` (hyphen) and `1.`
// (ordered) markers must render through a REPLACE WIDGET, not a Decoration.mark.
//
// A Decoration.mark (a class on the existing marker text) does not force
// chan-desktop's WKWebView to repaint the line when the list decoration first
// applies, so typing `- ` or `1. ` left the item un-flowed (no hanging indent)
// until an unrelated event forced a repaint - the "sporadic list mode" bug.
// A replace widget swaps in a real DOM node, forcing the line to re-layout and
// apply the hanging-indent line decoration with it, exactly as the `*` / `+`
// glyph already does. Blink/WebView2 repaint either way, so this is invisible
// off WKWebView; reverting to a mark reintroduces the desktop bug.

import blocksSrc from "./blocks.ts?raw";
import { describe, expect, test } from "vitest";

// The rendered DOM (that a `-` / `1.` list decorates its marker with the hyphen
// / ordered class) is already covered by blocks.test.ts; this file pins the
// MECHANISM: those markers must go through a replace widget, not a mark.
describe("list markers render as replace widgets (WKWebView repaint fix)", () => {
  test("hyphen and ordered markers use LiteralMarkerWidget, not Decoration.mark", () => {
    // The hyphen marker branch returns a replace widget.
    expect(blocksSrc).toMatch(
      /markerChar === "-"[\s\S]*?Decoration\.replace\(\{\s*widget: new LiteralMarkerWidget/,
    );
    // The ordered marker is pushed as a replace widget carrying the marker text.
    expect(blocksSrc).toMatch(
      /Decoration\.replace\(\{\s*widget: new LiteralMarkerWidget\(markerText/,
    );
    // The old mark constants are gone (reverting to them reintroduces the bug).
    expect(blocksSrc).not.toMatch(/const HYPHEN_MARK\b/);
    expect(blocksSrc).not.toMatch(/const ORDERED_MARK\b/);
  });
});
