// Heading-level-aware fold service.
//
// Per design.md spec #9: line `^#{n} ` folds end-of-line → start of
// the next `#{<=n}` heading line (or doc end). Replaces the legacy
// editor's FoldHeadingExtension (~207 LOC) with ~60 LOC by leaning
// on @codemirror/language's foldService + foldGutter primitives.
//
// Fold state lives in CM6's standard fold state field — shows in the
// gutter as a chevron, persists per-session, maps through edits.
// Unfolding via the chevron also reveals child headings (no special
// "fold all" / "unfold all" semantics in v1; CM6's standard fold
// commands cover that).

import { foldGutter, foldService } from "@codemirror/language";
import type { Extension } from "@codemirror/state";

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
  // Walk forward looking for the next line that's a heading of
  // equal-or-higher level. The fold range is from the end of THIS
  // line to the start of that line (or doc end).
  const fromLine = state.doc.lineAt(lineStart);
  const total = state.doc.lines;
  for (let n = fromLine.number + 1; n <= total; n++) {
    const nextLine = state.doc.line(n);
    const nextLevel = headingLevel(nextLine.text);
    if (nextLevel > 0 && nextLevel <= level) {
      // Fold to just before the next heading.
      return { from: lineEnd, to: nextLine.from - 1 };
    }
  }
  // No subsequent heading at this level or higher — fold to end of
  // doc. Skip if there's nothing after this line (single-line file
  // ending in a heading).
  const docEnd = state.doc.length;
  if (lineEnd >= docEnd) return null;
  return { from: lineEnd, to: docEnd };
});

export function headingFold(): Extension {
  return [headingFoldService, foldGutter()];
}
