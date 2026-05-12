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

export const HeadingNoInputRule = Heading.extend({
  addInputRules() {
    return [];
  },
});

const HEADING_PREFIX = /^(#{1,6}) (.+)$/;

export const HeadingOnEnter = Extension.create({
  name: "headingOnEnter",
  addKeyboardShortcuts() {
    return {
      Enter: ({ editor }) => {
        const sel = editor.state.selection;
        if (!sel.empty) return false;
        const $from = sel.$from;
        const parent = $from.parent;
        // Only fire on a paragraph that hasn't already been promoted.
        // Code blocks, lists, headings, blockquotes etc. all keep their
        // own Enter semantics.
        if (parent.type.name !== "paragraph") return false;
        const text = parent.textContent;
        const m = HEADING_PREFIX.exec(text);
        if (!m) return false;
        // Caret must be at the end of the line: pressing Enter
        // mid-paragraph reads as "split this paragraph", not "render
        // the prefix as a heading". The split path then writes the
        // tail line as a fresh paragraph that the user can keep
        // editing without committing the heading yet.
        const blockStart = $from.start();
        const blockEnd = $from.end();
        if (sel.from !== blockEnd) return false;
        const level = m[1].length;
        const prefixLen = m[1].length + 1;
        const headingType = editor.schema.nodes.heading;
        if (!headingType) return false;
        editor
          .chain()
          .focus()
          // Drop the `#…# ` prefix first so the heading text doesn't
          // double-prefix on round-trip through the markdown
          // serializer.
          .deleteRange({ from: blockStart, to: blockStart + prefixLen })
          // Flip the (now prefix-less) block to a heading at the
          // matching level.
          .setNode(headingType, { level })
          // Drop a fresh paragraph below for the next line; caret
          // lands inside it via splitBlock + setNode.
          .splitBlock()
          .setNode(editor.schema.nodes.paragraph)
          .run();
        return true;
      },
    };
  },
});
