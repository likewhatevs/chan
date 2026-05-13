// Format commands for the StyleToolbar contract.
//
// All commands operate on the markdown SOURCE directly — no
// PM-command-object indirection. Mark toggles wrap/unwrap markers;
// block toggles add/remove line prefixes; isActive walks the syntax
// tree.
//
// Naming follows the legacy editor's contract so StyleToolbar.svelte
// works at cutover with no edits beyond importing from the new
// component.

import { syntaxTree } from "@codemirror/language";
import type { EditorView } from "@codemirror/view";

// ---- mark toggles --------------------------------------------------------

interface MarkSpec {
  name: string;
  marker: string;
  innerNode: string; // syntax node name when the mark is applied
}

const MARK_BOLD: MarkSpec = {
  name: "bold",
  marker: "**",
  innerNode: "StrongEmphasis",
};
const MARK_ITALIC: MarkSpec = {
  name: "italic",
  marker: "*",
  innerNode: "Emphasis",
};
const MARK_STRIKE: MarkSpec = {
  name: "strike",
  marker: "~~",
  innerNode: "Strikethrough",
};
const MARK_CODE: MarkSpec = {
  name: "code",
  marker: "`",
  innerNode: "InlineCode",
};

function toggleMark(view: EditorView, spec: MarkSpec): void {
  const sel = view.state.selection.main;
  const m = spec.marker;
  // Are we currently inside the mark? Walk ancestors at caret head.
  const inner = findAncestor(view, sel.head, spec.innerNode);
  if (inner) {
    // Unwrap: remove the open and close markers and keep inner content.
    // The marker positions are the first and last children of `inner`
    // (per lezer's mark layout: open EmphasisMark, content, close
    // EmphasisMark — same shape for Strong/Strike/InlineCode).
    const cursor = inner.cursor();
    if (!cursor.firstChild()) return;
    const openFrom = cursor.from;
    const openTo = cursor.to;
    let closeFrom = openFrom;
    let closeTo = openTo;
    while (cursor.nextSibling()) {
      closeFrom = cursor.from;
      closeTo = cursor.to;
    }
    // Apply close-then-open so positions remain valid.
    view.dispatch({
      changes: [
        { from: closeFrom, to: closeTo },
        { from: openFrom, to: openTo },
      ],
    });
    return;
  }
  if (sel.empty) {
    // Insert paired markers and place caret between them.
    view.dispatch({
      changes: { from: sel.from, to: sel.to, insert: `${m}${m}` },
      selection: { anchor: sel.from + m.length },
    });
    return;
  }
  // Wrap selection.
  const text = view.state.doc.sliceString(sel.from, sel.to);
  view.dispatch({
    changes: { from: sel.from, to: sel.to, insert: `${m}${text}${m}` },
    selection: {
      anchor: sel.from + m.length,
      head: sel.to + m.length,
    },
  });
}

export function toggleBold(view: EditorView): void {
  toggleMark(view, MARK_BOLD);
}
export function toggleItalic(view: EditorView): void {
  toggleMark(view, MARK_ITALIC);
}
export function toggleStrike(view: EditorView): void {
  toggleMark(view, MARK_STRIKE);
}
export function toggleInlineCode(view: EditorView): void {
  toggleMark(view, MARK_CODE);
}

// ---- block toggles -------------------------------------------------------

export type BlockKind = "h1" | "h2" | "h3" | "normal" | "code" | "quote";

const HEADING_PREFIX: Record<BlockKind, string | null> = {
  h1: "# ",
  h2: "## ",
  h3: "### ",
  normal: null,
  code: null,
  quote: null,
};

/// Set the current block (the line containing the caret) to the given
/// kind. Strips any existing heading / quote / fence prefix first, then
/// applies the new prefix (or wraps the line in a code fence for "code").
/// "normal" strips all prefixes.
export function setBlockKind(view: EditorView, kind: BlockKind): void {
  const sel = view.state.selection.main;
  const line = view.state.doc.lineAt(sel.head);
  // Strip leading `#`/`>` runs + the trailing space.
  let text = line.text;
  const m = /^((?:[#]+\s+)|(?:>\s+))/.exec(text);
  if (m) text = text.slice(m[0].length);
  if (kind === "h1" || kind === "h2" || kind === "h3") {
    const prefix = HEADING_PREFIX[kind]!;
    view.dispatch({
      changes: { from: line.from, to: line.to, insert: `${prefix}${text}` },
    });
    return;
  }
  if (kind === "quote") {
    view.dispatch({
      changes: { from: line.from, to: line.to, insert: `> ${text}` },
    });
    return;
  }
  if (kind === "code") {
    // Wrap the current line in a fenced block.
    const fenced = `\`\`\`\n${text}\n\`\`\``;
    view.dispatch({
      changes: { from: line.from, to: line.to, insert: fenced },
    });
    return;
  }
  // "normal" — strip prefixes only.
  view.dispatch({
    changes: { from: line.from, to: line.to, insert: text },
  });
}

/// Toggle a list prefix on the current line. If the line already starts
/// with the target prefix, strip it; otherwise replace any existing
/// list / heading / quote prefix with the new one.
function toggleLinePrefix(view: EditorView, target: string): void {
  const sel = view.state.selection.main;
  const line = view.state.doc.lineAt(sel.head);
  const text = line.text;
  // Existing prefix detection (any list / task / quote / heading).
  const m = /^((?:[-*+]\s+(?:\[[ xX]\]\s+)?)|(?:\d+\.\s+)|(?:>\s+)|(?:[#]+\s+))/.exec(text);
  const existing = m ? m[0] : "";
  const inner = m ? text.slice(existing.length) : text;
  if (existing === target) {
    // Already this prefix — strip it.
    view.dispatch({
      changes: { from: line.from, to: line.to, insert: inner },
    });
    return;
  }
  view.dispatch({
    changes: { from: line.from, to: line.to, insert: `${target}${inner}` },
  });
}

export function toggleBulletList(view: EditorView): void {
  toggleLinePrefix(view, "- ");
}
export function toggleOrderedList(view: EditorView): void {
  toggleLinePrefix(view, "1. ");
}
export function toggleTaskList(view: EditorView): void {
  toggleLinePrefix(view, "- [ ] ");
}

export function insertHorizontalRule(view: EditorView): void {
  const sel = view.state.selection.main;
  const line = view.state.doc.lineAt(sel.head);
  // Insert a fresh `---` line below the current; if the current line
  // is empty, replace it.
  if (line.text.trim() === "") {
    view.dispatch({
      changes: { from: line.from, to: line.to, insert: "---" },
    });
  } else {
    view.dispatch({
      changes: { from: line.to, to: line.to, insert: "\n\n---\n" },
    });
  }
}

export function insertImage(view: EditorView): void {
  // Insert `![](|)` at caret; the `![` triggers the image bubble on
  // the next transaction.
  const sel = view.state.selection.main;
  const insert = "![](";
  view.dispatch({
    changes: { from: sel.from, to: sel.to, insert: `${insert})` },
    selection: { anchor: sel.from + insert.length },
  });
}

export function toggleLink(view: EditorView, url: string): void {
  const sel = view.state.selection.main;
  if (sel.empty) {
    const insert = `[](${url})`;
    view.dispatch({
      changes: { from: sel.from, to: sel.to, insert },
      selection: { anchor: sel.from + 1 }, // caret in the label
    });
    return;
  }
  const text = view.state.doc.sliceString(sel.from, sel.to);
  view.dispatch({
    changes: { from: sel.from, to: sel.to, insert: `[${text}](${url})` },
  });
}

// ---- introspection -------------------------------------------------------

export function isActive(view: EditorView, name: string): boolean {
  const pos = view.state.selection.main.head;
  switch (name) {
    case "bold":
      return !!findAncestor(view, pos, "StrongEmphasis");
    case "italic":
      return !!findAncestor(view, pos, "Emphasis");
    case "strike":
      return !!findAncestor(view, pos, "Strikethrough");
    case "code":
      return !!findAncestor(view, pos, "InlineCode");
    case "link":
      return !!findAncestor(view, pos, "Link") || !!findAncestor(view, pos, "Autolink");
    case "bulletList":
      return !!findAncestor(view, pos, "BulletList");
    case "orderedList":
      return !!findAncestor(view, pos, "OrderedList");
    case "taskList":
      return !!findAncestor(view, pos, "Task");
    case "blockquote":
      return !!findAncestor(view, pos, "Blockquote");
    default:
      return false;
  }
}

export function currentBlockKind(view: EditorView): BlockKind {
  const pos = view.state.selection.main.head;
  // Walk to the innermost block-level ancestor.
  let node = syntaxTree(view.state).resolveInner(pos, 0);
  while (node) {
    switch (node.name) {
      case "ATXHeading1":
        return "h1";
      case "ATXHeading2":
        return "h2";
      case "ATXHeading3":
        return "h3";
      case "ATXHeading4":
      case "ATXHeading5":
      case "ATXHeading6":
        return "h3"; // collapsed into h3 (StyleToolbar only exposes h1-h3)
      case "Blockquote":
        return "quote";
      case "FencedCode":
      case "CodeBlock":
        return "code";
    }
    if (!node.parent) break;
    node = node.parent;
  }
  return "normal";
}

// ---- helpers -------------------------------------------------------------

function findAncestor(
  view: EditorView,
  pos: number,
  name: string,
): import("@lezer/common").SyntaxNode | null {
  let node: import("@lezer/common").SyntaxNode | null = syntaxTree(
    view.state,
  ).resolveInner(pos, 0);
  while (node) {
    if (node.name === name) return node;
    node = node.parent;
  }
  return null;
}
