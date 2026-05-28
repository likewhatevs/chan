<script lang="ts">
  // Empty-pane visual-experimentation surface (fullstack-35).
  //
  // The carousel sits inside the Dashboard tab. It auto-rotates
  // every 5 s. Pointer hover and focus-within pause auto-rotate so
  // the user can read whichever slide they happen to be looking at;
  // the timer resumes when both signals clear. Left / right arrow
  // keys nudge manually when the carousel container has focus.
  //
  // Round-1 closing-2 (lane-b-empty-pane-menu): the legacy
  // `oncontextmenu` forwarder prop was removed. The carousel is
  // hosted inside the Dashboard tab (per `fullstack-a-75b`) and
  // DashboardTab does NOT wire a right-click handler through it;
  // right-clicks fall through to the tab strip's own context menu.
  //
  // Phase-13 slice 3b-1: slides 0 + 1 retooled into the new
  // Dashboard widget set. Slide 0 is an About widget (version,
  // attributions, donation QR, project links). Slide 1 mounts
  // `WorkspaceInfoBody` so the workspace-root inspector lives
  // alongside the file-browser inspector surface.
  //
  // Phase-13 slice 3b-2: slide 2 (the search/indexing graph) was
  // a custom radial-tree SVG. It is now a read-only mount of
  // `GraphCanvas`, the same renderer the main Graph tab uses,
  // fed a synthesized directory-only spine + per-directory
  // `indexState` for the green/grey/pulsing-orange palette. The
  // depth slider, inspector, filter chips and scope picker stay
  // out — this surface is purely a status read-out.

  import { onDestroy, onMount } from "svelte";
  import { api } from "../api/client";
  import { withTokenQuery } from "../api/transport";
  import type {
    BuildInfo,
    GraphViewEdge,
    GraphViewNode,
    IndexingStateResponse,
  } from "../api/types";
  import {
    workspace,
    indexStatus,
    ui,
    openGraphForContact,
    openGraphForLanguage,
  } from "../state/store.svelte";
  import GraphCanvas from "./GraphCanvas.svelte";
  import WorkspaceInfoBody from "./WorkspaceInfoBody.svelte";
  import {
    ChevronLeft,
    ChevronRight,
    Code2,
    Globe,
    Pause,
    Play,
  } from "lucide-svelte";

  /// `GraphCanvas` narrows folder/file/tag/mention/language nodes
  /// out of the broader `GraphViewNode` union. The Dashboard
  /// indexing slide only emits directory (folder) nodes, but the
  /// arrays still need to satisfy the wider canvas prop shape.
  type CanvasNode = Extract<
    GraphViewNode,
    { kind: "file" | "tag" | "mention" | "language" | "folder" }
  >;
  type CanvasEdgeKind =
    | "link"
    | "tag"
    | "mention"
    | "contains"
    | "language"
    | "group"
    | "drafts_link";
  type CanvasEdge = GraphViewEdge & { kind: CanvasEdgeKind };


  // ---- About slide (slice 3b-1) ------------------------------------------
  //
  // Sole home for the version / embeddings flag / attribution surface
  // now that slice 3c retired the global Settings overlay. The shape
  // mirrors the retired overlay's `<section class="about">` block
  // verbatim so muscle memory survives the move.

  let buildInfo = $state<BuildInfo | null>(null);

  async function loadBuildInfo(): Promise<void> {
    try {
      buildInfo = await api.buildInfo();
    } catch {
      buildInfo = null;
    }
  }

  onMount(() => {
    void loadBuildInfo();
  });

  // ---- shared index status label (slide 2 stub copy) -------------------------

  const indexLabel = $derived.by<string | null>(() => {
    const s = indexStatus.value;
    if (!s) return null;
    if (s.state === "building") {
      if (s.total > 0) return `indexing ${s.current}/${s.total}`;
      return "indexing…";
    }
    if (s.state === "reindexing") return "reindexing…";
    if (s.state === "error") return "index error";
    return null;
  });

  // ---- slide 2 — indexing graph (read-only spine) ------------------------

  /// Indexing state response cache. Re-fetched whenever slide 2
  /// becomes active and again every 3 s while it stays active so
  /// orange (in-flight) nodes can flip to green as the indexer
  /// makes progress. Polling stops the moment slide 2 hides; the
  /// effect cleanup clears the timer.
  let indexing = $state<IndexingStateResponse | null>(null);
  let indexingError = $state<string | null>(null);
  let indexingLoading = $state(false);

  async function refreshIndexing(): Promise<void> {
    indexingLoading = true;
    try {
      indexing = await api.indexingState();
      indexingError = null;
    } catch (e) {
      indexingError = (e as Error).message;
    } finally {
      indexingLoading = false;
    }
  }

  function basename(path: string): string {
    if (path === "") return "/";
    const slash = path.lastIndexOf("/");
    return slash < 0 ? path : path.slice(slash + 1);
  }

  function parentPath(path: string): string {
    const slash = path.lastIndexOf("/");
    if (slash <= 0) return "";
    return path.slice(0, slash);
  }

  /// Folder-node id matching chan-server's `directory_node_id`
  /// convention: workspace root is the empty string, every other
  /// directory uses `directory:<workspace-relative path>`. This
  /// has to match exactly so GraphCanvas's `workspace`-kind
  /// classification (id === "") fires on the root.
  function directoryId(path: string): string {
    return path === "" ? "" : `directory:${path}`;
  }

  /// Synthesize the read-only directory spine for GraphCanvas. We
  /// emit one `folder` node per indexed directory (plus the
  /// workspace root sentinel) and one `contains` edge per
  /// parent → child relationship. The `indexState` field on each
  /// folder node drives the green/grey/pulsing-orange palette
  /// inside GraphCanvas; the main Graph-tab view leaves
  /// `indexState` unset and falls back to the standard folder
  /// fill, so this surface owns the colour override.
  const indexingGraph = $derived.by<{
    nodes: CanvasNode[];
    edges: CanvasEdge[];
  }>(() => {
    const data = indexing;
    if (!data) return { nodes: [], edges: [] };
    const nodes: CanvasNode[] = [];
    const known = new Set<string>();
    for (const n of data.nodes) known.add(n.path);
    for (const n of data.nodes) {
      nodes.push({
        kind: "folder",
        id: directoryId(n.path),
        label: basename(n.path),
        path: n.path,
        files: 0,
        code: 0,
        indexState: n.state,
      });
    }
    const edges: CanvasEdge[] = [];
    for (const n of data.nodes) {
      if (n.path === "") continue;
      const parent = parentPath(n.path);
      if (!known.has(parent)) continue;
      edges.push({
        source: directoryId(parent),
        target: directoryId(n.path),
        kind: "contains",
      });
    }
    return { nodes, edges };
  });

  /// `visibleNodeIds` mirrors the full synthesized set: this
  /// surface is depth-max + spine-only, there's no filter chip /
  /// scope picker to thin it. Same for `visibleEdges`.
  const indexingNodeIds = $derived.by<Set<string>>(() => {
    const out = new Set<string>();
    for (const n of indexingGraph.nodes) out.add(n.id);
    return out;
  });

  /// Workspace root is the focal anchor so the spine grows
  /// upward from it (GraphCanvas's `hierarchyY` + `parentX`
  /// forces lay the depth tiers vertically with focal pinning
  /// at origin).
  const indexingFocal = ["" as string];

  /// B12: clicks toggle the selected node so GraphCanvas surfaces
  /// the clicked node's label plus its 1-hop neighbours (siblings
  /// + parent + children), matching the main Graph tab's
  /// selection-labeling rule. The dashboard surface still has no
  /// inspector — selection is purely a labeling cue.
  let selectedIndexId = $state<string | null>(null);
  function onIndexingSelect(id: string | null): void {
    selectedIndexId = id;
  }

  // ---- carousel state ----------------------------------------------------

  const slideCount = 3;
  let slideIndex = $state(0);
  let hovering = $state(false);
  let focused = $state(false);
  /// `cycling` is the explicit, persisted preference from
  /// `fullstack-44`. `hovering` / `focused` form the transient
  /// pause that lets users finish reading a slide; both axes
  /// independently suppress the timer. Server-default true so
  /// `undefined` (older servers without the field) reads as
  /// "auto-rotate on".
  const cycling = $derived<boolean>(
    workspace.info?.preferences?.empty_pane_carousel_cycling ?? true,
  );
  const paused = $derived(hovering || focused || !cycling);
  let containerEl: HTMLDivElement | undefined = $state();

  /// Auto-rotate while neither hovered nor focused AND the user
  /// hasn't explicitly stopped the cycle. Reset the interval on
  /// every dependency change (slideIndex bumped by keyboard
  /// nudges, paused toggled by hover/focus/cycling) so manual nav
  /// doesn't lose the next-tick budget.
  $effect(() => {
    void slideIndex;
    if (paused) return;
    const handle = window.setInterval(() => {
      slideIndex = (slideIndex + 1) % slideCount;
    }, 5000);
    return () => window.clearInterval(handle);
  });

  async function toggleCycling(): Promise<void> {
    const next = !cycling;
    if (workspace.info) {
      workspace.info = {
        ...workspace.info,
        preferences: {
          ...workspace.info.preferences,
          empty_pane_carousel_cycling: next,
        },
      };
    }
    try {
      await api.setEmptyPaneCarouselCycling(next);
    } catch (err) {
      ui.status = `carousel toggle failed: ${(err as Error).message}`;
    }
  }

  function prev(): void {
    slideIndex = (slideIndex - 1 + slideCount) % slideCount;
  }
  function next(): void {
    slideIndex = (slideIndex + 1) % slideCount;
  }
  function goTo(i: number): void {
    slideIndex = ((i % slideCount) + slideCount) % slideCount;
  }
  function onKeyDown(e: KeyboardEvent): void {
    if (e.key === "ArrowLeft") {
      e.preventDefault();
      prev();
    } else if (e.key === "ArrowRight") {
      e.preventDefault();
      next();
    }
  }

  /// Re-fetch indexing state when slide 3 becomes active, and
  /// poll every 3 s while it stays visible so orange (in-flight)
  /// nodes can flip to green as the indexer makes progress. The
  /// cleanup clears the timer on slide change / unmount so we
  /// never hammer the server in the background.
  $effect(() => {
    if (slideIndex !== 2) return;
    void refreshIndexing();
    const handle = window.setInterval(() => {
      void refreshIndexing();
    }, 3000);
    return () => window.clearInterval(handle);
  });

  onDestroy(() => {
    // The $effect cleanup already clears the interval, but if the
    // component is unmounted mid-tick we make sure nothing
    // references a stale element.
    containerEl = undefined;
  });
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<div
  class="carousel"
  bind:this={containerEl}
  role="region"
  aria-label="empty pane carousel"
  aria-roledescription="carousel"
  tabindex="0"
  onmouseenter={() => (hovering = true)}
  onmouseleave={() => (hovering = false)}
  onfocusin={() => (focused = true)}
  onfocusout={() => (focused = false)}
  onkeydown={onKeyDown}
>
  <div class="slide-stage">
    {#if slideIndex === 0}
      <!-- Phase-13 slice 3b-1: slide 0 is the About widget. It
           mirrors the (retired) global Settings overlay's about
           section verbatim and adds a "Fund the work" CTA with the
           donation QR + the website / source links so the user has
           one self-contained surface to learn what chan is and how
           to support it. Slice 3c retired the overlay; the About
           widget here is the sole home for this surface now. -->
      <div class="slide slide-about" aria-label="About">
        <div class="slide-title">About</div>
        <div class="about-grid">
          <span class="k">chan version</span>
          <span class="v mono">{buildInfo?.version ?? "n/a"}</span>
          <span class="k">embeddings</span>
          <span class="v">
            {#if buildInfo === null}
              n/a
            {:else if buildInfo.features.embeddings}
              <span class="ok">on</span>
              <span class="muted">(hybrid search available)</span>
            {:else}
              <span class="muted">off (BM25 only)</span>
            {/if}
          </span>
          <!-- Source Code Pro attribution. Ships with chan under
               the SIL Open Font License 1.1; the OFL notice is at
               /static/fonts/OFL.txt next to the .woff2. -->
          <span class="k">terminal font</span>
          <span class="v">
            Source Code Pro Regular
            <span class="muted">
              (<a href="/static/fonts/OFL.txt" target="_blank" rel="noopener">SIL OFL 1.1</a>)
            </span>
          </span>
          <!-- Matrix screen-lock attribution. The renderer and
               font assets are adapted from the MIT-licensed
               dcragusa/MatrixScreensaver project; the license
               notice ships in the embedded app bundle. -->
          <span class="k">matrix screen lock</span>
          <span class="v">
            <a href="https://github.com/dcragusa/MatrixScreensaver" target="_blank" rel="noopener">dcragusa/MatrixScreensaver</a>
            <span class="muted">
              (<a href="/static/matrix/LICENSE-MatrixScreensaver.txt" target="_blank" rel="noopener">MIT</a>)
            </span>
          </span>
        </div>

        <div class="about-links">
          <a
            class="about-link"
            href="https://chan.app"
            target="_blank"
            rel="noopener"
            title="chan.app"
          >
            <Globe size={14} strokeWidth={1.75} aria-hidden="true" />
            <span>chan.app</span>
          </a>
          <a
            class="about-link"
            href="https://github.com/fiorix/chan"
            target="_blank"
            rel="noopener"
            title="github.com/fiorix/chan"
          >
            <Code2 size={14} strokeWidth={1.75} aria-hidden="true" />
            <span>github.com/fiorix/chan</span>
          </a>
        </div>

        <div class="about-fund">
          <div class="fund-copy">
            <div class="fund-title">Fund the work</div>
            <p class="fund-text">
              Chan is independent software. Small tips help cover time
              spent on releases, packaging, and documentation.
            </p>
          </div>
          <img
            class="fund-qr"
            src={withTokenQuery("/qr-donate.png")}
            alt="Donation QR code"
            width="160"
            height="160"
          />
        </div>
      </div>
    {:else if slideIndex === 1}
      <!-- Phase-13 slice 3b-1: slide 1 hosts `WorkspaceInfoBody`,
           the same inspector body the file browser shows when the
           workspace-root row is selected. Folder-mode parity plus
           the Notes directories config. `WorkspaceInfoBody` owns
           its own scroll affordance via the slide's `overflow:
           auto`. -->
      <div class="slide slide-workspace" aria-label="Workspace info">
        <div class="slide-title">Workspace</div>
        <div class="workspace-info-host">
          <WorkspaceInfoBody
            variant="dashboard"
            onLanguageClick={openGraphForLanguage}
            onContactNavigate={openGraphForContact}
          />
        </div>
      </div>
    {:else}
      <!-- Phase-13 slice 3b-2: slide 2 is the read-only, spine-only
           indexing graph. We synthesize a directory-only
           `folder`-node spine from `/api/indexing/state` and feed
           it to the same `GraphCanvas` the main Graph tab uses;
           the per-directory `indexState` drives the green / grey /
           pulsing-orange palette inside the canvas. No chrome
           (inspector / scope picker / depth slider / filter chips)
           - the slide is purely a status read-out. -->
      <div class="slide slide-indexing" aria-label="Indexing graph">
        <div class="slide-title">Indexing</div>
        {#if indexingError}
          <div class="indexing-stub">
            <p>Couldn't load indexing state.</p>
            <p class="indexing-state">{indexingError}</p>
          </div>
        {:else if !indexing && indexingLoading}
          <div class="indexing-stub">
            <p>Loading indexing state…</p>
          </div>
        {:else if !indexing}
          <div class="indexing-stub">
            <p>Indexing state unavailable.</p>
          </div>
        {:else if indexingGraph.nodes.length === 0}
          <div class="indexing-stub">
            <p>No directories to graph yet.</p>
            {#if indexLabel}
              <p class="indexing-state">currently {indexLabel}</p>
            {/if}
          </div>
        {:else}
          <div class="indexing-graph-host">
            <GraphCanvas
              open={slideIndex === 2}
              nodes={indexingGraph.nodes}
              edges={indexingGraph.edges}
              visibleNodeIds={indexingNodeIds}
              visibleEdges={indexingGraph.edges}
              focalIds={indexingFocal}
              selectedId={selectedIndexId}
              onSelect={onIndexingSelect}
            />
          </div>
          <div class="indexing-legend" aria-hidden="true">
            <span class="legend-pair">
              <span class="dot" style="background: var(--accent);"></span>
              indexed
            </span>
            <span class="legend-pair">
              <span class="dot pulse" style="background: var(--g-doc);"></span>
              indexing
            </span>
            <span class="legend-pair">
              <span class="dot" style="background: var(--text-secondary);"></span>
              pending
            </span>
          </div>
        {/if}
      </div>
    {/if}
  </div>

  <div class="carousel-controls">
    <button
      class="nav-arrow"
      type="button"
      onclick={prev}
      aria-label="previous slide"
    >
      <ChevronLeft size={16} strokeWidth={1.75} aria-hidden="true" />
    </button>
    <div class="dots" role="tablist" aria-label="carousel slides">
      {#each Array.from({ length: slideCount }) as _, i}
        <button
          type="button"
          class="dot-btn"
          class:active={slideIndex === i}
          role="tab"
          aria-selected={slideIndex === i}
          aria-label={`slide ${i + 1}`}
          onclick={() => goTo(i)}
        ></button>
      {/each}
    </div>
    <button
      class="nav-arrow"
      type="button"
      onclick={next}
      aria-label="next slide"
    >
      <ChevronRight size={16} strokeWidth={1.75} aria-hidden="true" />
    </button>
    <!-- Persisted cycle toggle (fullstack-44). Sits to the right
         of the dots so it doesn't compete with the navigation
         affordances; the icon mirrors the standard
         play/pause-while-cycling convention. Pointer-hover-pause
         and focus-pause stay independent — those are transient,
         this one is the explicit user choice. -->
    <button
      class="cycle-toggle"
      type="button"
      onclick={toggleCycling}
      aria-label={cycling ? "stop carousel cycle" : "resume carousel cycle"}
      title={cycling ? "Stop cycling" : "Resume cycling"}
    >
      {#if cycling}
        <Pause size={14} strokeWidth={1.75} aria-hidden="true" />
      {:else}
        <Play size={14} strokeWidth={1.75} aria-hidden="true" />
      {/if}
    </button>
  </div>
</div>

<style>
  .carousel {
    /* `phase-13 slice 3b-1`: the carousel must size to its tab
       host. `flex: 1` + `min-height: 0` lets it fill the Dashboard
       tab's flex column without trapping overflow at the carousel
       root. Slide-level scroll handles content that overflows the
       current size; resizing the host tab reflows naturally because
       the slide-stage uses the parent's box. */
    flex: 1;
    min-height: 0;
    min-width: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 2rem 1rem 1rem;
    outline: none;
    gap: 1rem;
  }
  /* Slides themselves keep the old placeholder rhythm (centered
     stack, soft type, secondary color). The stage fills the
     remaining vertical space inside the carousel; each slide
     scrolls independently when its content exceeds the stage. */
  .slide-stage {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    flex: 1;
    min-height: 0;
    width: 100%;
    max-width: 720px;
  }
  .slide {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1.25rem;
    color: var(--text-secondary);
    flex: 1;
    min-height: 0;
    width: 100%;
    /* Carousel resize spec (phase-13 slice 3b-1): slide owns the
       vertical overflow so the inspector / about content stays
       scrollable when the tab shrinks. Horizontal stays hidden so
       the dot/nav row below the stage never wraps. */
    overflow-y: auto;
    overflow-x: hidden;
  }
  .slide-title {
    font-size: 14px;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: var(--text-secondary);
    opacity: 0.7;
  }
  /* --- Slide 0 (About) --- */
  .slide-about {
    align-items: stretch;
    gap: 0.9rem;
    color: var(--text);
    opacity: 1;
    padding: 0 0.25rem;
  }
  .slide-about .slide-title {
    align-self: center;
  }
  .about-grid {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 6px 14px;
    font-size: 13px;
    width: 100%;
  }
  .about-grid .k {
    color: var(--text-secondary);
  }
  .about-grid .v {
    color: var(--text);
  }
  .about-grid .v.mono {
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  }
  .about-grid .muted {
    color: var(--text-secondary);
    opacity: 0.85;
  }
  .about-grid .ok {
    color: var(--accent, var(--text));
  }
  .about-grid a {
    color: var(--link, var(--text));
    text-decoration: underline;
  }
  .about-links {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem 1rem;
    font-size: 13px;
  }
  .about-link {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--link, var(--text));
    text-decoration: none;
  }
  .about-link:hover,
  .about-link:focus-visible {
    text-decoration: underline;
  }
  .about-fund {
    display: flex;
    gap: 1rem;
    align-items: center;
    padding-top: 0.5rem;
    border-top: 1px dashed var(--border);
    flex-wrap: wrap;
  }
  .fund-copy {
    flex: 1;
    min-width: 200px;
  }
  .fund-title {
    font-size: 14px;
    font-weight: 600;
    color: var(--text);
    margin-bottom: 4px;
  }
  .fund-text {
    margin: 0;
    color: var(--text-secondary);
    font-size: 13px;
    line-height: 1.4;
  }
  .fund-qr {
    width: 160px;
    height: 160px;
    image-rendering: pixelated;
    border-radius: 6px;
    background: #fff;
    padding: 6px;
    flex-shrink: 0;
  }
  /* --- Slide 1 (Workspace info) --- */
  .slide-workspace {
    align-items: stretch;
    padding: 0;
    gap: 0.5rem;
    color: var(--text);
    opacity: 1;
  }
  .slide-workspace .slide-title {
    align-self: center;
  }
  .workspace-info-host {
    flex: 1;
    min-height: 0;
    width: 100%;
  }
  /* --- Slide 2 (Indexing graph) --- */
  /* `slide` defaults to `align-items: center` which would collapse
     the canvas host to its intrinsic width. The indexing slide
     wants the spine to occupy the full slide width, so we stretch
     the cross-axis here. */
  .slide-indexing {
    align-items: stretch;
    gap: 0.5rem;
    color: var(--text);
    opacity: 1;
    overflow: hidden;
  }
  .slide-indexing .slide-title {
    align-self: center;
  }
  .indexing-stub {
    text-align: center;
    max-width: 360px;
    font-size: 13px;
    line-height: 1.45;
    color: var(--text-secondary);
  }
  .indexing-stub p {
    margin: 0 0 0.5rem;
  }
  .indexing-state {
    color: var(--warn-text);
    font-size: 12px;
  }
  /* Phase-13 slice 3b-2: GraphCanvas fills its host. The host
     itself flex-grows inside the slide so the spine renders into
     the full available area and reflows with the Dashboard tab
     resize, just like the main Graph tab. */
  .indexing-graph-host {
    flex: 1;
    min-height: 0;
    width: 100%;
    position: relative;
  }
  .indexing-legend {
    display: flex;
    gap: 14px;
    font-size: 11px;
    color: var(--text-secondary);
    margin-top: 4px;
    align-self: center;
  }
  .indexing-legend .legend-pair {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }
  .indexing-legend .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
  }
  /* Indexing-state legend pulse mirrors the GraphCanvas pulse on
     in-flight directory nodes (alpha modulation, ~1100ms cycle).
     The canvas paint loop handles the node-level animation in JS;
     this keyframe keeps the legend swatch in sync visually. */
  .indexing-legend .dot.pulse {
    animation: indexing-pulse 1.1s ease-in-out infinite;
  }
  @keyframes indexing-pulse {
    0%, 100% { opacity: 1; }
    50%      { opacity: 0.55; }
  }
  @media (prefers-reduced-motion: reduce) {
    .indexing-legend .dot.pulse {
      animation: none;
    }
  }
  /* --- Controls --- */
  .carousel-controls {
    display: flex;
    align-items: center;
    gap: 8px;
    padding-top: 0.25rem;
  }
  .nav-arrow,
  .cycle-toggle {
    background: none;
    border: 0;
    padding: 4px;
    color: var(--text-secondary);
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    opacity: 0.5;
    transition: opacity 120ms ease, background 120ms ease;
  }
  .nav-arrow:hover,
  .cycle-toggle:hover {
    opacity: 1;
    background: var(--hover-bg);
  }
  /* Soft separator between the navigation cluster and the
     cycle toggle so they read as two control groups. */
  .cycle-toggle {
    margin-left: 6px;
    border-left: 1px solid var(--border);
    border-radius: 0 4px 4px 0;
    padding-left: 8px;
  }
  .dots {
    display: inline-flex;
    gap: 6px;
  }
  .dot-btn {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    border: 0;
    padding: 0;
    background: var(--text-secondary);
    opacity: 0.35;
    cursor: pointer;
    transition: opacity 120ms ease, transform 120ms ease;
  }
  .dot-btn:hover {
    opacity: 0.7;
  }
  .dot-btn.active {
    opacity: 0.9;
    transform: scale(1.2);
  }
  /* `fullstack-85`: dropped the inset focus ring here. The
     surrounding `.pane.focused` style (Pane.svelte) already draws
     the focus indicator around the entire pane in the multi-pane
     case; stacking a second 2px ring around just the carousel
     body painted the empty pane with a visibly thicker border on
     the body than along the top-bar chrome. Single-pane empty
     carousels have no indicator either way — there's only one
     pane to be focused. */
</style>
