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
//     Obsidian convention — the `>` IS the visual cue that the line
//     is quoted; hiding it removes meaning).
//   - HorizontalRule: line decoration `cm-md-hr` for the rule
//     styling; replace-decoration over the source text (`---` etc.)
//     when caret line doesn't intersect, so the rule looks like an
//     actual horizontal line.
//   - FencedCode: per-line decoration distinguishing opener row,
//     content rows, closer row, plus a mark for the language info
//     (CodeInfo). No hide — the fences stay visible (we want the
//     user to see the block structure as they edit).
//   - Task (GFM task-list item): TaskMarker `[ ]` / `[x]` is replaced
//     by the CheckboxWidget from widgets/checkbox.ts. The replace is
//     boundary-inclusive — clicking the box edits the source.
//   - BulletList: each item's ListMark (`-` / `*` / `+`) is replaced
//     by a `•` widget. Items that carry a TaskMarker keep their
//     source marker so the checkbox handler stays in charge of the
//     visual.
//   - OrderedList: no marker replacement (numbers are content the
//     user types); only the per-line indent decoration applies.
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
const LINE_HR = Decoration.line({ attributes: { class: "cm-md-hr" } });
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

const LIST_LINE_DECO = new Map<number, ReturnType<typeof Decoration.line>>();

export function listDepthClass(text: string): string {
  const leading = text.match(/^[ \t]*/)?.[0] ?? "";
  let columns = 0;
  for (const ch of leading) columns += ch === "\t" ? 2 : 1;
  return `cm-md-list-depth-${Math.min(6, Math.floor(columns / 2))}`;
}

function listLineDecoration(text: string): ReturnType<typeof Decoration.line> {
  const depth = Number(listDepthClass(text).slice("cm-md-list-depth-".length));
  const cached = LIST_LINE_DECO.get(depth);
  if (cached) return cached;
  const deco = Decoration.line({
    attributes: { class: `cm-md-list-line cm-md-list-depth-${depth}` },
  });
  LIST_LINE_DECO.set(depth, deco);
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
  const line = ctx.state.doc.lineAt(ctx.node.from);
  // While the caret sits on this line, skip the rule entirely:
  // the `cm-md-hr` class paints a transparent-foreground horizontal
  // rule (border-bottom + color: transparent), which means the
  // `---` source becomes both unreadable and visually fused with
  // the rendered rule the moment the user tries to edit it. Showing
  // raw source while editing matches the other block widgets'
  // "click-to-edit reveals the literal markdown" pattern.
  if (ctx.lineIntersect(ctx.node.from, ctx.node.to)) return;
  ctx.push(LINE_HR, line.from, line.from);
  if (line.from < line.to) ctx.push(HIDE, line.from, line.to);
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
  // BELOW the opener — without this, the markers hide and the user
  // has no signal they're typing inside an unclosed code block;
  // (b) drop a ghost ` ``` ` placeholder on the line past the last
  // body line so the missing closer is visually obvious.
  const isUnclosed = closeMarkFrom === openMarkFrom;

  // Show the fence verbatim (backticks + lang inline) whenever the
  // caret is ANYWHERE inside the block — not just on the opener /
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
  // caret silently — typed content keeps extending the fence with
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
    wrap.title = "Unclosed code block — click to add the closing ```";
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
  // covering exactly `[ ]` / `[x]` / `[X]` — 3 chars).
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

/// Replaces a bullet-list ListMark (`-` / `*` / `+`) with a centered
/// `•` glyph. Pure visual: the underlying source character stays
/// unchanged, so toolbar toggles + Enter/Tab list editing see the
/// raw markdown they expect.
class BulletWidget extends WidgetType {
  eq(_other: BulletWidget): boolean {
    return true;
  }

  toDOM(): HTMLElement {
    const el = document.createElement("span");
    el.className = "cm-md-bullet";
    el.textContent = "•";
    return el;
  }

  ignoreEvent(): boolean {
    return true;
  }
}

const BULLET_DECO = Decoration.replace({ widget: new BulletWidget() });

/// True when the ListItem's direct children include a TaskMarker —
/// i.e. this item is actually a task. The GFM extension emits the
/// TaskMarker as a sibling of ListMark inside the same ListItem,
/// so a single shallow walk is enough; we don't descend into
/// Paragraph etc.
function itemHasTaskMarker(item: import("@lezer/common").SyntaxNode): boolean {
  const cursor = item.cursor();
  if (!cursor.firstChild()) return false;
  do {
    if (cursor.name === "TaskMarker") return true;
  } while (cursor.nextSibling());
  return false;
}

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
  // Replace `*` markers with a • glyph so the asterisk reads as a
  // proper bullet; leave `-` and `+` literal so the rendered text
  // still tells the user exactly what's in the source. Skip task
  // items either way: the checkbox handler owns their visual.
  const cursor = ctx.node.node.cursor();
  if (!cursor.firstChild()) return;
  do {
    if (cursor.name !== "ListItem") continue;
    const item = cursor.node;
    if (itemHasTaskMarker(item)) continue;
    const sub = item.cursor();
    if (!sub.firstChild()) continue;
    do {
      if (sub.name === "ListMark") {
        const mark = ctx.state.doc.sliceString(sub.from, sub.to);
        // Require a trailing whitespace after the marker. lezer-markdown
        // tentatively flags a lone `*` on an empty line as a ListMark
        // while the user is still typing; without this gate the
        // asterisk flips to • before the user even hits space, which
        // reads as the editor "swallowing" the keystroke.
        const next = sub.to < ctx.state.doc.length
          ? ctx.state.doc.sliceString(sub.to, sub.to + 1)
          : "";
        if (mark === "*" && (next === " " || next === "\t")) {
          ctx.push(BULLET_DECO, sub.from, sub.to);
        }
        break;
      }
    } while (sub.nextSibling());
  } while (cursor.nextSibling());
};

const handleOrderedList: TokenHandler = (ctx) => {
  const startLine = ctx.state.doc.lineAt(ctx.node.from).number;
  const endLine = ctx.state.doc.lineAt(
    Math.min(ctx.node.to, ctx.state.doc.length),
  ).number;
  for (let n = startLine; n <= endLine; n++) {
    const line = ctx.state.doc.line(n);
    ctx.push(listLineDecoration(line.text), line.from, line.from);
  }
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
