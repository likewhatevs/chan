// Lightweight markdown -> sanitized HTML helper for assistant chat
// bubbles. Unlike the editor (TipTap-driven, round-tripped via
// tiptap-markdown), the chat panel just needs read-only rendering
// of headers / lists / code / links / paragraphs.
//
// Output is sanitized via DOMPurify before any {@html ...} insert
// even though the assistant is our own backend: the model can be
// influenced by tool results and we want a single chokepoint that
// stays safe regardless of upstream behavior changes.

import DOMPurify from "dompurify";
import { marked } from "marked";

// Defaults are fine for chat output: GFM (tables, strikethrough),
// auto-linking off (we don't want to silently rewrite text), no
// breaks (let the model decide paragraph boundaries).
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
