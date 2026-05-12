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
import { NodeSelection, Plugin, PluginKey } from "@tiptap/pm/state";
import { Decoration, DecorationSet } from "@tiptap/pm/view";

/// Marker text per mark type. The plugin only adds widgets for
/// marks whose name is a key here; the schema may carry other marks
/// (link) we deliberately don't decorate this way.
///
/// Bold / italic / strike are NOT in this map any more: those marks
/// route through Wysiwyg.svelte's `syncLiveMarkSource`, which mutates
/// the doc on caret enter to surface the literal `**` / `*` / `~~`
/// chars as editable text. Leaving them as widgets here too would
/// double-paint the markers and the doc text would shift relative
/// to the rendered output.
const MARK_MARKER: Record<string, string> = {
  code: "`",
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
            const decos: Decoration[] = [];

            // Bracket shading runs regardless of selection because
            // `[[...]]` only appears in the doc while a wiki link
            // is being created or edited; muting the brackets makes
            // the user's actual label / query stand out. Scans each
            // textblock once; atoms count as one position via the
            // `\0` leaf-text placeholder so regex offsets stay in
            // sync with PM positions.
            doc.descendants((node, pos) => {
              if (!node.isTextblock) return true;
              const startContent = pos + 1;
              const endContent = pos + node.nodeSize - 1;
              if (endContent <= startContent) return false;
              const text = doc.textBetween(
                startContent,
                endContent,
                "\n",
                "\0",
              );
              const re = /\[\[([^\]]*)\]\]/g;
              let m: RegExpExecArray | null;
              while ((m = re.exec(text)) !== null) {
                const openStart = startContent + m.index;
                const openEnd = openStart + 2;
                const closeStart = openStart + 2 + (m[1] ?? "").length;
                const closeEnd = closeStart + 2;
                decos.push(
                  Decoration.inline(openStart, openEnd, {
                    class: "md-wiki-bracket",
                  }),
                );
                decos.push(
                  Decoration.inline(closeStart, closeEnd, {
                    class: "md-wiki-bracket",
                  }),
                );
              }

              // Image-source markdown syntax coloring. Same idea as
              // wiki brackets: when an image atom is in edit mode
              // its `![alt](src)` is laid down as plain text in the
              // textblock; we tint the punctuation so the alt and
              // src text stand out from the markers.
              const imgRe = /!\[([^\]]*)\]\(([^)]*)\)/g;
              let im: RegExpExecArray | null;
              while ((im = imgRe.exec(text)) !== null) {
                const matchStart = startContent + im.index;
                const altLen = (im[1] ?? "").length;
                const srcLen = (im[2] ?? "").length;
                // `![`
                decos.push(
                  Decoration.inline(matchStart, matchStart + 2, {
                    class: "md-image-marker",
                  }),
                );
                // alt text
                if (altLen > 0) {
                  decos.push(
                    Decoration.inline(
                      matchStart + 2,
                      matchStart + 2 + altLen,
                      { class: "md-image-alt" },
                    ),
                  );
                }
                // `](`
                decos.push(
                  Decoration.inline(
                    matchStart + 2 + altLen,
                    matchStart + 2 + altLen + 2,
                    { class: "md-image-marker" },
                  ),
                );
                // src text
                if (srcLen > 0) {
                  decos.push(
                    Decoration.inline(
                      matchStart + 2 + altLen + 2,
                      matchStart + 2 + altLen + 2 + srcLen,
                      { class: "md-image-src" },
                    ),
                  );
                }
                // `)`
                decos.push(
                  Decoration.inline(
                    matchStart + 2 + altLen + 2 + srcLen,
                    matchStart + 2 + altLen + 2 + srcLen + 1,
                    { class: "md-image-marker" },
                  ),
                );
              }
              return false;
            });

            // Horizontal rule live source. HR is a void leaf, so it
            // can't carry the cursor or hold text we could edit in
            // place. Instead, when the rule is NodeSelected (click or
            // arrow-cursor onto it), tag it with `data-cursor-in` so
            // the CSS collapses the line, and inject a sibling widget
            // showing the literal `---` source. NodeSelections have
            // `from + 1 == to`, so this branch must run before the
            // `!selection.empty` early return below.
            if (
              selection instanceof NodeSelection &&
              selection.node.type.name === "horizontalRule"
            ) {
              const from = selection.from;
              const to = selection.to;
              decos.push(
                Decoration.node(from, to, {
                  "data-cursor-in": "",
                }),
              );
              decos.push(
                Decoration.widget(
                  from,
                  () => {
                    const el = document.createElement("div");
                    el.className = "md-hr-source";
                    el.textContent = "---";
                    el.contentEditable = "false";
                    return el;
                  },
                  { side: -1, key: "live-hr-source" },
                ),
              );
            }

            if (!selection.empty) {
              return decos.length ? DecorationSet.create(doc, decos) : null;
            }
            const $from = doc.resolve(selection.from);

            // 1a. Code block cursor-in marker. The `CodeBlockFenced`
            //     NodeView hides its fence rows and shows a small
            //     language badge by default; when the caret is
            //     inside, CSS flips that around so the user can edit
            //     the language slot and see the literal backticks.
            const parent = $from.parent;
            if (parent.type.name === "codeBlock") {
              const blockStart = $from.before($from.depth);
              const blockEnd = blockStart + parent.nodeSize;
              decos.push(
                Decoration.node(blockStart, blockEnd, {
                  "data-cursor-in": "",
                }),
              );
            }

            // 1b. Blockquote cursor-in marker. Mirrors the codeBlock
            //     branch: when the caret is inside a blockquote, tag
            //     the wrapping `<blockquote>` so CSS can hide the
            //     "quote" badge (visible by default, like the
            //     codeblock language badge) and surface the `> `
            //     source hint on the active line. The caret can sit
            //     at any depth inside a blockquote (paragraphs,
            //     nested lists), so we walk up from $from to find
            //     the enclosing blockquote node rather than checking
            //     only the direct parent.
            for (let d = $from.depth; d > 0; d--) {
              const node = $from.node(d);
              if (node.type.name === "blockquote") {
                const blockStart = $from.before(d);
                const blockEnd = blockStart + node.nodeSize;
                decos.push(
                  Decoration.node(blockStart, blockEnd, {
                    "data-cursor-in": "",
                  }),
                );
                break;
              }
            }

            // 1. Heading prefix. Wraps the entire heading block so
            //    the CSS `::before` selector lands on the H1..H6
            //    element. data-cursor-prefix carries the hash run.
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

            // 3. Pending-mark markers. While the caret is in a
            //    textblock, scan its plain text for completed
            //    `*…*` / `**…**` / `~~…~~` patterns and decorate
            //    the marker chars so the user sees, at a glance,
            //    that the editor has recognised the pattern and
            //    will apply the corresponding mark once they leave
            //    the block (the block-leave render pass lives in
            //    Wysiwyg.svelte's `syncLiveMarkSource`). Bold and
            //    italic share `*` so we run bold first and exclude
            //    bordering `*` from the italic match.
            const parentForPending = $from.parent;
            if (parentForPending.isTextblock) {
              const blockTextStart = $from.start();
              const blockText = parentForPending.textContent;
              // Heading prefix decoration. A paragraph that starts
              // with `^(#{1,6}) ` is a pending heading; the prefix
              // (the hashes AND the space) is the marker the user
              // typed, so highlight it the same way as `*` / `**`.
              // We ALSO stamp the block with `data-expanded-heading-
              // level` so the CSS can size it like the matching real
              // heading — without that the H1→paragraph swap visibly
              // shrinks the line and shoves the rest of the doc
              // around.
              if (parentForPending.type.name === "paragraph") {
                const hm = /^(#{1,6}) /.exec(blockText);
                if (hm) {
                  decos.push(
                    Decoration.inline(
                      blockTextStart,
                      blockTextStart + hm[0].length,
                      { class: "md-mark-pending" },
                    ),
                  );
                  const blockOuterStart = $from.before($from.depth);
                  const blockOuterEnd =
                    blockOuterStart + parentForPending.nodeSize;
                  decos.push(
                    Decoration.node(blockOuterStart, blockOuterEnd, {
                      "data-expanded-heading-level": String(hm[1].length),
                    }),
                  );
                }
              }
              const pendingPatterns: Array<{
                name: string;
                re: RegExp;
                len: number;
              }> = [
                { name: "bold", re: /\*\*([^*\n]+?)\*\*/g, len: 2 },
                { name: "italic", re: /(?<!\*)\*([^*\n]+?)\*(?!\*)/g, len: 1 },
                { name: "strike", re: /~~([^~\n]+?)~~/g, len: 2 },
                // Wiki-link brackets. Same shape as the mark
                // markers: the two-char `[[` / `]]` runs read as
                // syntax while the bracket-aware render pass in
                // Wysiwyg.svelte is still waiting for the caret to
                // move outside the pattern.
                { name: "wiki", re: /\[\[([^\[\]\n]+?)\]\]/g, len: 2 },
              ];
              for (const p of pendingPatterns) {
                p.re.lastIndex = 0;
                let mm: RegExpExecArray | null;
                while ((mm = p.re.exec(blockText)) !== null) {
                  const matchFrom = blockTextStart + mm.index;
                  const openTo = matchFrom + p.len;
                  const closeFrom = matchFrom + mm[0].length - p.len;
                  const closeTo = matchFrom + mm[0].length;
                  decos.push(
                    Decoration.inline(matchFrom, openTo, {
                      class: "md-mark-pending",
                    }),
                  );
                  decos.push(
                    Decoration.inline(closeFrom, closeTo, {
                      class: "md-mark-pending",
                    }),
                  );
                }
              }
            }

            return decos.length ? DecorationSet.create(doc, decos) : null;
          },
        },
      }),
    ];
  },
});
