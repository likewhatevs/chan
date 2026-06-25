import { syntaxTree } from "@codemirror/language";
import {
  Decoration,
  EditorView,
  ViewPlugin,
  type DecorationSet,
  type ViewUpdate,
} from "@codemirror/view";
import {
  foldEffect,
  foldedRanges,
  unfoldEffect,
} from "@codemirror/language";
import { RangeSetBuilder, type Extension } from "@codemirror/state";

const trailingWhitespaceMark = Decoration.mark({
  class: "cm-trailing-whitespace",
});

// Keep these tools CM-native so stripping/folding maps selections and
// history like ordinary editor edits instead of rewriting the Svelte buffer.
function buildTrailingWhitespaceDecorations(view: EditorView): DecorationSet {
  const builder = new RangeSetBuilder<Decoration>();
  const lineCount = view.state.doc.lines;
  for (let n = 1; n <= lineCount; n++) {
    const line = view.state.doc.line(n);
    const match = /[ \t]+$/.exec(line.text);
    if (!match) continue;
    builder.add(line.from + match.index, line.to, trailingWhitespaceMark);
  }
  return builder.finish();
}

export function trailingWhitespaceHighlight(): Extension {
  const plugin = ViewPlugin.fromClass(
    class {
      decorations: DecorationSet;

      constructor(view: EditorView) {
        this.decorations = buildTrailingWhitespaceDecorations(view);
      }

      update(update: ViewUpdate): void {
        if (update.docChanged || update.viewportChanged) {
          this.decorations = buildTrailingWhitespaceDecorations(update.view);
        }
      }
    },
    {
      decorations: (plugin) => plugin.decorations,
    },
  );
  return [plugin] as Extension;
}

export function stripTrailingWhitespaceText(text: string): string {
  return text.replace(/[ \t]+$/gm, "");
}

export function removeTrailingWhitespace(view: EditorView): boolean {
  const changes: Array<{ from: number; to: number; insert: string }> = [];
  const lineCount = view.state.doc.lines;
  for (let n = 1; n <= lineCount; n++) {
    const line = view.state.doc.line(n);
    const match = /[ \t]+$/.exec(line.text);
    if (!match) continue;
    changes.push({ from: line.from + match.index, to: line.to, insert: "" });
  }
  if (changes.length === 0) return false;
  view.dispatch({ changes });
  return true;
}

function fencedCodeRanges(view: EditorView): Array<{ from: number; to: number }> {
  const ranges: Array<{ from: number; to: number }> = [];
  syntaxTree(view.state).iterate({
    enter(node) {
      if (node.name !== "FencedCode" && node.name !== "CodeBlock") return;
      const fromLine = view.state.doc.lineAt(node.from);
      const toLine = view.state.doc.lineAt(node.to);
      if (fromLine.number === toLine.number) return;
      const from = fromLine.to;
      const to = toLine.from > from ? toLine.from - 1 : node.to;
      if (to > from) ranges.push({ from, to });
    },
  });
  return ranges;
}

function foldedRangeAt(view: EditorView, from: number): { from: number; to: number } | null {
  let hit: { from: number; to: number } | null = null;
  foldedRanges(view.state).between(from, from, (foldFrom, foldTo) => {
    hit = { from: foldFrom, to: foldTo };
    return false;
  });
  return hit;
}

export function toggleCodeBlocks(view: EditorView): boolean {
  const ranges = fencedCodeRanges(view);
  if (ranges.length === 0) return false;
  const hasOpenBlock = ranges.some((range) => !foldedRangeAt(view, range.from));
  if (hasOpenBlock) {
    view.dispatch({ effects: ranges.map((range) => foldEffect.of(range)) });
  } else {
    const folded = ranges
      .map((range) => foldedRangeAt(view, range.from))
      .filter((range): range is { from: number; to: number } => Boolean(range));
    if (folded.length === 0) return false;
    view.dispatch({ effects: folded.map((range) => unfoldEffect.of(range)) });
  }
  return true;
}
