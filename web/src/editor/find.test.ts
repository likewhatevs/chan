// @vitest-environment jsdom
//
// Coverage for the FindAdapter built by base.ts. The pure
// `scanMatches` already gets exercised by its callers and is just
// a wrapper around String.indexOf; the meaningful new behavior is
// `placeCursor`, which moves the editor selection without focus
// so Enter / Shift+Enter inside the FindBar leaves the caret on
// the navigated match for the user to land on after Esc.

import { describe, expect, test, beforeEach, afterEach } from "vitest";
import { EditorState } from "@codemirror/state";
import { EditorView } from "@codemirror/view";
import { findField, makeFindAdapter, setFindEffect } from "./base";

let host: HTMLDivElement;
let view: EditorView;

function mount(doc: string): void {
  host = document.createElement("div");
  document.body.append(host);
  view = new EditorView({
    state: EditorState.create({
      doc,
      extensions: [findField],
    }),
    parent: host,
  });
}

afterEach(() => {
  view?.destroy();
  host?.remove();
});

describe("makeFindAdapter.placeCursor", () => {
  test("moves the selection to the start of the match", () => {
    mount("hello world hello again");
    const adapter = makeFindAdapter(() => view);
    const matches = adapter.scan("hello", { caseSensitive: false });
    expect(matches).toEqual([
      { from: 0, to: 5 },
      { from: 12, to: 17 },
    ]);
    adapter.highlightAll(matches, 1);
    adapter.placeCursor(1);
    const sel = view.state.selection.main;
    expect(sel.empty).toBe(true);
    expect(sel.head).toBe(12);
  });

  test("no-op when index is out of range", () => {
    mount("foo bar");
    const adapter = makeFindAdapter(() => view);
    const before = view.state.selection.main.head;
    const matches = adapter.scan("foo", { caseSensitive: true });
    adapter.highlightAll(matches, 0);
    adapter.placeCursor(99);
    expect(view.state.selection.main.head).toBe(before);
  });

  test("no-op when there are no highlighted matches yet", () => {
    mount("anything");
    const adapter = makeFindAdapter(() => view);
    adapter.placeCursor(0);
    expect(view.state.selection.main.head).toBe(0);
  });

  test("placeCursor does not steal focus from the document body", () => {
    mount("hello world");
    const adapter = makeFindAdapter(() => view);
    const matches = adapter.scan("world", { caseSensitive: false });
    adapter.highlightAll(matches, 0);
    // Force focus elsewhere to assert placeCursor does not pull
    // it back to the editor.
    const input = document.createElement("input");
    document.body.append(input);
    input.focus();
    expect(document.activeElement).toBe(input);
    adapter.placeCursor(0);
    expect(document.activeElement).toBe(input);
    input.remove();
  });

  test("setFindEffect updates the state field directly", () => {
    mount("a b c");
    view.dispatch({
      effects: setFindEffect.of({
        ranges: [{ from: 0, to: 1 }],
        currentIndex: 0,
      }),
    });
    const f = view.state.field(findField);
    expect(f.ranges).toEqual([{ from: 0, to: 1 }]);
    expect(f.currentIndex).toBe(0);
  });
});
