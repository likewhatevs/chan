// Aggregator for all per-token handler registries.
//
// Each step contributes a registry from its module:
//   - step 4: marks (Emphasis, StrongEmphasis, Strikethrough, InlineCode,
//     Link, URL, Autolink) + headings (ATXHeading1..6)
//   - step 5: blocks (lists, task lists, blockquote, hr, fenced code)
//   - step 6: atoms (wikilink, image, date, tag pill, contact pill)
//
// chanDecorations() returns the composed decoration ViewPlugin
// extension; drop into the editor's extension array.

import type { Extension } from "@codemirror/state";
import { decorationWalker, type HandlerRegistry } from "./walker";
import { inlineMarkHandlers } from "./marks";
import { headingHandlers } from "./headings";

const ALL_HANDLERS: HandlerRegistry = {
  ...inlineMarkHandlers,
  ...headingHandlers,
};

export function chanDecorations(): Extension {
  return decorationWalker(ALL_HANDLERS);
}
