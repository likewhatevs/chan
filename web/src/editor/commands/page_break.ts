import { StateField, type Extension } from "@codemirror/state";
import {
  Decoration,
  type DecorationSet,
  EditorView,
  WidgetType,
} from "@codemirror/view";
import type { EditorView as EditorViewType } from "@codemirror/view";
import { lineIntersect } from "../decorations/selection";

export const PAGE_BREAK_MARKER = '<hr class="chan-page-break">';

const TRIGGERS = ["@pagebreak", "@break"] as const;
const PAGE_BREAK_LINE_RE =
  /^\s*<hr\s+class=(["'])chan-page-break\1\s*\/?>\s*$/i;

export function isPageBreakLine(text: string): boolean {
  return PAGE_BREAK_LINE_RE.test(text);
}

function detectTrigger(view: EditorViewType): {
  from: number;
  to: number;
} | null {
  const sel = view.state.selection.main;
  if (!sel.empty) return null;
  const pos = sel.head;
  const line = view.state.doc.lineAt(pos);
  const before = line.text.slice(0, pos - line.from);
  for (const keyword of TRIGGERS) {
    if (!before.endsWith(keyword)) continue;
    const start = before.length - keyword.length;
    if (start > 0 && !/\s/.test(before[start - 1]!)) continue;
    return { from: line.from + start, to: pos };
  }
  return null;
}

function consumeLineBreak(state: EditorViewType["state"], to: number): number {
  return to < state.doc.length ? to + 1 : to;
}

function trimInlineSpaceAroundTrigger(
  view: EditorViewType,
  from: number,
  to: number,
): { from: number; to: number } {
  const line = view.state.doc.lineAt(from);
  let nextFrom = from;
  let nextTo = to;
  while (nextFrom > line.from) {
    const prev = view.state.doc.sliceString(nextFrom - 1, nextFrom);
    if (prev !== " " && prev !== "\t") break;
    nextFrom -= 1;
  }
  while (nextTo < line.to) {
    const next = view.state.doc.sliceString(nextTo, nextTo + 1);
    if (next !== " " && next !== "\t") break;
    nextTo += 1;
  }
  return { from: nextFrom, to: nextTo };
}

export function expandPageBreakMacro(view: EditorViewType): boolean {
  const hit = detectTrigger(view);
  if (!hit) return false;
  const line = view.state.doc.lineAt(hit.from);
  const before = line.text.slice(0, hit.from - line.from);
  const after = line.text.slice(hit.to - line.from);
  let from: number;
  let to: number;
  let insert: string;

  if (before.trim() === "") {
    from = line.from;
    to = after.trim() === ""
      ? consumeLineBreak(view.state, line.to)
      : trimInlineSpaceAroundTrigger(view, hit.from, hit.to).to;
    insert = `${PAGE_BREAK_MARKER}\n\n`;
  } else {
    const trimmed = trimInlineSpaceAroundTrigger(view, hit.from, hit.to);
    from = trimmed.from;
    to = after.trim() === "" ? line.to : trimmed.to;
    insert = `\n\n${PAGE_BREAK_MARKER}\n\n`;
  }

  view.dispatch({
    changes: { from, to, insert },
    selection: { anchor: from + insert.length },
  });
  return true;
}

class PageBreakWidget extends WidgetType {
  eq(_other: PageBreakWidget): boolean {
    return true;
  }

  toDOM(): HTMLElement {
    const wrap = document.createElement("div");
    wrap.className = "cm-md-page-break";
    const rule = document.createElement("span");
    rule.className = "cm-md-page-break-rule";
    const label = document.createElement("span");
    label.className = "cm-md-page-break-label";
    label.textContent = "Page break";
    wrap.append(rule, label);
    return wrap;
  }

  ignoreEvent(): boolean {
    return false;
  }
}

function scanPageBreaks(state: EditorViewType["state"]): DecorationSet {
  const decos: Array<{ from: number; to: number; deco: Decoration }> = [];
  for (let lineNo = 1; lineNo <= state.doc.lines; lineNo++) {
    const line = state.doc.line(lineNo);
    if (!isPageBreakLine(line.text)) continue;
    if (lineIntersect(state, line.from, line.to, state.selection)) continue;
    decos.push({
      from: line.from,
      to: line.to,
      deco: Decoration.replace({
        widget: new PageBreakWidget(),
        block: true,
      }),
    });
  }
  return Decoration.set(
    decos.map((d) => d.deco.range(d.from, d.to)),
    true,
  );
}

export function pageBreakDecorations(): Extension {
  const field = StateField.define<DecorationSet>({
    create(state) {
      return scanPageBreaks(state);
    },
    update(decorations, tr) {
      if (!tr.docChanged && !tr.selection) return decorations;
      return scanPageBreaks(tr.state);
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
