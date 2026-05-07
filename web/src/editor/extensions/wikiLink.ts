// [[wiki link]] smart node + non-focus-stealing bubble.
//
// UX:
//   - Typing `[[` autopairs to `[[]]` with the caret between the
//     brackets. The trigger is NOT consumed: the literal brackets
//     stay in the editor and the caret stays inside.
//   - A bubble opens under the caret showing a header (the typed
//     query / doc name), a hint row advertising `# ^ |` modifiers,
//     a results list (top 5 file matches), and a "<enter> to accept"
//     row that appears once results are present.
//   - The bubble does not take focus. The editor's caret remains
//     active; the user types into the brackets and the bubble
//     re-renders on each keystroke. Enter / Escape / Arrow keys
//     are routed to the bubble by Wysiwyg.svelte's keydown handler
//     while the caret is inside the bracket range.
//
// Markdown:
//   - On accept the literal `[[query]]` text range is replaced
//     with a `wikiLink` atom node carrying `target` + `label`.
//   - The wikiLink node serializes to `[label](path)` so files on
//     disk stay portable across markdown readers.
//
// `#`, `^`, `|` modifier modes are added in follow-up commits;
// the bubble shell here is shaped to host them.

import { Node, mergeAttributes } from "@tiptap/core";

import { api } from "../../api/client";
import { wikiLinkToMarkdown } from "../../api/wasm";
import { openInActivePane } from "../../state/tabs.svelte";
import { positionPopover, watchViewport } from "./popover";

export const WikiLinkNode = Node.create({
  name: "wikiLink",
  group: "inline",
  inline: true,
  atom: true,
  selectable: true,

  addAttributes() {
    return {
      target: { default: "", parseHTML: (el) => el.getAttribute("data-target") ?? "" },
      label: { default: "", parseHTML: (el) => el.getAttribute("data-label") ?? "" },
    };
  },

  parseHTML() {
    return [{ tag: "span[data-md-wiki]" }];
  },

  renderHTML({ HTMLAttributes, node }) {
    return [
      "span",
      mergeAttributes(HTMLAttributes, {
        "data-md-wiki": "true",
        "data-target": node.attrs.target,
        "data-label": node.attrs.label,
        class: "md-smart md-smart-wiki",
        title: `→ ${node.attrs.target}`,
      }),
      (node.attrs.label as string) || (node.attrs.target as string),
    ];
  },

  addStorage() {
    return {
      markdown: {
        serialize(state: unknown, node: { attrs: { target: string; label: string } }) {
          const md = wikiLinkToMarkdown(node.attrs.target, node.attrs.label || undefined);
          (state as { write(s: string): void }).write(md);
        },
        parse: { setup() {} },
      },
    };
  },
});

/// Click handler for existing wiki nodes. Open the target in a new tab.
export function handleWikiClick(target: string): void {
  void openInActivePane(target);
}

// ---------------------------------------------------------------------------
// Bubble controller
// ---------------------------------------------------------------------------

export type WikiBubbleAccept =
  | { kind: "file"; target: string; label: string };

export interface WikiBubbleOpts {
  /// Element to anchor the bubble to (for positioning). Typically
  /// the cursor's parent element so the bubble sits under the caret.
  host: HTMLElement;
  /// Optional path prefix passed to `/api/search/files`. When set,
  /// suggestions stay scoped to that subdirectory (used to keep
  /// project-internal links project-bound).
  prefix?: string | null;
  /// Fires when the user clicks a result. The host commits the
  /// selection the same way it would on Enter (call `accept()` and
  /// replace the bracket range).
  onClickAccept?: () => void;
}

export interface WikiBubble {
  /// Update the query string (the text between the brackets) and
  /// re-render. The bubble debounces network calls; safe to call
  /// on every keystroke.
  setQuery(query: string): void;
  /// Move the active result selection by `delta` (+1 / -1), clamping
  /// to the result list bounds.
  moveActive(delta: number): void;
  /// Resolve the currently-highlighted result, or `null` if there
  /// are no results to commit. Caller is responsible for replacing
  /// the bracket range in the editor with a wikiLink node.
  accept(): WikiBubbleAccept | null;
  /// Tear down the DOM + listeners. Idempotent.
  dismiss(): void;
}

export function openWikiBubble(opts: WikiBubbleOpts): WikiBubble {
  const wrap = document.createElement("div");
  wrap.className = "md-wiki-bubble";
  wrap.style.position = "absolute";
  // Above any overlay (InlineAssist + SearchPanel sit at 25000),
  // so [[ inside the assistant prompt's Wysiwyg shows the bubble
  // ABOVE the chat backdrop instead of behind it.
  wrap.style.zIndex = "30000";

  const head = document.createElement("div");
  head.className = "md-wiki-bubble-head";
  wrap.appendChild(head);

  const hint = document.createElement("div");
  hint.className = "md-wiki-bubble-hint";
  hint.innerHTML =
    '<span><b>type #</b> to link heading</span>' +
    '<span><b>type ^</b> to link blocks</span>' +
    '<span><b>type |</b> to change display text</span>';
  wrap.appendChild(hint);

  const list = document.createElement("ul");
  list.className = "md-wiki-bubble-results";
  wrap.appendChild(list);

  const accept = document.createElement("div");
  accept.className = "md-wiki-bubble-accept";
  accept.textContent = "⏎  to accept"; // U+23CE return symbol
  wrap.appendChild(accept);

  document.body.appendChild(wrap);

  let active = 0;
  let entries: string[] = [];
  let lastQuery = "";
  let alive = true;
  let searchToken = 0;

  const renderHead = (q: string): void => {
    if (q.trim().length === 0) {
      head.textContent = "Linked note";
      head.classList.add("is-empty");
    } else {
      head.textContent = q;
      head.classList.remove("is-empty");
    }
  };

  const renderResults = (): void => {
    list.innerHTML = "";
    if (entries.length === 0) {
      list.classList.add("is-empty");
      accept.classList.add("is-hidden");
      // Re-position even when empty: the bubble height changes.
      if (wrap.isConnected) positionPopover(opts.host, wrap);
      return;
    }
    list.classList.remove("is-empty");
    accept.classList.remove("is-hidden");
    entries.forEach((path, i) => {
      const li = document.createElement("li");
      li.textContent = path;
      li.className = i === active ? "active" : "";
      // mousedown (not click) so the editor doesn't lose focus
      // before the picker can run; keep the editor selection alive
      // while the bubble commits.
      li.addEventListener("mousedown", (ev) => {
        ev.preventDefault();
        active = i;
        opts.onClickAccept?.();
      });
      list.appendChild(li);
    });
    if (wrap.isConnected) positionPopover(opts.host, wrap);
  };

  const runSearch = async (q: string): Promise<void> => {
    const token = ++searchToken;
    const trimmed = q.trim();
    if (!trimmed) {
      entries = [];
      active = 0;
      renderResults();
      return;
    }
    try {
      const hits = await api.search(trimmed, 5, opts.prefix ?? undefined);
      if (!alive || token !== searchToken) return;
      entries = hits.map((h) => h.path);
      active = 0;
      renderResults();
    } catch {
      if (!alive || token !== searchToken) return;
      entries = [];
      renderResults();
    }
  };

  positionPopover(opts.host, wrap);
  const stopWatch = watchViewport(opts.host, wrap);

  // Initial paint
  renderHead("");
  renderResults();

  return {
    setQuery(query: string): void {
      if (!alive) return;
      renderHead(query);
      if (query === lastQuery) return;
      lastQuery = query;
      void runSearch(query);
    },
    moveActive(delta: number): void {
      if (!alive || entries.length === 0) return;
      active = Math.max(0, Math.min(entries.length - 1, active + delta));
      renderResults();
    },
    accept(): WikiBubbleAccept | null {
      if (!alive || entries.length === 0) return null;
      const target = entries[active];
      if (!target) return null;
      const label = (target.split("/").pop() ?? target).replace(/\.md$/, "");
      return { kind: "file", target, label };
    },
    dismiss(): void {
      if (!alive) return;
      alive = false;
      stopWatch();
      wrap.remove();
    },
  };
}
