<script lang="ts">
  // Search palette. Open with Cmd/Ctrl+K (or the toolbar button),
  // type, see ranked hits across the drive: chunks (BM25 + dense
  // from /api/search/content) plus client-side tag and image hits
  // computed off the loaded graph and tree. Click to open the
  // inspector on the right; double-click / Enter routes to the
  // editor (or graph, for a tag).
  //
  // Q&A used to live here as a second tab; it moved to the global
  // assistant overlay (Cmd/Ctrl+H) in v3 so all assistant flows
  // share one panel and one context picker. This file is search-
  // only now.

  import { untrack } from "svelte";
  import { ArrowLeft, ArrowRight } from "lucide-svelte";
  import { api } from "../api/client";
  import type { ContentHit } from "../api/types";
  import { isImage } from "../state/fileTypes";
  import {
    ensureGraphLoaded,
    graphData,
  } from "../state/graphData.svelte";
  import { openInActivePane } from "../state/tabs.svelte";
  import {
    availableSearchScopes,
    browserOverlay,
    openBrowser,
    openGraphForTag,
    paneWidths,
    persistPaneWidths,
    revealAndSelect,
    searchPanel,
    tree,
  } from "../state/store.svelte";
  import { type ScopeOption, defaultScopeId } from "../state/scope.svelte";
  import { chordFor } from "../state/shortcuts";
  import Bubble from "./Bubble.svelte";
  import HamburgerMenu from "./HamburgerMenu.svelte";
  import Inspector from "./Inspector.svelte";
  import InspectorBody, { type InspectorSelection } from "./InspectorBody.svelte";
  import OverlayShell from "./OverlayShell.svelte";

  /// Unified row type. Chunk hits come from the server; tag and
  /// image hits are computed client-side from the already-loaded
  /// graph and tree. The discriminant on `kind` lets the row
  /// renderer pick the right chip and lets the inspector
  /// dispatcher know what selection to construct.
  type SearchRow =
    | { kind: "chunk"; hit: ContentHit; key: string }
    | { kind: "image"; path: string; key: string }
    | {
        kind: "tag";
        nodeId: string;
        label: string;
        documents: number;
        key: string;
      };

  let inputEl: HTMLInputElement | undefined = $state();
  /// Query is module-level (lives on `searchPanel.query`) so the
  /// URL hash can round-trip it: a copy-pasted chan URL with
  /// `search=foo` reopens the panel preloaded with that term.
  let chunkHits = $state<ContentHit[]>([]);
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
  //
  // Selection prefill: if the user had real text selected when they
  // hit the search shortcut, seed the input with it and run the
  // first search immediately. Capture the selection BEFORE focusing
  // the input — focus collapses the selection, so reading it inside
  // the queueMicrotask would always come back empty. Gate keeps the
  // prefill quiet for the common cases where the selection isn't
  // useful (long paragraph, multi-line, focus inside another input).
  $effect(() => {
    if (!searchPanel.open) return;
    // Only react to the open transition; tracking the query here
    // would fire on every keystroke and re-select the input,
    // making typing impossible. `untrack` reads the value without
    // subscribing.
    untrack(() => {
      // Three open paths converge here:
      //   (a) URL hash restore — searchPanel.query was set by
      //       applyOverlaysFromHash before the panel went open.
      //       Keep it as-is and run a search.
      //   (b) Text-selection seed — query was empty, the user
      //       had real text selected when they hit Cmd+K. Adopt
      //       it and run a search.
      //   (c) Fresh empty open — wait for the user to type.
      const restored = searchPanel.query.trim();
      const seed = restored ? null : extractSearchSeed();
      if (seed) searchPanel.query = seed;
      chunkHits = [];
      active = 0;
      error = null;
      // Make sure the graph is loaded so tag hits work on the
      // first query of the session.
      void ensureGraphLoaded();
      queueMicrotask(() => {
        inputEl?.focus();
        if (seed || restored) inputEl?.select();
      });
      if (restored || seed) scheduleSearch();
    });
  });

  /// Caps for the selection-prefill gate. ≤8 words because BM25
  /// loses ranking signal past that and a longer selection is
  /// almost always an accidental paragraph copy. 200 chars is a
  /// belt-and-braces safety net for the case where someone typed
  /// one giant word.
  const SEED_MAX_WORDS = 8;
  const SEED_MAX_CHARS = 200;

  function extractSearchSeed(): string | null {
    const sel = typeof window !== "undefined" ? window.getSelection() : null;
    const raw = sel?.toString() ?? "";
    const trimmed = raw.trim();
    if (trimmed === "") return null;
    if (trimmed.length > SEED_MAX_CHARS) return null;
    if (/[\r\n]/.test(trimmed)) return null;
    const words = trimmed.split(/\s+/).filter((w) => w.length > 0);
    if (words.length > SEED_MAX_WORDS) return null;
    const focused = typeof document !== "undefined" ? document.activeElement : null;
    if (focused) {
      const tag = focused.tagName;
      if (tag === "INPUT" || tag === "TEXTAREA") return null;
    }
    return trimmed;
  }

  function close(): void {
    searchPanel.open = false;
  }

  /// Scope picker shape — same dropdown contract as Graph and
  /// Assistant. Re-derives whenever the layout shifts so opening a
  /// new pane or switching the visible tab refreshes the options
  /// without reopening the panel.
  const scopeOptions = $derived<ScopeOption[]>(availableSearchScopes());
  const currentScope = $derived<ScopeOption | null>(
    scopeOptions.find((o) => o.id === searchPanel.scopeId) ?? null,
  );

  /// Pinned to "drive" for now: the selector UI is live (it'll be
  /// wired to actual scoped queries once /api/search/content takes
  /// a scope param), but every open snaps back to the whole drive
  /// so the result set stays predictable in the meantime.
  $effect(() => {
    if (!searchPanel.open) return;
    untrack(() => {
      searchPanel.scopeId = "drive";
    });
  });

  /// Predicate: does `path` belong to the active scope? Currently
  /// short-circuits to true while the search overlay is pinned to
  /// the whole drive (the selector UI is live but the filter is
  /// disabled until /api/search/content takes a scope param). The
  /// previous narrow-scope branches are kept below the early
  /// return for the moment so reactivating per-kind filtering is a
  /// one-line revert.
  function pathInScope(_path: string): boolean {
    return true;
    /*
    const s = currentScope;
    if (!s) return true;
    if (s.kind === "drive" || s.kind === "global") return true;
    if (s.kind === "file") return _path === s.path;
    if (s.kind === "dir") {
      if (!s.path) return true;
      return _path === s.path || _path.startsWith(`${s.path}/`);
    }
    if (s.kind === "git_repo") {
      if (!s.root) return true;
      return _path === s.root || _path.startsWith(`${s.root}/`);
    }
    if (s.kind === "group") {
      return s.paths.includes(_path);
    }
    return true;
    */
  }

  function scheduleSearch(): void {
    if (debounceTimer) clearTimeout(debounceTimer);
    const q = searchPanel.query.trim();
    if (!q) {
      chunkHits = [];
      loading = false;
      return;
    }
    // Search is pinned to the whole drive for now; the snappy
    // 25-row cap is the right size for drive-wide queries.
    const limit = 25;
    queryToken += 1;
    const myToken = queryToken;
    loading = true;
    debounceTimer = setTimeout(async () => {
      try {
        const res = await api.searchContent(q, { limit });
        if (myToken !== queryToken) return; // stale
        chunkHits = res.hits;
        active = 0;
        error = null;
      } catch (e) {
        if (myToken !== queryToken) return;
        error = (e as Error).message;
        chunkHits = [];
      } finally {
        if (myToken === queryToken) loading = false;
      }
    }, 200);
  }


  /// Per-kind caps so a query like "n" doesn't drown the chunk
  /// hits under hundreds of folder/image rows. Chunks come from
  /// the server already capped at 25.
  const TAG_LIMIT = 8;
  const IMAGE_LIMIT = 8;

  /// Image hits: filenames matching the typed query, drawn from
  /// the in-memory tree. Substring match is case-insensitive and
  /// hits anywhere in the path so "kitten" finds
  /// "trips/2024/kitten.jpg" as well as "kitten.jpg" at the root.
  const imageRows = $derived.by<SearchRow[]>(() => {
    const q = searchPanel.query.trim().toLowerCase();
    if (!q) return [];
    const out: SearchRow[] = [];
    for (const e of tree.entries) {
      if (e.is_dir) continue;
      if (!isImage(e.path)) continue;
      if (!e.path.toLowerCase().includes(q)) continue;
      out.push({ kind: "image", path: e.path, key: `image:${e.path}` });
      if (out.length >= IMAGE_LIMIT) break;
    }
    return out;
  });

  /// Tag hits: tag-kind nodes whose label contains the typed query.
  /// Doc counts are SCOPE-aware: when the scope narrows to a file /
  /// dir / repo / group, we count only edges whose source doc is in
  /// scope, and a tag with zero in-scope refs is dropped. Drive /
  /// global scopes count everything. Surfacing the count lets the
  /// user pick the more active tag when several near-matches share
  /// a prefix.
  const tagRows = $derived.by<SearchRow[]>(() => {
    const q = searchPanel.query.trim().toLowerCase();
    if (!q) return [];
    const view = graphData.view;
    if (!view) return [];
    // Map node id -> file path so we can scope-test the edge source.
    // The graph view emits file nodes with `kind: "file"` carrying a
    // path; tag / mention / date nodes don't.
    const filePath = new Map<string, string>();
    for (const n of view.nodes) {
      if (n.kind === "file" && n.path) filePath.set(n.id, n.path);
    }
    const docCounts = new Map<string, number>();
    for (const e of view.edges) {
      if (e.kind !== "tag") continue;
      const src = filePath.get(e.source);
      if (src && !pathInScope(src)) continue;
      docCounts.set(e.target, (docCounts.get(e.target) ?? 0) + 1);
    }
    const out: SearchRow[] = [];
    for (const n of view.nodes) {
      if (n.kind !== "tag") continue;
      if (!n.label.toLowerCase().includes(q)) continue;
      const refs = docCounts.get(n.id) ?? 0;
      // Drop tags that don't touch the current scope. drive / global
      // scopes leave every tag with a positive count so nothing is
      // hidden there.
      if (refs === 0) continue;
      out.push({
        kind: "tag",
        nodeId: n.id,
        label: n.label,
        documents: refs,
        key: `tag:${n.id}`,
      });
      if (out.length >= TAG_LIMIT) break;
    }
    // Most-referenced first so prominent tags rank above one-off
    // hashtags that happen to share a prefix.
    out.sort((a, b) =>
      a.kind === "tag" && b.kind === "tag" ? b.documents - a.documents : 0,
    );
    return out;
  });

  /// Final ordered row list: tags first (compact, high signal),
  /// images next, then content chunks (the long tail). Each row
  /// has a stable key for the {#each}. The scope predicate filters
  /// chunks + images by path; tags fall through unfiltered (a tag
  /// is a graph node, not a file, and the document-count column
  /// already gives the user a sense of breadth).
  const rows = $derived.by<SearchRow[]>(() => {
    const out: SearchRow[] = [];
    out.push(...tagRows);
    for (const r of imageRows) {
      if (r.kind === "image" && !pathInScope(r.path)) continue;
      out.push(r);
    }
    for (const h of chunkHits) {
      if (!pathInScope(h.path)) continue;
      out.push({ kind: "chunk", hit: h, key: `chunk:${h.path}#${h.chunk_id}` });
    }
    return out;
  });

  /// Selection driving the inspector. Recomputed from the active
  /// row index so arrow keys keep the inspector in sync without an
  /// explicit click.
  const selection = $derived.by<InspectorSelection>(() => {
    const r = rows[active];
    if (!r) return null;
    if (r.kind === "chunk") return { kind: "file", path: r.hit.path };
    if (r.kind === "image") return { kind: "file", path: r.path };
    return { kind: "tag", nodeId: r.nodeId, label: r.label };
  });

  /// Primary action for a row: open the underlying entity in the
  /// editor (file / image stays in the inspector, since images
  /// don't open in the editor and the inspector preview is the
  /// useful destination) or jump to the graph (tag). Always
  /// closes the search overlay.
  async function activate(r: SearchRow): Promise<void> {
    if (r.kind === "chunk") {
      close();
      await openInActivePane(r.hit.path);
    } else if (r.kind === "image") {
      // Images can't be opened in the editor; treat the row click
      // as "select" only. The inspector pane already shows the
      // preview thanks to the active-row selection effect, so no
      // further action is needed here.
    } else {
      // Tag hits route to a tag-scoped graph (depth-hop
      // neighbourhood around the tag) rather than drive scope. The
      // remaining SearchRow variants today are tag-only; if mention
      // / date kinds get added back, give them their own branch.
      close();
      openGraphForTag(r.nodeId, r.label);
    }
  }

  function selectRow(i: number): void {
    if (i < 0 || i >= rows.length) return;
    active = i;
    if (!searchPanel.inspectorOpen) searchPanel.inspectorOpen = true;
  }

  function onKeyDown(e: KeyboardEvent): void {
    if (e.key === "Escape") {
      e.preventDefault();
      close();
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      active = Math.min(active + 1, Math.max(0, rows.length - 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      active = Math.max(active - 1, 0);
    } else if (e.key === "Enter") {
      const r = rows[active];
      if (r) {
        e.preventDefault();
        void activate(r);
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

  function basename(path: string): string {
    const slash = path.lastIndexOf("/");
    return slash >= 0 ? path.slice(slash + 1) : path;
  }

  /// Hamburger menu state. Mirrors the file browser / graph
  /// overlays: a single (≡) on the left, plus the same items
  /// available via right-click anywhere in the search body.
  let menu: HamburgerMenu | undefined = $state();
  let menuOpen = $state(false);
  const POPOVER_HEIGHT = 80;
  const POPOVER_WIDTH = 240;

  function toggleInspector(): void {
    searchPanel.inspectorOpen = !searchPanel.inspectorOpen;
    menu?.close();
  }

  function onSearchContextMenu(e: MouseEvent): void {
    // Bail if the right-click landed on the input — let the browser
    // show its native context menu (paste, spell, etc.) there.
    const t = e.target as HTMLElement | null;
    if (t?.closest("input, textarea")) return;
    e.preventDefault();
    menu?.openAtCursor(e.clientX, e.clientY);
  }
</script>

<OverlayShell id="search" open={searchPanel.open} onClose={close}>
  <div class="search" oncontextmenu={onSearchContextMenu} role="presentation">
    <div class="results">
      <header>
        <span class="title">Scope</span>
        <select
          class="scope-select"
          value={searchPanel.scopeId}
          onchange={(e) =>
            (searchPanel.scopeId = (e.currentTarget as HTMLSelectElement).value)}
          title="search scope"
        >
          {#each scopeOptions as opt (opt.id)}
            <option value={opt.id} disabled={opt.enabled === false}>
              {opt.label}
            </option>
          {/each}
        </select>
        <HamburgerMenu
          bind:this={menu}
          bind:open={menuOpen}
          width={POPOVER_WIDTH}
          height={POPOVER_HEIGHT}
        >
          {@render menuItems()}
        </HamburgerMenu>
      </header>
      <ul class="hits">
        {#each rows as r, i (r.key)}
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
          <li
            onmousedown={(e) => {
              e.preventDefault();
              selectRow(i);
            }}
            ondblclick={() => void activate(r)}
            onmouseenter={() => (active = i)}
          >
            <Bubble active={i === active}>
              {#if r.kind === "chunk"}
                <div class="row1">
                  <span class="kind-pill doc">doc</span>
                  <span class="path">{r.hit.path}</span>
                  {#if r.hit.heading}<span class="heading">· {r.hit.heading}</span>{/if}
                  <span class="score">{r.hit.score.toFixed(4)}</span>
                </div>
                <div class="snippet">{@html renderSnippet(r.hit.snippet)}</div>
              {:else if r.kind === "image"}
                <div class="row1">
                  <span class="kind-pill img">image</span>
                  <span class="path">{r.path}</span>
                </div>
                <div class="snippet muted">{basename(r.path)}</div>
              {:else}
                <div class="row1">
                  <span class="kind-pill tag">tag</span>
                  <span class="path">{r.label}</span>
                  <span class="score">{r.documents} doc{r.documents === 1 ? "" : "s"}</span>
                </div>
              {/if}
            </Bubble>
          </li>
        {/each}
      </ul>
      <div class="status-line">
        {#if loading}
          <span>searching…</span>
        {:else if error}
          <span class="err">{error}</span>
        {:else if searchPanel.query.trim() && rows.length === 0}
          <span>no matches</span>
        {:else if rows.length > 0}
          <span>
            {rows.length} hit{rows.length === 1 ? "" : "s"}
            {#if tagRows.length > 0 || imageRows.length > 0}
              ({chunkHits.length} doc · {imageRows.length} image · {tagRows.length} tag)
            {/if}
          </span>
        {:else}
          <span class="muted">type to search · ↵ open · ↑↓ select</span>
        {/if}
      </div>
      <div class="head">
        <input
          bind:this={inputEl}
          bind:value={searchPanel.query}
          oninput={scheduleSearch}
          onkeydown={onKeyDown}
          placeholder={`search content, tags, images${chordFor("app.search.toggle") ? " (" + chordFor("app.search.toggle") + ")" : ""}`}
          spellcheck="false"
          autocomplete="off"
        />
      </div>
    </div>
    {#if searchPanel.inspectorOpen}
      <Inspector
        title="Details"
        bind:width={paneWidths.search}
        onResize={persistPaneWidths}
        onClose={() => (searchPanel.inspectorOpen = false)}
      >
        <InspectorBody
          selection={selection}
          onClose={() => (searchPanel.inspectorOpen = false)}
          onNavigate={(p) => {
            close();
            void openInActivePane(p);
          }}
          onOpen={() => {
            const sel = selection;
            if (sel?.kind === "file") {
              close();
              void openInActivePane(sel.path);
            }
          }}
          onReveal={() => {
            const sel = selection;
            if (sel?.kind === "file") {
              revealAndSelect(sel.path);
              openBrowser();
              browserOverlay.inspectorOpen = true;
              close();
            }
          }}
          onSetAsScope={() => {
            const sel = selection;
            if (sel?.kind === "tag") {
              close();
              openGraphForTag(sel.nodeId, sel.label);
            }
            // Mention "Set as Scope" stays unwired here — the
            // search panel doesn't surface mention rows yet, so
            // resolving a contact label would never fire.
          }}
        />
      </Inspector>
    {/if}
  </div>
</OverlayShell>

{#snippet menuItems()}
  <!-- Search is read-only by design — only the Details toggle goes
       in the menu. Keeping the slot present (vs. a bare button) for
       parity with the file browser / graph menus. -->
  <li>
    <button role="menuitem" onclick={toggleInspector}>
      {#if searchPanel.inspectorOpen}
        <ArrowRight size={16} strokeWidth={1.75} aria-hidden="true" />
      {:else}
        <ArrowLeft size={16} strokeWidth={1.75} aria-hidden="true" />
      {/if}
      <span>{searchPanel.inspectorOpen ? "Hide Details" : "Show Details"}</span>
    </button>
  </li>
{/snippet}

<style>
  .search {
    display: flex;
    flex: 1;
    min-height: 0;
    min-width: 0;
  }
  .results {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
    min-height: 0;
  }
  /* Top bar mirrors the file-browser header. Title left, action
     buttons right; inspector toggle lives here so it sits next to
     the close-overlay chrome and is one click away regardless of
     where the user is in the result list. */
  header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.4rem 0.6rem;
    border-bottom: 1px solid var(--border);
    background: var(--bg-card);
    font-weight: 600;
    font-size: 15px;
    color: var(--text-heading);
    flex-shrink: 0;
  }
  header .title {
    flex-shrink: 0;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    font-size: 13px;
    font-weight: 600;
    color: var(--text-secondary);
  }
  /* SCOPE label + select sit on the left; the hamburger gets
     margin-left: auto via the wrapping :global() rule below so it
     pins to the right edge. */
  header { gap: 0.5rem; }
  header :global(.hamburger-trigger) { margin-left: auto; }
  /* Input row anchored at the bottom of the results column; status
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
  header .scope-select {
    background: var(--bg);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 4px 6px;
    font: inherit;
    font-size: 13px;
    max-width: 220px;
    cursor: pointer;
    flex-shrink: 1;
    min-width: 0;
  }
  header .scope-select:focus { outline: none; border-color: var(--link); }
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
    padding: 6px 10px;
    overflow-y: auto;
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .hits li {
    cursor: pointer;
    display: flex;
    /* Stretch the bubble's flex-start alignment to fill the row so
       result cards extend across the panel width (the 85% cap on
       the bubble component is for chat use). */
    align-self: stretch;
  }
  .hits li :global(.bubble) {
    max-width: none;
    width: 100%;
  }
  .row1 {
    display: flex;
    gap: 6px;
    align-items: baseline;
    font-size: 14px;
  }
  .path { font-weight: 600; color: var(--text); min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .heading { color: var(--text-secondary); }
  .score {
    margin-left: auto;
    color: var(--text-secondary);
    font-family: ui-monospace, monospace;
    font-size: 13px;
  }
  /* Per-kind chip. Width fixed so doc / image / tag align in a
     vertical column even though their text width differs. Mirrors
     the graph palette so search and graph speak the same visual
     language. */
  .kind-pill {
    display: inline-block;
    width: 44px;
    text-align: center;
    color: #fff;
    text-transform: uppercase;
    font-size: 10.5px;
    font-weight: 600;
    letter-spacing: 0.04em;
    padding: 1px 0;
    border-radius: 3px;
    flex-shrink: 0;
  }
  .kind-pill.doc { background: var(--g-doc); }
  .kind-pill.img { background: var(--g-img); }
  .kind-pill.tag { background: var(--g-tag); }
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
  .snippet.muted { font-style: italic; }
  :global(.snippet mark) {
    background: var(--smart-bg);
    color: inherit;
    padding: 0 2px;
    border-radius: 2px;
  }
</style>
