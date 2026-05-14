// List-editing keybinds for the WYSIWYG editor.
//
// All commands operate on the markdown source via regex on the
// current line. We do NOT consult the syntax tree: the user's intent
// while typing is "I'm on a line that LOOKS like a list item", which
// the prefix regex captures faithfully even when @lezer/markdown
// hasn't reparsed yet (e.g. the line above just changed).
//
// The matchers here are intentionally permissive about the marker
// character. Bullet lists accept `-`, `*`, `+`. Ordered lists accept
// any run of digits followed by `.` or `)`. Task lists are detected
// as a bullet item whose content starts with `[ ]` / `[x]` / `[X]`.

import type { EditorView } from "@codemirror/view";

/// Anchored at line start. Captures:
///   1: leading whitespace (indent)
///   2: marker (`-` / `*` / `+` / `123.` / `123)`)
///   3: trailing whitespace after the marker (always present per CommonMark)
///   4: optional task box (`[ ]` / `[x]` / `[X]`)
///   5: trailing whitespace after the task box
const LIST_PREFIX_RE =
  /^([ \t]*)([-*+]|\d+[.)])([ \t]+)(\[[ xX]\])?([ \t]*)/;

interface ListPrefix {
  indent: string;
  marker: string;
  markerSpace: string;
  taskBox: string | null;
  taskSpace: string;
  /// Length of the full prefix in chars (indent + marker + spaces +
  /// optional task box + trailing spaces).
  length: number;
  /// True when marker is a digit run.
  ordered: boolean;
  /// Parsed digit value when `ordered`, else null.
  number: number | null;
}

function parseListPrefix(text: string): ListPrefix | null {
  const m = LIST_PREFIX_RE.exec(text);
  if (!m) return null;
  const [whole, indent, marker, markerSpace, taskBox, taskSpace] = m;
  const ordered = /^\d/.test(marker);
  return {
    indent,
    marker,
    markerSpace,
    taskBox: taskBox ?? null,
    taskSpace: taskSpace ?? "",
    length: whole.length,
    ordered,
    number: ordered ? parseInt(marker, 10) : null,
  };
}

/// Build the next marker for the same list. Bullets keep their char;
/// ordered lists increment the number and reuse the original
/// separator (`.` or `)`); task lists start fresh as `[ ]` regardless
/// of the source line's checked state.
function nextPrefix(prev: ListPrefix): string {
  let marker = prev.marker;
  if (prev.ordered && prev.number !== null) {
    const sep = prev.marker.endsWith(")") ? ")" : ".";
    marker = `${prev.number + 1}${sep}`;
  }
  const task = prev.taskBox ? `[ ]${prev.taskSpace || " "}` : "";
  return `${prev.indent}${marker}${prev.markerSpace}${task}`;
}

/// Enter on a list line.
///   - Empty item (no content after the prefix) at any caret position
///     on that line → strip the prefix entirely. This is how the user
///     exits the list: hit Enter on a blank bullet.
///   - Non-empty item, caret at end of line → split: insert newline
///     plus the next marker, drop caret after it.
///   - Non-empty item, caret elsewhere → fall through (default
///     newline). Auto-continuing mid-line is more annoying than
///     useful — it splits a sentence with a stray bullet.
export function continueListOnEnter(view: EditorView): boolean {
  const sel = view.state.selection.main;
  if (!sel.empty) return false;
  const line = view.state.doc.lineAt(sel.head);
  const prefix = parseListPrefix(line.text);
  if (!prefix) return false;
  const content = line.text.slice(prefix.length);
  if (content.length === 0) {
    // Blank bullet — exit the list. Replace the whole line with
    // empty text and drop the caret at line.from.
    view.dispatch({
      changes: { from: line.from, to: line.to, insert: "" },
      selection: { anchor: line.from },
    });
    return true;
  }
  if (sel.head !== line.to) return false;
  const insert = `\n${nextPrefix(prefix)}`;
  view.dispatch({
    changes: { from: sel.head, to: sel.head, insert },
    selection: { anchor: sel.head + insert.length },
  });
  return true;
}

const INDENT_UNIT = "  "; // 2 spaces

/// Tab on a list line indents the item one level (2 spaces). When the
/// selection covers multiple lines and any of them is a list item,
/// indent every line in the range. Returns false when no touched
/// line is a list item so the caret falls through to CM6's default
/// (which inserts an indent character / focus-leaves on Tab,
/// depending on configuration).
export function indentListItem(view: EditorView): boolean {
  return shiftListLines(view, +1);
}

/// Shift-Tab outdents one level. Same line-range rule as
/// indentListItem. Returns false when nothing changed (so the
/// keypress can route to its default).
export function outdentListItem(view: EditorView): boolean {
  return shiftListLines(view, -1);
}

function shiftListLines(view: EditorView, dir: 1 | -1): boolean {
  const sel = view.state.selection.main;
  const startLine = view.state.doc.lineAt(sel.from);
  const endLine = view.state.doc.lineAt(sel.to);
  // Eligibility: at least one touched line is a list item.
  let anyList = false;
  for (let n = startLine.number; n <= endLine.number; n++) {
    if (parseListPrefix(view.state.doc.line(n).text)) {
      anyList = true;
      break;
    }
  }
  if (!anyList) return false;
  const changes: { from: number; to: number; insert: string }[] = [];
  for (let n = startLine.number; n <= endLine.number; n++) {
    const line = view.state.doc.line(n);
    if (!parseListPrefix(line.text)) continue;
    if (dir === +1) {
      changes.push({ from: line.from, to: line.from, insert: INDENT_UNIT });
    } else {
      // Strip up to INDENT_UNIT.length leading space chars. Tabs
      // count as one char each — we don't try to expand them.
      let strip = 0;
      while (strip < INDENT_UNIT.length && line.text[strip] === " ") strip++;
      if (strip === 0) continue;
      changes.push({ from: line.from, to: line.from + strip, insert: "" });
    }
  }
  if (changes.length === 0) return false;
  view.dispatch({ changes });
  return true;
}
