// In-document find for the WYSIWYG editor.
//
// ProseMirror plugin that:
//   - Holds the current query + case-sensitive flag in plugin state.
//   - Walks the doc on every state change, finding all matches in
//     plain text nodes (skipping code blocks / inline code / atom
//     smart nodes which the user shouldn't expect to be searchable).
//   - Renders a DecorationSet: every match gets `.md-find-match`,
//     and the "current" match gets an additional `.md-find-current`
//     so it stands out as the user navigates.
//   - Exposes a single `setMeta(findKey, payload)` interface for the
//     host (FindBar / Wysiwyg.svelte) to drive the plugin from
//     outside.
//
// We deliberately do NOT touch the doc; matches are decorations,
// not real nodes, so cmd+F has zero side-effects on save / undo /
// the markdown round-trip. Decorations live in the plugin's view
// layer only.

import { Extension } from "@tiptap/core";
import { Plugin, PluginKey, EditorState, Transaction } from "@tiptap/pm/state";
import { Decoration, DecorationSet } from "@tiptap/pm/view";

export type FindMatch = { from: number; to: number };

export type FindState = {
  query: string;
  caseSensitive: boolean;
  matches: FindMatch[];
  /// 0-indexed match marked `.md-find-current`. `-1` when there
  /// are no matches.
  current: number;
  decorations: DecorationSet;
};

/// Subset of FindState we expose across the component boundary.
/// Drops `decorations` (a ProseMirror internal the FindBar
/// shouldn't peek at) so consumers can fall back to a literal
/// without instantiating a DecorationSet.
export type FindSnapshot = Pick<
  FindState,
  "query" | "caseSensitive" | "matches" | "current"
>;

/// Payload accepted via `tr.setMeta(findKey, ...)`. We discriminate
/// on `kind` so a single plugin can serve multiple commands.
export type FindMeta =
  | { kind: "set"; query: string; caseSensitive: boolean }
  | { kind: "step"; delta: number }
  | { kind: "clear" };

export const findKey = new PluginKey<FindState>("md-find");

const EMPTY: FindState = {
  query: "",
  caseSensitive: false,
  matches: [],
  current: -1,
  decorations: DecorationSet.empty,
};

/// Walk the doc and collect every match for `query`. Skips text
/// inside code blocks and text marked with `code` (inline code), so
/// a search for "TODO" doesn't pile up matches inside fenced code.
/// Also skips atom node text (date / wiki / image) since their
/// rendered text isn't part of the markdown source the user is
/// searching.
function findAllMatches(
  state: EditorState,
  query: string,
  caseSensitive: boolean,
): FindMatch[] {
  if (!query) return [];
  const out: FindMatch[] = [];
  const needle = caseSensitive ? query : query.toLowerCase();
  state.doc.descendants((node, pos, parent) => {
    if (node.isAtom) return false;
    if (!node.isText || !node.text) return;
    const parentName = parent?.type.name ?? "";
    if (parentName === "codeBlock") return false;
    if (node.marks.some((m) => m.type.name === "code")) return;
    const hay = caseSensitive ? node.text : node.text.toLowerCase();
    let i = 0;
    while (i <= hay.length - needle.length) {
      const idx = hay.indexOf(needle, i);
      if (idx < 0) break;
      out.push({ from: pos + idx, to: pos + idx + needle.length });
      // Advance past this hit so `aaaa` searched for `aa` returns
      // 2 non-overlapping matches, not 3 overlapping.
      i = idx + needle.length;
    }
  });
  return out;
}

function buildDecorations(
  state: EditorState,
  matches: FindMatch[],
  current: number,
): DecorationSet {
  if (matches.length === 0) return DecorationSet.empty;
  const decos = matches.map((m, i) =>
    Decoration.inline(m.from, m.to, {
      class: i === current ? "md-find-match md-find-current" : "md-find-match",
    }),
  );
  return DecorationSet.create(state.doc, decos);
}

/// Pick the best "current" match after a doc edit: keep the
/// existing index if it's still in range, else snap to 0.
function clampCurrent(prev: number, total: number): number {
  if (total === 0) return -1;
  if (prev < 0) return 0;
  if (prev >= total) return total - 1;
  return prev;
}

export const FindExtension = Extension.create({
  name: "find",
  addProseMirrorPlugins() {
    return [
      new Plugin<FindState>({
        key: findKey,
        state: {
          init: () => ({ ...EMPTY }),
          apply(tr: Transaction, prev: FindState, _old: EditorState, next: EditorState): FindState {
            const meta = tr.getMeta(findKey) as FindMeta | undefined;
            if (meta?.kind === "set") {
              const matches = findAllMatches(next, meta.query, meta.caseSensitive);
              const current = matches.length > 0 ? 0 : -1;
              return {
                query: meta.query,
                caseSensitive: meta.caseSensitive,
                matches,
                current,
                decorations: buildDecorations(next, matches, current),
              };
            }
            if (meta?.kind === "clear") {
              return { ...EMPTY };
            }
            if (meta?.kind === "step") {
              if (prev.matches.length === 0) return prev;
              const total = prev.matches.length;
              const nextIdx = ((prev.current + meta.delta) % total + total) % total;
              return {
                ...prev,
                current: nextIdx,
                decorations: buildDecorations(next, prev.matches, nextIdx),
              };
            }
            // Doc may have changed; re-scan when the query is set
            // so highlights track edits live. Cheap-ish for typical
            // doc sizes; matches the cost we already pay for the
            // smart-node decorate path.
            if (tr.docChanged && prev.query) {
              const matches = findAllMatches(next, prev.query, prev.caseSensitive);
              const current = clampCurrent(prev.current, matches.length);
              return {
                ...prev,
                matches,
                current,
                decorations: buildDecorations(next, matches, current),
              };
            }
            return prev;
          },
        },
        props: {
          decorations(state) {
            return this.getState(state)?.decorations ?? DecorationSet.empty;
          },
        },
      }),
    ];
  },
});
