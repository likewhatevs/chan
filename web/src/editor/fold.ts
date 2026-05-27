// Heading-level-aware fold service + a custom gutter that ONLY shows
// chevrons on heading lines.
//
// Per design.md spec #9: line `^#{n} ` folds end-of-line → start of
// the next `#{<=n}` heading line (or doc end).
//
// Why a custom gutter instead of `foldGutter()`: @codemirror/lang-
// markdown adds `foldNodeProp` to many block types (paragraphs,
// blockquotes, tables, fenced code, ...). The default foldGutter
// renders a chevron for ANY line where `foldable(state, ...)` returns
// non-null, so every paragraph in a doc got its own chevron.
// Filtering to "headings only" via foldGutter config isn't possible
// (no per-line callback), so we render the gutter ourselves and
// workspace fold / unfold via the existing foldEffect / unfoldEffect.

import {
  codeFolding,
  foldEffect,
  foldedRanges,
  foldService,
  unfoldEffect,
} from "@codemirror/language";
import { type Extension } from "@codemirror/state";
import { gutter, GutterMarker } from "@codemirror/view";

const HEADING_RE = /^(#{1,6}) /;

/// Detect the heading level of a line, or 0 if not a heading.
function headingLevel(text: string): number {
  const m = HEADING_RE.exec(text);
  return m ? m[1]!.length : 0;
}

const headingFoldService = foldService.of((state, lineStart, lineEnd) => {
  const line = state.doc.sliceString(lineStart, lineEnd);
  const level = headingLevel(line);
  if (level === 0) return null;
  const fromLine = state.doc.lineAt(lineStart);
  const total = state.doc.lines;
  for (let n = fromLine.number + 1; n <= total; n++) {
    const nextLine = state.doc.line(n);
    const nextLevel = headingLevel(nextLine.text);
    if (nextLevel > 0 && nextLevel <= level) {
      return { from: lineEnd, to: nextLine.from - 1 };
    }
  }
  const docEnd = state.doc.length;
  if (lineEnd >= docEnd) return null;
  return { from: lineEnd, to: docEnd };
});

/// Gutter marker classes. One DOM per fold state so CM6 can reuse.
class ChevronMarker extends GutterMarker {
  constructor(readonly folded: boolean) {
    super();
  }
  eq(other: ChevronMarker): boolean {
    return this.folded === other.folded;
  }
  toDOM(): HTMLElement {
    const el = document.createElement("span");
    el.className = "cm-md-fold-chevron";
    el.dataset.folded = this.folded ? "true" : "false";
    el.textContent = this.folded ? "▸" : "▾";
    return el;
  }
}
const CHEVRON_UNFOLDED = new ChevronMarker(false);
const CHEVRON_FOLDED = new ChevronMarker(true);

/// Find the existing fold range whose start is at the end of `line`.
/// Returns null when this heading isn't currently folded.
function findHeadingFold(
  view: import("@codemirror/view").EditorView,
  line: { from: number; to: number },
): { from: number; to: number } | null {
  const folded = foldedRanges(view.state);
  let hit: { from: number; to: number } | null = null;
  folded.between(line.to, line.to, (foldFrom, foldTo) => {
    hit = { from: foldFrom, to: foldTo };
    return false;
  });
  return hit;
}

const headingFoldGutter = gutter({
  class: "cm-md-fold-gutter",
  lineMarker(view, blockInfo) {
    // Resolve to the actual document line: blockInfo can extend
    // through folded ranges that follow this heading, so sampling
    // text via `sliceString(blockInfo.from, blockInfo.to)` would
    // return the heading + the folded body and our heading regex
    // would still match — but `findHeadingFold` (below) needs the
    // exact line.to to locate the fold start, which is what the
    // foldService emits.
    const line = view.state.doc.lineAt(blockInfo.from);
    if (headingLevel(line.text) === 0) return null;
    return findHeadingFold(view, { from: line.from, to: line.to })
      ? CHEVRON_FOLDED
      : CHEVRON_UNFOLDED;
  },
  // Re-render gutter markers whenever the fold state changes; without
  // this the chevron stays on `▾` after a click even though the fold
  // applied (lineMarker is otherwise only re-evaluated on doc /
  // viewport changes, which folding alone doesn't trigger).
  lineMarkerChange: (update) => {
    for (const tr of update.transactions) {
      for (const e of tr.effects) {
        if (e.is(foldEffect) || e.is(unfoldEffect)) return true;
      }
    }
    return false;
  },
  initialSpacer: () => CHEVRON_UNFOLDED,
  domEventHandlers: {
    click(view, blockInfo) {
      const line = view.state.doc.lineAt(blockInfo.from);
      const level = headingLevel(line.text);
      if (level === 0) return false;
      const existing = findHeadingFold(view, { from: line.from, to: line.to });
      if (existing) {
        view.dispatch({ effects: unfoldEffect.of(existing) });
        return true;
      }
      // Compute the fold range from the heading service inline
      // (foldedRanges is read-only; we don't get the range from
      // foldService output here, so re-derive).
      const total = view.state.doc.lines;
      let foldTo = view.state.doc.length;
      for (let n = line.number + 1; n <= total; n++) {
        const next = view.state.doc.line(n);
        const nextLevel = headingLevel(next.text);
        if (nextLevel > 0 && nextLevel <= level) {
          foldTo = next.from - 1;
          break;
        }
      }
      if (foldTo <= line.to) return false;
      view.dispatch({
        effects: foldEffect.of({ from: line.to, to: foldTo }),
      });
      return true;
    },
  },
});

export function headingFold(): Extension {
  // `codeFolding()` registers the fold state field that foldEffect /
  // unfoldEffect mutate. Without it the chevron click dispatches
  // an effect that nothing listens to and the fold silently no-ops —
  // gutter clicks logged the right blockInfo and dispatched, but
  // foldedRanges stayed empty.
  return [codeFolding(), headingFoldService, headingFoldGutter];
}
