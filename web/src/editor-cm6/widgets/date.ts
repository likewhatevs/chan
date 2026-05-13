// Date pill widget.
//
// Detection: lezer-markdown doesn't recognize dates, so this lives in
// its own ViewPlugin (mirrors the tag pattern from widgets/tag.ts).
// We delegate matching to dateFormats.findDateMatches — the same
// matcher the legacy editor used, kept under web/src/editor/ so date
// catalog evolution stays in one place.
//
// Rendering: per design.md #3 / #5, dates are atomic widgets.
//   - When selection intersects the date range, suppress the widget
//     so the user can edit the literal source. The boundary-inclusive
//     intersection rule (selectionInRange semantics) means clicking
//     the pill places the caret AT the pill's boundary, which counts
//     as intersection on the next update tick — pill collapses to
//     source.
//   - When selection doesn't intersect, emit Decoration.replace with
//     a DateWidget; the widget's DOM is a styled span.
//
// EditorView.atomicRanges registered for the same ranges so caret
// motion (arrow keys) skips the pill in one keystroke.
//
// v1 scope: no calendar popover. Editing happens via "click → source
// reveals → text edit". The calendar is v1.1 polish.

import { syntaxTree } from "@codemirror/language";
import { type Extension } from "@codemirror/state";
import {
  Decoration,
  type DecorationSet,
  EditorView,
  ViewPlugin,
  type ViewUpdate,
  WidgetType,
} from "@codemirror/view";
import { findDateMatches } from "../../editor/dateFormats";
import { selectionInRange } from "../decorations/selection";

const SKIP_INSIDE = new Set<string>([
  "InlineCode",
  "FencedCode",
  "CodeBlock",
  "CodeText",
  "CodeMark",
  "CodeInfo",
  "URL",
  "WikiLinkBody",
]);

class DateWidget extends WidgetType {
  constructor(readonly text: string, readonly formatId: string) {
    super();
  }

  eq(other: DateWidget): boolean {
    return this.text === other.text && this.formatId === other.formatId;
  }

  toDOM(): HTMLElement {
    const el = document.createElement("span");
    el.className = "cm-md-date-pill";
    el.dataset.formatId = this.formatId;
    el.textContent = this.text;
    return el;
  }

  ignoreEvent(): boolean {
    // Allow CM6 to handle clicks naturally — clicking the pill places
    // the caret at its boundary, which collapses the widget on the
    // next update via the selection-intersect rule.
    return false;
  }
}

export function dateDecorations(): Extension {
  const plugin = ViewPlugin.fromClass(
    class {
      decorations: DecorationSet;

      constructor(view: EditorView) {
        this.decorations = scanDates(view);
      }

      update(u: ViewUpdate): void {
        if (u.docChanged || u.viewportChanged || u.selectionSet) {
          this.decorations = scanDates(u.view);
        }
      }
    },
    {
      decorations: (v) => v.decorations,
    },
  );
  return [
    plugin,
    // Atomic ranges share the same DecorationSet — caret motion
    // skips over the pill in one keystroke. When the selection
    // intersects a date and the widget gets suppressed (source
    // revealed), the corresponding atomic range disappears too,
    // so the caret can move freely through the source for editing.
    EditorView.atomicRanges.of(
      (view) => view.plugin(plugin)?.decorations ?? Decoration.none,
    ),
  ];
}

function scanDates(view: EditorView): DecorationSet {
  const { state } = view;
  const sel = state.selection;
  const { from, to } = view.viewport;
  const skip: Array<[number, number]> = [];
  syntaxTree(state).iterate({
    from,
    to,
    enter(node) {
      if (SKIP_INSIDE.has(node.name)) skip.push([node.from, node.to]);
    },
  });
  const decos: Array<{ from: number; to: number; deco: Decoration }> = [];
  const startLine = state.doc.lineAt(from).number;
  const endLine = state.doc.lineAt(Math.min(to, state.doc.length)).number;
  for (let n = startLine; n <= endLine; n++) {
    const line = state.doc.line(n);
    const matches = findDateMatches(line.text);
    for (const m of matches) {
      const matchFrom = line.from + m.start;
      const matchTo = line.from + m.end;
      if (overlapsAny(matchFrom, matchTo, skip)) continue;
      // Suppress the widget when the selection intersects the date
      // range; the source then becomes editable in place. The atomic
      // range ALSO disappears (since it tracks decorations) so the
      // caret can navigate through the source freely.
      if (selectionInRange(sel, matchFrom, matchTo)) continue;
      const widget = new DateWidget(m.text, m.formatId);
      decos.push({
        from: matchFrom,
        to: matchTo,
        deco: Decoration.replace({ widget }),
      });
    }
  }
  decos.sort((a, b) => a.from - b.from);
  return Decoration.set(
    decos.map((d) => d.deco.range(d.from, d.to)),
    true,
  );
}

function overlapsAny(
  from: number,
  to: number,
  ranges: Array<[number, number]>,
): boolean {
  for (const [a, b] of ranges) {
    if (from < b && to > a) return true;
  }
  return false;
}
