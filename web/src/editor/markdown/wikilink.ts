// Lezer-markdown extension for `[[wikilink]]` syntax.
//
// Lezer-markdown does not ship a wikilink parser; the GFM bundle covers
// strikethrough/tables/tasklists/autolinks but not Obsidian-style internal
// links. This extension adds three node types:
//
//   WikiLink         the whole `[[...]]` range
//   WikiLinkMark     the `[[` and `]]` runs
//   WikiLinkBody     the inner content (target, optional `#anchor`/`^block`,
//                    optional `|alias`)
//
// We use an eager parse (not the delimiter-pair machinery) because `[[`/`]]`
// don't nest meaningfully and the syntax is line-bound. The InlineParser
// fires on `[`; if the next char is also `[` and we find a `]]` within the
// same inline section, we emit the structured nodes and return the
// post-`]]` position. Otherwise we return -1 so the regular `[link]` /
// `[ref]` parsers get their chance.
//
// Empty bodies (`[[]]`) and bodies containing `[[` are rejected - the
// former would create zero-width pills with no semantic value, the latter
// would conflict with the regular link parser's stack.

import type { MarkdownConfig } from "@lezer/markdown";
import { tags } from "@lezer/highlight";

const OPEN_BRACKET = 91; // '['
const CLOSE_BRACKET = 93; // ']'

export const WikiLink: MarkdownConfig = {
  defineNodes: [
    {
      name: "WikiLink",
      style: tags.link,
    },
    {
      name: "WikiLinkMark",
      style: tags.processingInstruction,
    },
    {
      name: "WikiLinkBody",
      style: tags.url,
    },
  ],
  parseInline: [
    {
      name: "WikiLink",
      parse(cx, next, pos) {
        if (next !== OPEN_BRACKET || cx.char(pos + 1) !== OPEN_BRACKET) {
          return -1;
        }
        // Scan for `]]` within the inline section. cx.end is the section
        // terminator (typically end-of-paragraph or end-of-block); we
        // stop earlier on `\n` because wikilinks are line-bound.
        const bodyStart = pos + 2;
        const max = cx.end;
        let scan = bodyStart;
        while (scan < max - 1) {
          const ch = cx.char(scan);
          if (ch === 10 /* \n */) return -1;
          if (
            ch === OPEN_BRACKET &&
            cx.char(scan + 1) === OPEN_BRACKET
          ) {
            // Nested `[[` inside the body - bail and let regular parsers
            // try (none of them will match either, but the body would
            // confuse downstream consumers).
            return -1;
          }
          if (ch === CLOSE_BRACKET && cx.char(scan + 1) === CLOSE_BRACKET) {
            const bodyEnd = scan;
            const closeEnd = scan + 2;
            if (bodyEnd <= bodyStart) return -1; // empty body
            return cx.addElement(
              cx.elt("WikiLink", pos, closeEnd, [
                cx.elt("WikiLinkMark", pos, pos + 2),
                cx.elt("WikiLinkBody", bodyStart, bodyEnd),
                cx.elt("WikiLinkMark", bodyEnd, closeEnd),
              ]),
            );
          }
          scan++;
        }
        return -1;
      },
      // Run before the regular link parser so `[[...]]` doesn't get
      // interpreted as `[` + `[...]` + `]`. The lezer-markdown link
      // parser is named "Link"; "before: 'Link'" gives us higher
      // precedence on shared trigger characters.
      before: "Link",
    },
  ],
};
