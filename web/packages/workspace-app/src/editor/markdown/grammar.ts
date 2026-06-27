// Markdown grammar setup for the chan editor.
//
// Wires:
//   - @codemirror/lang-markdown (CommonMark base)
//   - GFM (strikethrough, tables, task lists, autolinks)
//   - Frontmatter (custom, sees `---...---` at doc start)
//   - WikiLink (custom, `[[note|alias]]` syntax)
//   - RefAwareLink (custom, lets `[[foo] bar](path)` form the outer link)
//
// Returned as a LanguageSupport that callers drop into their CM6
// extension array (same shape as the built-in `markdown()` returns).
//
// `addKeymap: false` because the editor owns its own keymap (Enter
// handling, format commands). `codeLanguages` wires per-language
// LanguageDescription entries with lazy-loaded packs; vite emits one
// chunk per language and the main bundle stays small.

import { markdown } from "@codemirror/lang-markdown";
import { GFM, type MarkdownExtension } from "@lezer/markdown";
import type { LanguageSupport } from "@codemirror/language";
import { WikiLink } from "./wikilink";
import { Frontmatter } from "./frontmatter";
import { RefAwareLink } from "./refAwareLink";
import { codeLanguages } from "./code_languages";

const CHAN_EXTENSIONS: MarkdownExtension = [
  Frontmatter,
  ...GFM,
  WikiLink,
  RefAwareLink,
];

export function chanMarkdown(): LanguageSupport {
  return markdown({
    extensions: CHAN_EXTENSIONS,
    addKeymap: false,
    codeLanguages,
  });
}
