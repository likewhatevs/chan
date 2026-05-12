// Find-on-page highlight layer for the Wysiwyg editor.
//
// A ProseMirror plugin keyed off external state: the host
// (FindBar via Wysiwyg.svelte.findAdapter) dispatches transactions
// carrying SET_FIND_RANGES meta payloads; the plugin folds those
// into a DecorationSet that paints `.find-match` on every match
// and `.find-match--current` on the active one.
//
// We don't compute matches inside the plugin: the scan walks PM
// text nodes from outside (so it can share the pure matcher in
// find.ts with the CodeMirror adapter) and just hands us the
// ranges. The plugin's only job is rendering + map-through-edits.

import { Extension } from "@tiptap/core";
import { Plugin, PluginKey } from "@tiptap/pm/state";
import type { EditorState, Transaction } from "@tiptap/pm/state";
import { Decoration, DecorationSet } from "@tiptap/pm/view";

export type FindRange = { from: number; to: number };

export type FindPluginState = {
  ranges: FindRange[];
  currentIndex: number;
  /// Latest decoration set; regenerated whenever ranges/current
  /// change and mapped through unrelated edits so single-character
  /// insertions don't force a full rescan.
  decos: DecorationSet;
};

export const findPluginKey = new PluginKey<FindPluginState>("findHighlight");

export const SET_FIND_RANGES_META = "chan.find.setRanges";

export type SetFindRangesPayload = {
  ranges: FindRange[];
  currentIndex: number;
};

function buildDecos(
  doc: import("@tiptap/pm/model").Node,
  ranges: FindRange[],
  currentIndex: number,
): DecorationSet {
  if (ranges.length === 0) return DecorationSet.empty;
  const docEnd = doc.content.size;
  const decos: Decoration[] = [];
  for (let i = 0; i < ranges.length; i++) {
    const r = ranges[i]!;
    if (r.from >= r.to) continue;
    if (r.from < 0 || r.to > docEnd) continue;
    const cls = i === currentIndex ? "find-match find-match--current" : "find-match";
    decos.push(Decoration.inline(r.from, r.to, { class: cls }));
  }
  return DecorationSet.create(doc, decos);
}

export function createFindHighlightExtension() {
  return Extension.create({
    name: "findHighlight",
    addProseMirrorPlugins() {
      return [
        new Plugin<FindPluginState>({
          key: findPluginKey,
          state: {
            init(): FindPluginState {
              return {
                ranges: [],
                currentIndex: -1,
                decos: DecorationSet.empty,
              };
            },
            apply(
              tr: Transaction,
              prev: FindPluginState,
              _oldState: EditorState,
              newState: EditorState,
            ): FindPluginState {
              const meta = tr.getMeta(SET_FIND_RANGES_META) as
                | SetFindRangesPayload
                | undefined;
              if (meta) {
                return {
                  ranges: meta.ranges,
                  currentIndex: meta.currentIndex,
                  decos: buildDecos(newState.doc, meta.ranges, meta.currentIndex),
                };
              }
              if (!tr.docChanged) return prev;
              // Map existing ranges through the edit so the
              // highlight tracks the doc without a synchronous
              // rescan. The host re-scans on a debounce when the
              // bar is open; until then, mapped ranges are a
              // good-enough approximation.
              const mapped: FindRange[] = [];
              for (const r of prev.ranges) {
                const from = tr.mapping.map(r.from, 1);
                const to = tr.mapping.map(r.to, -1);
                if (to > from) mapped.push({ from, to });
              }
              return {
                ranges: mapped,
                currentIndex: prev.currentIndex,
                decos: buildDecos(newState.doc, mapped, prev.currentIndex),
              };
            },
          },
          props: {
            decorations(state: EditorState) {
              return findPluginKey.getState(state)?.decos ?? null;
            },
          },
        }),
      ];
    },
  });
}
