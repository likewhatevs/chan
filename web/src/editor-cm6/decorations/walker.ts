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
//   - `state` is the EditorState — handlers read doc text via
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
// are silently skipped — handlers fill in coverage progressively
// across steps 4 (marks, naked URL, headings), 5 (blocks), 6 (atoms).
//
// Re-run triggers (each one independently invalidates the decoration
// set):
//   - `update.docChanged` — text edits, paste, undo/redo.
//   - `update.viewportChanged` — scroll, fold toggle, window resize.
//   - `update.selectionSet` — caret/selection movement. Required so
//     the visibility rule (which hides decorations the selection
//     intersects) re-evaluates on caret moves.
//
// All re-runs walk only the visible viewport (`view.viewport.from..to`)
// per CM6 convention — cost stays proportional to visible content,
// not document size.

import { syntaxTree } from "@codemirror/language";
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
import type { SyntaxNodeRef } from "@lezer/common";
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

      constructor(view: EditorView) {
        this.decorations = computeDecorations(view, handlers);
      }

      update(u: ViewUpdate): void {
        if (u.docChanged || u.viewportChanged || u.selectionSet) {
          this.decorations = computeDecorations(u.view, handlers);
        }
      }
    },
    {
      decorations: (v) => v.decorations,
    },
  );
}

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
  syntaxTree(state).iterate({
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
