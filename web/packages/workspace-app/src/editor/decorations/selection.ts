// Selection helpers used by the decoration walker.
//
// Two visibility primitives, per design.md spec #4:
//
//   selectionInRange(sel, from, to)
//     true if any selection range touches [from, to]. Boundary equality
//     counts as intersection - caret AT a marker's edge reveals the
//     marker. This is the rule for inline marks (bold, italic, strike,
//     code, link/image markers, wikilink brackets).
//
//   lineIntersect(state, from, to, sel)
//     true if any selection range's line span overlaps the line span of
//     [from, to]. Block prefixes (heading `#`, list bullet, blockquote
//     `>`, code fence) use this so they don't flicker on/off as the
//     caret crosses the prefix mid-line.
//
// Pure functions; no CM6 view dependency. Intended to be cheap (no
// allocations beyond computing line objects via state.doc.lineAt).

import type { EditorSelection, EditorState } from "@codemirror/state";

export function selectionInRange(
  sel: EditorSelection,
  from: number,
  to: number,
): boolean {
  for (const r of sel.ranges) {
    if (r.from <= to && r.to >= from) return true;
  }
  return false;
}

export function lineIntersect(
  state: EditorState,
  from: number,
  to: number,
  sel: EditorSelection,
): boolean {
  const tokenLineStart = state.doc.lineAt(from).from;
  const tokenLineEnd = state.doc.lineAt(to).to;
  for (const r of sel.ranges) {
    const selLineStart = state.doc.lineAt(r.from).from;
    const selLineEnd = state.doc.lineAt(r.to).to;
    if (selLineStart <= tokenLineEnd && selLineEnd >= tokenLineStart) {
      return true;
    }
  }
  return false;
}
