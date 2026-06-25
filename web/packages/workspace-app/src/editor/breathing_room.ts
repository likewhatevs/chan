// Google Docs-style "the page lifts under the cursor" effect for the
// CM6 editors. Two pieces:
//
//   1. A static 60px padding-bottom on `.cm-content` (in each
//      editor's <style>) so there's always physical room below the
//      last line for the viewport to scroll into.
//   2. No bottom scrollMargin during regular typing. CM's default
//      behavior only scrolls when the cursor would leave the visible
//      viewport; the padding above gives it room to do that at EOF
//      without pre-emptively moving on every keystroke.
//
// The smooth motion comes from `scroll-behavior: smooth` on the
// scroller; CM sets `scrollTop` programmatically and the browser
// animates the change. No JS animation, no per-keystroke measuring.
//
// Tunable below.

import { EditorView } from "@codemirror/view";

const BOTTOM_MARGIN_PX = 0;

export function breathingRoom() {
  return EditorView.scrollMargins.of(() => ({ bottom: BOTTOM_MARGIN_PX }));
}
