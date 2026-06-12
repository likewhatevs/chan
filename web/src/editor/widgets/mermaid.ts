// Mermaid code-block cursor-render. Mirrors the image / table
// atoms: while the cursor is INSIDE a ```mermaid block the normal code
// block shows (blocks.ts is untouched, so it looks byte-for-byte as
// before); when the cursor LEAVES a COMPLETE (closed) mermaid block it
// is replaced by the rendered diagram, which flips in on the horizontal
// (rotateX) axis. Cursor back inside reveals the source again. There is
// no button: the cursor is the only trigger, like every other atom.
//
// The flip is symmetric: cursor-LEAVE plays the forward flip-in on the
// new widget's mount (CSS, Wysiwyg.svelte); cursor-ENTER plays a reverse
// flip-out. The reverse can't animate the widget (CM removes its DOM the
// instant the caret lands inside), so a throwaway ghost of the cached
// diagram face is folded away over the block while the editable source
// takes its place underneath - see flipOutGhost / the ghostFlip plugin.
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
  ViewPlugin,
  type ViewUpdate,
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
import { type MermaidResult, renderMermaid } from "../mermaid_render";

// Snapshot of each rendered diagram's face (the SVG, or the error
// markup), keyed by source + theme. The reverse flip (rendered ->
// source, when the cursor ENTERS a block) can't animate the widget
// itself: CM removes the block-replace widget's DOM instantly the moment
// the caret lands inside. So we rebuild the diagram face from this
// snapshot as a throwaway "ghost" overlay and flip THAT out while the
// editable source takes the widget's place underneath. The forward flip
// (source -> rendered) needs none of this - it just plays on the new
// widget's mount (see the CSS in Wysiwyg.svelte).
const FACE_CACHE_MAX = 48;
const faceCache = new Map<string, string>();
function faceKey(source: string, dark: boolean): string {
  return (dark ? "1" : "0") + source;
}
function cacheFace(source: string, dark: boolean, html: string): void {
  const k = faceKey(source, dark);
  faceCache.delete(k); // re-insert so it counts as most-recently-used
  faceCache.set(k, html);
  if (faceCache.size > FACE_CACHE_MAX) {
    const oldest = faceCache.keys().next().value;
    if (oldest !== undefined) faceCache.delete(oldest);
  }
}

// Last parse error per source (theme-independent), so the editor can
// accent the failing line while the cursor is in the source view. Set
// on a failed render, cleared on a successful one (see toDOM). Keyed by
// source alone; bounded the same way as the face cache.
interface MermaidError {
  line: number;
  col?: number;
  message: string;
}
const errorCache = new Map<string, MermaidError>();
function cacheError(source: string, err: MermaidError | null): void {
  errorCache.delete(source);
  if (err) {
    errorCache.set(source, err);
    if (errorCache.size > FACE_CACHE_MAX) {
      const oldest = errorCache.keys().next().value;
      if (oldest !== undefined) errorCache.delete(oldest);
    }
  }
}

/// Actionable error face (D3): lead with the failing line number, echo
/// that source line's text, then mermaid's reason - so the user can
/// locate the problem before stepping into the source. Falls back to the
/// raw message when mermaid didn't report a line.
function renderErrorFace(diagram: HTMLElement, source: string, res: MermaidResult): void {
  diagram.classList.add("cm-md-mermaid-error");
  diagram.replaceChildren();
  if (res.errorLine) {
    const head = document.createElement("div");
    head.className = "cm-md-mermaid-error-head";
    head.textContent = `Mermaid error - line ${res.errorLine}`;
    diagram.append(head);
    const lineText = source.split("\n")[res.errorLine - 1];
    if (lineText !== undefined) {
      const code = document.createElement("div");
      code.className = "cm-md-mermaid-error-src";
      code.textContent = lineText;
      diagram.append(code);
    }
  }
  const reason = document.createElement("div");
  reason.textContent = res.error ?? "render failed";
  diagram.append(reason);
}

function prefersReducedMotion(): boolean {
  try {
    return window.matchMedia?.("(prefers-reduced-motion: reduce)").matches ?? false;
  } catch {
    return false;
  }
}

const FLIP_MS = 450; // matches the forward flip-in (Wysiwyg.svelte).

/// Play the reverse flip for a block that just de-rendered to source.
/// A self-styled fixed-position ghost of the cached diagram face is
/// dropped over the block's old location and rotateX-folded away (the
/// mirror of the forward flip-in), then removed. The live source is
/// already in place underneath, so editing is never blocked by the
/// animation. No-ops when the face was never cached (entered before the
/// first render) or the block is off-screen.
function flipOutGhost(view: EditorView, from: number, widget: MermaidWidget): void {
  if (prefersReducedMotion()) return;
  const html = faceCache.get(faceKey(widget.source, widget.dark));
  if (!html) return; // entered before the first render cached a face
  // Geometry can't be read during the dispatch that de-renders the block
  // ("Reading the editor layout isn't allowed during an update"), so the
  // measure pass reads coordsAtPos and the following write pass builds
  // the ghost. By then the source has replaced the widget at the same
  // top, which is exactly where the diagram folds away from.
  view.requestMeasure<{ top: number; left: number; width: number } | null>({
    read: () => {
      const coords = view.coordsAtPos(from);
      if (!coords) return null; // scrolled out of the viewport
      const content = view.contentDOM.getBoundingClientRect();
      return { top: coords.top, left: content.left, width: content.width };
    },
    write: (box) => {
      if (!box) return;
      const ghost = document.createElement("div");
      ghost.className = "cm-md-mermaid-ghost";
      ghost.setAttribute("aria-hidden", "true");
      Object.assign(ghost.style, {
        position: "fixed",
        top: `${box.top}px`,
        left: `${box.left}px`,
        width: `${box.width}px`,
        margin: "0",
        pointerEvents: "none",
        zIndex: "40",
        transformOrigin: "center top",
      } satisfies Partial<CSSStyleDeclaration>);

      const diagram = document.createElement("div");
      Object.assign(diagram.style, {
        display: "flex",
        justifyContent: "center",
      } satisfies Partial<CSSStyleDeclaration>);
      // Inject the cached SVG verbatim: mermaid bakes its own
      // `max-width` inline style into it, so the ghost renders at the
      // exact width the editor showed (centered by the flex above).
      // Do NOT override the SVG sizing - that's what made the ghost
      // balloon to the full content width.
      diagram.innerHTML = html;
      ghost.append(diagram);
      document.body.append(ghost);

      const cleanup = () => ghost.remove();
      try {
        ghost
          .animate(
            [
              { transform: "perspective(1200px) rotateX(0deg)", opacity: 1 },
              { transform: "perspective(1200px) rotateX(-90deg)", opacity: 0.2 },
            ],
            { duration: FLIP_MS, easing: "ease", fill: "forwards" },
          )
          .finished.then(cleanup, cleanup);
      } catch {
        // No Web Animations API (jsdom): just don't leave a stray ghost.
        cleanup();
      }
    },
  });
}

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
        // Stash the face so the reverse (enter) flip can ghost it after
        // CM tears the widget down, and clear any stale error so the
        // source view stops accenting a now-fixed line.
        cacheFace(this.source, this.dark, res.svg);
        cacheError(this.source, null);
      } else {
        // Cursor-out always renders, even on a bad diagram: show
        // mermaid's error on the diagram face, never crash / fall back.
        renderErrorFace(diagram, this.source, res);
        // Remember the failing line so the source view can accent it
        // when the cursor steps in (D2-A, cached-on-entry).
        cacheError(
          this.source,
          res.errorLine
            ? { line: res.errorLine, col: res.errorCol, message: res.error ?? "" }
            : null,
        );
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
  // Failing-line accent for the source view. Kept separate from `field`
  // so its line decorations never leak into atomicRanges (which only
  // wants the block-replace widgets).
  const errorLines = StateField.define<DecorationSet>({
    create(state) {
      return scanErrorLines(state);
    },
    update(decorations, tr) {
      if (!tr.docChanged && !tr.selection) return decorations;
      return scanErrorLines(tr.state);
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

  // Reverse flip. One chokepoint for every entry path (click, left/right,
  // up/down all land the caret inside the block, which drops its widget
  // from `field`). When a block leaves the rendered set on a pure cursor
  // move, ghost its face and flip it out. Doc edits are skipped (no flip
  // on structural changes); cursor-LEAVE adds to the set, never removes,
  // so the forward flip is untouched.
  const ghostFlip = ViewPlugin.fromClass(
    class {
      update(update: ViewUpdate): void {
        if (update.docChanged || !update.selectionSet) return;
        const prev = update.startState.field(field, false);
        const cur = update.state.field(field, false);
        if (!prev || !cur) return;
        const stillRendered = new Set<number>();
        for (const it = cur.iter(); it.value; it.next()) {
          stillRendered.add(it.from);
        }
        for (const it = prev.iter(); it.value; it.next()) {
          if (stillRendered.has(it.from)) continue;
          const widget = (it.value.spec as { widget?: WidgetType }).widget;
          if (widget instanceof MermaidWidget) {
            flipOutGhost(update.view, it.from, widget);
          }
        }
      }
    },
  );

  return [
    field,
    errorLines,
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
    ghostFlip,
  ];
}

function scan(state: EditorState, dark: boolean): DecorationSet {
  const sel = state.selection;
  const decos: Array<{ from: number; to: number; deco: Decoration }> = [];
  syntaxTree(state).iterate({
    enter(node) {
      if (node.name !== "FencedCode") return;
      const info = mermaidSource(state, node.node);
      if (info === null) return; // not mermaid / unclosed / empty
      // Cursor inside -> show the raw editable code block (blocks.ts).
      if (selectionInRange(sel, node.from, node.to)) return;
      decos.push({
        from: node.from,
        to: node.to,
        deco: Decoration.replace({
          widget: new MermaidWidget(info.source, dark),
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

/// Error-line accent. While the cursor is INSIDE a mermaid block whose
/// last render failed (its source is in errorCache), mark the failing
/// source line so the user can find it without a line-number gutter.
/// Cached-on-entry (D2-A): the error comes from the prior render, so the
/// accent shows when you step in and clears once you edit the source
/// (cache miss on the changed text) until the next render re-validates.
const LINE_ERROR_DECO = Decoration.line({
  attributes: { class: "cm-md-mermaid-error-line" },
});

function scanErrorLines(state: EditorState): DecorationSet {
  const sel = state.selection;
  const lines: number[] = [];
  syntaxTree(state).iterate({
    enter(node) {
      if (node.name !== "FencedCode") return;
      const info = mermaidSource(state, node.node);
      if (info === null) return;
      // Only while editing the source (cursor inside); the rendered
      // error FACE carries the message when the cursor is outside.
      if (!selectionInRange(sel, node.from, node.to)) return;
      const err = errorCache.get(info.source);
      if (!err) return;
      // Source line 1 sits at openLine + 1, so source line N is doc line
      // openLine + N.
      const docLine = info.openLine + err.line;
      if (docLine >= 1 && docLine <= state.doc.lines) {
        lines.push(state.doc.line(docLine).from);
      }
    },
  });
  lines.sort((a, b) => a - b);
  return Decoration.set(
    lines.map((from) => LINE_ERROR_DECO.range(from)),
    true,
  );
}

/// The mermaid source of a CLOSED fenced block (plus the opener line
/// number, for mapping parse-error lines into the document), or null
/// when the block is not `mermaid`, unclosed (mid-typing), or empty.
/// Source is the text between the opener and closer fence lines.
function mermaidSource(
  state: EditorState,
  node: SyntaxNode,
): { source: string; openLine: number } | null {
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
  return source.trim() ? { source, openLine } : null;
}
