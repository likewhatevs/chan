// Trigger detection: scan the doc text immediately around the caret for
// `[[`, `![`, `#`, `@` patterns and return the corresponding BubbleSpec.
//
// Rules:
//   - Empty selection only (a non-point selection is editing, not a
//     trigger). Multi-cursor: only the primary range is checked.
//   - Trigger may not span lines (if the user puts a `[[` on one line
//     and the caret on the next line, no trigger).
//   - Wiki / image triggers must not contain a `]` (which would close
//     the bracket pattern; a closed `[[x]]` shouldn't keep the bubble
//     open while the caret is at end).
//   - Tag / contact triggers require a word boundary before the
//     trigger char (so `foo#bar` and `email@domain.com` don't open
//     bubbles).
//   - Skip when the caret is inside an InlineCode / FencedCode
//     syntax range — those characters are literal source.
//
// Multiple patterns can never overlap (their trigger characters are
// disjoint) so we check in order: wiki > image > contact > tag. The
// first match wins.

import { syntaxTree } from "@codemirror/language";
import type { EditorState } from "@codemirror/state";
import type { BubbleSpec } from "./types";

const SKIP_INSIDE = new Set<string>([
  "InlineCode",
  "FencedCode",
  "CodeBlock",
  "CodeText",
  "CodeMark",
  "CodeInfo",
]);

export function computeBubbleSpec(state: EditorState): BubbleSpec | null {
  const sel = state.selection.main;
  if (!sel.empty) return null;
  const pos = sel.head;
  if (caretInsideSkipRange(state, pos)) return null;
  // Special case: caret inside an existing Image's URL portion ->
  // image bubble in "raw" template mode. Detect this BEFORE the
  // generic `[[` / `![` text scans because the caret is inside an
  // already-rendered `![alt](url)` and the surrounding brackets must
  // not be eaten on commit.
  const imageUrl = imageUrlAtCaret(state, pos);
  if (imageUrl !== null) {
    return {
      kind: "image",
      triggerStart: imageUrl.from,
      triggerEnd: imageUrl.to,
      query: imageUrl.queryUpToCaret,
      templateMode: "raw",
    };
  }
  // Same idea for an existing Link's URL portion (the `[label](path)`
  // form). The wiki bubble takes over in raw mode — commit replaces
  // just the URL, leaves the surrounding `[label](`...`)` intact.
  const linkUrl = linkUrlAtCaret(state, pos);
  if (linkUrl !== null) {
    return {
      kind: "wiki",
      triggerStart: linkUrl.from,
      triggerEnd: linkUrl.to,
      query: linkUrl.queryUpToCaret,
      templateMode: "raw",
    };
  }
  // Caret inside an existing `[[...]]` source range (the wikilink
  // pill is suppressed because selection-intersect revealed source).
  // The matchBracket text-scan below would happily fire too, but its
  // triggerEnd = caret, so a commit only replaces from `[[` to caret
  // and leaves the trailing `]]` behind. Detect the WikiLink syntax
  // node and use ITS full range so commit replaces the whole pill.
  const wikiNode = wikiLinkAtCaret(state, pos);
  if (wikiNode !== null) {
    return {
      kind: "wiki",
      triggerStart: wikiNode.from,
      triggerEnd: wikiNode.to,
      query: wikiNode.queryUpToCaret,
      templateMode: "wrap",
    };
  }
  const line = state.doc.lineAt(pos);
  const before = line.text.slice(0, pos - line.from);
  // Wiki: `[[query` (caret after the typed query, no `]` between).
  const wiki = matchBracket(before, "[[", "]");
  if (wiki !== null) {
    return {
      kind: "wiki",
      triggerStart: line.from + wiki.start,
      triggerEnd: pos,
      query: wiki.query,
    };
  }
  // Image: `![query` similarly. The opener is `![` (2 chars).
  const image = matchBracket(before, "![", "]");
  if (image !== null) {
    return {
      kind: "image",
      triggerStart: line.from + image.start,
      triggerEnd: pos,
      query: image.query,
      templateMode: "wrap",
    };
  }
  // Contact: `@word` at start-of-word. `\b@` -- but JS \b doesn't
  // include `@`, so we anchor manually.
  const contact = matchAtTrigger(before, "@");
  if (contact !== null) {
    return {
      kind: "contact",
      triggerStart: line.from + contact.start,
      triggerEnd: pos,
      query: contact.query,
    };
  }
  // Tag: `#word` at start-of-word. Decoration walker handles the
  // rendered pill; the bubble is only for picker assistance during
  // typing.
  const tag = matchAtTrigger(before, "#");
  if (tag !== null) {
    return {
      kind: "tag",
      triggerStart: line.from + tag.start,
      triggerEnd: pos,
      query: tag.query,
    };
  }
  return null;
}

/// Match `opener` followed by query chars (no `forbidden` char in
/// between, no opener char repeated which would mean a NEW opener took
/// over). Returns the start offset of the opener within `line` and the
/// query text. Returns null when no match.
function matchBracket(
  line: string,
  opener: string,
  forbidden: string,
): { start: number; query: string } | null {
  const lastOpen = line.lastIndexOf(opener);
  if (lastOpen < 0) return null;
  const queryStart = lastOpen + opener.length;
  const query = line.slice(queryStart);
  if (query.includes(forbidden)) return null;
  // For wiki `[[`, also reject when query contains `[` (a new `[[`
  // started between this opener and the caret).
  if (query.includes("[")) return null;
  // Newline guard: matchBracket runs on a single line already, so this
  // is implicit, but be defensive.
  if (query.includes("\n")) return null;
  return { start: lastOpen, query };
}

/// Match `trigger` (single char like `#` / `@`) followed by word
/// chars, with a word boundary before the trigger. Returns the
/// trigger offset within `line` and the matched query text.
function matchAtTrigger(
  line: string,
  trigger: string,
): { start: number; query: string } | null {
  // Walk back from end of line. The trigger must be the last
  // non-word/dash run terminated by the trigger char.
  // Pattern: (^|[^A-Za-z0-9_])({trigger})([A-Za-z0-9_-]*)$
  const safe = trigger.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const re = new RegExp(`(?:^|[^A-Za-z0-9_])(${safe})([A-Za-z0-9_-]*)$`);
  const m = re.exec(line);
  if (!m) return null;
  const triggerLen = m[1]!.length;
  const queryLen = m[2]!.length;
  // Reconstruct the offset of the trigger char.
  const triggerOffset = m.index + (m[0].length - queryLen - triggerLen);
  return { start: triggerOffset, query: m[2]! };
}

function caretInsideSkipRange(state: EditorState, pos: number): boolean {
  const node = syntaxTree(state).resolveInner(pos, -1);
  let cur: typeof node | null = node;
  while (cur) {
    if (SKIP_INSIDE.has(cur.name)) return true;
    cur = cur.parent;
  }
  return false;
}

function linkUrlAtCaret(
  state: EditorState,
  pos: number,
): { from: number; to: number; queryUpToCaret: string } | null {
  return urlSlotAtCaret(state, pos, "Link");
}

/// Walk up from pos looking for a WikiLink syntax node. When found
/// returns the OUTER node range (covering `[[`...`]]`) plus a query
/// extracted from the body up to the caret — what the user has typed
/// so far inside the existing pill. The bubble then replaces the
/// whole node on commit instead of just the prefix-before-caret.
function wikiLinkAtCaret(
  state: EditorState,
  pos: number,
): { from: number; to: number; queryUpToCaret: string } | null {
  let node: ReturnType<typeof syntaxTree>["topNode"] | null = syntaxTree(
    state,
  ).resolveInner(pos, 0);
  while (node) {
    if (node.name === "WikiLink") {
      // Find the WikiLinkBody child for an accurate query slice.
      const cursor = node.cursor();
      if (!cursor.firstChild()) return null;
      let bodyFrom = -1;
      let bodyTo = -1;
      do {
        if (cursor.name === "WikiLinkBody") {
          bodyFrom = cursor.from;
          bodyTo = cursor.to;
          break;
        }
      } while (cursor.nextSibling());
      if (bodyFrom < 0) return null;
      const clampedPos = Math.max(bodyFrom, Math.min(pos, bodyTo));
      return {
        from: node.from,
        to: node.to,
        queryUpToCaret: state.doc.sliceString(bodyFrom, clampedPos),
      };
    }
    node = node.parent;
  }
  return null;
}

function imageUrlAtCaret(
  state: EditorState,
  pos: number,
): { from: number; to: number; queryUpToCaret: string } | null {
  return urlSlotAtCaret(state, pos, "Image");
}

/// Common URL-slot detector for both Link and Image syntax nodes.
/// Anchor on the LinkMark `(` / `)` children rather than the URL
/// child: when the URL is empty (`![]()`) or the caret sits at a
/// URL boundary (`![](|foo)` with caret at `(`), there's no URL node
/// to resolveInner onto. Falling back to the slot-between-parens
/// gives a consistent trigger range and an empty query.
function urlSlotAtCaret(
  state: EditorState,
  pos: number,
  parentName: "Link" | "Image",
): { from: number; to: number; queryUpToCaret: string } | null {
  let node: ReturnType<typeof syntaxTree>["topNode"] | null = syntaxTree(
    state,
  ).resolveInner(pos, 0);
  while (node) {
    if (node.name === parentName) {
      const cursor = node.cursor();
      if (!cursor.firstChild()) return null;
      const linkMarks: Array<{ from: number; to: number }> = [];
      do {
        if (cursor.name === "LinkMark") {
          linkMarks.push({ from: cursor.from, to: cursor.to });
        }
      } while (cursor.nextSibling());
      // Link / Image have four LinkMarks: [, ], (, ). The URL slot
      // sits between linkMarks[2] (`(`) and linkMarks[3] (`)`).
      if (linkMarks.length < 4) return null;
      const slotFrom = linkMarks[2]!.to;
      const slotTo = linkMarks[3]!.from;
      if (pos < slotFrom || pos > slotTo) return null;
      return {
        from: slotFrom,
        to: slotTo,
        queryUpToCaret: state.doc.sliceString(slotFrom, pos),
      };
    }
    node = node.parent;
  }
  return null;
}
