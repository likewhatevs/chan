// Block-level decoration handlers.
//
// Per design.md spec #4 (block prefixes use line-intersect): the
// blockquote `>` and HR text both reveal/hide based on whether the
// caret line touches them, not whether the selection intersects the
// token range. Heading prefix uses the same rule (handled in
// headings.ts).
//
// What we cover here:
//   - Blockquote: line decoration on every quoted line so CSS can
//     paint a left border + indent. Quote markers stay visible (per
//     Obsidian convention - the `>` IS the visual cue that the line
//     is quoted; hiding it removes meaning).
//   - HorizontalRule: leave source text visible. Many notes use
//     `---` as an authoring separator, and replacing it with a
//     rendered rule makes the markdown harder to edit.
//   - FencedCode: per-line decoration distinguishing opener row,
//     content rows, closer row, plus a mark for the language info
//     (CodeInfo). No hide - the fences stay visible (we want the
//     user to see the block structure as they edit).
//   - Task (GFM task-list item): TaskMarker `[ ]` / `[x]` is replaced
//     by the CheckboxWidget from widgets/checkbox.ts. The replace is
//     boundary-inclusive - clicking the box edits the source.
//   - BulletList: source markers (`-` / `*` / `+`) render as
//     themselves; a `cm-md-ul-marker` mark applies the styling
//     class so CSS can color/space the marker without replacing
//     the source character.
//   - OrderedList: source markers (`1.` / `2)` / etc.) render as
//     themselves; a `cm-md-ol-marker` mark applies the styling
//     class. The rendered editor reflects whatever the author
//     typed, both for portability and so a dash-typed list still
//     reads as a dash on screen.
//   - All three list kinds emit a `cm-md-list-line` line decoration
//     on every line within their range so CSS can add the small
//     left indent that signals "this is a list".

import { Decoration, EditorView, WidgetType } from "@codemirror/view";
import type { TokenContext, TokenHandler } from "./walker";
import { CheckboxWidget } from "../widgets/checkbox";

/// Floating badge anchored at the top-right of a fenced-code block.
/// Shows the language label (from the ```lang opener) and a copy
/// button that lifts the block's body text into the clipboard.
///
/// Widget is rendered ONCE per block at the end of the opener line;
/// the surrounding line has `position: relative` so the badge's
/// `position: absolute` resolves against it. The badge therefore
/// sits at the top-right of the visible block (the opener line is
/// the topmost row of the block in the rendered flow).
class FenceBadgeWidget extends WidgetType {
  constructor(
    readonly lang: string,
    readonly code: string,
  ) {
    super();
  }

  eq(other: FenceBadgeWidget): boolean {
    return this.lang === other.lang && this.code === other.code;
  }

  toDOM(): HTMLElement {
    const wrap = document.createElement("span");
    wrap.className = "cm-md-fence-badge";
    wrap.contentEditable = "false";
    if (this.lang) {
      const lang = document.createElement("span");
      lang.className = "cm-md-fence-badge-lang";
      lang.textContent = this.lang;
      wrap.append(lang);
    }
    const btn = document.createElement("button");
    btn.type = "button";
    btn.className = "cm-md-fence-badge-copy";
    btn.title = "Copy code";
    btn.setAttribute("aria-label", "Copy code");
    btn.append(makeCopyIcon());
    btn.addEventListener("mousedown", (e) => {
      // Prevent CM6 from absorbing the click into its selection
      // handling (which would steal focus + collapse the caret).
      e.preventDefault();
      e.stopPropagation();
    });
    btn.addEventListener("click", (e) => {
      e.preventDefault();
      e.stopPropagation();
      void navigator.clipboard.writeText(this.code).then(
        () => flashCopied(btn),
        () => flashCopied(btn, "fail"),
      );
    });
    wrap.append(btn);
    return wrap;
  }

  ignoreEvent(): boolean {
    return true;
  }
}

const SVG_NS = "http://www.w3.org/2000/svg";
function makeCopyIcon(): SVGElement {
  // Lucide `copy` icon (2 stacked rectangles). Stroke = currentColor
  // so the glyph follows the badge's text colour.
  const svg = document.createElementNS(SVG_NS, "svg");
  svg.setAttribute("viewBox", "0 0 24 24");
  svg.setAttribute("fill", "none");
  svg.setAttribute("stroke", "currentColor");
  svg.setAttribute("stroke-width", "2");
  svg.setAttribute("stroke-linecap", "round");
  svg.setAttribute("stroke-linejoin", "round");
  svg.setAttribute("aria-hidden", "true");
  svg.setAttribute("width", "14");
  svg.setAttribute("height", "14");
  const rect = document.createElementNS(SVG_NS, "rect");
  rect.setAttribute("width", "14");
  rect.setAttribute("height", "14");
  rect.setAttribute("x", "8");
  rect.setAttribute("y", "8");
  rect.setAttribute("rx", "2");
  rect.setAttribute("ry", "2");
  const path = document.createElementNS(SVG_NS, "path");
  path.setAttribute(
    "d",
    "M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2",
  );
  svg.append(rect, path);
  return svg;
}

function flashCopied(btn: HTMLButtonElement, kind: "ok" | "fail" = "ok"): void {
  btn.classList.add(kind === "ok" ? "copied" : "copy-failed");
  setTimeout(() => {
    btn.classList.remove("copied");
    btn.classList.remove("copy-failed");
  }, 900);
}

const LINE_QUOTE = Decoration.line({ attributes: { class: "cm-md-quote" } });
const LINE_FENCE_OPENER = Decoration.line({
  attributes: { class: "cm-md-fence-opener" },
});
const LINE_FENCE_CLOSER = Decoration.line({
  attributes: { class: "cm-md-fence-closer" },
});
const LINE_CODE_BLOCK = Decoration.line({
  attributes: { class: "cm-md-code-block" },
});
const MARK_FENCE_INFO = Decoration.mark({
  attributes: { class: "cm-md-fence-info" },
});
const LINE_FRONTMATTER = Decoration.line({
  attributes: { class: "cm-md-frontmatter" },
});
const HIDE = Decoration.replace({});

const LIST_LINE_DECO = new Map<string, ReturnType<typeof Decoration.line>>();
const MARKDOWN_IMAGE_RE = /(^|[^\\])!\[[^\]\n]*\]\([^)\n]*\)/;

// Soft cap on depth so the inline `--cm-md-list-depth` style can't be
// abused into a pathological repeating-gradient width (e.g. pasted
// content with 200 leading spaces). 20 covers the smoke-test target
// and any sane nesting; deeper indents render at the 20-level guide
// width without further visual change.
const LIST_DEPTH_CAP = 20;

/// Integer depth of a list line, computed from leading whitespace
/// (2 columns = 1 depth level; tabs count as 2 columns). Capped at
/// LIST_DEPTH_CAP so unbounded indentation can't blow up the cache or
/// the rendered guide width.
export function listDepth(text: string): number {
  const leading = text.match(/^[ \t]*/)?.[0] ?? "";
  let columns = 0;
  for (const ch of leading) columns += ch === "\t" ? 2 : 1;
  return Math.min(LIST_DEPTH_CAP, Math.floor(columns / 2));
}

/// Stable class string for a list line at the given indent. Same
/// `cm-md-list-depth-N` shape as before but no longer capped at 6;
/// `N` can range 0..LIST_DEPTH_CAP. The class survives for CSS hooks
/// + grep; the load-bearing depth value rides on the line's inline
/// `--cm-md-list-depth` style.
export function listDepthClass(text: string): string {
  return `cm-md-list-depth-${listDepth(text)}`;
}

export function listLineClass(text: string): string {
  const classes = ["cm-md-list-line", listDepthClass(text)];
  if (MARKDOWN_IMAGE_RE.test(text)) classes.push("cm-md-list-line-image");
  return classes.join(" ");
}

function listLineDecoration(text: string): ReturnType<typeof Decoration.line> {
  const className = listLineClass(text);
  const cached = LIST_LINE_DECO.get(className);
  if (cached) return cached;
  // Inline style carries the depth into CSS so the guide and prefix
  // rules can render N stripes / 2N+2ch padding without per-depth
  // selectors. Cache by class string; same string ⇒ same depth.
  const depth = listDepth(text);
  const deco = Decoration.line({
    attributes: {
      class: className,
      style: `--cm-md-list-depth: ${depth};`,
    },
  });
  LIST_LINE_DECO.set(className, deco);
  return deco;
}

const handleBlockquote: TokenHandler = (ctx) => {
  // Walk every line within the blockquote's range and emit the line
  // decoration. Blockquote can wrap multiple paragraphs; the line
  // decoration covers the visible run.
  const startLine = ctx.state.doc.lineAt(ctx.node.from).number;
  const endLine = ctx.state.doc.lineAt(
    Math.min(ctx.node.to, ctx.state.doc.length),
  ).number;
  for (let n = startLine; n <= endLine; n++) {
    const line = ctx.state.doc.line(n);
    ctx.push(LINE_QUOTE, line.from, line.from);
  }
};

const handleHorizontalRule: TokenHandler = (ctx) => {
  void ctx;
};

const handleFencedCode: TokenHandler = (ctx) => {
  const cursor = ctx.node.node.cursor();
  if (!cursor.firstChild()) return;
  // Find opener CodeMark (first CodeMark child), closer CodeMark
  // (last CodeMark child), and CodeInfo (optional, immediately
  // after the opener).
  let openMarkFrom = -1;
  let openMarkTo = -1;
  let closeMarkFrom = -1;
  let closeMarkTo = -1;
  let infoFrom = -1;
  let infoTo = -1;
  do {
    if (cursor.name === "CodeMark") {
      if (openMarkFrom === -1) {
        openMarkFrom = cursor.from;
        openMarkTo = cursor.to;
      }
      closeMarkFrom = cursor.from;
      closeMarkTo = cursor.to;
    } else if (cursor.name === "CodeInfo") {
      infoFrom = cursor.from;
      infoTo = cursor.to;
    }
  } while (cursor.nextSibling());
  if (openMarkFrom === -1) return;

  const openLineObj = ctx.state.doc.lineAt(openMarkFrom);
  const closeLineObj = ctx.state.doc.lineAt(closeMarkFrom);
  const openLine = openLineObj.number;
  const closeLine = closeLineObj.number;
  const blockEndLine = ctx.state.doc.lineAt(
    Math.min(ctx.node.to, ctx.state.doc.length),
  ).number;
  // The closer fence may sit OUTSIDE the FencedCode's syntactic
  // range when the parser is recovering from a missing fence; clamp
  // to be safe.
  const lastLine = Math.max(closeLine, blockEndLine);
  // For an unclosed fence @lezer/markdown emits a single CodeMark
  // (the opener) and stretches the FencedCode node to doc end.
  // We detect that here so we can (a) keep the opener visible
  // while the caret sits anywhere in the block, even on body lines
  // BELOW the opener - without this, the markers hide and the user
  // has no signal they're typing inside an unclosed code block;
  // (b) drop a ghost ` ``` ` placeholder on the line past the last
  // body line so the missing closer is visually obvious.
  const isUnclosed = closeMarkFrom === openMarkFrom;

  // Show the fence verbatim (backticks + lang inline) whenever the
  // caret is ANYWHERE inside the block - not just on the opener /
  // closer rows. For closed fences the opener-to-closer line span
  // covers everything; for unclosed fences the closer line equals
  // the opener line, so we must use the FencedCode node's full
  // extent instead, otherwise the opener marker hides as soon as
  // the caret leaves line 1 and the user has no visual signal
  // they're still inside a code block.
  const blockEnd = Math.max(closeLineObj.to, ctx.node.to);
  const caretInBlock = ctx.lineIntersect(openLineObj.from, blockEnd);
  if (!caretInBlock) {
    if (openMarkFrom < openMarkTo) {
      ctx.push(HIDE, openMarkFrom, openMarkTo);
    }
    // The lang text rides into the floating badge; hide its inline
    // copy too so the opener row reads as an empty bar with the
    // badge floating at the right edge.
    if (infoFrom !== -1 && infoTo !== -1 && infoFrom < infoTo) {
      ctx.push(HIDE, infoFrom, infoTo);
    }
    if (
      closeMarkFrom !== openMarkFrom &&
      closeMarkFrom < closeMarkTo
    ) {
      ctx.push(HIDE, closeMarkFrom, closeMarkTo);
    }
  } else if (infoFrom !== -1 && infoTo !== -1 && infoFrom < infoTo) {
    // Caret IS in the block: keep the lang readable inline with
    // the link-style underline so the user can edit it.
    ctx.push(MARK_FENCE_INFO, infoFrom, infoTo);
  }

  // Compute the inner block text once for the copy button.
  // Range: from start of first content line to end of last content
  // line. If the block has no content lines (opener+closer adjacent
  // or unclosed), the slice degenerates to "".
  let codeText = "";
  if (closeLine > openLine + 1) {
    const firstContent = ctx.state.doc.line(openLine + 1);
    const lastContent = ctx.state.doc.line(closeLine - 1);
    codeText = ctx.state.doc.sliceString(firstContent.from, lastContent.to);
  } else if (closeLine === openLine && lastLine > openLine) {
    // Unclosed block: everything after the opener line up to lastLine.
    const firstContent = ctx.state.doc.line(openLine + 1);
    const lastContent = ctx.state.doc.line(lastLine);
    codeText = ctx.state.doc.sliceString(firstContent.from, lastContent.to);
  }
  const langText = infoFrom !== -1 ? ctx.state.doc.sliceString(infoFrom, infoTo) : "";

  for (let n = openLine; n <= lastLine; n++) {
    const line = ctx.state.doc.line(n);
    let deco;
    if (n === openLine) deco = LINE_FENCE_OPENER;
    else if (n === closeLine && closeLine !== openLine) deco = LINE_FENCE_CLOSER;
    else deco = LINE_CODE_BLOCK;
    ctx.push(deco, line.from, line.from);
  }

  // Floating badge anchored on the opener line. Placed at the END
  // of the line so an arrow-right that lands at line.to still sees
  // the user's text content (the badge is past line.to). Position:
  // absolute inside the .cm-md-fence-opener row pins it visually
  // to the top-right of the block.
  const badge = Decoration.widget({
    widget: new FenceBadgeWidget(langText, codeText),
    side: 1,
  });
  ctx.push(badge, openLineObj.to, openLineObj.to);

  // Ghost closer for unclosed fences. Drops a dimmed ` ``` ` widget
  // at the end of the last body line so the user can see the
  // block isn't closed. Without it, an unclosed fence reads as
  // "regular text inside a slightly-shaded slab" and traps the
  // caret silently - typed content keeps extending the fence with
  // no visible cue.
  if (isUnclosed) {
    const lastLineObj = ctx.state.doc.line(lastLine);
    const ghost = Decoration.widget({
      widget: new GhostCloserWidget(),
      side: 1,
    });
    ctx.push(ghost, lastLineObj.to, lastLineObj.to);
  }
};

/// Dimmed inline placeholder for an unclosed code fence. Renders
/// as a faint `\`\`\`` chip at the end of the fence's last body
/// line so the user can see the block lacks a closer. Click
/// inserts a real closer on a fresh line below.
class GhostCloserWidget extends WidgetType {
  eq(_other: GhostCloserWidget): boolean {
    return true;
  }

  toDOM(view: EditorView): HTMLElement {
    const wrap = document.createElement("span");
    wrap.className = "cm-md-fence-ghost-closer";
    wrap.contentEditable = "false";
    wrap.title = "Unclosed code block - click to add the closing ```";
    wrap.textContent = "```";
    wrap.addEventListener("mousedown", (e) => {
      e.preventDefault();
      e.stopPropagation();
      const pos = view.posAtDOM(wrap);
      const line = view.state.doc.lineAt(pos);
      const insert = line.text.length === 0 ? "```\n" : "\n```\n";
      const at = line.to;
      view.dispatch({
        changes: { from: at, to: at, insert },
        selection: { anchor: at + insert.length },
      });
      view.focus();
    });
    return wrap;
  }

  ignoreEvent(): boolean {
    // Let our explicit mousedown above run; CM6's default handling
    // would otherwise eat the click into selection placement.
    return false;
  }
}

const handleTask: TokenHandler = (ctx) => {
  // Walk children to find the TaskMarker (lezer-markdown's GFM
  // TaskList emits a Task block-level node with a TaskMarker child
  // covering exactly `[ ]` / `[x]` / `[X]` - 3 chars).
  const line = ctx.state.doc.lineAt(ctx.node.from);
  ctx.push(listLineDecoration(line.text), line.from, line.from);
  const cursor = ctx.node.node.cursor();
  if (!cursor.firstChild()) return;
  do {
    if (cursor.name === "TaskMarker") {
      const text = ctx.state.doc.sliceString(cursor.from, cursor.to);
      if (text.length !== 3) return;
      const checked = text === "[x]" || text === "[X]";
      const widget = Decoration.replace({
        widget: new CheckboxWidget(checked),
      });
      ctx.push(widget, cursor.from, cursor.to);
      return;
    }
  } while (cursor.nextSibling());
};

const handleBulletList: TokenHandler = (ctx) => {
  // Per-line indent decoration. Walking the BulletList's full line
  // range also covers nested lists; CM6 dedupes identical line
  // attributes, so the duplicate is harmless.
  const startLine = ctx.state.doc.lineAt(ctx.node.from).number;
  const endLine = ctx.state.doc.lineAt(
    Math.min(ctx.node.to, ctx.state.doc.length),
  ).number;
  for (let n = startLine; n <= endLine; n++) {
    const line = ctx.state.doc.line(n);
    ctx.push(listLineDecoration(line.text), line.from, line.from);
  }
  decorateBulletList(ctx, ctx.node.node);
};

/// Depth-cycling glyph decorations for `*` / `+` lists. The class chosen
/// here only ADDS a styling hook; the source bytes are untouched (so
/// source mode + round-trip still show the literal `*` / `+`). The
/// wysiwyg glyph is pure CSS (Wysiwyg.svelte) and keys off NESTING
/// DEPTH, not the typed marker char, matching Google Docs: level 1 =
/// filled disc, level 2 = open circle, level 3 = filled square, then the
/// cycle repeats (depth % 3).
///
/// Hyphen (`-`) lists are deliberately EXCLUDED from this cycle (see
/// HYPHEN_MARK below): @@Alex's phase-17 google-docs request was for the
/// `*` bullet style only, so hyphen lists keep their literal dash and
/// stay visually distinct.
const BULLET_GLYPHS = [
  Decoration.mark({ class: "cm-md-ul-marker cm-md-ul-bullet cm-md-ul-disc" }),
  Decoration.mark({ class: "cm-md-ul-marker cm-md-ul-bullet cm-md-ul-circle" }),
  Decoration.mark({ class: "cm-md-ul-marker cm-md-ul-bullet cm-md-ul-square" }),
];

/// Hyphen (`-`) lists render their literal dash, NOT a depth glyph. The
/// `cm-md-ul-hyphen` class styles the marker (color) without the
/// `cm-md-ul-bullet` font-size:0 + ::before substitution, so the source
/// `-` shows through at every nesting level. As a side benefit the
/// marker stays real visible text (like an ordered `1.` marker), so a
/// vertical caret move lands past it onto the text instead of on a
/// zero-width glyph - hyphen lists get ordered-list cursor parity for
/// free.
const HYPHEN_MARK = Decoration.mark({
  class: "cm-md-ul-marker cm-md-ul-hyphen",
});

/// Nesting depth of this ListItem: the count of ancestor ListItems
/// above it (0 = top-level). Drives the Google-Docs glyph cycle
/// (depth % 3). Walks the syntax ancestry rather than the indent so it
/// tracks the actual list structure (a `*` one space deeper is still
/// top-level until it nests under a parent item).
function bulletDepth(item: import("@lezer/common").SyntaxNode): number {
  let depth = 0;
  let cur = item.parent;
  while (cur) {
    if (cur.name === "ListItem") depth++;
    cur = cur.parent;
  }
  return depth;
}

/// Decoration for a bullet ListItem's marker. Hyphen (`-`) markers stay
/// a literal dash (HYPHEN_MARK); `*` / `+` markers map their nesting
/// depth to a Google-Docs glyph (disc / circle / square, depth % 3).
function bulletMarkerDecoration(markerChar: string, depth: number): Decoration {
  if (markerChar === "-") return HYPHEN_MARK;
  return BULLET_GLYPHS[depth % BULLET_GLYPHS.length];
}

function decorateBulletList(
  ctx: TokenContext,
  list: import("@lezer/common").SyntaxNode,
): void {
  const cursor = list.cursor();
  if (!cursor.firstChild()) return;
  do {
    if (cursor.name !== "ListItem") continue;
    const item = cursor.node;
    const sub = item.cursor();
    let markFrom = -1;
    let markTo = -1;
    let hasTask = false;
    if (sub.firstChild()) {
      do {
        if (sub.name === "ListMark") {
          markFrom = sub.from;
          markTo = sub.to;
        } else if (sub.name === "Task") {
          hasTask = true;
        }
      } while (sub.nextSibling());
    }
    if (!hasTask && markFrom !== -1 && markTo !== -1) {
      const markerChar = ctx.state.doc.sliceString(markFrom, markTo);
      ctx.push(
        bulletMarkerDecoration(markerChar, bulletDepth(item)),
        markFrom,
        markTo,
      );
    }
  } while (cursor.nextSibling());
}

/// `cm-md-ol-marker` styles the ordered-list marker (`1.` / `2)` /
/// etc.) in the source. We attach the class via `Decoration.mark`
/// instead of replacing the source text with a generated label so
/// the rendered editor reflects whatever the author typed. The
/// outline-style dotted chain (`1.1.1.`) was removed: it diverged
/// from the source bytes and surprised users who expected the
/// rendered numbers to match what they typed.
const ORDERED_MARK = Decoration.mark({ class: "cm-md-ol-marker" });

/// Walk an OrderedList's direct ListItem children, mark each
/// ListMark with the styling class, and recurse into nested
/// OrderedLists so deeper levels pick up the same class. The
/// recursion is local so an outer OL handler still drives the
/// inner OLs without depending on the per-node handler firing
/// again for nested OLs.
function decorateOrderedList(
  ctx: TokenContext,
  ol: import("@lezer/common").SyntaxNode,
): void {
  const cursor = ol.cursor();
  if (!cursor.firstChild()) return;
  do {
    if (cursor.name !== "ListItem") continue;
    const item = cursor.node;
    const sub = item.cursor();
    if (sub.firstChild()) {
      do {
        if (sub.name === "ListMark") {
          ctx.push(ORDERED_MARK, sub.from, sub.to);
          break;
        }
      } while (sub.nextSibling());
    }
    const childCursor = item.cursor();
    if (childCursor.firstChild()) {
      do {
        if (childCursor.name === "OrderedList") {
          decorateOrderedList(ctx, childCursor.node);
        }
      } while (childCursor.nextSibling());
    }
  } while (cursor.nextSibling());
}

function ancestorOrderedList(
  node: import("@lezer/common").SyntaxNode,
): boolean {
  let cur = node.parent;
  while (cur) {
    if (cur.name === "OrderedList") return true;
    cur = cur.parent;
  }
  return false;
}

const handleOrderedList: TokenHandler = (ctx) => {
  const startLine = ctx.state.doc.lineAt(ctx.node.from).number;
  const endLine = ctx.state.doc.lineAt(
    Math.min(ctx.node.to, ctx.state.doc.length),
  ).number;
  for (let n = startLine; n <= endLine; n++) {
    const line = ctx.state.doc.line(n);
    ctx.push(listLineDecoration(line.text), line.from, line.from);
  }
  // Skip the inner walk when this OrderedList is nested inside
  // another OrderedList - the outer pass already drove this subtree.
  if (ancestorOrderedList(ctx.node.node)) return;
  decorateOrderedList(ctx, ctx.node.node);
};

const handleFrontmatter: TokenHandler = (ctx) => {
  // Dim every line in the frontmatter range with a line decoration.
  // The Frontmatter node from markdown/frontmatter.ts covers the
  // opening `---`, the YAML body, and the closing `---`.
  const startLine = ctx.state.doc.lineAt(ctx.node.from).number;
  const endLine = ctx.state.doc.lineAt(
    Math.min(ctx.node.to, ctx.state.doc.length),
  ).number;
  for (let n = startLine; n <= endLine; n++) {
    const line = ctx.state.doc.line(n);
    ctx.push(LINE_FRONTMATTER, line.from, line.from);
  }
};

export const blockHandlers = {
  Blockquote: handleBlockquote,
  HorizontalRule: handleHorizontalRule,
  FencedCode: handleFencedCode,
  BulletList: handleBulletList,
  OrderedList: handleOrderedList,
  Task: handleTask,
  Frontmatter: handleFrontmatter,
};
