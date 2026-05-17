// Lightweight markdown -> sanitized HTML helper for read-only
// previews that need headers / lists / code / links / paragraphs.
//
// Output is sanitized via DOMPurify before any {@html ...} insert
// so every {@html ...} insertion goes through the same chokepoint.

import DOMPurify from "dompurify";
import { marked } from "marked";

// Defaults are fine for previews: GFM (tables, strikethrough),
// auto-linking off (we don't want to silently rewrite text), no
// breaks (preserve authored paragraph boundaries).
marked.setOptions({
  gfm: true,
  breaks: false,
});

/// Render a markdown string to a sanitized HTML string suitable
/// for `{@html ...}`. Synchronous (we use marked's sync `parse`
/// and DOMPurify on the result) so callers can use it inline in
/// templates without `await`.
export function renderMarkdown(input: string): string {
  // marked.parse can return Promise<string> if any extension is
  // async; we use no async extensions, so cast through unknown.
  const raw = marked.parse(input ?? "", { async: false }) as string;
  return DOMPurify.sanitize(raw);
}
