// Wiki bubble: file picker for the `[[query` trigger.
//
// On open, fetches /api/search/files for the query (debounced) and
// renders the top results as a clickable list. Keyboard model:
// ArrowUp / ArrowDown navigates, Enter commits, Escape dismisses.
//
// Commit: replaces `[[query` (the trigger range) with `[[target]]` and
// dismisses. The trigger range is provided at open time; subsequent
// query updates extend the trigger end as the user types more chars.
// The bubble keeps its own copy of the trigger range and updates it
// when the host calls setQuery (which the controller does on every
// non-trivial transaction).
//
// v1.0 scope: file search only. Anchor (`#`) and block (`^`) modes
// are deferred to v1.1 — when the user types `#` or `^` after the
// resolved target, the controller will switch to a heading / block
// picker. For now those keys just extend the query and probably
// produce no matches.
//
// Error handling: an in-flight fetch superseded by a newer one is
// dropped (req-seq pattern). Network errors render an inline "search
// failed" footer; the list stays empty so nothing commits.

import type { EditorView } from "@codemirror/view";
import { openBubbleShell, type BubbleShell } from "../../editor/bubble";
import type { BubbleHandle } from "./types";
import { createCaretAnchor, type CaretAnchor } from "./anchor";
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
const FETCH_DEBOUNCE_MS = 60;

interface SearchHit {
  path: string;
}

interface WikiBubbleHandle extends BubbleHandle {
  /// Update the trigger end after user typing extends the query.
  /// Called by the controller on every transaction so commits target
  /// the right range.
  setTriggerEnd(end: number): void;
}

export function openWikiBubble(opts: WikiBubbleOpts): WikiBubbleHandle {
  const anchor = createCaretAnchor(opts.view, opts.triggerStart);
  const shell = openBubbleShell({
    host: anchor.el,
    className: "md-wiki-bubble cm-bubble",
  });

  let query = opts.initialQuery;
  let triggerEnd = opts.triggerEnd;
  let hits: SearchHit[] = [];
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

  function render(): void {
    list.innerHTML = "";
    if (hits.length === 0) {
      status.textContent = query.length === 0
        ? "Type to search files"
        : "No matches";
      shell.reposition();
      return;
    }
    status.textContent = `${hits.length} result${hits.length === 1 ? "" : "s"} · ↵ to insert`;
    for (let i = 0; i < hits.length; i++) {
      const hit = hits[i]!;
      const row = document.createElement("div");
      row.className = "md-bubble-row";
      if (i === selectedIndex) row.classList.add("md-bubble-row-selected");
      row.textContent = hit.path;
      row.addEventListener("mousedown", (e) => {
        e.preventDefault();
        e.stopPropagation();
        commit(hit);
      });
      list.appendChild(row);
    }
    shell.reposition();
  }

  function fetchHits(): void {
    if (debounceTimer !== undefined) clearTimeout(debounceTimer);
    const seq = ++reqSeq;
    debounceTimer = window.setTimeout(() => {
      api
        .search(query, SEARCH_LIMIT, opts.prefix)
        .then((results) => {
          if (!alive || seq !== reqSeq) return;
          hits = results as SearchHit[];
          if (selectedIndex >= hits.length) selectedIndex = 0;
          render();
        })
        .catch((err) => {
          if (!alive || seq !== reqSeq) return;
          hits = [];
          status.textContent = `Search failed: ${err.message ?? err}`;
        });
    }, FETCH_DEBOUNCE_MS);
  }

  function commit(hit: SearchHit): void {
    const insert = `[[${hit.path}]]`;
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

  // Initial fetch.
  fetchHits();
  render();

  return {
    handleKey(event: KeyboardEvent): boolean {
      if (event.key === "Escape") {
        dismiss();
        return true;
      }
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
      if (q === query) return;
      query = q;
      selectedIndex = 0;
      fetchHits();
    },
    setTriggerEnd(end: number): void {
      triggerEnd = end;
    },
    reposition(): void {
      anchor.update(opts.view, opts.triggerStart);
      shell.reposition();
    },
    dismiss,
  };
}
