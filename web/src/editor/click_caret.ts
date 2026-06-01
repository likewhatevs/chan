// Place the caret when a click lands in a row's blank space rather than
// directly on a glyph.
//
// The WYSIWYG editor caps `.cm-editor` to a centered page width and
// pads `.cm-content` (60px at the bottom), so a click to the right of a
// short line, in a row's empty trailing space, or below the last line
// misses every glyph box. CodeMirror's default mouse handling hit-tests
// precisely there and, finding nothing, leaves the caret where it was -
// the click reads as "dead". We resolve those clicks to the nearest
// document position so the caret follows the pointer (a past-EOL click
// lands at the end of that row's text).
//
// Conservative by construction: we only act when the PRECISE hit-test
// misses (`posAtCoords(coords)` returns null). A click that lands on a
// glyph still resolves precisely, so CM6's default handling - and the
// widget/pill mousedown handlers that stop propagation before this runs
// - are untouched. Mirrors the existing `listCaretGuard` pattern.

import { EditorView } from "@codemirror/view";

export function clickToPlaceCaret(): ReturnType<
  typeof EditorView.domEventHandlers
> {
  return EditorView.domEventHandlers({
    mousedown(event, view) {
      // Plain primary single-clicks only. Modified clicks (shift to
      // extend, alt for rectangular select), multi-clicks (word / line
      // select), and non-primary buttons fall through to CM6's defaults.
      if (event.button !== 0 || event.detail > 1) return false;
      if (event.shiftKey || event.altKey || event.metaKey || event.ctrlKey) {
        return false;
      }
      const coords = { x: event.clientX, y: event.clientY };
      // A precise hit means a glyph sits under the cursor; CM6 places the
      // caret correctly there, so leave it alone.
      if (view.posAtCoords(coords) !== null) return false;
      // No precise hit: the click is in a row's blank space. The
      // non-precise resolve never returns null for an in-editor point
      // and already snaps a past-EOL click to that line's text end.
      const near = view.posAtCoords(coords, false);
      if (near === null) return false;
      event.preventDefault();
      view.dispatch({ selection: { anchor: near } });
      view.focus();
      return true;
    },
  });
}
