// Mermaid code-block cursor-render (wave-3). Mirrors the image / table
// atoms: while the cursor is INSIDE a ```mermaid block the normal code
// block shows (blocks.ts is untouched, so it looks byte-for-byte as
// before); when the cursor LEAVES a COMPLETE (closed) mermaid block it
// is replaced by the rendered diagram, which flips in on the horizontal
// (rotateX) axis. Cursor back inside reveals the source again. There is
// no button: the cursor is the only trigger, like every other atom.
//
// Only closed fences render (never a mid-typing/unclosed block). A bad
// diagram renders mermaid's own error on the diagram face rather than
// falling back to source or throwing. mermaid is dynamic-imported on
// first render (mermaid_render.ts), so the editor bundle never pulls it
// until a diagram is actually shown.

import {
  type Command,
  Decoration,
  type DecorationSet,
  EditorView,
  keymap,
  WidgetType,
} from "@codemirror/view";
import { syntaxTree } from "@codemirror/language";
import {
  EditorSelection,
  type EditorState,
  type Extension,
  Prec,
  StateField,
} from "@codemirror/state";
import type { SyntaxNode } from "@lezer/common";
import { selectionInRange } from "../decorations/selection";
import { renderMermaid } from "../mermaid_render";

class MermaidWidget extends WidgetType {
  constructor(
    readonly source: string,
    readonly dark: boolean,
  ) {
    super();
  }

  eq(other: MermaidWidget): boolean {
    // Same source + theme -> CM6 reuses this DOM, so the already-rendered
    // diagram (and its flip-in) is not replayed on unrelated updates.
    return this.source === other.source && this.dark === other.dark;
  }

  toDOM(): HTMLElement {
    const wrap = document.createElement("div");
    wrap.className = "cm-md-mermaid-rendered";
    wrap.contentEditable = "false";

    const inner = document.createElement("div");
    inner.className = "cm-md-mermaid-inner";
    const diagram = document.createElement("div");
    diagram.className = "cm-md-mermaid-diagram";
    diagram.textContent = "rendering…";
    inner.append(diagram);
    wrap.append(inner);

    void renderMermaid(this.source, this.dark).then((res) => {
      if (res.ok && res.svg) {
        diagram.innerHTML = res.svg;
      } else {
        // Cursor-out always renders, even on a bad diagram: show
        // mermaid's error on the diagram face, never crash / fall back.
        diagram.classList.add("cm-md-mermaid-error");
        diagram.textContent = `mermaid: ${res.error ?? "render failed"}`;
      }
    });
    return wrap;
  }

  ignoreEvent(): boolean {
    // Click defers to CM6 caret placement; the selection-intersect rule
    // then reveals the source for editing on the next tick (atom idiom).
    return false;
  }
}

export function mermaidDecorations(isDark: () => boolean): Extension {
  const field = StateField.define<DecorationSet>({
    create(state) {
      return scan(state, isDark());
    },
    update(decorations, tr) {
      if (!tr.docChanged && !tr.selection) return decorations;
      return scan(tr.state, isDark());
    },
    provide: (f) => EditorView.decorations.from(f),
  });
  // Vertical caret entry. A rendered mermaid block is a single
  // block-replace widget: it has no internal lines for the caret to
  // land on, so ArrowUp/ArrowDown skip over it (atomicRanges then snaps
  // the caret past the atom). Left/right entry already works because the
  // caret can sit at the block's edge and selectionInRange reveals the
  // source; vertical motion never lands there. This command catches a
  // vertical move that would step OVER a rendered block and instead
  // lands the caret on the entered edge - inside the range - so the next
  // scan() de-renders it to editable source, matching left/right entry.
  const stepInto =
    (forward: boolean): Command =>
    (view) => {
      const deco = view.state.field(field, false);
      if (!deco || deco.size === 0) return false;
      const range = view.state.selection.main;
      if (!range.empty) return false; // plain caret motion only
      const head = range.head;
      // Geometry-aware target (respects wrapped lines); with the block
      // atomic this lands past the widget when the move crosses it.
      const target = view.moveVertically(range, forward).head;
      let enter = -1;
      deco.between(
        Math.min(head, target),
        Math.max(head, target),
        (from, to) => {
          // Did this move start outside the block and reach (or
          // overshoot) the edge we'd enter from? Down enters at `from`,
          // up at `to`.
          if (forward ? head < from && target >= from : head > to && target <= to) {
            enter = forward ? from : to;
            return false;
          }
          return undefined;
        },
      );
      if (enter < 0 || enter === head) return false;
      view.dispatch({
        selection: EditorSelection.cursor(enter),
        scrollIntoView: true,
      });
      return true;
    };

  return [
    field,
    EditorView.atomicRanges.of(
      (view) => view.state.field(field, false) ?? Decoration.none,
    ),
    // Beats the default-precedence cursorLineUp/Down so we can redirect
    // into the block before atomicRanges snaps the caret past it.
    Prec.high(
      keymap.of([
        { key: "ArrowUp", run: stepInto(false) },
        { key: "ArrowDown", run: stepInto(true) },
      ]),
    ),
  ];
}

function scan(state: EditorState, dark: boolean): DecorationSet {
  const sel = state.selection;
  const decos: Array<{ from: number; to: number; deco: Decoration }> = [];
  syntaxTree(state).iterate({
    enter(node) {
      if (node.name !== "FencedCode") return;
      const source = mermaidSource(state, node.node);
      if (source === null) return; // not mermaid / unclosed / empty
      // Cursor inside -> show the raw editable code block (blocks.ts).
      if (selectionInRange(sel, node.from, node.to)) return;
      decos.push({
        from: node.from,
        to: node.to,
        deco: Decoration.replace({
          widget: new MermaidWidget(source, dark),
          block: true,
        }),
      });
    },
  });
  decos.sort((a, b) => a.from - b.from);
  return Decoration.set(
    decos.map((d) => d.deco.range(d.from, d.to)),
    true,
  );
}

/// The mermaid source of a CLOSED fenced block, or null when the block
/// is not `mermaid`, unclosed (mid-typing), or empty. Source is the text
/// between the opener and closer fence lines.
function mermaidSource(state: EditorState, node: SyntaxNode): string | null {
  const cursor = node.cursor();
  if (!cursor.firstChild()) return null;
  let openFrom = -1;
  let closeFrom = -1;
  let infoFrom = -1;
  let infoTo = -1;
  do {
    if (cursor.name === "CodeMark") {
      if (openFrom === -1) openFrom = cursor.from;
      closeFrom = cursor.from;
    } else if (cursor.name === "CodeInfo") {
      infoFrom = cursor.from;
      infoTo = cursor.to;
    }
  } while (cursor.nextSibling());
  if (openFrom === -1) return null;
  const lang =
    infoFrom !== -1 ? state.doc.sliceString(infoFrom, infoTo).trim().toLowerCase() : "";
  if (lang !== "mermaid") return null;
  // Unclosed fences emit a single CodeMark (closeFrom === openFrom) and
  // stretch to doc end: never render those.
  if (closeFrom === openFrom) return null;
  const openLine = state.doc.lineAt(openFrom).number;
  const closeLine = state.doc.lineAt(closeFrom).number;
  if (closeLine <= openLine + 1) return null; // empty block
  const first = state.doc.line(openLine + 1);
  const last = state.doc.line(closeLine - 1);
  const source = state.doc.sliceString(first.from, last.to);
  return source.trim() ? source : null;
}
