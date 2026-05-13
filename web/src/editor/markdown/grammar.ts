// Markdown grammar setup for the chan editor.
//
// Wires:
//   - @codemirror/lang-markdown (CommonMark base)
//   - GFM (strikethrough, tables, task lists, autolinks)
//   - Frontmatter (custom, sees `---...---` at doc start)
//   - WikiLink (custom, `[[note|alias]]` syntax)
//
// Returned as a LanguageSupport that callers drop into their CM6
// extension array (same shape as the built-in `markdown()` returns).
//
// Note: lezer-markdown's `addKeymap` and `codeLanguages` config flags
// are intentionally NOT exposed here. The editor wires its own keymap
// (see commands/format.ts when step 10 lands) and code-language
// highlighting inside fenced blocks is v2 work per design.md.

import { markdown } from "@codemirror/lang-markdown";
import { GFM, type MarkdownExtension } from "@lezer/markdown";
import type { LanguageSupport } from "@codemirror/language";
import { WikiLink } from "./wikilink";
import { Frontmatter } from "./frontmatter";

const CHAN_EXTENSIONS: MarkdownExtension = [Frontmatter, ...GFM, WikiLink];

export function chanMarkdown(): LanguageSupport {
  // `addKeymap: false` so we don't get lang-markdown's built-in keymap
  // (it owns Enter, which conflicts with our list / task / heading
  // behavior that we'll add via commands/format.ts).
  return markdown({
    extensions: CHAN_EXTENSIONS,
    addKeymap: false,
  });
}
