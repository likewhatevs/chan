// Mermaid code-block flip-to-render (wave-3). A fenced ```mermaid block
// renders as a two-face flip card when the caret is OUTSIDE it (the
// table-atom pattern): front face shows the source with a copy button
// and a flip button (bottom-right of the same right column); the flip
// button rotateY-flips the card to the rendered diagram (back face) and
// back. When the caret is INSIDE the block, the card is suppressed so
// the raw ```mermaid source is editable (blocks.ts skips its own fence
// styling for mermaid, leaving the block to this module).
//
// mermaid is loaded lazily on the FIRST flip (see mermaid_render.ts), so
// the editor bundle never pulls it until a diagram is actually shown.

import {
  Decoration,
  type DecorationSet,
  EditorView,
  WidgetType,
} from "@codemirror/view";
import { syntaxTree } from "@codemirror/language";
import { type EditorState, type Extension, StateField } from "@codemirror/state";
import type { SyntaxNode } from "@lezer/common";
import { selectionInRange } from "../decorations/selection";
import { renderMermaid } from "../mermaid_render";

class MermaidFlipCard extends WidgetType {
  constructor(
    readonly source: string,
    readonly dark: boolean,
  ) {
    super();
  }

  eq(other: MermaidFlipCard): boolean {
    // Same source + theme -> CM6 reuses this DOM, so a flipped card
    // stays flipped (and an already-rendered diagram stays rendered)
    // across unrelated editor updates.
    return this.source === other.source && this.dark === other.dark;
  }

  toDOM(): HTMLElement {
    const card = document.createElement("div");
    card.className = "cm-md-mermaid-card";
    card.contentEditable = "false";

    const inner = document.createElement("div");
    inner.className = "cm-md-mermaid-inner";
    card.append(inner);

    // FRONT: the mermaid source.
    const front = document.createElement("div");
    front.className = "cm-md-mermaid-face cm-md-mermaid-front";
    const pre = document.createElement("pre");
    pre.className = "cm-md-mermaid-source";
    pre.textContent = this.source;
    front.append(pre);

    const lang = document.createElement("span");
    lang.className = "cm-md-mermaid-lang";
    lang.textContent = "mermaid";
    front.append(lang);

    // BACK: the rendered diagram (filled lazily on first flip).
    const back = document.createElement("div");
    back.className = "cm-md-mermaid-face cm-md-mermaid-back";
    const diagram = document.createElement("div");
    diagram.className = "cm-md-mermaid-diagram";
    back.append(diagram);

    // Both faces are absolutely stacked for the rotateY flip, so the
    // card has no intrinsic height. Size `inner` to whichever face is
    // showing; a CSS height transition makes it grow/shrink with the
    // flip instead of clipping a tall diagram to the short source.
    const syncHeight = (): void => {
      const showingBack = card.classList.contains("cm-md-mermaid-flipped");
      inner.style.height = `${(showingBack ? back : front).scrollHeight}px`;
    };

    let rendered = false;
    const render = (): void => {
      if (rendered) return;
      rendered = true;
      diagram.textContent = "rendering…";
      void renderMermaid(this.source, this.dark).then((res) => {
        if (res.ok && res.svg) {
          diagram.innerHTML = res.svg;
        } else {
          diagram.classList.add("cm-md-mermaid-error");
          diagram.textContent = `mermaid: ${res.error ?? "render failed"}`;
        }
        syncHeight();
      });
    };
    const onFlip = (): void => {
      render(); // lazy: load + render the diagram on the first flip only
      card.classList.toggle("cm-md-mermaid-flipped");
      syncHeight();
    };

    // Copy (top-right) + flip (bottom-right) share the right column.
    front.append(this.copyButton());
    front.append(this.flipButton(onFlip, "Render diagram"));
    back.append(this.flipButton(onFlip, "Show source"));

    inner.append(front, back);
    // scrollHeight is 0 until attached; size on the next frame.
    requestAnimationFrame(syncHeight);
    return card;
  }

  private copyButton(): HTMLButtonElement {
    const btn = document.createElement("button");
    btn.type = "button";
    btn.className = "cm-md-mermaid-btn cm-md-mermaid-copy";
    btn.title = "Copy code";
    btn.setAttribute("aria-label", "Copy code");
    btn.textContent = "copy";
    btn.addEventListener("mousedown", stop);
    btn.addEventListener("click", (e) => {
      stop(e);
      void navigator.clipboard.writeText(this.source).then(
        () => flash(btn, "copied"),
        () => flash(btn, "copy-failed"),
      );
    });
    return btn;
  }

  private flipButton(onFlip: () => void, title: string): HTMLButtonElement {
    const btn = document.createElement("button");
    btn.type = "button";
    btn.className = "cm-md-mermaid-btn cm-md-mermaid-flip";
    btn.title = title;
    btn.setAttribute("aria-label", title);
    btn.textContent = "flip";
    btn.addEventListener("mousedown", stop);
    btn.addEventListener("click", (e) => {
      stop(e);
      onFlip();
    });
    return btn;
  }

  ignoreEvent(): boolean {
    // Clicks on the card (outside the buttons) defer to CM6's caret
    // placement; the selection-intersect rule then reveals the source
    // for editing on the next tick.
    return false;
  }
}

function stop(e: Event): void {
  e.preventDefault();
  e.stopPropagation();
}

function flash(btn: HTMLButtonElement, cls: string): void {
  btn.classList.add(cls);
  setTimeout(() => btn.classList.remove(cls), 900);
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
  return [
    field,
    EditorView.atomicRanges.of(
      (view) => view.state.field(field, false) ?? Decoration.none,
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
      if (source === null) return;
      // Caret inside -> show the raw editable source, not the card.
      if (selectionInRange(sel, node.from, node.to)) return;
      decos.push({
        from: node.from,
        to: node.to,
        deco: Decoration.replace({
          widget: new MermaidFlipCard(source, dark),
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

/// The mermaid source of a fenced block, or null when the block is not
/// `mermaid` / has no content / is unclosed. Source is the text between
/// the opener and closer fence lines (mirrors blocks.ts's line-based
/// extraction rather than trusting a CodeText node).
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
  const openLine = state.doc.lineAt(openFrom).number;
  const closeLine = state.doc.lineAt(closeFrom).number;
  if (closeLine <= openLine + 1) return null; // unclosed or empty
  const first = state.doc.line(openLine + 1);
  const last = state.doc.line(closeLine - 1);
  const source = state.doc.sliceString(first.from, last.to);
  return source.trim() ? source : null;
}
