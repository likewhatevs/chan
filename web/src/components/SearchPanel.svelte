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

  import { onDestroy } from "svelte";
  import { untrack } from "svelte";
  import { ArrowLeft, ArrowRight, Database, Maximize2, Minimize2, Settings, X } from "lucide-svelte";
  import {
    overlayMaximized,
    setOverlayMaximized,
  } from "../state/pageWidth.svelte";
  import { ApiError, api, withTokenQuery } from "../api/client";
  import type { ContentHit, ReportFileStats } from "../api/types";
  import { isEditableText, isImage } from "../state/fileTypes";
  import {
    ensureGraphLoaded,
    graphData,
  } from "../state/graphData.svelte";
  import { openInActivePane } from "../state/tabs.svelte";
  import {
    availableSearchScopes,
    browserOverlay,
    loadTreeDir,
    openBrowser,
    openGraphForTag,
    openSettings,
    paneWidths,
    persistPaneWidths,
    revealAndSelect,
    searchPanel,
    searchStatusOverlay,
    tree,
  } from "../state/store.svelte";
  import { type ScopeOption, defaultScopeId } from "../state/scope.svelte";
  import { chordFor } from "../state/shortcuts";
  import Bubble from "./Bubble.svelte";
  import HamburgerMenu from "./HamburgerMenu.svelte";
  import Inspector from "./Inspector.svelte";
  import InspectorBody, { type InspectorSelection } from "./InspectorBody.svelte";
  import KindChip from "./KindChip.svelte";
  import OverlayShell from "./OverlayShell.svelte";

  /// Unified row type. Chunk hits come from the server (BM25 +
  /// dense over file content); tag, image, file, and contact hits
  /// are computed client-side from the already-loaded graph and
  /// tree (filename / label substring matches). The discriminant
  /// on `kind` lets the row renderer pick the right chip + preview
  /// shape and lets the inspector dispatcher know what selection
  /// to construct.
  type SearchRow =
    | { kind: "chunk"; hit: ContentHit; key: string }
    | { kind: "image"; path: string; key: string }
    | { kind: "file"; path: string; key: string }
    | {
        kind: "language_file";
        path: string;
        language: string;
        code: number;
        key: string;
      }
    | { kind: "contact"; path: string; key: string }
    | {
        kind: "tag";
        nodeId: string;
        label: string;
        documents: number;
        key: string;
      };

  let inputEl: HTMLInputElement | undefined = $state();
  let hitsEl: HTMLUListElement | undefined = $state();
  /// Query is module-level (lives on `searchPanel.query`) so the
  /// URL hash can round-trip it: a copy-pasted chan URL with
  /// `search=foo` reopens the panel preloaded with that term.
  let chunkHits = $state<ContentHit[]>([]);
  let languageHits = $state<SearchRow[]>([]);
  let loading = $state(false);
  let active = $state(0);
  let error = $state<string | null>(null);

  /// Debounce token. Bumped on every input change; any in-flight
  /// promise that resolves with a stale token discards its result.
  let queryToken = 0;
  let debounceTimer: ReturnType<typeof setTimeout> | null = null;
  let hitsResizeObs: ResizeObserver | null = null;
  const reportCache = new Map<string, ReportFileStats | null>();

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
      languageHits = [];
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
      languageHits = [];
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
        const language = parseLanguageQuery(q);
        if (language) {
          chunkHits = [];
          const hits = await searchLanguage(language);
          if (myToken !== queryToken) return; // stale
          languageHits = hits;
          active = 0;
          error = null;
          return;
        }
        const res = await api.searchContent(q, { limit });
        if (myToken !== queryToken) return; // stale
        chunkHits = res.hits;
        languageHits = [];
        active = 0;
        error = null;
      } catch (e) {
        if (myToken !== queryToken) return;
        error = (e as Error).message;
        chunkHits = [];
        languageHits = [];
      } finally {
        if (myToken === queryToken) loading = false;
      }
    }, 200);
  }

  const LANGUAGE_LIMIT = 25;

  function parseLanguageQuery(q: string): string | null {
    const m = q.match(/^language\s*:\s*(.+)$/i);
    const value = m?.[1]?.trim();
    return value ? value.toLowerCase() : null;
  }

  async function reportForPath(path: string): Promise<ReportFileStats | null> {
    if (reportCache.has(path)) return reportCache.get(path) ?? null;
    try {
      const stats = await api.reportFile(path);
      reportCache.set(path, stats);
      return stats;
    } catch (e) {
      if (e instanceof ApiError && e.status === 404) {
        reportCache.set(path, null);
        return null;
      }
      throw e;
    }
  }

  async function searchLanguage(language: string): Promise<SearchRow[]> {
    await loadSearchTree();
    const files = tree.entries.filter((e) => !e.is_dir && pathInScope(e.path));
    const out: SearchRow[] = [];
    let next = 0;
    const workers = Array.from({ length: Math.min(8, files.length) }, async () => {
      while (next < files.length && out.length < LANGUAGE_LIMIT) {
        const entry = files[next++];
        if (!entry) continue;
        const stats = await reportForPath(entry.path);
        if (!stats || stats.language.toLowerCase() !== language) continue;
        out.push({
          kind: "language_file",
          path: entry.path,
          language: stats.language,
          code: stats.code,
          key: `language:${stats.language}:${entry.path}`,
        });
      }
    });
    await Promise.all(workers);
    out.sort((a, b) => {
      if (a.kind !== "language_file" || b.kind !== "language_file") return 0;
      return a.path.localeCompare(b.path);
    });
    return out.slice(0, LANGUAGE_LIMIT);
  }

  async function loadSearchTree(): Promise<void> {
    // The file tree is normally loaded on demand as folders open.
    // `language:<name>` is a drive-wide query, so hydrate folder
    // listings before scanning per-file report rows.
    for (let i = 0; i < 1000; i += 1) {
      const pending = tree.entries
        .filter((e) => e.is_dir && !tree.loadedDirs[e.path] && !tree.loadingDirs[e.path])
        .map((e) => e.path);
      if (pending.length === 0) return;
      await Promise.allSettled(pending.map((path) => loadTreeDir(path)));
    }
  }


  /// Per-kind caps so a query like "n" doesn't drown the chunk
  /// hits under hundreds of folder / image / file / contact rows.
  /// Chunks come from the server already capped at 25.
  const TAG_LIMIT = 8;
  const IMAGE_LIMIT = 8;
  const FILE_LIMIT = 8;
  const CONTACT_LIMIT = 8;

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

  /// Contact hits: tree entries the server tagged as `chan.kind:
  /// contact` whose path matches the typed query. Surfaced as a
  /// distinct row kind so a search for "alex" lands on the contact
  /// file in addition to any markdown that mentions them. Substring
  /// match across the full path (not just basename) so contacts
  /// nested in subfolders still surface.
  const contactRows = $derived.by<SearchRow[]>(() => {
    const q = searchPanel.query.trim().toLowerCase();
    if (!q) return [];
    const out: SearchRow[] = [];
    for (const e of tree.entries) {
      if (e.is_dir) continue;
      if (e.kind !== "contact") continue;
      if (!e.path.toLowerCase().includes(q)) continue;
      out.push({ kind: "contact", path: e.path, key: `contact:${e.path}` });
      if (out.length >= CONTACT_LIMIT) break;
    }
    return out;
  });

  /// Markdown / text file hits: filename matches separate from the
  /// BM25 content chunks above. A file that matches by NAME (the
  /// user typed part of its title) is a different signal from a
  /// file that matches by content, and surfacing both lets the user
  /// pick whichever they were after. Contact-kind files are
  /// excluded — they get their own row kind above. Files that also
  /// appear in chunk hits are deduped in the `rows` combiner so the
  /// same path doesn't render twice.
  const markdownFileRows = $derived.by<SearchRow[]>(() => {
    const q = searchPanel.query.trim().toLowerCase();
    if (!q) return [];
    const out: SearchRow[] = [];
    for (const e of tree.entries) {
      if (e.is_dir) continue;
      if (!isEditableText(e.path)) continue;
      if (e.kind === "contact") continue;
      if (!e.path.toLowerCase().includes(q)) continue;
      out.push({ kind: "file", path: e.path, key: `file:${e.path}` });
      if (out.length >= FILE_LIMIT) break;
    }
    return out;
  });

  /// Helper: format a contact entry's display name from its path.
  /// Strips the .md extension and leaves whatever folder structure
  /// the user has (e.g. `Contacts/Alex Park.md` -> `Alex Park`).
  /// Mirrors what FileInfoBody does for the contact name pill.
  function contactDisplayName(path: string): string {
    const base = basename(path);
    return base.replace(/\.(md|txt)$/i, "");
  }


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
  /// then contacts (people get top billing among file-name matches),
  /// images, markdown filename matches, and finally content chunks
  /// (the long tail). Each row has a stable key for the {#each}.
  /// The scope predicate filters file-bearing kinds by path; tags
  /// fall through unfiltered (a tag is a graph node, not a file,
  /// and the document-count column already gives the user a sense
  /// of breadth). Markdown filename matches are deduped against
  /// chunk hits so a file that surfaces under both shows once.
  const rows = $derived.by<SearchRow[]>(() => {
    const out: SearchRow[] = [];
    const chunkPaths = new Set<string>();
    for (const h of chunkHits) chunkPaths.add(h.path);
    out.push(...tagRows);
    out.push(...languageHits);
    for (const r of contactRows) {
      if (r.kind === "contact" && !pathInScope(r.path)) continue;
      out.push(r);
    }
    for (const r of imageRows) {
      if (r.kind === "image" && !pathInScope(r.path)) continue;
      out.push(r);
    }
    for (const r of markdownFileRows) {
      if (r.kind !== "file") continue;
      if (!pathInScope(r.path)) continue;
      if (chunkPaths.has(r.path)) continue;
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
    if (r.kind === "file") return { kind: "file", path: r.path };
    if (r.kind === "language_file") return { kind: "file", path: r.path };
    if (r.kind === "contact") return { kind: "file", path: r.path };
    return { kind: "tag", nodeId: r.nodeId, label: r.label };
  });
  const activeKey = $derived(rows[active]?.key ?? null);

  function scrollActiveIntoView(): void {
    if (!searchPanel.open || !hitsEl || !activeKey) return;
    requestAnimationFrame(() => {
      const el = hitsEl?.querySelector<HTMLElement>('li[data-active="true"]');
      el?.scrollIntoView({ block: "nearest" });
    });
  }

  $effect(() => {
    if (!hitsEl) return;
    hitsResizeObs?.disconnect();
    hitsResizeObs = new ResizeObserver(scrollActiveIntoView);
    hitsResizeObs.observe(hitsEl);
    return () => {
      hitsResizeObs?.disconnect();
      hitsResizeObs = null;
    };
  });

  $effect(() => {
    void activeKey;
    void searchPanel.open;
    scrollActiveIntoView();
  });

  onDestroy(() => {
    hitsResizeObs?.disconnect();
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
    } else if (r.kind === "file" || r.kind === "language_file" || r.kind === "contact") {
      // Filename / contact matches open the underlying file in the
      // editor exactly like a chunk hit would — the path IS the
      // payload, no chunk anchor to honour.
      close();
      await openInActivePane(r.path);
    } else if (r.kind === "image") {
      // Images can't be opened in the editor; treat the row click
      // as "select" only. The inspector pane already shows the
      // preview thanks to the active-row selection effect, so no
      // further action is needed here.
    } else {
      // Tag hits route to a tag-scoped graph (depth-hop
      // neighbourhood around the tag) rather than drive scope.
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

  function doToggleOverlayMaximized(): void {
    setOverlayMaximized(!overlayMaximized.on);
    menu?.close();
  }

  function doOpenSettings(): void {
    menu?.close();
    openSettings();
  }

  function openSearchStatus(): void {
    searchStatusOverlay.open = true;
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
        <button
          type="button"
          class="chrome-btn"
          onclick={doToggleOverlayMaximized}
          title={overlayMaximized.on ? "Restore size" : "Maximize"}
          aria-label={overlayMaximized.on ? "Restore size" : "Maximize"}
        >
          {#if overlayMaximized.on}
            <Minimize2 size={14} strokeWidth={1.75} aria-hidden="true" />
          {:else}
            <Maximize2 size={14} strokeWidth={1.75} aria-hidden="true" />
          {/if}
        </button>
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
        <button
          type="button"
          class="chrome-btn"
          onclick={openSearchStatus}
          title="Show search index status"
          aria-label="Show search index status"
        >
          <Database size={14} strokeWidth={1.75} aria-hidden="true" />
        </button>
        <HamburgerMenu
          bind:this={menu}
          bind:open={menuOpen}
          width={POPOVER_WIDTH}
          height={POPOVER_HEIGHT}
        >
          {@render menuItems()}
        </HamburgerMenu>
        <button
          type="button"
          class="chrome-btn close"
          onclick={close}
          title="Close"
          aria-label="Close"
        >
          <X size={14} strokeWidth={1.75} aria-hidden="true" />
        </button>
      </header>
      <ul class="hits" bind:this={hitsEl}>
        {#each rows as r, i (r.key)}
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
          <li
            data-active={i === active ? "true" : undefined}
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
                  <KindChip kind="document" compact />
                  <span class="path">{r.hit.path}</span>
                  {#if r.hit.heading}<span class="heading">· {r.hit.heading}</span>{/if}
                </div>
                <div class="snippet">{@html renderSnippet(r.hit.snippet)}</div>
              {:else if r.kind === "image"}
                <!-- Image-name match. Thumbnail floats to the right
                     of the path so the bubble stays the same height
                     as doc / file rows; the image loads through
                     `withTokenQuery` because the browser's `<img>`
                     can't carry an auth header. The capped height
                     keeps portrait + landscape both sitting cleanly
                     in the row without making the bubble taller
                     than the neighbouring kinds. -->
                <div class="row1 image-row">
                  <KindChip kind="media" compact />
                  <span class="path">{r.path}</span>
                  <span class="image-thumb">
                    <img
                      src={withTokenQuery(`/api/files/${encodeURIComponent(r.path).replace(/%2F/g, "/")}`)}
                      alt={basename(r.path)}
                      loading="lazy"
                    />
                  </span>
                </div>
              {:else if r.kind === "contact"}
                <!-- Contact-name match. Displays the contact's name
                     (basename with the .md/.txt suffix stripped) and
                     the underlying drive path below it. Same row
                     shape as image / file so the bubbles read alike
                     once preview content fills the body. -->
                <div class="row1">
                  <KindChip kind="contact" compact />
                  <span class="path">{contactDisplayName(r.path)}</span>
                </div>
                <div class="preview muted mono">{r.path}</div>
              {:else if r.kind === "language_file"}
                <div class="row1">
                  <KindChip kind="document" compact dim />
                  <span class="path">{r.path}</span>
                  <span class="score">{r.language}</span>
                </div>
                <div class="preview muted">{r.code.toLocaleString()} SLOC</div>
              {:else if r.kind === "file"}
                <!-- Markdown / text filename match (deduped against
                     chunk hits in the rows combiner). The preview
                     line carries the basename so the user can scan
                     filenames quickly when the path is long; the
                     full path stays in row1 for disambiguation. No
                     trailing metadata: the doc score below is a
                     relevance signal worth showing; bytes / mtime
                     aren't, and would just clutter the row. `dim`
                     marks this as the same kind as a content hit
                     but with less emphasis (filename match, not
                     content match). -->
                <div class="row1">
                  <KindChip kind="document" compact dim />
                  <span class="path">{r.path}</span>
                </div>
                <div class="preview muted">{basename(r.path)}</div>
              {:else}
                <div class="row1">
                  <KindChip kind="tag" compact />
                  <span class="path">{r.label}</span>
                  <span class="score">{r.documents} doc{r.documents === 1 ? "" : "s"}</span>
                </div>
              {/if}
            </Bubble>
          </li>
        {/each}
      </ul>
      <!-- Input row first, then the status line beneath it. The
           status reads as a footer hint anchored to the bottom of
           the panel ("type to search · ↵ open · ↑↓ select" /
           "12 hits (5 doc · 2 image · …)" / "no matches") so the
           cursor and the kbd hint live next to each other instead
           of with the results list above. -->
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
            {#if tagRows.length + imageRows.length + contactRows.length + markdownFileRows.length + languageHits.length > 0}
              ({chunkHits.length} doc · {imageRows.length} image · {tagRows.length} tag
              {#if contactRows.length > 0} · {contactRows.length} contact{/if}
              {#if markdownFileRows.length > 0} · {markdownFileRows.length} file{/if}
              {#if languageHits.length > 0} · {languageHits.length} language{/if})
            {/if}
          </span>
        {:else}
          <span class="muted">type to search · ↵ open · ↑↓ select</span>
        {/if}
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
  <!-- Section order matches the rest of the right-click menus:
       view toggles, (no content/navigation actions here — search is
       read-only), Settings footer. -->
  <li>
    <button role="menuitem" onclick={toggleInspector}>
      {#if searchPanel.inspectorOpen}
        <ArrowRight size={16} strokeWidth={1.75} aria-hidden="true" />
      {:else}
        <ArrowLeft size={16} strokeWidth={1.75} aria-hidden="true" />
      {/if}
      <span class="menu-row-label">
        {searchPanel.inspectorOpen ? "Hide Details" : "Show Details"}
      </span>
      <span class="menu-row-chord"></span>
    </button>
  </li>
  <li class="sep" role="separator"></li>
  <li>
    <button role="menuitem" onclick={doOpenSettings}>
      <Settings size={14} strokeWidth={1.75} aria-hidden="true" />
      <span class="menu-row-label">Settings</span>
      <span class="menu-row-chord">{chordFor("app.settings.toggle") ?? ""}</span>
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
     pins to the right edge, just before the close-button chrome. */
  header { gap: 0.5rem; }
  header :global(.hamburger-trigger) { margin-left: auto; }
  /* Window-manager chrome: maximize/restore lives at the far left
     of the header, close at the far right. Matches the
     scope-history button style so all overlay headers wear the
     same skin. */
  .chrome-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 26px;
    height: 24px;
    padding: 0;
    background: var(--bg);
    color: var(--text-secondary);
    border: 1px solid var(--border);
    border-radius: 4px;
    cursor: pointer;
    transition: color 0.15s ease, border-color 0.15s ease;
    flex-shrink: 0;
  }
  .chrome-btn:hover {
    color: var(--text);
    border-color: var(--btn-hover);
  }
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

  /* Generic preview lives below the row1 in image / file / contact
     rows; matches the chunk snippet's spacing rhythm so all bubbles
     have the same vertical cadence. `.muted` is the secondary-text
     variant used by file + contact rows; image rows replace the
     text preview with a thumbnail (see `.image-preview` below). */
  .preview {
    margin-top: 4px;
    font-size: 13px;
    color: var(--text);
    line-height: 1.4;
  }
  .preview.muted { color: var(--text-secondary); }
  .preview.mono { font-family: ui-monospace, monospace; font-size: 12px; }
  /* Image thumbnail floats to the right of the path so the bubble
     keeps the same one-line height as doc / file / contact rows.
     `margin-left: auto` pins it past the score column; the fixed
     height + object-fit:contain keeps portrait + landscape sitting
     cleanly inside the same vertical box. A subtle border + radius
     lets transparent PNGs / SVGs read as cards instead of bleeding
     into the bubble body. */
  .image-row { align-items: center; }
  .image-thumb {
    margin-left: auto;
    flex-shrink: 0;
    line-height: 0;
  }
  .image-thumb img {
    height: 36px;
    max-width: 80px;
    object-fit: contain;
    border-radius: 4px;
    border: 1px solid var(--border);
    background: var(--bg);
    display: block;
  }

  /* Body stretch: the Bubble component leaves the body at content
     width so right-aligned chat user bubbles stay tight. In the
     search list we want every body to fill the row so the panel
     reads as a uniform column of cards regardless of how much
     content the kind brings. The min-height matches a two-line
     row (row1 + preview/snippet) so single-line kinds (image,
     tag) sit at the same height as doc / file / contact and the
     scroll rhythm stays even. Center the row1 in single-line
     bodies so a lone path doesn't read as top-anchored against
     a taller doc neighbour. */
  .hits li :global(.bubble) {
    align-items: stretch;
  }
  .hits li :global(.bubble .body) {
    width: 100%;
    box-sizing: border-box;
    min-height: 58px;
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 2px;
  }
</style>
