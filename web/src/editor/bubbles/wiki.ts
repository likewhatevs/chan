// Wiki bubble: file picker for the `[[query` trigger, plus heading
// picker for `[[target#headingFilter`.
//
// Mode switch happens automatically inside setQuery: a `#` in the
// query splits it into target + headingFilter and switches the bubble
// to heading mode, fetching /api/headings/{target} once per target
// (in-memory filter on subsequent typing). When the user backspaces
// past the `#` the bubble drops back to file mode.
//
// File mode: /api/search/files for the query (debounced), top
// SEARCH_LIMIT results, commit replaces `[[query` with `[[target]]`.
// Heading mode: filtered headings list, commit replaces `[[query`
// with `[[target#anchor]]` where anchor is the slug returned by the
// server (so the on-disk link survives a heading rename without
// chasing the source text).
//
// Block mode (`^`) is v1.2 — needs the file body to parse `^id` block
// markers.
//
// Errors render in the status footer; the list stays empty so nothing
// commits. Stale fetches drop via reqSeq.

import type { EditorView } from "@codemirror/view";
import { openBubbleShell } from "../bubble";
import type { BubbleHandle } from "./types";
import { createCaretAnchor } from "./anchor";
import { api } from "../../api/client";

export interface WikiBubbleOpts {
  view: EditorView;
  triggerStart: number;
  triggerEnd: number;
  initialQuery: string;
  /// Optional path scope passed through to /api/search/files (project-
  /// bound suggestions). null for unscoped global search.
  prefix: string | null;
  onDismiss: () => void;
}

const SEARCH_LIMIT = 5;
const HEADING_LIMIT = 8;
const FETCH_DEBOUNCE_MS = 60;

interface SearchHit {
  path: string;
}
interface HeadingHit {
  level: number;
  text: string;
  anchor: string;
}

interface WikiBubbleHandle extends BubbleHandle {
  setTriggerEnd(end: number): void;
}

type Mode =
  | { kind: "file" }
  | { kind: "heading"; target: string; filter: string };

function classifyQuery(q: string): Mode {
  const hashIdx = q.indexOf("#");
  if (hashIdx < 0) return { kind: "file" };
  return {
    kind: "heading",
    target: q.slice(0, hashIdx),
    filter: q.slice(hashIdx + 1),
  };
}

export function openWikiBubble(opts: WikiBubbleOpts): WikiBubbleHandle {
  // Anchor under the live caret (not the trigger start) so the bubble
  // follows the cursor as the user types — important when a long
  // typed query wraps to a second visual line, where positioning at
  // the trigger would leave the bubble overlapping the wrapped text.
  const caretPos = (): number => opts.view.state.selection.main.head;
  const anchor = createCaretAnchor(opts.view, caretPos());
  const shell = openBubbleShell({
    host: anchor.el,
    className: "md-wiki-bubble cm-bubble",
  });

  let query = opts.initialQuery;
  let triggerEnd = opts.triggerEnd;
  let mode: Mode = classifyQuery(query);
  // File-mode results.
  let fileHits: SearchHit[] = [];
  // Heading-mode all-headings cache (keyed by target so target switch
  // re-fetches only when needed) + filtered display list.
  let headingTarget: string | null = null;
  let headingAll: HeadingHit[] = [];
  let headingHits: HeadingHit[] = [];
  let selectedIndex = 0;
  let reqSeq = 0;
  let debounceTimer: number | undefined;
  let alive = true;

  const list = document.createElement("div");
  list.className = "md-bubble-list";
  shell.wrap.appendChild(list);
  const status = document.createElement("div");
  status.className = "md-bubble-status";
  shell.wrap.appendChild(status);

  function activeHits(): Array<SearchHit | HeadingHit> {
    return mode.kind === "heading" ? headingHits : fileHits;
  }

  function render(): void {
    list.innerHTML = "";
    const hits = activeHits();
    if (hits.length === 0) {
      if (mode.kind === "file") {
        status.textContent = query.length === 0
          ? "Type to search files"
          : "No matches";
      } else {
        status.textContent = headingTarget === mode.target
          ? "No matching headings"
          : "Loading headings...";
      }
      shell.reposition();
      return;
    }
    status.textContent = mode.kind === "heading"
      ? `${hits.length} heading${hits.length === 1 ? "" : "s"} in ${mode.target} · ↵ to insert`
      : `${hits.length} result${hits.length === 1 ? "" : "s"} · ↵ to insert`;
    for (let i = 0; i < hits.length; i++) {
      const hit = hits[i]!;
      const row = document.createElement("div");
      row.className = "md-bubble-row";
      if (i === selectedIndex) row.classList.add("md-bubble-row-selected");
      if (mode.kind === "heading") {
        const h = hit as HeadingHit;
        const level = document.createElement("span");
        level.className = "md-bubble-row-level";
        level.textContent = `H${h.level}`;
        const text = document.createElement("span");
        text.textContent = h.text;
        row.appendChild(level);
        row.appendChild(text);
      } else {
        row.textContent = (hit as SearchHit).path;
      }
      row.addEventListener("mousedown", (e) => {
        e.preventDefault();
        e.stopPropagation();
        commit(hit);
      });
      list.appendChild(row);
    }
    shell.reposition();
  }

  function fetchFile(): void {
    if (debounceTimer !== undefined) clearTimeout(debounceTimer);
    const seq = ++reqSeq;
    debounceTimer = window.setTimeout(() => {
      api
        .search(query, SEARCH_LIMIT, opts.prefix)
        .then((results) => {
          if (!alive || seq !== reqSeq || mode.kind !== "file") return;
          fileHits = results as SearchHit[];
          if (selectedIndex >= fileHits.length) selectedIndex = 0;
          render();
        })
        .catch((err) => {
          if (!alive || seq !== reqSeq) return;
          fileHits = [];
          status.textContent = `Search failed: ${err.message ?? err}`;
        });
    }, FETCH_DEBOUNCE_MS);
  }

  function fetchHeadings(target: string): void {
    if (headingTarget === target) {
      // Cache hit — just re-filter.
      filterHeadings();
      return;
    }
    if (debounceTimer !== undefined) clearTimeout(debounceTimer);
    const seq = ++reqSeq;
    headingAll = [];
    headingHits = [];
    headingTarget = null;
    render();
    api
      .headings(target)
      .then((results) => {
        if (!alive || seq !== reqSeq) return;
        if (mode.kind !== "heading" || mode.target !== target) return;
        headingTarget = target;
        headingAll = results as HeadingHit[];
        filterHeadings();
      })
      .catch((err) => {
        if (!alive || seq !== reqSeq) return;
        headingAll = [];
        headingHits = [];
        status.textContent = `Headings failed: ${err.message ?? err}`;
      });
  }

  function filterHeadings(): void {
    if (mode.kind !== "heading") return;
    const f = mode.filter.toLowerCase();
    headingHits = (f.length === 0
      ? headingAll
      : headingAll.filter((h) => h.text.toLowerCase().includes(f))
    ).slice(0, HEADING_LIMIT);
    if (selectedIndex >= headingHits.length) selectedIndex = 0;
    render();
  }

  function refetchForMode(): void {
    if (mode.kind === "file") {
      fetchFile();
    } else {
      fetchHeadings(mode.target);
    }
  }

  function commit(hit: SearchHit | HeadingHit): void {
    let insert: string;
    if (mode.kind === "heading") {
      const h = hit as HeadingHit;
      insert = `[[${mode.target}#${h.anchor}]]`;
    } else {
      insert = `[[${(hit as SearchHit).path}]]`;
    }
    opts.view.dispatch({
      changes: { from: opts.triggerStart, to: triggerEnd, insert },
      selection: { anchor: opts.triggerStart + insert.length },
    });
    dismiss();
  }

  function dismiss(): void {
    if (!alive) return;
    alive = false;
    if (debounceTimer !== undefined) clearTimeout(debounceTimer);
    shell.dismiss();
    anchor.dismiss();
    opts.onDismiss();
  }

  // Initial fetch for whichever mode the opening query parses to.
  refetchForMode();
  render();

  return {
    handleKey(event: KeyboardEvent): boolean {
      if (event.key === "Escape") {
        dismiss();
        return true;
      }
      const hits = activeHits();
      if (event.key === "Enter") {
        const hit = hits[selectedIndex];
        if (hit) {
          commit(hit);
          return true;
        }
        return false;
      }
      if (event.key === "ArrowDown") {
        if (hits.length === 0) return false;
        selectedIndex = (selectedIndex + 1) % hits.length;
        render();
        return true;
      }
      if (event.key === "ArrowUp") {
        if (hits.length === 0) return false;
        selectedIndex = (selectedIndex - 1 + hits.length) % hits.length;
        render();
        return true;
      }
      return false;
    },
    setQuery(q: string): void {
      // Always re-anchor on every spec update — the user may have
      // arrowed inside the trigger range without changing the query
      // text, and the bubble needs to follow the live caret so it
      // doesn't overlap wrapped text.
      anchor.update(opts.view, caretPos());
      shell.reposition();
      if (q === query) return;
      query = q;
      const newMode = classifyQuery(q);
      const modeChanged = newMode.kind !== mode.kind;
      const targetChanged =
        newMode.kind === "heading" &&
        mode.kind === "heading" &&
        newMode.target !== mode.target;
      mode = newMode;
      selectedIndex = 0;
      if (modeChanged || targetChanged) {
        refetchForMode();
      } else if (mode.kind === "heading") {
        // Same target, filter changed — local re-filter only.
        filterHeadings();
      } else {
        fetchFile();
      }
    },
    setTriggerEnd(end: number): void {
      triggerEnd = end;
    },
    reposition(): void {
      anchor.update(opts.view, caretPos());
      shell.reposition();
    },
    dismiss,
  };
}
