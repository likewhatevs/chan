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
      // Anchor inside the target file. Heading anchors are slugs
      // (`section-name`); block anchors carry the leading `^`
      // (`^abc123`). Empty when the link points at a whole file.
      anchor: { default: "", parseHTML: (el) => el.getAttribute("data-anchor") ?? "" },
    };
  },

  parseHTML() {
    return [{ tag: "span[data-md-wiki]" }];
  },

  renderHTML({ HTMLAttributes, node }) {
    const anchor = (node.attrs.anchor as string) ?? "";
    const titleSuffix = anchor ? `#${anchor}` : "";
    return [
      "span",
      mergeAttributes(HTMLAttributes, {
        "data-md-wiki": "true",
        "data-target": node.attrs.target,
        "data-label": node.attrs.label,
        "data-anchor": anchor,
        class: "md-smart md-smart-wiki",
        title: `→ ${node.attrs.target}${titleSuffix}`,
      }),
      (node.attrs.label as string) || (node.attrs.target as string),
    ];
  },

  addStorage() {
    return {
      markdown: {
        serialize(
          state: unknown,
          node: { attrs: { target: string; label: string; anchor: string } },
        ) {
          const md = wikiLinkToMarkdown(
            node.attrs.target,
            node.attrs.label || undefined,
            node.attrs.anchor || undefined,
          );
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
  | { kind: "file"; target: string; label: string }
  | { kind: "heading"; target: string; anchor: string; label: string };

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

import type { HeadingRow } from "../../api/types";

type Mode = "file" | "heading";

interface FileEntry {
  kind: "file";
  path: string;
}

interface HeadingEntry {
  kind: "heading";
  row: HeadingRow;
}

type Entry = FileEntry | HeadingEntry;

/// Split the bubble query into `(filePart, sigil, sigilPart)`. The
/// sigil is the first occurrence of `#`, `^`, or `|`. Sigil and
/// sigilPart are empty when the query carries none of them. The
/// modifier modes consume only ONE sigil; subsequent ones are part
/// of the sigilPart text (e.g. block ids may contain `^`).
function splitQuery(q: string): {
  filePart: string;
  sigil: "" | "#" | "^" | "|";
  sigilPart: string;
} {
  for (let i = 0; i < q.length; i++) {
    const c = q[i];
    if (c === "#" || c === "^" || c === "|") {
      return {
        filePart: q.slice(0, i),
        sigil: c as "#" | "^" | "|",
        sigilPart: q.slice(i + 1),
      };
    }
  }
  return { filePart: q, sigil: "", sigilPart: "" };
}

function fileLabel(target: string): string {
  return (target.split("/").pop() ?? target).replace(/\.md$/, "");
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

  let mode: Mode = "file";
  /// File picked when transitioning into heading mode. Held while
  /// the query keeps a `#`; cleared when the user backspaces past
  /// the sigil and we revert to file mode.
  let lockedFile: string | null = null;
  /// All headings of `lockedFile`, fetched once on transition. The
  /// user's post-`#` text filters this in-memory (no per-keystroke
  /// HTTP call).
  let lockedHeadings: HeadingRow[] = [];
  let entries: Entry[] = [];
  let active = 0;
  let lastQuery = "";
  let alive = true;
  let searchToken = 0;
  let headingToken = 0;

  const renderHead = (q: string): void => {
    const { filePart, sigil } = splitQuery(q);
    let label: string;
    if (mode === "heading" && lockedFile) {
      // In heading mode the header reflects the file we're
      // anchoring into, with the typed heading suffix so the user
      // sees the link they're building.
      const sigilPart = q.slice(filePart.length);
      label = `${fileLabel(lockedFile)}${sigilPart}`;
    } else if (q.trim().length === 0) {
      label = "Linked note";
    } else if (sigil) {
      // File-mode rendering of a query that already has a sigil but
      // didn't transition (e.g. `#` typed before any file matched).
      label = q;
    } else {
      label = q;
    }
    head.textContent = label;
    head.classList.toggle("is-empty", q.trim().length === 0);
  };

  const renderResults = (): void => {
    list.innerHTML = "";
    if (entries.length === 0) {
      list.classList.add("is-empty");
      accept.classList.add("is-hidden");
      if (wrap.isConnected) positionPopover(opts.host, wrap);
      return;
    }
    list.classList.remove("is-empty");
    accept.classList.remove("is-hidden");
    entries.forEach((entry, i) => {
      const li = document.createElement("li");
      if (entry.kind === "file") {
        li.textContent = entry.path;
      } else {
        // Heading rows render as `## Heading text` so the user can
        // see the level at a glance. Indent by level after the
        // hashes for outline shape.
        const hashes = "#".repeat(Math.min(6, Math.max(1, entry.row.level)));
        li.textContent = `${hashes} ${entry.row.text}`;
        li.classList.add("is-heading");
      }
      li.className += i === active ? " active" : "";
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

  const runFileSearch = async (filePart: string): Promise<void> => {
    const token = ++searchToken;
    const trimmed = filePart.trim();
    if (!trimmed) {
      entries = [];
      active = 0;
      renderResults();
      return;
    }
    try {
      const hits = await api.search(trimmed, 5, opts.prefix ?? undefined);
      if (!alive || token !== searchToken) return;
      entries = hits.map((h) => ({ kind: "file", path: h.path }));
      active = 0;
      renderResults();
    } catch {
      if (!alive || token !== searchToken) return;
      entries = [];
      renderResults();
    }
  };

  /// Filter `lockedHeadings` against the post-`#` text. Substring
  /// match on text, case-insensitive. Top 5 only so the bubble
  /// stays compact.
  const filterHeadings = (sigilPart: string): void => {
    const needle = sigilPart.trim().toLowerCase();
    const matches = needle
      ? lockedHeadings.filter((h) => h.text.toLowerCase().includes(needle))
      : lockedHeadings.slice();
    entries = matches.slice(0, 5).map((row) => ({ kind: "heading", row }));
    active = 0;
    renderResults();
  };

  /// Switch into heading mode with `lockedFile = path`. Loads the
  /// heading list once; subsequent keystrokes filter in-memory.
  const enterHeadingMode = async (path: string, sigilPart: string): Promise<void> => {
    const token = ++headingToken;
    mode = "heading";
    lockedFile = path;
    lockedHeadings = [];
    // Show an empty list until the fetch completes; first paint
    // also re-renders the head with the locked file name.
    entries = [];
    active = 0;
    renderResults();
    try {
      const headings = await api.headings(path);
      if (!alive || token !== headingToken) return;
      lockedHeadings = headings;
      filterHeadings(sigilPart);
    } catch {
      if (!alive || token !== headingToken) return;
      lockedHeadings = [];
      entries = [];
      renderResults();
    }
  };

  const exitHeadingMode = (): void => {
    mode = "file";
    lockedFile = null;
    lockedHeadings = [];
    headingToken++; // invalidate any in-flight fetch
  };

  positionPopover(opts.host, wrap);
  const stopWatch = watchViewport(opts.host, wrap);

  // Initial paint
  renderHead("");
  renderResults();

  return {
    setQuery(query: string): void {
      if (!alive) return;
      const { filePart, sigil, sigilPart } = splitQuery(query);

      // Heading-mode transitions. We only enter heading mode when
      // there IS a file to lock onto: with no resolved file, `#` in
      // the query keeps file mode and just gets searched literally
      // (which is fine; usually returns nothing).
      if (sigil === "#") {
        if (mode !== "heading") {
          // Pick the file to lock: prefer the currently-active file
          // entry if we're already showing file results; otherwise
          // take the top hit. If there is no file at all, stay in
          // file mode and let the literal `#` filter below.
          let candidate: string | null = null;
          if (entries.length > 0 && entries[active]?.kind === "file") {
            candidate = (entries[active] as FileEntry).path;
          } else if (entries.length > 0 && entries[0]?.kind === "file") {
            candidate = (entries[0] as FileEntry).path;
          }
          if (candidate) {
            void enterHeadingMode(candidate, sigilPart);
            renderHead(query);
            lastQuery = query;
            return;
          }
        } else if (lockedFile) {
          // Already in heading mode: just re-filter in memory.
          filterHeadings(sigilPart);
          renderHead(query);
          lastQuery = query;
          return;
        }
      } else if (mode === "heading") {
        // No `#` in query but we're in heading mode: the user
        // backspaced past the sigil. Revert to file mode.
        exitHeadingMode();
      }

      renderHead(query);
      if (query === lastQuery) return;
      lastQuery = query;
      // File-mode: search on the part before any sigil so a stray
      // `^` or `|` (handled in later commits) doesn't poison the
      // search query.
      void runFileSearch(filePart || query);
    },
    moveActive(delta: number): void {
      if (!alive || entries.length === 0) return;
      active = Math.max(0, Math.min(entries.length - 1, active + delta));
      renderResults();
    },
    accept(): WikiBubbleAccept | null {
      if (!alive || entries.length === 0) return null;
      const entry = entries[active];
      if (!entry) return null;
      if (entry.kind === "file") {
        return {
          kind: "file",
          target: entry.path,
          label: fileLabel(entry.path),
        };
      }
      // heading
      if (!lockedFile) return null;
      return {
        kind: "heading",
        target: lockedFile,
        anchor: entry.row.anchor,
        label: fileLabel(lockedFile),
      };
    },
    dismiss(): void {
      if (!alive) return;
      alive = false;
      stopWatch();
      wrap.remove();
    },
  };
}
