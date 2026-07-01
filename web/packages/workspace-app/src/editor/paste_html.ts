// Paste handling: HTML -> markdown conversion, plus a plain-text list-dedent.
//
// The paste handler covers three cases: image-file pastes defer to the
// image-drop handler; rich HTML is converted to markdown via turndown; and a
// plain-text paste of a list item into an existing list line has its leading
// marker stripped (dedentListPaste) so a copied "- item" merges into the
// current bullet instead of nesting as "- - item". Everything else defers to
// CM6's default paste.
//
// When the clipboard carries an HTML representation (typical for a
// paste from another rich-text editor, a Notion / Obsidian / Office
// doc, or a browser's selection copy), convert it to markdown via
// turndown and insert the markdown instead of the raw HTML.
//
// Skip rules:
//   - No HTML in the clipboard -> defer to CM6's default plain-text
//     paste.
//   - HTML is just a wrapped plain-text run (no semantic tags) ->
//     defer (we don't want to over-process Cmd+V'd plaintext).
//   - Clipboard also contains image files -> defer to the image-drop
//     handler (image upload + ![](path) insert).
//
// turndown is lazy-imported on first paste so the converter (~25 KB
// gzip) doesn't ship in the main bundle until the user actually
// pastes rich content.

import { EditorView } from "@codemirror/view";
import type { EditorState, Extension } from "@codemirror/state";
import { parseListPrefix } from "./commands/list";

// Semantic HTML tags we treat as "this paste is actually rich" and
// worth running through turndown. Without these, a paste of bold or
// link-decorated plain text would skip conversion (current heuristic
// favors aggressive conversion). Includes definition lists, figures,
// and inline phrasing tags (mark/kbd/samp) so pastes from docs that
// rely on those don't fall back to raw HTML.
const RICH_TAG_RE =
  /<(?:a|b|blockquote|br|code|dd|del|dl|dt|em|figcaption|figure|h[1-6]|hr|i|img|kbd|li|mark|ol|p|pre|s|samp|strike|strong|sub|sup|table|td|th|tr|u|ul)\b/i;

export function pasteHandler(): Extension {
  return EditorView.domEventHandlers({
    paste(event, view) {
      const cd = event.clipboardData;
      if (!cd) return false;
      // Image-file paste: let the image-drop handler take it.
      for (const item of Array.from(cd.items)) {
        if (item.kind === "file" && item.type.startsWith("image/")) {
          return false;
        }
      }
      const html = cd.getData("text/html");
      if (html && RICH_TAG_RE.test(html)) {
        event.preventDefault();
        // Lazy import - the converter is only fetched on first rich
        // paste. Vite emits this as its own chunk.
        void htmlToMarkdown(html).then((md) => {
          if (!md) return;
          const sel = view.state.selection.main;
          const insert = dedentListPaste(view.state, sel.from, md);
          view.dispatch({
            changes: { from: sel.from, to: sel.to, insert },
            selection: { anchor: sel.from + insert.length },
          });
        });
        return true;
      }
      // Plain-text paste of a list item INTO a list line: strip the pasted
      // marker so a copied "- item" merges into the current bullet instead of
      // nesting under it (the "- - item" double-marker). Same dedent as the
      // rich path, but for the common chan-to-chan copy, which is plain text
      // (navigator.clipboard.writeText). Only intercept when the dedent
      // actually changes the text; every other paste defers to CM6's default.
      const text = cd.getData("text/plain");
      if (text) {
        const sel = view.state.selection.main;
        const insert = dedentListPaste(view.state, sel.from, text);
        if (insert !== text) {
          event.preventDefault();
          view.dispatch({
            changes: { from: sel.from, to: sel.to, insert },
            selection: { anchor: sel.from + insert.length },
          });
          return true;
        }
      }
      return false;
    },
  });
}

/// When rich content is pasted INTO a list item, turndown converts a
/// copied list-item link (clipboard HTML `<ul><li><a>...`, which is
/// what copying a link out of a list / web page yields) to
/// `-   [url](url)` - a leading bullet marker. Inserting that verbatim
/// into an existing `- ` bullet yields `- -   [url]`, which parses as a
/// stray NESTED bullet - pasting a link indented the list.
/// When the caret line is already a list item, strip a leading list
/// marker from the FIRST pasted line so the content flows into the
/// current bullet as a sibling instead of nesting under it. Only the
/// first line is touched, so a genuine multi-item paste keeps its later
/// bullets. A bare-anchor paste (turndown emits inline `[url](url)`, no
/// marker) is unaffected: `parseListPrefix` returns null and `md` passes
/// through unchanged. Exported for the unit test.
export function dedentListPaste(
  state: EditorState,
  pos: number,
  md: string,
): string {
  if (!parseListPrefix(state.doc.lineAt(pos).text)) return md;
  const nl = md.indexOf("\n");
  const first = nl === -1 ? md : md.slice(0, nl);
  const firstPrefix = parseListPrefix(first);
  if (!firstPrefix) return md;
  return first.slice(firstPrefix.length) + (nl === -1 ? "" : md.slice(nl));
}

// Exported for the vitest pin in `paste_html.test.ts`. Production
// callers go through `pasteHandler` above; the converter is
// kept exported so the escape-override behaviour can be exercised
// directly without spinning up a CM6 view.
export async function htmlToMarkdown(html: string): Promise<string> {
  const { default: TurndownService } = await import("turndown");
  const td = new TurndownService({
    headingStyle: "atx",
    hr: "---",
    bulletListMarker: "-",
    codeBlockStyle: "fenced",
    fence: "```",
    emDelimiter: "*",
    strongDelimiter: "**",
    linkStyle: "inlined",
  });
  // Turndown's default text-node escape inserts a backslash before
  // every markdown special character (`*` / `_` / `[` / `]` /
  // `` ` `` / `#` / etc.) so a pasted "*bold*" arrives as literal
  // `\*bold\*` in the editor instead of rendering as **bold**.
  // Override the escape with identity so pasted text round-trips
  // verbatim through the parser. The accepted side effect -- literal
  // stray asterisks in pasted plain text now trigger emphasis -- is
  // fine for the markdown-pipeline workflow; users who need the
  // escaped shape can flip to source mode before pasting.
  td.escape = (s: string) => s;
  // Strikethrough rule: turndown's default doesn't include strike;
  // GFM has it via ~~text~~.
  td.addRule("strikethrough", {
    filter: ["del", "s"],
    replacement: (content: string) => `~~${content}~~`,
  });
  // Task-list rule: <li> with a leading checkbox <input> becomes
  // `- [x]` / `- [ ]` markdown.
  td.addRule("taskListItem", {
    filter: (node: HTMLElement) => {
      if (node.nodeName !== "LI") return false;
      const cb = node.querySelector("input[type=checkbox]");
      return cb !== null;
    },
    replacement: (content: string, node) => {
      const el = node as HTMLElement;
      const cb = el.querySelector(
        "input[type=checkbox]",
      ) as HTMLInputElement | null;
      const mark = cb && cb.checked ? "[x]" : "[ ]";
      return `- ${mark} ${content.trim()}\n`;
    },
  });
  return td.turndown(html);
}
