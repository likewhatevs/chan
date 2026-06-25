import { afterEach, describe, expect, test } from "vitest";
import { EditorState } from "@codemirror/state";
import { EditorView } from "@codemirror/view";
import { createValueSync } from "./base";

// CRLF convergence. CodeMirror stores its document with '\n' line
// endings (CM6 normalizes any '\r\n' / '\r' on the way in), so a file
// read from disk with CRLF endings (the Windows default) reaches
// applyExternal as `value` with '\r\n'. applyExternal must compare and
// insert against the same '\n' normalization; otherwise `cur` (LF, from
// the live doc) never equals `value` (CRLF), the dedupe guard never
// short-circuits, and it re-dispatches on every prop->doc $effect run.
// Each dispatch's selectionSet write re-triggers that effect, an
// unbounded reactive loop that trips Svelte's effect_update_depth_exceeded
// and freezes the editor. It is Windows-only because LF files converge on
// the first compare. These pins assert the loop cannot form: CRLF lands as
// LF in the doc and a re-apply of the same value does not dispatch again.

let views: EditorView[] = [];

function mkCountingView(doc: string): {
  view: EditorView;
  dispatches: () => number;
} {
  // Count doc-changing transactions: applyExternal only dispatches when
  // `cur !== normalized`, so a deduped re-apply produces no update and the
  // count holds steady. That is exactly the "no repeat dispatch" pin.
  let count = 0;
  const view = new EditorView({
    state: EditorState.create({
      doc,
      extensions: [
        EditorView.updateListener.of((u) => {
          if (u.docChanged) count += 1;
        }),
      ],
    }),
    parent: document.body,
  });
  views.push(view);
  return { view, dispatches: () => count };
}

afterEach(() => {
  for (const view of views) view.destroy();
  views = [];
});

describe("createValueSync CRLF normalization", () => {
  test("a CRLF value lands as LF and converges (no repeat dispatch)", () => {
    const sync = createValueSync();
    const { view, dispatches } = mkCountingView("");
    sync.applyExternal(view, "a\r\nb\r\nc", { focus: false });
    // CM6 holds the doc as LF.
    expect(view.state.doc.toString()).toBe("a\nb\nc");
    expect(dispatches()).toBe(1);
    // Re-applying the same CRLF value must dedupe: cur (LF) === normalized
    // (LF). Without normalization this re-dispatches and, in the live
    // editor, loops via the selection-mirror write-back.
    sync.applyExternal(view, "a\r\nb\r\nc", { focus: false });
    expect(dispatches()).toBe(1);
    sync.applyExternal(view, "a\r\nb\r\nc", { focus: false });
    expect(dispatches()).toBe(1);
  });

  test("a lone CR value also normalizes and converges", () => {
    const sync = createValueSync();
    const { view, dispatches } = mkCountingView("");
    sync.applyExternal(view, "x\ry\r", { focus: false });
    expect(view.state.doc.toString()).toBe("x\ny\n");
    expect(dispatches()).toBe(1);
    sync.applyExternal(view, "x\ry\r", { focus: false });
    expect(dispatches()).toBe(1);
  });

  test("an LF value (no carriage return) is unchanged and converges", () => {
    const sync = createValueSync();
    const { view, dispatches } = mkCountingView("");
    sync.applyExternal(view, "plain\nlf\n", { focus: false });
    expect(view.state.doc.toString()).toBe("plain\nlf\n");
    expect(dispatches()).toBe(1);
    sync.applyExternal(view, "plain\nlf\n", { focus: false });
    expect(dispatches()).toBe(1);
  });
});
