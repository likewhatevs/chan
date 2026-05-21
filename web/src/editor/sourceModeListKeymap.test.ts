// @vitest-environment jsdom

import { afterEach, describe, expect, test } from "vitest";
import { EditorState } from "@codemirror/state";
import { EditorView, keymap } from "@codemirror/view";
import { defaultKeymap } from "@codemirror/commands";
import { markdown } from "@codemirror/lang-markdown";

/// Mounts a minimal CM6 editor with the markdown language seeded
/// the same way Source.svelte's source mode does. Runs Enter at
/// the given caret position and returns the resulting doc.
function runEnterAt(seed: string, caret: number, addKeymap: boolean): string {
  const target = document.createElement("div");
  document.body.append(target);
  try {
    const state = EditorState.create({
      doc: seed,
      selection: { anchor: caret },
      extensions: [
        keymap.of(defaultKeymap),
        markdown({ addKeymap }),
      ],
    });
    const view = new EditorView({ state, parent: target });
    try {
      const evt = new KeyboardEvent("keydown", { key: "Enter", bubbles: true });
      view.contentDOM.dispatchEvent(evt);
      return view.state.doc.toString();
    } finally {
      view.destroy();
    }
  } finally {
    target.remove();
  }
}

afterEach(() => {
  document.body.innerHTML = "";
});

describe("source-mode markdown extension (fullstack-a-41)", () => {
  test("with addKeymap=true (default), Enter after `1. ` auto-continues to `2. `", () => {
    // Sanity check against the default lang-markdown behaviour —
    // confirms the bug exists without our fix.
    const seed = "1. item";
    const after = runEnterAt(seed, seed.length, true);
    // Default markdownKeymap should insert "\n2. " after the
    // existing item.
    expect(after).toBe("1. item\n2. ");
  });

  test("with addKeymap=false, Enter after `1. ` just inserts a newline (raw editing)", () => {
    const seed = "1. item";
    const after = runEnterAt(seed, seed.length, false);
    expect(after).toBe("1. item\n");
  });

  test("with addKeymap=false, Enter after `- ` does not auto-continue a bullet", () => {
    const seed = "- item";
    const after = runEnterAt(seed, seed.length, false);
    expect(after).toBe("- item\n");
  });

  test("with addKeymap=false, Enter after `* ` does not auto-continue a star bullet", () => {
    const seed = "* item";
    const after = runEnterAt(seed, seed.length, false);
    expect(after).toBe("* item\n");
  });

  test("with addKeymap=false, Enter after `1) ` does not auto-continue the alternate ordered marker", () => {
    const seed = "1) item";
    const after = runEnterAt(seed, seed.length, false);
    expect(after).toBe("1) item\n");
  });
});
