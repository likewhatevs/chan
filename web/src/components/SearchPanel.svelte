<script lang="ts">
  // Content-search command palette. Open with Cmd/Ctrl+P (or the
  // toolbar button), type, see ranked hits across the drive,
  // click (or arrow-keys + Enter) to open the file at the right
  // section.
  //
  // Q&A used to live here as a second tab; it moved to the global
  // assistant overlay (Cmd/Ctrl+H) in v3 so all assistant flows
  // share one panel and one context picker. This file is search-
  // only now.

  import { onDestroy, onMount } from "svelte";
  import { api } from "../api/client";
  import type { ContentHit } from "../api/types";
  import { openInActivePane } from "../state/tabs.svelte";
  import { searchPanel } from "../state/store.svelte";
  import OverlayShell from "./OverlayShell.svelte";

  let inputEl: HTMLInputElement | undefined = $state();
  let query = $state("");
  let hits = $state<ContentHit[]>([]);
  let loading = $state(false);
  let active = $state(0);
  let error = $state<string | null>(null);

  /// Debounce token. Bumped on every input change; any in-flight
  /// promise that resolves with a stale token discards its result.
  let queryToken = 0;
  let debounceTimer: ReturnType<typeof setTimeout> | null = null;

  // Reset transient state when the panel reopens so a stale set
  // of hits from the previous session doesn't flash before the
  // first new query lands.
  $effect(() => {
    if (searchPanel.open) {
      query = "";
      hits = [];
      active = 0;
      error = null;
      queueMicrotask(() => inputEl?.focus());
    }
  });

  function close(): void {
    searchPanel.open = false;
  }

  function scheduleSearch(): void {
    if (debounceTimer) clearTimeout(debounceTimer);
    const q = query.trim();
    if (!q) {
      hits = [];
      loading = false;
      return;
    }
    queryToken += 1;
    const myToken = queryToken;
    loading = true;
    debounceTimer = setTimeout(async () => {
      try {
        const res = await api.searchContent(q, { limit: 25 });
        if (myToken !== queryToken) return; // stale
        hits = res.hits;
        active = 0;
        error = null;
      } catch (e) {
        if (myToken !== queryToken) return;
        error = (e as Error).message;
        hits = [];
      } finally {
        if (myToken === queryToken) loading = false;
      }
    }, 200);
  }

  async function pick(h: ContentHit): Promise<void> {
    close();
    await openInActivePane(h.path);
    // Note: scrolling to start_line happens via the editor's
    // existing scrollToHeading / scrollToLine paths once we wire
    // them up to a "jump to chunk" callback. For v1 we just open
    // the file; the user can find the section via the inspector's
    // outline view (toggle ≡).
  }

  function onKeyDown(e: KeyboardEvent): void {
    if (e.key === "Escape") {
      e.preventDefault();
      close();
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      active = Math.min(active + 1, Math.max(0, hits.length - 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      active = Math.max(active - 1, 0);
    } else if (e.key === "Enter") {
      const h = hits[active];
      if (h) {
        e.preventDefault();
        void pick(h);
      }
    }
  }

  /// Escape `<` / `>` / `&` in the BM25 snippet, then unescape the
  /// markup we trust (the `<b>...</b>` highlight pair tantivy emits)
  /// into <mark> so the active match stands out.
  function renderSnippet(s: string): string {
    const escaped = s
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;");
    return escaped
      .replace(/&lt;b&gt;/g, "<mark>")
      .replace(/&lt;\/b&gt;/g, "</mark>");
  }

  function onWindowKey(e: KeyboardEvent): void {
    // Cmd+P (mac) or Ctrl+P (others) toggles the panel from
    // anywhere, including when this panel is closed. Same letter
    // as VSCode's quick-open. We avoid Cmd+K because the editor
    // binds it to the readline-style "delete to end of line".
    // preventDefault stops the browser's print dialog.
    if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "p") {
      e.preventDefault();
      searchPanel.open = !searchPanel.open;
    }
  }
  onMount(() => document.addEventListener("keydown", onWindowKey));
  onDestroy(() => document.removeEventListener("keydown", onWindowKey));
</script>

<OverlayShell open={searchPanel.open} onClose={close}>
  <ul class="hits">
    {#each hits as h, i (h.path + h.chunk_id)}
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
      <li
        class:active={i === active}
        onmousedown={(e) => {
          e.preventDefault();
          void pick(h);
        }}
        onmouseenter={() => (active = i)}
      >
        <div class="row1">
          <span class="path">{h.path}</span>
          {#if h.heading}<span class="heading">· {h.heading}</span>{/if}
          <span class="score">{h.score.toFixed(4)}</span>
        </div>
        <div class="snippet">{@html renderSnippet(h.snippet)}</div>
      </li>
    {/each}
  </ul>
  <div class="status-line">
    {#if loading}
      <span>searching…</span>
    {:else if error}
      <span class="err">{error}</span>
    {:else if query.trim() && hits.length === 0}
      <span>no matches</span>
    {:else if hits.length > 0}
      <span>{hits.length} hit{hits.length === 1 ? "" : "s"}</span>
    {:else}
      <span class="muted">type to search</span>
    {/if}
  </div>
  <div class="head">
    <input
      bind:this={inputEl}
      bind:value={query}
      oninput={scheduleSearch}
      onkeydown={onKeyDown}
      placeholder="search content (Cmd+P)"
      spellcheck="false"
      autocomplete="off"
    />
  </div>
</OverlayShell>

<style>
  /* Input row anchored at the bottom of the OverlayShell; status
     and results stack above it. Top border (was bottom) so the
     seam reads as "input separated from the content above it". */
  .head {
    display: flex;
    gap: 6px;
    padding: 8px;
    border-top: 1px solid var(--border);
  }
  .head input {
    flex: 1;
    background: var(--bg);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 6px 8px;
    font: inherit;
    outline: none;
  }
  .head input:focus { border-color: var(--link); }
  .status-line {
    padding: 4px 10px;
    font-size: 13px;
    color: var(--text-secondary);
    border-top: 1px solid var(--border);
  }
  .status-line .err { color: #d33; }
  .status-line .muted { opacity: 0.7; }
  .hits {
    list-style: none;
    margin: 0;
    padding: 4px 0;
    overflow-y: auto;
    flex: 1;
    min-height: 0;
  }
  .hits li {
    padding: 6px 10px;
    cursor: pointer;
    border-left: 2px solid transparent;
  }
  .hits li.active {
    background: var(--hover-bg);
    border-left-color: var(--link);
  }
  .row1 {
    display: flex;
    gap: 6px;
    align-items: baseline;
    font-size: 14px;
  }
  .path { font-weight: 600; color: var(--text); }
  .heading { color: var(--text-secondary); }
  .score {
    margin-left: auto;
    color: var(--text-secondary);
    font-family: ui-monospace, monospace;
    font-size: 13px;
  }
  .snippet {
    margin-top: 2px;
    font-size: 14px;
    color: var(--text-secondary);
    line-height: 1.45;
    overflow: hidden;
    text-overflow: ellipsis;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
  }
  :global(.snippet mark) {
    background: var(--smart-bg);
    color: inherit;
    padding: 0 2px;
    border-radius: 2px;
  }
</style>
