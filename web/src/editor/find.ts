// Pure find-on-page matcher shared by both editor adapters.
//
// scanMatches walks `text` for occurrences of `query` and returns
// every hit as a {from,to} half-open range in string-offset space.
// The Wysiwyg adapter feeds it one ProseMirror text node at a time
// and offsets the result by the node position; the Source adapter
// feeds it the whole doc since CodeMirror positions ARE string
// offsets.

export type FindRange = { from: number; to: number };

export type FindOptions = {
  caseSensitive: boolean;
};

/// Upper bound on per-document matches. A 1-char query on a big
/// doc could otherwise allocate millions of decoration entries and
/// stall the editor; capping keeps memory + scan cost bounded.
/// The FindBar surfaces "10000+" when this ceiling fires so the
/// user knows the count is truncated.
export const MAX_FIND_MATCHES = 10_000;

export function scanMatches(
  text: string,
  query: string,
  opts: FindOptions,
): FindRange[] {
  if (!query) return [];
  const out: FindRange[] = [];
  const hay = opts.caseSensitive ? text : text.toLowerCase();
  const needle = opts.caseSensitive ? query : query.toLowerCase();
  const n = needle.length;
  if (n === 0 || hay.length < n) return out;
  let i = 0;
  while (i <= hay.length - n) {
    const j = hay.indexOf(needle, i);
    if (j < 0) break;
    out.push({ from: j, to: j + n });
    if (out.length >= MAX_FIND_MATCHES) break;
    i = j + n;
  }
  return out;
}

/// Imperative surface each editor exposes to FindBar.svelte.
/// `scan` is pure (no side effects); the other three drive the
/// view's decoration / scroll state.
export interface FindAdapter {
  scan(query: string, opts: FindOptions): FindRange[];
  highlightAll(matches: FindRange[], currentIndex: number): void;
  clearHighlights(): void;
  scrollIntoView(currentIndex: number): void;
}
