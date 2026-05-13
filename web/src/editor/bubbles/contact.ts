// Contact picker bubble for the `@word` trigger.
//
// Source: api.contacts(query, limit) — debounced 60ms per keystroke.
// req-seq pattern drops stale fetches if a newer query starts before
// the older response arrives. On commit, replaces `@query` with
// `[[<contact-path>]]` (a wikilink to the contact note).

import type { EditorView } from "@codemirror/view";
import { openBubbleShell } from "../bubble";
import { createCaretAnchor } from "./anchor";
import type { BubbleHandle } from "./types";
import { api } from "../../api/client";

export interface ContactBubbleOpts {
  view: EditorView;
  triggerStart: number;
  triggerEnd: number;
  initialQuery: string;
  onDismiss: () => void;
}

const PAGE_LIMIT = 8;
const FETCH_DEBOUNCE_MS = 60;

interface Contact {
  path: string;
  label: string;
  emails?: string[];
}

interface ContactBubbleHandle extends BubbleHandle {
  setTriggerEnd(end: number): void;
}

export function openContactBubble(opts: ContactBubbleOpts): ContactBubbleHandle {
  const caretPos = (): number => opts.view.state.selection.main.head;
  const anchor = createCaretAnchor(opts.view, caretPos());
  const shell = openBubbleShell({
    host: anchor.el,
    className: "md-contact-bubble cm-bubble",
  });
  let query = opts.initialQuery;
  let triggerEnd = opts.triggerEnd;
  let hits: Contact[] = [];
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

  function fetchContacts(): void {
    if (debounceTimer !== undefined) clearTimeout(debounceTimer);
    const seq = ++reqSeq;
    debounceTimer = window.setTimeout(() => {
      api
        .contacts(query, PAGE_LIMIT)
        .then((results) => {
          if (!alive || seq !== reqSeq) return;
          hits = results;
          if (selectedIndex >= hits.length) selectedIndex = 0;
          render();
        })
        .catch((err) => {
          if (!alive || seq !== reqSeq) return;
          hits = [];
          status.textContent = `Contact lookup failed: ${err.message ?? err}`;
        });
    }, FETCH_DEBOUNCE_MS);
  }

  function render(): void {
    list.innerHTML = "";
    if (hits.length === 0) {
      status.textContent = query.length === 0
        ? "Loading contacts..."
        : "No matches";
      shell.reposition();
      return;
    }
    status.textContent = `${hits.length} result${hits.length === 1 ? "" : "s"} · ↵ to insert`;
    for (let i = 0; i < hits.length; i++) {
      const c = hits[i]!;
      const row = document.createElement("div");
      row.className = "md-bubble-row";
      if (i === selectedIndex) row.classList.add("md-bubble-row-selected");
      const label = document.createElement("div");
      label.textContent = c.label;
      row.appendChild(label);
      if (c.emails && c.emails.length > 0) {
        const sub = document.createElement("div");
        sub.className = "md-bubble-row-sub";
        sub.textContent = c.emails[0]!;
        row.appendChild(sub);
      }
      row.addEventListener("mousedown", (e) => {
        e.preventDefault();
        e.stopPropagation();
        commit(c);
      });
      list.appendChild(row);
    }
    shell.reposition();
  }

  function commit(c: Contact): void {
    // Insert a wikilink to the contact note. The wikilink atom widget
    // (step 6c) renders this as a pill on the next decoration tick.
    const insert = `[[${c.path}|${c.label}]]`;
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

  fetchContacts();
  render();

  return {
    handleKey(event) {
      if (event.key === "Escape") {
        dismiss();
        return true;
      }
      if (event.key === "Enter") {
        const c = hits[selectedIndex];
        if (c) {
          commit(c);
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
    setQuery(q) {
      anchor.update(opts.view, caretPos());
      shell.reposition();
      if (q === query) return;
      query = q;
      fetchContacts();
    },
    setTriggerEnd(end) {
      triggerEnd = end;
    },
    reposition() {
      anchor.update(opts.view, caretPos());
      shell.reposition();
    },
    dismiss,
  };
}
