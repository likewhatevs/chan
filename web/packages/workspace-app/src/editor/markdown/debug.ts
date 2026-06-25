// Lezer tree dump utilities for debugging the new editor's grammar.
//
// Not user-facing. Imported on demand from a dev-console command (see
// the bottom-of-file `(window as any).chanDumpTree` registration when
// the WYSIWYG mounts) so we can poke at the tree shape while wiring
// decoration handling in steps 3+.
//
// Pure functions only - no CM imports beyond types so this module can
// be tree-shaken out of production builds if a future step decides to.

import type { EditorState } from "@codemirror/state";
import { syntaxTree } from "@codemirror/language";

export type TreeDumpLine = {
  depth: number;
  name: string;
  from: number;
  to: number;
  text: string;
};

/// Walk the syntax tree across a range and return a flat list of
/// node descriptors. Intended for printing - keeps text snippets short
/// (max 40 chars, ellipsised) so the output stays readable.
export function dumpTree(
  state: EditorState,
  from = 0,
  to = state.doc.length,
): TreeDumpLine[] {
  const out: TreeDumpLine[] = [];
  syntaxTree(state).iterate({
    from,
    to,
    enter(node) {
      const slice = state.doc.sliceString(node.from, Math.min(node.to, node.from + 40));
      const text = slice.length === 40 ? `${slice}...` : slice;
      out.push({
        depth: 0, // filled in below
        name: node.name,
        from: node.from,
        to: node.to,
        text: text.replace(/\n/g, "\\n"),
      });
    },
  });
  // The iterator doesn't surface depth directly; reconstruct it from
  // the [from, to] containment relationship in the order we walked.
  const stack: Array<{ to: number }> = [];
  for (const line of out) {
    while (stack.length && line.from >= stack[stack.length - 1]!.to) {
      stack.pop();
    }
    line.depth = stack.length;
    stack.push({ to: line.to });
  }
  return out;
}

/// Format the dump for console output. Each line: indent + name +
/// `[from, to]` + first 40 chars of source.
export function formatTreeDump(lines: TreeDumpLine[]): string {
  return lines
    .map((l) => {
      const indent = "  ".repeat(l.depth);
      return `${indent}${l.name} [${l.from}, ${l.to}] ${JSON.stringify(l.text)}`;
    })
    .join("\n");
}
