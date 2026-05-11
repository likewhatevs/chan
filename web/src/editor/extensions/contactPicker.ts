// `@` contact-picker bubble.
//
// Trigger (in Wysiwyg.svelte's onInput): a fresh `@` keystroke at
// start-of-word opens the bubble. The bubble re-queries
// /api/contacts on each keystroke until the caret leaves the
// trigger range or the user accepts (Enter / click) / dismisses
// (Esc / line change / `@` followed by space). On accept, the
// host replaces the `@<query>` range with `[[<rel_path>]]`.
//
// Same UX shape as TagBubble: the bubble does NOT take focus; the
// host routes Enter / Escape / Arrow keys while it's open via the
// shared `BubbleHandle.handleKey`. Layout / lifecycle live behind
// `openBubbleShell` so the popover floats correctly under the
// caret across scrolls and reflows.

import { openBubbleShell, type BubbleHandle } from "../bubble";
import { api } from "../../api/client";

export type ContactRow = { path: string; label: string; emails?: string[] };

export interface ContactBubble extends BubbleHandle {
  /// Update the typed query (without the leading `@`) and re-fetch.
  setQuery(query: string): void;
  /// Move the active row by +1 / -1, clamped to bounds.
  moveActive(delta: number): void;
  /// Resolve the highlighted contact, or null when there are no
  /// results. Caller takes the path for wiki-link insertion.
  accept(): ContactRow | null;
  /// Tear down DOM + listeners. Idempotent.
  dismiss(): void;
}

export interface ContactBubbleOpts {
  /// Anchor for positioning. Pass the caret-anchor shim so the
  /// bubble sits under the cursor.
  host: HTMLElement;
  /// Click-to-commit path; the host commits the same way as on
  /// Enter (calls accept() and replaces the trigger range).
  onClickAccept?: () => void;
  /// Fires on Enter inside the bubble. Wired to the same host
  /// accept path as `onClickAccept`.
  onCommit?: () => void;
  /// Fires on Escape. Host runs its full dismiss path.
  onDismiss?: () => void;
}

/// Result cap. Picker is keyboard-driven; 8 rows fits a typical
/// viewport without scrolling and keeps the round-trip small.
const PAGE_LIMIT = 8;

/// Debounce window for the per-keystroke fetch. Trades a bit of
/// freshness for fewer HTTP round-trips when the user types fast;
/// 60ms is below typical typing-pause perception.
const DEBOUNCE_MS = 60;

export function openContactBubble(opts: ContactBubbleOpts): ContactBubble {
  const shell = openBubbleShell({
    host: opts.host,
    className: "md-contact-bubble",
  });
  const { wrap } = shell;

  const list = document.createElement("ul");
  list.className = "md-contact-bubble-results";
  wrap.appendChild(list);

  let entries: ContactRow[] = [];
  let active = 0;
  let alive = true;
  let lastQuery = "";
  let pendingTimer: ReturnType<typeof setTimeout> | null = null;
  // Monotonic request id so a slow earlier fetch can't overwrite
  // the results from a later, faster one.
  let reqSeq = 0;

  const renderResults = (): void => {
    list.innerHTML = "";
    if (entries.length === 0) {
      // Hide the bubble entirely when there's nothing to commit;
      // an empty popover under the caret is just visual noise.
      // The host still keeps the bubble alive (so Enter / Esc are
      // routed) until the trigger range itself is gone.
      wrap.style.display = "none";
      return;
    }
    wrap.style.display = "";
    entries.forEach((row, i) => {
      const li = document.createElement("li");
      li.className = i === active ? "active" : "";
      // Primary line: the contact's display name (title / basename).
      // Secondary line: first email, shown only when distinct from
      // the primary so an email-only contact (no name in the source
      // CSV) doesn't render the same string twice. The host inserts
      // `row.label` as the link text on accept, so for email-only
      // contacts the link text is the email itself.
      const firstEmail = row.emails?.[0];
      const primary = document.createElement("span");
      primary.className = "md-contact-bubble-primary";
      primary.textContent = row.label;
      li.appendChild(primary);
      if (firstEmail && firstEmail !== row.label) {
        const secondary = document.createElement("span");
        secondary.className = "md-contact-bubble-secondary";
        secondary.textContent = firstEmail;
        li.appendChild(secondary);
      }
      li.addEventListener("mousedown", (ev) => {
        ev.preventDefault();
        active = i;
        opts.onClickAccept?.();
      });
      list.appendChild(li);
    });
    shell.reposition();
  };

  const fetchNow = async (q: string): Promise<void> => {
    const seq = ++reqSeq;
    try {
      const rows = await api.contacts(q, PAGE_LIMIT);
      if (!alive || seq !== reqSeq) return;
      entries = rows;
      active = 0;
      renderResults();
    } catch {
      // Silent: the picker just stays empty if /api/contacts fails
      // (e.g., transient disconnect). Surfacing the error inline
      // would be more noise than signal for a typeahead.
      if (!alive || seq !== reqSeq) return;
      entries = [];
      active = 0;
      renderResults();
    }
  };

  const filter = (q: string): void => {
    if (pendingTimer) clearTimeout(pendingTimer);
    pendingTimer = setTimeout(() => {
      pendingTimer = null;
      void fetchNow(q);
    }, DEBOUNCE_MS);
  };

  // Initial fetch (empty query -> alphabetical head, mirrors the
  // `[[` empty-state behavior).
  void fetchNow("");

  return {
    setQuery(q: string): void {
      if (!alive) return;
      if (q === lastQuery) return;
      lastQuery = q;
      filter(q);
    },
    moveActive(delta: number): void {
      if (!alive || entries.length === 0) return;
      active = Math.max(0, Math.min(entries.length - 1, active + delta));
      renderResults();
    },
    accept(): ContactRow | null {
      if (!alive || entries.length === 0) return null;
      return entries[active] ?? null;
    },
    dismiss(): void {
      if (!alive) return;
      alive = false;
      if (pendingTimer) clearTimeout(pendingTimer);
      shell.dismiss();
    },
    handleKey(event: KeyboardEvent): boolean {
      if (!alive) return false;
      switch (event.key) {
        case "Enter":
          opts.onCommit?.();
          return true;
        case "Escape":
          opts.onDismiss?.();
          return true;
        case "ArrowDown":
          this.moveActive(1);
          return true;
        case "ArrowUp":
          this.moveActive(-1);
          return true;
      }
      return false;
    },
  };
}
