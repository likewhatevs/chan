// Bug 5: a pasted image landed at document offset 0 (the first row,
// above the title) when the editor was freshly opened and the user had
// not clicked into the body yet. `pasteInsertPos` gates the caret on
// `view.hasFocus`: trust the caret only when the editor is focused,
// otherwise append at the end of the document so the paste never
// clobbers the first row.

import { describe, expect, test } from "vitest";
import { EditorState } from "@codemirror/state";
import { EditorView } from "@codemirror/view";
import { pasteInsertPos } from "./image_drop";

/// Build a view over `doc` with the caret at `head` and a forced
/// `hasFocus`. CM6's `hasFocus` reads the DOM in jsdom; we override it
/// so the test exercises the branch deterministically without a real
/// focus event.
function viewWith(doc: string, head: number, hasFocus: boolean): EditorView {
  const view = new EditorView({
    state: EditorState.create({
      doc,
      selection: { anchor: head },
    }),
  });
  Object.defineProperty(view, "hasFocus", {
    get: () => hasFocus,
    configurable: true,
  });
  return view;
}

describe("pasteInsertPos (bug 5)", () => {
  const doc = "# Title\n\nbody line\n";

  test("focused editor: insert at the caret", () => {
    const head = 3; // mid-title
    const view = viewWith(doc, head, true);
    expect(pasteInsertPos(view)).toBe(head);
    view.destroy();
  });

  test("unfocused editor with caret at 0: append at end, not row 1", () => {
    const view = viewWith(doc, 0, false);
    expect(pasteInsertPos(view)).toBe(doc.length);
    expect(pasteInsertPos(view)).not.toBe(0);
    view.destroy();
  });

  test("unfocused editor ignores a stale mid-doc caret too", () => {
    // Even with a non-zero stale caret, an unfocused paste appends:
    // the caret is not a reliable signal when focus is elsewhere.
    const view = viewWith(doc, 5, false);
    expect(pasteInsertPos(view)).toBe(doc.length);
    view.destroy();
  });
});
