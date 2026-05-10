// `#tag` autocomplete bubble.
//
// Trigger: caret sits at the end of a `#word` token preceded by
// whitespace or block start, in a textblock that isn't a heading
// or code block. The bubble shows the top 5 existing tags from
// the drive's graph (file -> tag edges) filtered by the typed
// substring. Accepting replaces `#typed` with `#chosen`.
//
// The bubble does NOT take focus (same pattern as the wiki bubble);
// the host (Wysiwyg.svelte) routes Enter / Escape / Arrow keys
// while the trigger range is active.

import { positionPopover, watchViewport } from "./popover";
import { ensureGraphLoaded, graphData } from "../../state/graphData.svelte";

export interface TagBubble {
  /// Update the typed query and re-render. Empty query shows the
  /// catalog's first 5 tags.
  setQuery(query: string): void;
  /// Move the active row by +1 / -1, clamped to bounds.
  moveActive(delta: number): void;
  /// Resolve the highlighted tag name (without leading `#`), or
  /// null when there are no results.
  accept(): string | null;
  /// Tear down DOM + listeners. Idempotent.
  dismiss(): void;
}

export interface TagBubbleOpts {
  /// Anchor for positioning. Pass the caret-anchor shim so the
  /// bubble sits under the cursor.
  host: HTMLElement;
  /// Click-to-commit; the host commits the same way it would on
  /// Enter (calls accept() and replaces the trigger range).
  onClickAccept?: () => void;
}

export function openTagBubble(opts: TagBubbleOpts): TagBubble {
  const wrap = document.createElement("div");
  wrap.className = "md-tag-bubble";
  wrap.style.position = "absolute";
  // Match the wiki / image picker so the bubble floats above any
  // overlay that sits at 25000.
  wrap.style.zIndex = "30000";

  const list = document.createElement("ul");
  list.className = "md-tag-bubble-results";
  wrap.appendChild(list);

  document.body.appendChild(wrap);
  positionPopover(opts.host, wrap);
  const stopWatch = watchViewport(opts.host, wrap);

  let entries: string[] = [];
  let active = 0;
  let alive = true;
  let allTags: string[] = [];
  let lastQuery = "";

  const renderResults = (): void => {
    list.innerHTML = "";
    if (entries.length === 0) {
      // Hide the bubble entirely when there's nothing to commit;
      // an empty popover under the caret is just visual noise.
      wrap.style.display = "none";
      return;
    }
    wrap.style.display = "";
    entries.forEach((name, i) => {
      const li = document.createElement("li");
      li.textContent = `#${name}`;
      li.className = i === active ? "active" : "";
      li.addEventListener("mousedown", (ev) => {
        ev.preventDefault();
        active = i;
        opts.onClickAccept?.();
      });
      list.appendChild(li);
    });
    if (wrap.isConnected) positionPopover(opts.host, wrap);
  };

  const filter = (q: string): void => {
    const needle = q.toLowerCase();
    entries = needle
      ? allTags.filter((t) => t.toLowerCase().includes(needle)).slice(0, 5)
      : allTags.slice(0, 5);
    active = 0;
    renderResults();
  };

  // Pull tags from the cached graph view. ensureGraphLoaded is
  // idempotent: a hot cache resolves immediately, the first call
  // per session pays one round-trip. The graph is invalidated on
  // watcher events, so a freshly-saved tag shows up the next time
  // the bubble opens without manual reload.
  void ensureGraphLoaded().then(() => {
    if (!alive) return;
    const view = graphData.view;
    if (!view) return;
    // chan-server emits tag node labels prefixed with `#` (graph
     // node ids match: `#recipe`, `#chicken`, ...). Strip it here
     // so callers always work with the bare tag name; the picker UI
     // and the editor's accept path each prepend their own `#`.
    allTags = view.nodes
      .filter((n) => n.kind === "tag")
      .map((n) => (n.label as string).replace(/^#/, ""))
      .sort((a, b) => a.localeCompare(b));
    filter(lastQuery);
  });

  // Initial paint (likely empty until the graph resolves).
  renderResults();

  return {
    setQuery(q: string): void {
      if (!alive) return;
      lastQuery = q;
      filter(q);
    },
    moveActive(delta: number): void {
      if (!alive || entries.length === 0) return;
      active = Math.max(0, Math.min(entries.length - 1, active + delta));
      renderResults();
    },
    accept(): string | null {
      if (!alive || entries.length === 0) return null;
      return entries[active] ?? null;
    },
    dismiss(): void {
      if (!alive) return;
      alive = false;
      stopWatch();
      wrap.remove();
    },
  };
}
