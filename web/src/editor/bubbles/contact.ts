// Contact picker bubble.
//
// Two trigger shapes, one bubble:
//   - `@word`   (mode "wiki", legacy): commits `[[<path>|<label>]]`
//     so the picked contact lands as a wiki-link pill in the source.
//   - `@@word`  (mode "mention", phase 5): commits `@@<alias-or-stem>`
//     so the picked contact lands as a mention pill that
//     chan-server's mention_to_contact map resolves back to the
//     contact file via aliases + basename stem.
//
// Source: api.contacts(query, limit) — debounced 60ms per keystroke.
// req-seq pattern drops stale fetches if a newer query starts before
// the older response arrives.

import type { EditorView } from "@codemirror/view";
import { openBubbleShell } from "../bubble";
import { createCaretAnchor } from "./anchor";
import type { BubbleHandle } from "./types";
import { api } from "../../api/client";

export type ContactBubbleMode = "wiki" | "mention";

export interface ContactBubbleOpts {
  view: EditorView;
  triggerStart: number;
  triggerEnd: number;
  initialQuery: string;
  onDismiss: () => void;
  /// Insertion mode. "wiki" is the legacy `@` trigger; "mention" is
  /// the new `@@` trigger that writes `@@<alias-or-stem>` so the
  /// graph keeps the mention sigil in the source.
  mode?: ContactBubbleMode;
}

const PAGE_LIMIT = 8;
const FETCH_DEBOUNCE_MS = 60;

interface Contact {
  path: string;
  label: string;
  emails?: string[];
  aliases?: string[];
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
      // Aliases ride as a tertiary line so power users see at a
      // glance which @@<alias> shortcuts resolve to this contact.
      // Suppressed when empty so the common (no-alias) case keeps
      // the row compact.
      if (c.aliases && c.aliases.length > 0) {
        const aliasLine = document.createElement("div");
        aliasLine.className = "md-bubble-row-sub md-bubble-row-aliases";
        aliasLine.textContent = c.aliases.map((a) => `@@${a}`).join(" · ");
        row.appendChild(aliasLine);
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
    // Two insertion shapes (see module docstring):
    //   - wiki mode (the legacy `@` trigger): `[[<path>|<label>]]`.
    //     The wikilink atom widget renders the pill on the next
    //     decoration tick.
    //   - mention mode (the `@@` trigger): `@@<alias-or-stem>`. The
    //     mention extractor + mention_to_contact map resolve this
    //     back to the contact file at graph query time.
    //
    // For mention mode we prefer the user's typed query when it
    // exactly matches one of the contact's aliases (or the basename
    // stem); otherwise fall back to the basename stem. That keeps
    // the user's intent ("they typed `@@ali`") visible in the
    // source rather than blindly rewriting to a canonical form.
    const mode: ContactBubbleMode = opts.mode ?? "wiki";
    const insert = mode === "mention"
      ? `@@${pickMentionName(c, query)}`
      : `[[${c.path}|${c.label}]]`;
    opts.view.dispatch({
      changes: { from: opts.triggerStart, to: triggerEnd, insert },
      selection: { anchor: opts.triggerStart + insert.length },
    });
    dismiss();
  }

  function pickMentionName(c: Contact, typed: string): string {
    const lower = typed.toLowerCase();
    const stem = basenameStem(c.path);
    if (lower === stem) return stem;
    if (c.aliases) {
      for (const a of c.aliases) {
        if (a.toLowerCase() === lower) return a;
      }
    }
    return stem;
  }

  function basenameStem(path: string): string {
    const slash = path.lastIndexOf("/");
    const base = slash < 0 ? path : path.slice(slash + 1);
    const dot = base.lastIndexOf(".");
    return (dot <= 0 ? base : base.slice(0, dot)).toLowerCase();
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
