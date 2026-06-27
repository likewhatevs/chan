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

import type { EditorState } from "@codemirror/state";
import { EditorView } from "@codemirror/view";

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

export function parseListPrefix(text: string): ListPrefix | null {
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
///     useful - it splits a sentence with a stray bullet.
export function continueListOnEnter(view: EditorView): boolean {
  const sel = view.state.selection.main;
  if (!sel.empty) return false;
  const line = view.state.doc.lineAt(sel.head);
  const prefix = parseListPrefix(line.text);
  if (!prefix) return false;
  const content = line.text.slice(prefix.length);
  if (content.length === 0) {
    // Blank bullet - exit the list. Replace the whole line with
    // empty text and drop the caret at line.from.
    view.dispatch({
      changes: { from: line.from, to: line.to, insert: "" },
      selection: { anchor: line.from },
    });
    return true;
  }
  if (sel.head !== line.to) return false;
  const insert = `\n${nextPrefix(prefix)}`;
  const changes: { from: number; to: number; insert: string }[] = [
    { from: sel.head, to: sel.head, insert },
  ];
  if (prefix.ordered && prefix.number !== null) {
    appendOrderedRenumber(view.state, line.number, prefix, changes);
  }
  view.dispatch({
    changes,
    selection: { anchor: sel.head + insert.length },
  });
  return true;
}

/// Shift the ordered-list tail below `lineNumber` up by one so a freshly
/// inserted item does not duplicate the next marker (…N, N+1[new], N+1,
/// N+2… becomes …N, N+1, N+2, N+3…). The renumber edits are appended to
/// the caller's `changes` so the insert + renumber land in ONE dispatch
/// (one undo). Walks the contiguous run of same-indent ordered siblings,
/// reusing each line's own `.`/`)` separator, and steps over a SINGLE
/// blank line between items so a LOOSE (blank-separated) list renumbers
/// too. It stops at the first line that is not a same-indent ordered item
/// continuing the +1 sequence, or at two blank lines in a row (which end
/// a CommonMark list), so a deliberately non-contiguous tail, a nested
/// sublist, or a separate following list is left intact. This matches
/// @codemirror/lang-markdown's renumberList (the rich-prompt Enter path)
/// for tight and single-blank loose ordered lists; the two can still
/// diverge on exotic multi-blank / nested-block boundaries that only the
/// full parse tree resolves.
function appendOrderedRenumber(
  state: EditorState,
  lineNumber: number,
  prefix: ListPrefix,
  changes: { from: number; to: number; insert: string }[],
): void {
  let prev = prefix.number!;
  let n = lineNumber + 1;
  while (n <= state.doc.lines) {
    let line = state.doc.line(n);
    if (line.text.trim() === "") {
      // A single blank line separates the items of a loose list; step
      // over it. Two or more blanks in a row end the list (CommonMark),
      // and so does a trailing blank, so stop there.
      if (n + 1 > state.doc.lines || state.doc.line(n + 1).text.trim() === "") {
        break;
      }
      n += 1;
      line = state.doc.line(n);
    }
    const tp = parseListPrefix(line.text);
    if (!tp || !tp.ordered || tp.number === null) break;
    if (tp.indent !== prefix.indent) break;
    if (tp.number !== prev + 1) break;
    const markerStart = line.from + tp.indent.length;
    const digits = tp.marker.length - 1; // marker = digit run + 1 separator
    changes.push({
      from: markerStart,
      to: markerStart + digits,
      insert: String(prev + 2),
    });
    prev = tp.number;
    n += 1;
  }
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
  const changed = shiftListLines(view, -1);
  // Shift-Tab must never escape the editor into surrounding chrome.
  // If there was nothing to outdent, consume it as an editor-local no-op.
  return changed || true;
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
      // count as one char each - we don't try to expand them. A
      // top-level item (no leading indent) is already at the outermost
      // level, so Shift-Tab is a NO-OP there: it must NOT strip the
      // list marker. Stripping the prefix silently turned the bullet
      // into a plain paragraph, which made cmd+shift+tab feel like it
      // made things worse. Leaving a list is Enter-on-an-empty-bullet,
      // not outdent.
      let strip = 0;
      while (strip < INDENT_UNIT.length && line.text[strip] === " ") strip++;
      if (strip > 0) {
        changes.push({ from: line.from, to: line.from + strip, insert: "" });
      }
    }
  }
  if (changes.length === 0) return false;
  view.dispatch({ changes });
  return true;
}

export function listLineAt(state: EditorState, pos: number): {
  from: number;
  to: number;
  prefix: ListPrefix;
} | null {
  const line = state.doc.lineAt(pos);
  const prefix = parseListPrefix(line.text);
  return prefix ? { from: line.from, to: line.to, prefix } : null;
}

// There is no bullet caret-snap scaffolding (clampListCaretPosition,
// listAwareArrowDown/Up, listCaretGuard, isListEolClick): `*`/`+` markers
// render as real-width glyph widgets (blocks.ts BulletGlyphWidget), not a
// zero-width source char + CSS ::before glyph, so bullet lists get
// default CodeMirror cursor / click / arrow behavior - the same path
// hyphen and ordered lists use. Snap logic would only compensate for that
// decoupling. listLineAt above stays (consumed by the image-drop handler).

export function stripUnusedInlineImageSpaceOnEnter(view: EditorView): boolean {
  const sel = view.state.selection.main;
  if (!sel.empty) return false;
  const line = view.state.doc.lineAt(sel.head);
  if (sel.head !== line.to || !parseListPrefix(line.text)) return false;
  if (!/!\[[^\]\n]*\]\([^)]+#w=\d+\)[ \t]$/.test(line.text)) return false;
  view.dispatch({
    changes: { from: sel.head - 1, to: sel.head, insert: "" },
    selection: { anchor: sel.head - 1 },
  });
  return false;
}

// (listCaretGuard / isListEolClick removed - see the note above
// stripUnusedInlineImageSpaceOnEnter. Bullet markers are real-width
// glyphs now, so the click path is plain CodeMirror.)
