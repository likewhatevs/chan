// Heading: render on Enter, not on `# ` close.
//
// StarterKit's Heading extension fires an input rule the instant the
// user types `# ` (or `## `, etc.) at the start of a paragraph, which
// flips the block to a heading mid-sentence. The chan UX shifts the
// trigger to the END of the line: as the user types `# Heading title`
// the markers stay in the doc and read as markdown syntax (the
// liveSource decoration highlights them); pressing Enter promotes the
// paragraph to the matching heading level, strips the prefix, and
// drops the caret into a fresh paragraph below.
//
// We swap out StarterKit's bundled heading for one with `addInputRules
// () => []` so the typing rule is gone but the schema entry, attrs,
// keyboard shortcuts (Cmd+Alt+N), and markdown serializer all stay
// the way they were. The Enter handler lives next to it as a tiny
// `Extension` so the heading mark itself doesn't have to know about
// it.

import { Extension } from "@tiptap/core";
import Heading from "@tiptap/extension-heading";
import { TextSelection } from "@tiptap/pm/state";

export const HeadingNoInputRule = Heading.extend({
  addInputRules() {
    return [];
  },
});

const HEADING_PREFIX = /^(#{1,6}) (.+)$/;

export const HeadingOnEnter = Extension.create({
  name: "headingOnEnter",
  // Tiptap default priority is 100. We want this Enter handler to
  // beat PM's baseKeymap splitBlock and StarterKit's bundled
  // text-block keymaps so the heading promotion takes the keystroke
  // when the pattern matches.
  priority: 1000,
  addKeyboardShortcuts() {
    return {
      Enter: ({ editor }) => {
        const sel = editor.state.selection;
        if (!sel.empty) return false;
        const $from = sel.$from;
        const parent = $from.parent;
        if (parent.type.name !== "paragraph") return false;
        const text = parent.textContent;
        const m = HEADING_PREFIX.exec(text);
        if (!m) return false;
        const blockStart = $from.start();
        const blockEnd = $from.end();
        if (sel.from !== blockEnd) return false;
        const level = m[1].length;
        const prefixLen = m[1].length + 1;
        const headingType = editor.schema.nodes.heading;
        const paragraphType = editor.schema.nodes.paragraph;
        if (!headingType || !paragraphType) return false;
        // Raw transaction so the position arithmetic is explicit
        // and we don't depend on the chain to map the selection
        // between commands correctly — the earlier chain-based
        // version was dropping the tail of the heading text (saw
        // `# Foo` reduced to `# F`), likely because setNode +
        // splitBlock in sequence picked up the wrong selection
        // offset after the deleteRange step.
        //
        // Steps in one tr:
        //   1. Strip the `#…# ` prefix from the block content.
        //   2. Flip the (now prefix-less) block to a heading at the
        //      matching level.
        //   3. Split the heading at its end, inserting a paragraph
        //      after it.
        //   4. Move the selection into the new paragraph.
        const tr = editor.state.tr;
        tr.delete(blockStart, blockStart + prefixLen);
        tr.setBlockType(blockStart, blockStart, headingType, { level });
        // After the delete, the heading's end position is
        // blockEnd - prefixLen. tr.split inserts an open + close
        // pair at the split point so the post-split position of
        // the new paragraph's content start is splitAt + 2.
        const splitAt = blockEnd - prefixLen;
        tr.split(splitAt, 1, [{ type: paragraphType }]);
        tr.setSelection(TextSelection.create(tr.doc, splitAt + 2));
        editor.view.dispatch(tr);
        return true;
      },
    };
  },
});
