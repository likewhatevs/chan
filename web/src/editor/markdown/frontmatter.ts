// Lezer-markdown extension for YAML frontmatter at doc start.
//
// CommonMark + GFM does not parse frontmatter; without isolation, a `# `
// inside the YAML body would be promoted to a heading and `---` separators
// would render as horizontal rules. We define two block node types:
//
//   Frontmatter        the whole `---\n…\n---\n` block at doc start
//   FrontmatterMark    the opening `---` and closing `---` lines
//
// Style: muted (`tags.meta`) so theme rules can dim it. The body is left
// unstyled — v1 doesn't ship YAML highlighting.
//
// Constraints:
//   - Block must start at document position 0 (no leading blank lines).
//   - The opening line must be exactly `---` (CommonMark allows trailing
//     whitespace on HRs, but for frontmatter we want a strict `^---$` to
//     avoid eating real horizontal rules at line 1).
//   - The closing fence is the next line that is exactly `---` (or `...`,
//     a YAML stream-end marker; we accept both for parity with Pandoc).
//   - If we never find a closing fence before EOF, we DO NOT emit a
//     Frontmatter node — falling back to the regular HR parser is more
//     graceful than dimming the entire document.

import type { MarkdownConfig } from "@lezer/markdown";
import { tags } from "@lezer/highlight";

export const Frontmatter: MarkdownConfig = {
  defineNodes: [
    {
      name: "Frontmatter",
      block: true,
      style: tags.meta,
    },
    {
      name: "FrontmatterMark",
      style: tags.processingInstruction,
    },
  ],
  parseBlock: [
    {
      name: "Frontmatter",
      parse(cx, line) {
        if (cx.lineStart !== 0) return false;
        if (line.text !== "---") return false;
        // The `line` parameter is the same Line object that gets
        // re-populated by cx.nextLine(), matching the pattern used by
        // lezer-markdown's own FencedCode / IndentedCode parsers.
        const openStart = cx.lineStart;
        const openEnd = openStart + line.text.length;
        const MAX_LINES = 10_000;
        let consumed = 0;
        // Move past the opener row.
        if (!cx.nextLine()) return false;
        while (consumed < MAX_LINES) {
          const text = line.text;
          if (text === "---" || text === "...") {
            const closeStart = cx.lineStart;
            const closeEnd = closeStart + text.length;
            cx.nextLine();
            cx.addElement(
              cx.elt("Frontmatter", openStart, closeEnd, [
                cx.elt("FrontmatterMark", openStart, openEnd),
                cx.elt("FrontmatterMark", closeStart, closeEnd),
              ]),
            );
            return true;
          }
          if (!cx.nextLine()) {
            // Reached EOF without a closing fence. Not a frontmatter
            // block — return false so the opener line gets re-parsed
            // as a horizontal rule by the standard parsers.
            return false;
          }
          consumed++;
        }
        return false;
      },
      // Must run before the HR parser; otherwise the opening `---`
      // line gets eaten as a horizontal rule and we never see it.
      before: "HorizontalRule",
    },
  ],
};
