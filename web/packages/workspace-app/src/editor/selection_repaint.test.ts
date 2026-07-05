// @vitest-environment jsdom

import { afterEach, describe, expect, test } from "vitest";
import { EditorState } from "@codemirror/state";
import { EditorView } from "@codemirror/view";
import { selectionLayerRepaintFix } from "./selection_repaint";

// jsdom has no layout geometry, so the WKWebView stale-selection-band this
// targets cannot be reproduced here; the fix needs desktop hand-smoke. This
// only guards that the extension is well-formed and its collapse-triggered
// measure path runs without throwing.
describe("selectionLayerRepaintFix", () => {
  afterEach(() => {
    document.body.innerHTML = "";
  });

  test("runs its measure path on a collapse without throwing", () => {
    const target = document.createElement("div");
    document.body.append(target);
    const view = new EditorView({
      state: EditorState.create({
        doc: "hello world",
        extensions: [selectionLayerRepaintFix()],
      }),
      parent: target,
    });
    // Select a word, then collapse back to a caret: the collapse path runs.
    view.dispatch({ selection: { anchor: 0, head: 5 } });
    expect(() =>
      view.dispatch({ selection: { anchor: 5 } }),
    ).not.toThrow();
    view.destroy();
    target.remove();
  });
});
