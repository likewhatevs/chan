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
// host routes Enter / Escape / Arrow keys while it's open. Layout
// reuses `positionPopover` + `watchViewport` so the popover
// floats correctly under the caret across scrolls and reflows.

import { positionPopover, watchViewport } from "./popover";
import { api } from "../../api/client";

export type ContactRow = { path: string; label: string; emails?: string[] };

export interface ContactBubble {
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
}

/// Result cap. Picker is keyboard-driven; 8 rows fits a typical
/// viewport without scrolling and keeps the round-trip small.
const PAGE_LIMIT = 8;

/// Debounce window for the per-keystroke fetch. Trades a bit of
/// freshness for fewer HTTP round-trips when the user types fast;
/// 60ms is below typical typing-pause perception.
const DEBOUNCE_MS = 60;

export function openContactBubble(opts: ContactBubbleOpts): ContactBubble {
  const wrap = document.createElement("div");
  wrap.className = "md-contact-bubble";
  wrap.style.position = "absolute";
  // Match the wiki / tag picker so the bubble floats above any
  // overlay that sits at 25000.
  wrap.style.zIndex = "30000";

  const list = document.createElement("ul");
  list.className = "md-contact-bubble-results";
  wrap.appendChild(list);

  document.body.appendChild(wrap);
  positionPopover(opts.host, wrap);
  const stopWatch = watchViewport(opts.host, wrap);

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
      // Primary line: contact's display name (or the basename
      // fallback when the import lost it). Secondary line: first
      // email so the user can disambiguate Alice (work) from Alice
      // (home) without committing the wrong wiki-link. The picker
      // stays keyboard-only so we wrap both in spans the host can
      // style independently.
      const primary = document.createElement("span");
      primary.className = "md-contact-bubble-primary";
      primary.textContent = row.label;
      li.appendChild(primary);
      const firstEmail = row.emails?.[0];
      if (firstEmail) {
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
    if (wrap.isConnected) positionPopover(opts.host, wrap);
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
      stopWatch();
      wrap.remove();
    },
  };
}
