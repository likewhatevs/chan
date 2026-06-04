// @vitest-environment jsdom
//
// Coverage for list-editing keybinds. Each test mounts a real
// EditorView in jsdom, dispatches a selection, and inspects the
// post-command state. Going through the live EditorView (rather
// than poking at internal helpers) lets us catch regressions in
// the dispatch wiring as well as the regex itself.

import { describe, expect, test, beforeEach, afterEach } from "vitest";
import { EditorState } from "@codemirror/state";
import { EditorView } from "@codemirror/view";
import {
  continueListOnEnter,
  indentListItem,
  outdentListItem,
  stripUnusedInlineImageSpaceOnEnter,
} from "./list";

let host: HTMLDivElement;
let view: EditorView;

function mount(doc: string, caret: number): void {
  host = document.createElement("div");
  document.body.append(host);
  view = new EditorView({
    state: EditorState.create({ doc, selection: { anchor: caret } }),
    parent: host,
  });
}

function snapshot(): { doc: string; head: number } {
  return {
    doc: view.state.doc.toString(),
    head: view.state.selection.main.head,
  };
}

beforeEach(() => {
  // jsdom carries DOM state across tests; we mount per-test below.
});

afterEach(() => {
  view?.destroy();
  host?.remove();
});

describe("continueListOnEnter", () => {
  test("inserts a fresh bullet at end of a `- ` line", () => {
    mount("- one", 5);
    expect(continueListOnEnter(view)).toBe(true);
    expect(snapshot()).toEqual({ doc: "- one\n- ", head: 8 });
  });

  test("respects the bullet character (`*` stays `*`)", () => {
    mount("* a", 3);
    expect(continueListOnEnter(view)).toBe(true);
    expect(snapshot().doc).toBe("* a\n* ");
  });

  test("increments an ordered marker", () => {
    mount("1. first", 8);
    expect(continueListOnEnter(view)).toBe(true);
    expect(snapshot().doc).toBe("1. first\n2. ");
  });

  test("preserves the `)` separator on ordered markers", () => {
    mount("3) third", 8);
    expect(continueListOnEnter(view)).toBe(true);
    expect(snapshot().doc).toBe("3) third\n4) ");
  });

  test("task list emits a fresh unchecked box", () => {
    mount("- [x] done", 10);
    expect(continueListOnEnter(view)).toBe(true);
    expect(snapshot().doc).toBe("- [x] done\n- [ ] ");
  });

  test("preserves indentation", () => {
    mount("  - nested", 10);
    expect(continueListOnEnter(view)).toBe(true);
    expect(snapshot().doc).toBe("  - nested\n  - ");
  });

  test("empty bullet exits the list (strips the prefix)", () => {
    mount("- ", 2);
    expect(continueListOnEnter(view)).toBe(true);
    expect(snapshot()).toEqual({ doc: "", head: 0 });
  });

  test("empty nested bullet strips the whole prefix incl. indent", () => {
    mount("  - ", 4);
    expect(continueListOnEnter(view)).toBe(true);
    expect(snapshot()).toEqual({ doc: "", head: 0 });
  });

  test("empty task item exits the list", () => {
    mount("- [ ] ", 6);
    expect(continueListOnEnter(view)).toBe(true);
    expect(snapshot().doc).toBe("");
  });

  test("falls through (returns false) on a non-list line", () => {
    mount("hello", 5);
    expect(continueListOnEnter(view)).toBe(false);
    expect(snapshot().doc).toBe("hello"); // untouched
  });

  test("falls through when caret is mid-line on a non-empty item", () => {
    mount("- hello", 4); // caret between '- h' and 'ello'
    expect(continueListOnEnter(view)).toBe(false);
    expect(snapshot().doc).toBe("- hello");
  });

  test("falls through when selection is non-empty", () => {
    mount("- hello", 2);
    view.dispatch({ selection: { anchor: 2, head: 5 } });
    expect(continueListOnEnter(view)).toBe(false);
  });
});

describe("indentListItem / outdentListItem", () => {
  test("Tab adds 2 spaces of indent on a list line", () => {
    mount("- a", 3);
    expect(indentListItem(view)).toBe(true);
    expect(snapshot().doc).toBe("  - a");
  });

  test("Tab is a no-op (returns false) on a non-list line", () => {
    mount("plain", 5);
    expect(indentListItem(view)).toBe(false);
    expect(snapshot().doc).toBe("plain");
  });

  test("Shift-Tab strips 2 spaces from an indented list line", () => {
    mount("    - a", 7);
    expect(outdentListItem(view)).toBe(true);
    expect(snapshot().doc).toBe("  - a");
  });

  test("Shift-Tab on a top-level list item is a no-op (keeps the bullet)", () => {
    // R2-2: a top-level item is already at the outermost level, so
    // outdent must NOT strip the marker (that silently demoted the
    // bullet to a plain paragraph). Still consumed (returns true) so
    // Shift-Tab never escapes the editor; the doc is unchanged.
    mount("- a", 3);
    expect(outdentListItem(view)).toBe(true);
    expect(snapshot().doc).toBe("- a");
  });

  test("Shift-Tab on a top-level task item keeps the bullet", () => {
    mount("- [x] done", 10);
    expect(outdentListItem(view)).toBe(true);
    expect(snapshot().doc).toBe("- [x] done");
  });

  test("Shift-Tab on a non-list line is an editor-local no-op", () => {
    mount("plain", 5);
    expect(outdentListItem(view)).toBe(true);
    expect(snapshot().doc).toBe("plain");
  });

  test("Shift-Tab strips only 1 space when only 1 space of indent", () => {
    mount(" - a", 4);
    expect(outdentListItem(view)).toBe(true);
    expect(snapshot().doc).toBe("- a");
  });

  test("Tab indents every list line in a multi-line selection", () => {
    mount("- a\n- b\n- c", 0);
    view.dispatch({ selection: { anchor: 0, head: 11 } });
    expect(indentListItem(view)).toBe(true);
    expect(snapshot().doc).toBe("  - a\n  - b\n  - c");
  });

  test("Tab leaves non-list lines untouched within a multi-line selection", () => {
    mount("- a\nplain\n- c", 0);
    view.dispatch({ selection: { anchor: 0, head: 13 } });
    expect(indentListItem(view)).toBe(true);
    expect(snapshot().doc).toBe("  - a\nplain\n  - c");
  });
});

describe("inline image paste helpers", () => {
  // Bullet caret-snap (clampListCaretPosition / listAwareArrowDown-Up /
  // isListEolClick) was removed in phase-18: `*`/`+` markers are real-
  // width glyph widgets now, so cursor/click/arrow are plain CodeMirror
  // (browser-smoked) with no snap helpers to unit-test here.
  test("retracts one unused pasted-image space before list continuation", () => {
    const doc = "- ![](photo.png#w=250) ";
    mount(doc, doc.length);
    expect(stripUnusedInlineImageSpaceOnEnter(view)).toBe(false);
    expect(snapshot()).toEqual({
      doc: "- ![](photo.png#w=250)",
      head: doc.length - 1,
    });
  });

  test("keeps normal trailing spaces outside pasted image markers", () => {
    const doc = "- words ";
    mount(doc, doc.length);
    expect(stripUnusedInlineImageSpaceOnEnter(view)).toBe(false);
    expect(snapshot()).toEqual({ doc, head: doc.length });
  });
});
