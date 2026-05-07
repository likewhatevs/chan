<script lang="ts">
  // Bottom-right collapsible status bar for the file editor:
  // backlinks count, word count, character count, plus a r/w
  // toggle "lamp" (green when writable, grey when read-only). The
  // toggle hides the floating format toolbar and the editor caret
  // for the spec's "maximize for reading" mode.
  //
  // Backlinks come from /api/backlinks/{path}; counts are debounced
  // off the live `content` prop so heavy edits don't spam the
  // network or recompute on every keystroke.
  //
  // Collapse state is local to the component (no persistence): the
  // bar is small enough that re-expanding on each open is cheap,
  // and avoiding a serialized field keeps tab-state simple.

  import { onDestroy } from "svelte";
  import { api } from "../api/client";
  import { idle } from "../state/idle.svelte";

  let {
    path,
    content,
    readMode = $bindable(false),
  }: {
    /// Drive-relative file path. Used as the backlinks query key.
    path: string;
    /// Live editor markdown buffer. Word / character counts derive
    /// from this; recompute is debounced so typing stays cheap.
    content: string;
    /// Two-way: clicking the lamp flips this and the parent passes
    /// it on to Wysiwyg + uses it to hide the format toolbar.
    readMode?: boolean;
  } = $props();

  let collapsed = $state(false);
  let backlinkCount = $state<number | null>(null);

  // Word / character counts are derived from `content`. Words
  // strip leading/trailing whitespace and split on any run of
  // whitespace; an empty buffer counts as 0 (not 1, which a naive
  // split would yield).
  const words = $derived.by(() => {
    const t = content.trim();
    if (!t) return 0;
    return t.split(/\s+/).length;
  });
  const chars = $derived(content.length);

  /// Backlinks fetch. Debounced so a burst of edits coalesces into
  /// one query. Hits an existing chan-server endpoint.
  let pending: ReturnType<typeof setTimeout> | null = null;
  let lastFetched = "";
  function scheduleBacklinks(): void {
    if (pending) clearTimeout(pending);
    pending = setTimeout(() => {
      pending = null;
      const target = path;
      if (target === lastFetched) return;
      lastFetched = target;
      void api
        .backlinks(target)
        .then((edges) => {
          // Drop the result if the path changed under us (tab swap).
          if (path !== target) return;
          backlinkCount = Array.isArray(edges) ? edges.length : 0;
        })
        .catch(() => {
          if (path !== target) return;
          backlinkCount = null;
        });
    }, 600);
  }

  // Re-query when path changes (tab swap).
  $effect(() => {
    lastFetched = "";
    backlinkCount = null;
    scheduleBacklinks();
  });

  onDestroy(() => {
    if (pending) clearTimeout(pending);
  });

  function toggleCollapse(): void {
    collapsed = !collapsed;
  }
  function toggleReadMode(): void {
    readMode = !readMode;
  }
</script>

<div
  class="wiki-statusbar"
  class:collapsed
  class:read-mode={readMode}
  class:idle={idle.active}
>
  <button
    class="collapse"
    title={collapsed ? "show stats" : "hide stats"}
    onclick={toggleCollapse}
    onmousedown={(e) => e.preventDefault()}
  >{collapsed ? "‹" : "›"}</button>
  {#if !collapsed}
    <span class="stat" title="incoming links">
      {backlinkCount ?? "-"}
      <span class="lbl">backlinks</span>
    </span>
    <span class="sep">·</span>
    <span class="stat" title="words">
      {words}
      <span class="lbl">words</span>
    </span>
    <span class="sep">·</span>
    <span class="stat" title="characters">
      {chars}
      <span class="lbl">chars</span>
    </span>
    <span class="sep">·</span>
    <button
      class="lamp"
      class:on={!readMode}
      title={readMode ? "switch to write mode" : "switch to read-only"}
      onclick={toggleReadMode}
      onmousedown={(e) => e.preventDefault()}
    >
      <span class="dot"></span>
      <span class="lamp-lbl">{readMode ? "read" : "write"}</span>
    </button>
  {/if}
</div>

<style>
  /* Anchored to the bottom-right by the parent (.editor-tab is
     position:relative). Sits above editor content but below the
     bottom-pill (which lives at z=20+). */
  .wiki-statusbar {
    position: absolute;
    right: 12px;
    bottom: 8px;
    z-index: 5;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 6px 10px;
    background: color-mix(in srgb, var(--bg-elev) 88%, transparent);
    border: 1px solid var(--border);
    border-radius: 999px;
    box-shadow: 0 2px 6px rgba(0,0,0,.18);
    font-size: 12px;
    color: var(--muted);
    user-select: none;
    transition: opacity 200ms ease;
  }
  .wiki-statusbar.collapsed {
    padding: 6px;
    gap: 0;
  }
  /* Idle: fade out + drop pointer events so the status bar
     disappears alongside the floating fmt-bar and bottom-pill,
     keeping the writing surface clean while the user reads.
     Pinning isn't needed: hover over the surrounding canvas wakes
     the global tracker before the user can reach this bar. */
  .wiki-statusbar.idle {
    opacity: 0;
    pointer-events: none;
  }
  .wiki-statusbar.read-mode {
    /* Grey-out the whole bar in read mode so the lamp's grey state
       carries through to a cohesive "reading" appearance. */
    color: var(--muted);
    opacity: 0.85;
  }
  .stat {
    color: var(--text);
    font-variant-numeric: tabular-nums;
  }
  .stat .lbl {
    color: var(--muted);
    margin-left: 2px;
    font-weight: normal;
  }
  .sep { color: var(--border); }
  .collapse {
    background: transparent;
    border: 0;
    color: var(--muted);
    cursor: pointer;
    padding: 0 4px;
    font: inherit;
    line-height: 1;
  }
  .collapse:hover { color: var(--text); }
  .lamp {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    background: transparent;
    border: 0;
    color: var(--muted);
    cursor: pointer;
    padding: 0;
    font: inherit;
  }
  .lamp .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    /* Grey by default = read mode. */
    background: var(--muted);
    box-shadow: 0 0 0 1px color-mix(in srgb, var(--text) 12%, transparent);
  }
  .lamp.on .dot {
    /* Green = write mode. The slight glow makes the "lit" state
       read as active without being aggressive. */
    background: #2ea043;
    box-shadow: 0 0 4px rgba(46, 160, 67, .55);
  }
  .lamp-lbl { letter-spacing: 0.02em; }
</style>
