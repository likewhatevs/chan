// Live-preview decorations: show the markdown source for the
// element currently under the caret while keeping the rendered
// styling (heading size / bold weight / etc.) intact.
//
// Implemented as a ProseMirror plugin that emits Decoration objects
// from selection state. PM owns the decoration lifecycle, so the
// markers cannot be wiped by other plugins re-rendering the host
// node (the previous setAttribute-based approach was vulnerable to
// the fold-chevron widget rebuild on every transaction).
//
// What's covered:
//
//   - Heading: `Decoration.node` on the heading block adds
//     `data-cursor-in` + `data-cursor-prefix="#..."` attributes
//     the CSS rule in Wysiwyg.svelte renders via `::before` as the
//     muted hash prefix.
//
//   - Bold / Italic / Strike inline marks: `Decoration.widget` at
//     the mark range boundaries inserts a muted `**` / `*` / `~~`
//     span before and after the marked text. The marker span is
//     non-editable; level / weight is changed via the standard
//     shortcuts (Cmd-B / Cmd-I / Cmd-Shift-S) or by typing the
//     marker characters into surrounding text.
//
// Wiki links and images use a different model (atom -> source-mode
// swap) and are not handled here.

import { Extension, getMarkRange } from "@tiptap/core";
import type { MarkType } from "@tiptap/pm/model";
import { Plugin, PluginKey } from "@tiptap/pm/state";
import { Decoration, DecorationSet } from "@tiptap/pm/view";

/// Marker text per mark type. The plugin only adds widgets for
/// marks whose name is a key here; the schema may carry other marks
/// (link, code) we deliberately don't decorate this way.
const MARK_MARKER: Record<string, string> = {
  bold: "**",
  italic: "*",
  strike: "~~",
};

function buildMarker(text: string): HTMLElement {
  const el = document.createElement("span");
  el.className = "md-source-marker";
  el.textContent = text;
  // The widget must not be editable; PM would otherwise try to
  // place the caret inside the phantom marker. `contenteditable=false`
  // also prevents the user from accidentally typing into it.
  el.contentEditable = "false";
  return el;
}

export const LiveSourceExtension = Extension.create({
  name: "liveSource",
  addProseMirrorPlugins() {
    return [
      new Plugin({
        key: new PluginKey("liveSource"),
        props: {
          decorations(state) {
            const { selection, doc, schema } = state;
            if (!selection.empty) return null;
            const $from = doc.resolve(selection.from);
            const decos: Decoration[] = [];

            // 1. Heading prefix. Wraps the entire heading block so
            //    the CSS `::before` selector lands on the H1..H6
            //    element. data-cursor-prefix carries the hash run.
            const parent = $from.parent;
            if (parent.type.name === "heading") {
              const level = Math.min(
                6,
                Math.max(1, (parent.attrs.level as number) || 1),
              );
              const blockStart = $from.before($from.depth);
              const blockEnd = blockStart + parent.nodeSize;
              decos.push(
                Decoration.node(blockStart, blockEnd, {
                  "data-cursor-in": "",
                  "data-cursor-prefix": "#".repeat(level),
                }),
              );
            }

            // 2. Inline-mark markers. For each tracked mark, find
            //    the contiguous range covering the caret position
            //    and add muted-text widgets at both ends. The
            //    widgets sit outside the marked text so the user
            //    can see the markers without us touching the
            //    marked content itself.
            //
            //    Strike additionally gets an inline decoration
            //    class so the CSS can drop the strikethrough line
            //    while the caret is inside — otherwise the text
            //    being edited stays crossed out and is hard to
            //    read.
            for (const markName of Object.keys(MARK_MARKER)) {
              const markType: MarkType | undefined = schema.marks[markName];
              if (!markType) continue;
              const range = getMarkRange($from, markType);
              if (!range) continue;
              const marker = MARK_MARKER[markName]!;
              decos.push(
                Decoration.widget(range.from, () => buildMarker(marker), {
                  side: -1,
                  // Marker tag so we can pinpoint the widget in
                  // tests / DOM walks without leaning on its text.
                  key: `live-mark-${markName}-open`,
                }),
              );
              decos.push(
                Decoration.widget(range.to, () => buildMarker(marker), {
                  side: 1,
                  key: `live-mark-${markName}-close`,
                }),
              );
              if (markName === "strike") {
                decos.push(
                  Decoration.inline(range.from, range.to, {
                    class: "md-mark-editing-strike",
                  }),
                );
              }
            }

            return decos.length ? DecorationSet.create(doc, decos) : null;
          },
        },
      }),
    ];
  },
});
