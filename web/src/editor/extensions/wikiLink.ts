// [[wiki link]] smart node + popover.
//
// Behavior:
//   - Typing `[[` opens a fuzzy file-search popover (queries /api/search).
//   - Picking a file inserts a `wikiLink` node with the chosen target +
//     visible label.
//   - Markdown serialization emits a standard markdown link `[label](path)`
//     so the file on disk stays portable to any markdown reader.
//   - Clicking an existing node opens the linked file in a new tab.
//
// The popover triggers + filtering logic live alongside this node so the
// extension is self-contained.

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

/// Open a popover anchored at `host`. Resolves with the chosen
/// path or null. `prefix`, when set, scopes the file-search to
/// that directory: the wiki-link picker for a file inside a git
/// repo passes the repo root so suggestions stay project-bound
/// (see scope.svelte.ts and FileResponse.repo_root).
export function showWikiPicker(
  host: HTMLElement,
  pick: (target: string | null) => void,
  prefix?: string | null,
): void {
  const wrap = document.createElement("div");
  wrap.className = "md-pick";
  wrap.style.position = "absolute";
  // Above any overlay (InlineAssist + SearchPanel sit at 25000),
  // so [[ inside the assistant prompt's Wysiwyg shows the picker
  // ABOVE the chat backdrop instead of behind it.
  wrap.style.zIndex = "30000";

  const input = document.createElement("input");
  input.placeholder = "search files...";
  input.className = "md-pick-input";
  wrap.appendChild(input);
  const list = document.createElement("ul");
  list.className = "md-pick-list";
  wrap.appendChild(list);

  let active = 0;
  let entries: string[] = [];

  const renderList = () => {
    list.innerHTML = "";
    entries.forEach((path, i) => {
      const li = document.createElement("li");
      li.textContent = path;
      li.className = i === active ? "active" : "";
      li.onmousedown = (ev) => {
        ev.preventDefault();
        cleanup();
        pick(path);
      };
      list.appendChild(li);
    });
    // The list-driven height swings between empty (just the input
    // box) and ~220px (a full result list). Re-position so a
    // popover that fit below an empty state but won't fit a full
    // list flips up rather than overflowing the viewport.
    if (wrap.isConnected) positionPopover(host, wrap);
  };

  const search = async (q: string) => {
    if (!q.trim()) {
      entries = [];
      renderList();
      return;
    }
    try {
      const hits = await api.search(q, 10, prefix ?? undefined);
      entries = hits.map((h) => h.path);
      active = 0;
      renderList();
    } catch {
      entries = [];
      renderList();
    }
  };

  input.addEventListener("input", () => void search(input.value));
  input.addEventListener("keydown", (e) => {
    if (e.key === "ArrowDown") {
      active = Math.min(active + 1, entries.length - 1);
      renderList();
      e.preventDefault();
    } else if (e.key === "ArrowUp") {
      active = Math.max(active - 1, 0);
      renderList();
      e.preventDefault();
    } else if (e.key === "Enter") {
      const sel = entries[active];
      if (sel) {
        cleanup();
        pick(sel);
      }
      e.preventDefault();
    } else if (e.key === "Escape") {
      cleanup();
      pick(null);
      e.preventDefault();
      // The InlineAssist overlay also listens for Escape at
      // window level; stop the bubble so dismissing the picker
      // does not also close the chat dialog around it.
      e.stopPropagation();
    }
  });

  const onAway = (ev: MouseEvent) => {
    // Qualify the DOM Node type: this file imports tiptap's Node
    // for its node spec, which shadows the global one.
    if (!wrap.contains(ev.target as globalThis.Node)) {
      cleanup();
      pick(null);
    }
  };
  document.body.appendChild(wrap);
  // After append + initial empty list, place the popover. The
  // search results may grow the popover height as the user types;
  // viewport-watch + a re-position on every renderList() keeps the
  // flip direction in sync.
  positionPopover(host, wrap);
  const stopWatch = watchViewport(host, wrap);
  const cleanup = () => {
    document.removeEventListener("mousedown", onAway);
    stopWatch();
    wrap.remove();
  };
  document.addEventListener("mousedown", onAway);
  setTimeout(() => input.focus(), 0);
}

/// Click handler for existing wiki nodes. Open the target in a new tab.
export function handleWikiClick(target: string): void {
  void openInActivePane(target);
}
