// Decoration walker: the load-bearing module per design.md.
//
// A CM6 ViewPlugin that, on every relevant update (doc change, viewport
// change, or selection change), iterates the visible portion of the
// markdown syntax tree and emits a DecorationSet by dispatching each
// node to a handler registered by node name.
//
// The handler API is intentionally minimal:
//
//   - `node` exposes the syntax-tree position (name, from, to). Handlers
//     can descend via `node.node.firstChild` / `nextSibling` if they
//     need internal structure (e.g. wikilink body, link URL).
//   - `state` is the EditorState - handlers read doc text via
//     `state.doc.sliceString(...)`.
//   - `sel` is the active selection.
//   - `view` is the EditorView (rarely needed, but useful for atomic
//     widgets that want `view.coordsAtPos`).
//   - `push(deco, from, to)` queues a decoration. We buffer everything
//     in an array and let `Decoration.set(..., true)` handle ordering.
//   - `selectionInRange` / `lineIntersect` are the visibility primitives
//     from `selection.ts`, bound to the current selection / state.
//
// Handlers are registered as `{ [nodeName]: handler }`. Unknown nodes
// are silently skipped - handlers fill in coverage progressively
// across steps 4 (marks, naked URL, headings), 5 (blocks), 6 (atoms).
//
// Re-run triggers (each one independently invalidates the decoration
// set):
//   - `update.docChanged` - text edits, paste, undo/redo.
//   - `update.viewportChanged` - scroll, fold toggle, window resize.
//   - `update.selectionSet` - caret/selection movement. Required so
//     the visibility rule (which hides decorations the selection
//     intersects) re-evaluates on caret moves.
//
// All re-runs walk only the visible viewport (`view.viewport.from..to`)
// per CM6 convention - cost stays proportional to visible content,
// not document size.

import { ensureSyntaxTree, syntaxTree } from "@codemirror/language";
import {
  type Extension,
  type EditorSelection,
  type EditorState,
} from "@codemirror/state";
import {
  Decoration,
  type DecorationSet,
  EditorView,
  ViewPlugin,
  type ViewUpdate,
} from "@codemirror/view";
import type { SyntaxNodeRef, Tree } from "@lezer/common";
import { lineIntersect, selectionInRange } from "./selection";

/// Context passed to each token handler.
export interface TokenContext {
  state: EditorState;
  sel: EditorSelection;
  view: EditorView;
  node: SyntaxNodeRef;
  push(deco: Decoration, from: number, to: number): void;
  selectionInRange(from: number, to: number): boolean;
  lineIntersect(from: number, to: number): boolean;
}

export type TokenHandler = (ctx: TokenContext) => void;

export type HandlerRegistry = {
  [nodeName: string]: TokenHandler;
};

type PendingDeco = { from: number; to: number; deco: Decoration };

/// Build the decoration ViewPlugin. Returns an Extension ready to drop
/// into the editor's extension array.
export function decorationWalker(handlers: HandlerRegistry): Extension {
  // Capture the registry once; constructed extension closes over it.
  // The plugin itself is stateless beyond the cached DecorationSet.
  return ViewPlugin.fromClass(
    class {
      decorations: DecorationSet;
      // Cached syntax-tree identity. The async ParseWorker publishes a
      // finished tree through an effects-only dispatch that sets none of
      // docChanged/viewportChanged/selectionSet/geometryChanged, so the
      // gate below would skip it and leave decorations stale (raw markers)
      // until the next interaction. Recomputing when the tree instance
      // changes mirrors @codemirror/language's TreeHighlighter.
      tree: Tree;

      constructor(view: EditorView) {
        this.tree = syntaxTree(view.state);
        this.decorations = computeDecorations(view, handlers);
      }

      update(u: ViewUpdate): void {
        const tree = syntaxTree(u.state);
        // `geometryChanged` covers the tab-switch remount case: editor
        // tabs are unmounted/remounted on switch (unlike terminals), so
        // the EditorView is reconstructed and the constructor walks the
        // INITIAL (pre-layout) viewport - only the top portion. The
        // post-layout measure settles the real viewport but does not
        // reliably fire `viewportChanged`, leaving the lower blocks showing
        // raw markers until a caret move or scroll. Recomputing on
        // `geometryChanged` re-decorates over the corrected viewport once
        // the geometry settles. The walk is viewport-bounded, so the extra
        // recompute stays cheap. (The race manifests under chan-desktop's
        // WKWebView, not Blink/Chrome, so it browser-smokes clean either
        // way; verified no Chrome regression, desktop confirmation pending.)
        if (
          u.docChanged ||
          u.viewportChanged ||
          u.selectionSet ||
          u.geometryChanged ||
          tree !== this.tree
        ) {
          this.tree = tree;
          this.decorations = computeDecorations(u.view, handlers);
        }
      }
    },
    {
      decorations: (v) => v.decorations,
    },
  );
}

/// Time budget for forcing the parse through the viewport before a walk. The
/// walk is viewport-bounded so the parse rarely needs forcing; the cap keeps a
/// pathological huge-document edit from blocking the render (it falls back to
/// the lazy tree and the next recompute catches up).
const PARSE_BUDGET_MS = 100;

function computeDecorations(
  view: EditorView,
  handlers: HandlerRegistry,
): DecorationSet {
  const { state } = view;
  const { sel, decos } = makeBuffers(state);
  const pushDeco = (deco: Decoration, from: number, to: number) => {
    if (from > to) return;
    decos.push({ from, to, deco });
  };
  // `from <= to` and `from === to` are both valid for replace
  // decorations (zero-width replace hides marker chars). The handler
  // is responsible for getting these right; we just push.
  const { from, to } = view.viewport;
  // Read a tree parsed THROUGH the viewport, not the lazy default. The walker
  // renders exactly what the tree says, and `syntaxTree(state)` is lazy and
  // viewport-budgeted: right after an edit it can return a tree whose just-
  // edited block has not been re-parsed yet, so a `- foo` promoted from a
  // paragraph (or a marker inserted at a line start) still parses as a
  // Paragraph and the walker renders a raw marker, persisting until an
  // unrelated recompute forces the block current. `ensureSyntaxTree` forces
  // the parse for the visible range under a small budget (falling back to the
  // lazy tree if it cannot finish, preserving responsiveness on huge docs),
  // so a freshly edited list block decorates immediately.
  const tree = ensureSyntaxTree(state, to, PARSE_BUDGET_MS) ?? syntaxTree(state);
  tree.iterate({
    from,
    to,
    enter(node) {
      const h = handlers[node.name];
      if (!h) return;
      const ctx: TokenContext = {
        state,
        sel,
        view,
        node,
        push: pushDeco,
        selectionInRange: (a, b) => selectionInRange(sel, a, b),
        lineIntersect: (a, b) => lineIntersect(state, a, b, sel),
      };
      h(ctx);
    },
  });
  // Sort + dedupe in `Decoration.set`. The `true` second arg tells CM6
  // to sort by from/startSide and reject overlapping replace
  // decorations (which would be a bug in a handler, not a graceful-
  // degrade case).
  return Decoration.set(
    decos.map((d) => d.deco.range(d.from, d.to)),
    true,
  );
}

function makeBuffers(state: EditorState): {
  sel: EditorSelection;
  decos: PendingDeco[];
} {
  return { sel: state.selection, decos: [] };
}
