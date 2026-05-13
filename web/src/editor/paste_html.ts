// HTML-paste -> markdown conversion.
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
import type { Extension } from "@codemirror/state";

// Semantic HTML tags we treat as "this paste is actually rich" and
// worth running through turndown. Without these, a paste of bold or
// link-decorated plain text would skip conversion (current heuristic
// favors aggressive conversion).
const RICH_TAG_RE =
  /<(?:a|b|blockquote|br|code|del|em|h[1-6]|hr|i|img|li|ol|p|pre|s|strike|strong|sub|sup|table|td|th|tr|u|ul)\b/i;

export function htmlPasteHandler(): Extension {
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
      if (!html) return false;
      if (!RICH_TAG_RE.test(html)) return false;
      event.preventDefault();
      // Lazy import — the converter is only fetched on first rich
      // paste. Vite emits this as its own chunk.
      void htmlToMarkdown(html).then((md) => {
        if (!md) return;
        const sel = view.state.selection.main;
        view.dispatch({
          changes: { from: sel.from, to: sel.to, insert: md },
          selection: { anchor: sel.from + md.length },
        });
      });
      return true;
    },
  });
}

async function htmlToMarkdown(html: string): Promise<string> {
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
