export type CaretRange = { from: number; to: number };

const IMAGE_RE = /!\[[^\]\n]*\]\([^\)\n]*\)/g;

export function renderedCaretForSourceCaret(content: string, caret: CaretRange): CaretRange {
  if (caret.from !== caret.to) return caret;
  const range = imageRangeAt(content, caret.from, "inside");
  return range ? { from: range.from, to: range.from } : caret;
}

export function sourceCaretForRenderedCaret(content: string, caret: CaretRange): CaretRange {
  if (caret.from !== caret.to) return caret;
  const range = imageRangeAt(content, caret.from, "boundary");
  if (!range) return caret;
  const urlStart = content.indexOf("](", range.from);
  if (urlStart < 0 || urlStart + 2 >= range.to) {
    return { from: Math.min(range.from + 1, range.to), to: Math.min(range.from + 1, range.to) };
  }
  const pos = Math.min(urlStart + 2, range.to - 1);
  return { from: pos, to: pos };
}

function imageRangeAt(
  content: string,
  pos: number,
  mode: "inside" | "boundary",
): CaretRange | null {
  IMAGE_RE.lastIndex = 0;
  for (let match = IMAGE_RE.exec(content); match; match = IMAGE_RE.exec(content)) {
    const from = match.index;
    const to = from + match[0].length;
    if (mode === "inside" && pos > from && pos < to) return { from, to };
    if (mode === "boundary" && (pos === from || pos === to)) return { from, to };
  }
  return null;
}
