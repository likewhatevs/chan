// Google Docs-style "the page lifts under the cursor" effect for the
// CM6 editors. Two pieces:
//
//   1. A static 30px padding-bottom on `.cm-content` (in each
//      editor's <style>) so there's always physical room below the
//      last line for the viewport to scroll into.
//   2. A 30px bottom scrollMargin so CM's internal scrollIntoView
//      (typing, arrow-key moves, dispatched scroll effects) keeps
//      the caret at least 30px above the scroller's bottom edge.
//
// The smooth motion comes from `scroll-behavior: smooth` on the
// scroller; CM sets `scrollTop` programmatically and the browser
// animates the change. No JS animation, no per-keystroke measuring.
//
// Tunable below.

import { EditorView } from "@codemirror/view";

const BOTTOM_MARGIN_PX = 60;

export function breathingRoom() {
  return EditorView.scrollMargins.of(() => ({ bottom: BOTTOM_MARGIN_PX }));
}
