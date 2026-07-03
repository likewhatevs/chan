// @vitest-environment jsdom

// Guards the wysiwyg-list lazy-parse lag (the "- foo raw marker at rest" bug).
//
// The walker walks the viewport reading `syntaxTree(state)`, which is LAZY and
// viewport-budgeted: right after an edit (or past the initial in-budget parse)
// it can hand back a tree whose visible list block is not re-parsed yet, so a
// `- foo` still parses as a Paragraph and the walker renders a raw marker that
// persists until an unrelated recompute. The walker now forces the parse
// through the viewport (`ensureSyntaxTree(state, viewport.to, budget)`) before
// the walk, so a freshly formed list block decorates immediately.
//
// A fully behavioral repro of the timing lag is fragile under jsdom (its
// viewport does not scroll and the parse settles differently than a live
// WKWebView/Blink editor), so the reliable guard is the source pin of the
// force; the behavioral check asserts the walker never decorates FEWER list
// nodes than the lazy tree exposes in the viewport (the fix can only add).

import walkerSrc from "./walker.ts?raw";
import { EditorState } from "@codemirror/state";
import { EditorView } from "@codemirror/view";
import { syntaxTree } from "@codemirror/language";
import { describe, expect, test } from "vitest";
import { chanMarkdown } from "../markdown/grammar";
import { decorationWalker, type TokenContext } from "./walker";

describe("decoration walker forces the parse for the viewport", () => {
  // Source pin: reverting to the lazy `syntaxTree(state).iterate` reintroduces
  // the raw-marker lag, so the walk must read a forced tree.
  test("walker forces the parse through the viewport before walking", () => {
    expect(walkerSrc).toMatch(/ensureSyntaxTree\(state,\s*to,\s*PARSE_BUDGET_MS\)/);
    // And it must fall back to the lazy tree so a huge-doc parse cannot block.
    expect(walkerSrc).toMatch(/\?\?\s*syntaxTree\(state\)/);
  });

  // Behavioral: with a document large enough that the lazy tree is incomplete
  // at mount, the walker must still decorate every list node the lazy tree
  // exposes in the viewport, and never fewer (the force only adds coverage).
  test("decorates at least every viewport list the lazy tree exposes", () => {
    const walkerHits = new Set<number>();
    const walker = decorationWalker({
      BulletList: (ctx: TokenContext) => walkerHits.add(ctx.node.from),
    });
    // Bullets interleaved through a large doc so some land past the initial
    // in-budget parse point (the recompute test uses the same 4000-line scale
    // to guarantee `syntaxTree` stops short of EOF).
    const block = "prose paragraph line\n".repeat(40) + "- a bullet item\n";
    const doc = block.repeat(100);
    const parent = document.createElement("div");
    document.body.appendChild(parent);
    const view = new EditorView({
      parent,
      state: EditorState.create({ doc, extensions: [chanMarkdown(), walker] }),
    });

    // What the LAZY tree exposes as BulletList within the walked viewport.
    const { from, to } = view.viewport;
    const lazyBullets = new Set<number>();
    syntaxTree(view.state).iterate({
      from,
      to,
      enter(n) {
        if (n.name === "BulletList") lazyBullets.add(n.from);
      },
    });

    // The walker must cover every bullet the lazy tree sees in the viewport.
    for (const pos of lazyBullets) expect(walkerHits.has(pos)).toBe(true);

    view.destroy();
    parent.remove();
  });
});
