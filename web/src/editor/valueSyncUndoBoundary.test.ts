import { afterEach, describe, expect, test } from "vitest";
import { EditorState } from "@codemirror/state";
import { EditorView } from "@codemirror/view";
import { history, undo } from "@codemirror/commands";
import { createValueSync } from "./base";

// The undo boundary at file load. The editor mounts before the file's
// async load resolves, so createValueSync's FIRST fill of the empty doc
// is the load itself: it must not enter the undo history, or Cmd+Z can
// walk back to the empty pre-load doc and autosave persists the EMPTY
// file to disk. These are behavioral pins on real CM6 transactions, not
// source regexes: the history() extension decides what undo reaches.
//
// Deliberately NOT covered (open product question, behavior must stay
// unchanged): whether a file-watch reload should also be non-undoable.
// The "later applies stay undoable" tests below pin that it currently
// IS undoable; if the round-close survey decides otherwise, those pins
// change with the decision.

let views: EditorView[] = [];

function mkView(doc: string): EditorView {
  const view = new EditorView({
    state: EditorState.create({ doc, extensions: [history()] }),
    parent: document.body,
  });
  views.push(view);
  return view;
}

afterEach(() => {
  for (const view of views) view.destroy();
  views = [];
});

describe("createValueSync undo boundary", () => {
  test("the initial empty->content fill is not undoable", () => {
    const sync = createValueSync();
    const view = mkView("");
    sync.applyExternal(view, "# Loaded\n\ncontent", { focus: false });
    expect(view.state.doc.toString()).toBe("# Loaded\n\ncontent");
    // Nothing to undo: the load is the floor of the history.
    expect(undo(view)).toBe(false);
    expect(view.state.doc.toString()).toBe("# Loaded\n\ncontent");
  });

  test("a user edit after the load IS undoable, and undo stops at the loaded content", () => {
    const sync = createValueSync();
    const view = mkView("");
    sync.applyExternal(view, "loaded", { focus: false });
    // A plain dispatch is history-recorded by default (user input path).
    view.dispatch({
      changes: { from: view.state.doc.length, insert: " typed" },
    });
    expect(view.state.doc.toString()).toBe("loaded typed");
    expect(undo(view)).toBe(true);
    expect(view.state.doc.toString()).toBe("loaded");
    // Undo-spam cannot cross the load boundary into the empty doc.
    expect(undo(view)).toBe(false);
    expect(view.state.doc.toString()).toBe("loaded");
  });

  test("a later external apply (file-watch reload path) stays undoable", () => {
    const sync = createValueSync();
    const view = mkView("");
    sync.applyExternal(view, "loaded", { focus: false });
    sync.applyExternal(view, "reloaded from disk", { focus: false });
    expect(view.state.doc.toString()).toBe("reloaded from disk");
    // The reload is a separate undo step back to the prior content —
    // pinned so the narrow fix cannot silently widen to this path.
    expect(undo(view)).toBe(true);
    expect(view.state.doc.toString()).toBe("loaded");
    // ...and undo still cannot cross the load boundary to empty.
    expect(undo(view)).toBe(false);
    expect(view.state.doc.toString()).toBe("loaded");
  });

  test("a doc seeded at creation never treats a reload as the initial fill", () => {
    const sync = createValueSync();
    // Mode-toggle remounts / keep-alive mounts with content pass the
    // doc at EditorState.create; the mount $effect then dedupes
    // (cur === value), which must consume the initial-fill window.
    const view = mkView("seeded content");
    sync.applyExternal(view, "seeded content", { focus: false });
    sync.applyExternal(view, "reloaded", { focus: false });
    expect(undo(view)).toBe(true);
    expect(view.state.doc.toString()).toBe("seeded content");
  });

  test("an empty->empty dedupe does not consume the initial-fill window", () => {
    const sync = createValueSync();
    const view = mkView("");
    // The mount $effect fires while value is still "" (content not yet
    // fetched): a no-op that must leave the boundary armed.
    sync.applyExternal(view, "", { focus: false });
    sync.applyExternal(view, "late content", { focus: false });
    expect(undo(view)).toBe(false);
    expect(view.state.doc.toString()).toBe("late content");
  });
});
