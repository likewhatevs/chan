<script lang="ts">
  // Empty-pane visual-experimentation surface (fullstack-35).
  //
  // The carousel sits in place of the old placeholder block on a
  // single-pane lone-pane empty workspace. It auto-rotates every
  // 5 s starting on slide 1 (Welcome). Pointer hover and focus-
  // within pause auto-rotate so the user can read whichever slide
  // they happen to be looking at; the timer resumes when both
  // signals clear. Left / right arrow keys nudge manually when the
  // carousel container has focus.
  //
  // The container forwards `oncontextmenu` straight through to the
  // pane's empty-menu handler so right-click still opens the pane
  // hamburger menu (per the spec).
  //
  // Slide 1 (Welcome) is the pre-fullstack-35 placeholder content
  // verbatim: chan-mark, drive dashboard header, "scope-for-graph"
  // hint, and the shortcut table. Slide 2 surfaces drive metadata
  // (file kind breakdown + total bytes on disk) from the existing
  // tree listing. Slide 3 (Indexing graph) is a directory-only
  // radial layout fed by `GET /api/indexing/state` — colors track
  // per-dir state (green = indexed, orange = indexing with a slow
  // pulse, grey = pending).

  import { onDestroy } from "svelte";
  import { api } from "../api/client";
  import type { IndexingStateNode, IndexingStateResponse } from "../api/types";
  import { drive, indexStatus, tree, ui } from "../state/store.svelte";
  import {
    SHORTCUTS,
    currentOS,
    currentPlatform,
    formatChord,
    renderTable,
  } from "../state/shortcuts";
  import { ChevronLeft, ChevronRight, Pause, Play } from "lucide-svelte";

  type Props = {
    /// Right-click forwarder. Same handler the empty pane uses to
    /// open the welcome menu; lifted here so the carousel surface
    /// preserves the right-click affordance.
    oncontextmenu?: (e: MouseEvent) => void;
  };
  let { oncontextmenu }: Props = $props();

  /// ASCII shortcut table. Picked at module init since platform +
  /// chord set don't change at runtime.
  const platform = currentPlatform();
  const os = currentOS();
  const shortcutTable = renderTable(platform, os);

  function chordLabel(id: string | undefined): string {
    if (!id) return "";
    const s = SHORTCUTS.find((x) => x.id === id);
    if (!s) return "";
    const chord = s[platform];
    if (!chord) return "";
    return formatChord(chord, os);
  }

  // ---- drive summary -----------------------------------------------------
  //
  // `fullstack-55` dropped the stats row under the brand mark on
  // slide 1, taking the `driveSummary` derived with it. Slide 2's
  // `metadata` derived keeps its own per-kind tallies for the bar
  // chart, so there's no other consumer to feed from here.

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

  /// Per-kind breakdown for the metadata slide. Re-derives on every
  /// tree refresh. Total bytes counts file sizes only (directories
  /// have no inherent size in the listing).
  type Metadata = {
    docs: number;
    contacts: number;
    text: number;
    media: number;
    binary: number;
    folders: number;
    totalBytes: number;
  };
  const metadata = $derived.by<Metadata>(() => {
    let docs = 0;
    let contacts = 0;
    let text = 0;
    let media = 0;
    let binary = 0;
    let folders = 0;
    let totalBytes = 0;
    for (const e of tree.entries) {
      if (e.is_dir) {
        folders++;
        continue;
      }
      totalBytes += e.size;
      switch (e.kind) {
        case "document":
          docs++;
          break;
        case "contact":
          contacts++;
          break;
        case "media":
          media++;
          break;
        case "binary":
          binary++;
          break;
        case "text":
        default:
          text++;
          break;
      }
    }
    return { docs, contacts, text, media, binary, folders, totalBytes };
  });

  /// Single-bar stacked breakdown for the metadata slide. Returns
  /// the file-kind segments only (directories aren't sized so they
  /// don't fit a "how much space is what" view). Empty drives
  /// render as a 100% --bg-elev segment so the bar still draws.
  type Segment = { key: string; label: string; count: number; color: string };
  const fileSegments = $derived.by<Segment[]>(() => {
    const m = metadata;
    const segs: Segment[] = [
      { key: "docs", label: "documents", count: m.docs, color: "var(--g-doc)" },
      { key: "contacts", label: "contacts", count: m.contacts, color: "var(--pill-contact-fg, var(--warn-text))" },
      { key: "text", label: "other text", count: m.text, color: "var(--text-secondary)" },
      { key: "media", label: "media", count: m.media, color: "var(--g-img)" },
      { key: "binary", label: "binary", count: m.binary, color: "var(--g-binary)" },
    ];
    return segs.filter((s) => s.count > 0);
  });

  const totalFiles = $derived(
    fileSegments.reduce((acc, s) => acc + s.count, 0),
  );

  function pctOf(seg: Segment): number {
    if (totalFiles === 0) return 0;
    return (seg.count / totalFiles) * 100;
  }

  function humanBytes(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    const units = ["KB", "MB", "GB", "TB"];
    let n = bytes / 1024;
    let u = 0;
    while (n >= 1024 && u < units.length - 1) {
      n /= 1024;
      u++;
    }
    return `${n.toFixed(n >= 10 ? 0 : 1)} ${units[u]}`;
  }

  // ---- slide 3 — indexing graph ------------------------------------------

  /// Indexing state response cache. Re-fetched whenever slide 3
  /// becomes active and again every 3 s while it stays active so
  /// orange (in-flight) nodes can flip to green as the indexer
  /// makes progress. Polling stops the moment slide 3 hides; the
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

  /// Build a parent → children adjacency map from the flat node
  /// list. The endpoint returns drive-relative paths; we keep the
  /// root sentinel ("" for the drive root) separate so the layout
  /// can anchor on it. Path separation is purely string-based to
  /// stay decoupled from the server's filesystem convention.
  type Hierarchy = {
    rootPath: string;
    byPath: Map<string, IndexingStateNode>;
    children: Map<string, string[]>;
  };
  const hierarchy = $derived.by<Hierarchy | null>(() => {
    const data = indexing;
    if (!data) return null;
    const byPath = new Map<string, IndexingStateNode>();
    const children = new Map<string, string[]>();
    for (const n of data.nodes) byPath.set(n.path, n);
    for (const n of data.nodes) {
      const parent = parentOf(n.path);
      if (parent === n.path) continue; // root
      const arr = children.get(parent) ?? [];
      arr.push(n.path);
      children.set(parent, arr);
    }
    return { rootPath: data.root, byPath, children };
  });

  function parentOf(path: string): string {
    const slash = path.lastIndexOf("/");
    if (slash <= 0) return "";
    return path.slice(0, slash);
  }
  function basename(path: string): string {
    if (path === "") return "/";
    const slash = path.lastIndexOf("/");
    return slash < 0 ? path : path.slice(slash + 1);
  }

  /// Per-node position in the SVG viewport. Depth-tiered radial
  /// layout: root sits at center, depth-N descendants are spread
  /// evenly around a circle of radius `BASE_R * depth`. Within a
  /// tier, children of the same parent share an arc proportional
  /// to the parent's slot so siblings stay clustered.
  type Placed = {
    path: string;
    depth: number;
    x: number;
    y: number;
  };
  const VIEW_SIZE = 280;
  const BASE_R = 56;

  const placed = $derived.by<Placed[]>(() => {
    const h = hierarchy;
    if (!h) return [];
    const cx = VIEW_SIZE / 2;
    const cy = VIEW_SIZE / 2;
    const out: Placed[] = [{ path: h.rootPath, depth: 0, x: cx, y: cy }];
    type Slot = { angleStart: number; angleEnd: number };
    const slots = new Map<string, Slot>();
    slots.set(h.rootPath, { angleStart: -Math.PI / 2, angleEnd: -Math.PI / 2 + Math.PI * 2 });
    const queue: Array<{ path: string; depth: number }> = [
      { path: h.rootPath, depth: 0 },
    ];
    while (queue.length > 0) {
      const cur = queue.shift()!;
      const kids = h.children.get(cur.path) ?? [];
      if (kids.length === 0) continue;
      const slot = slots.get(cur.path)!;
      const span = slot.angleEnd - slot.angleStart;
      const step = span / kids.length;
      for (let i = 0; i < kids.length; i++) {
        const kid = kids[i]!;
        const childSpanStart = slot.angleStart + step * i;
        const childSpanEnd = childSpanStart + step;
        const angle = (childSpanStart + childSpanEnd) / 2;
        const r = BASE_R * (cur.depth + 1);
        out.push({
          path: kid,
          depth: cur.depth + 1,
          x: cx + Math.cos(angle) * r,
          y: cy + Math.sin(angle) * r,
        });
        slots.set(kid, { angleStart: childSpanStart, angleEnd: childSpanEnd });
        queue.push({ path: kid, depth: cur.depth + 1 });
      }
    }
    return out;
  });

  /// Edges between each placed node and its parent. Pre-computed
  /// so the SVG draws lines first (under the circles) without
  /// repeating the parent lookup.
  type Edge = { fromX: number; fromY: number; toX: number; toY: number };
  const edges = $derived.by<Edge[]>(() => {
    const positions = placed;
    if (positions.length === 0) return [];
    const byPath = new Map(positions.map((p) => [p.path, p] as const));
    const out: Edge[] = [];
    for (const p of positions) {
      if (p.depth === 0) continue;
      const parent = byPath.get(parentOf(p.path));
      if (!parent) continue;
      out.push({ fromX: parent.x, fromY: parent.y, toX: p.x, toY: p.y });
    }
    return out;
  });

  let selectedPath = $state<string | null>(null);

  /// Same label rule as the main graph (fullstack-32): paint
  /// labels for the selected node plus its immediate neighbors
  /// (parent + direct children). Without a selection we label
  /// the root only so the user can see they're at the drive
  /// origin.
  const labeledPaths = $derived.by<Set<string>>(() => {
    const h = hierarchy;
    const out = new Set<string>();
    if (!h) return out;
    if (selectedPath === null) {
      out.add(h.rootPath);
      return out;
    }
    out.add(selectedPath);
    const parent = parentOf(selectedPath);
    if (h.byPath.has(parent) || parent === h.rootPath) out.add(parent);
    for (const kid of h.children.get(selectedPath) ?? []) out.add(kid);
    return out;
  });

  function nodeFill(state: IndexingStateNode["state"]): string {
    switch (state) {
      case "indexed":
        return "var(--accent)";
      case "indexing":
        return "var(--g-doc)";
      case "pending":
      default:
        return "var(--text-secondary)";
    }
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
    drive.info?.preferences?.empty_pane_carousel_cycling ?? true,
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
    if (drive.info) {
      drive.info = {
        ...drive.info,
        preferences: {
          ...drive.info.preferences,
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
  {oncontextmenu}
>
  <div class="slide-stage">
    {#if slideIndex === 0}
      <!-- Slide 1 — Welcome. Lifted verbatim from the pre-
           fullstack-35 placeholder so the dashboard rhythm
           (logo + drive summary + shortcut table) stays
           identical to what users already know. -->
      <div class="slide slide-welcome" aria-label="Welcome">
        <div class="placeholder-mark"></div>
        {#if drive.info}
          <!-- `fullstack-55`: the files / dirs / contacts / index
               row was dropped from under the brand mark — that
               information lives on slide 2 (metadata). Only the
               drive name remains under the logo. -->
          <div class="dashboard-header" aria-label="drive summary">
            <div class="dashboard-name">{drive.info.name ?? "(unnamed)"}</div>
          </div>
        {/if}
        <p class="placeholder-hint">
          Each pane's visible tab is part of the scope<br />
          for Graph.
        </p>
        <pre class="placeholder-shortcuts">{shortcutTable}</pre>
      </div>
    {:else if slideIndex === 1}
      <!-- Slide 2 — Metadata. Stacked bar of file kinds across
           the drive plus a small stats footer. Approximate is
           fine per the task spec; this is a UX-experimentation
           surface, not a billing dashboard. -->
      <div class="slide slide-metadata" aria-label="Drive metadata">
        <div class="slide-title">Drive metadata</div>
        <div class="metadata-bar" role="img" aria-label="file kind breakdown">
          {#if totalFiles === 0}
            <div class="bar-empty">empty drive</div>
          {:else}
            {#each fileSegments as seg (seg.key)}
              <div
                class="bar-seg"
                style="flex-basis: {pctOf(seg)}%; background: {seg.color};"
                title={`${seg.label}: ${seg.count} (${pctOf(seg).toFixed(0)}%)`}
              ></div>
            {/each}
          {/if}
        </div>
        <ul class="metadata-legend">
          {#each fileSegments as seg (seg.key)}
            <li>
              <span class="dot" style="background: {seg.color};"></span>
              <span class="legend-label">{seg.label}</span>
              <span class="legend-count">{seg.count}</span>
            </li>
          {/each}
        </ul>
        <div class="metadata-footer">
          <span>{metadata.folders} directories</span>
          <span class="sep" aria-hidden="true">·</span>
          <span>{humanBytes(metadata.totalBytes)} on disk</span>
        </div>
      </div>
    {:else}
      <!-- Slide 3 — Indexing graph. Directory-only radial layout
           fed by `GET /api/indexing/state`. Colors track per-dir
           state (green = indexed, orange = indexing with a slow
           pulse, grey = pending). Labels render for the selected
           node plus its immediate parent + children (same rule
           as the main graph). -->
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
        {:else if placed.length === 0}
          <div class="indexing-stub">
            <p>No directories to graph yet.</p>
            {#if indexLabel}
              <p class="indexing-state">currently {indexLabel}</p>
            {/if}
          </div>
        {:else}
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <svg
            class="indexing-graph"
            viewBox={`0 0 ${VIEW_SIZE} ${VIEW_SIZE}`}
            role="img"
            aria-label="directory indexing graph"
          >
            <g class="edges">
              {#each edges as e, i (i)}
                <line
                  x1={e.fromX}
                  y1={e.fromY}
                  x2={e.toX}
                  y2={e.toY}
                  stroke="var(--border)"
                  stroke-width="1"
                  opacity="0.6"
                />
              {/each}
            </g>
            <g class="nodes">
              {#each placed as p (p.path)}
                {@const node = hierarchy?.byPath.get(p.path)}
                {#if node}
                  <g
                    class="node"
                    class:pulsate={node.state === "indexing"}
                    class:selected={selectedPath === p.path}
                    transform={`translate(${p.x} ${p.y})`}
                    onclick={() =>
                      (selectedPath = selectedPath === p.path ? null : p.path)}
                  >
                    <circle
                      r={p.depth === 0 ? 8 : 5}
                      fill={nodeFill(node.state)}
                      stroke="var(--bg)"
                      stroke-width="1.5"
                    />
                    {#if labeledPaths.has(p.path)}
                      <text
                        x={0}
                        y={(p.depth === 0 ? -14 : -10)}
                        text-anchor="middle"
                        class="node-label"
                      >{basename(p.path) || "/"}</text>
                    {/if}
                  </g>
                {/if}
              {/each}
            </g>
          </svg>
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
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 2rem 1rem 1rem;
    overflow: auto;
    outline: none;
    gap: 1rem;
  }
  /* Slides themselves keep the old placeholder rhythm (centered
     stack, soft type, secondary color) so the welcome content
     reads identically. Width cap keeps long-line content like the
     shortcut table from sprawling. */
  .slide-stage {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    flex: 1;
    min-height: 0;
    width: 100%;
    max-width: 520px;
  }
  .slide {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1.25rem;
    color: var(--text-secondary);
    opacity: 0.6;
  }
  .slide-title {
    font-size: 14px;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: var(--text-secondary);
    opacity: 0.7;
  }
  /* --- Slide 1 (Welcome) bits, ported from Pane.svelte --- */
  .placeholder-mark {
    width: 160px;
    height: 160px;
    background-color: var(--text-secondary);
    -webkit-mask: url('/chan-mark.png') center / contain no-repeat;
            mask: url('/chan-mark.png') center / contain no-repeat;
    opacity: 0.45;
  }
  .placeholder-shortcuts {
    margin: 0;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 12px;
    line-height: 1.5;
    white-space: pre;
    color: var(--text-secondary);
  }
  .placeholder-hint {
    margin: 0;
    text-align: center;
    color: var(--text-secondary);
    font-size: 13px;
    line-height: 1.4;
    max-width: 360px;
  }
  .dashboard-header {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    margin-top: -0.5rem;
  }
  .dashboard-name {
    font-size: 18px;
    color: var(--text);
    opacity: 0.85;
    letter-spacing: 0.01em;
  }
  /* --- Slide 2 (Metadata) --- */
  .metadata-bar {
    display: flex;
    width: 100%;
    height: 12px;
    border-radius: 6px;
    overflow: hidden;
    background: var(--bg-elev);
  }
  .bar-seg {
    height: 100%;
    flex-grow: 0;
  }
  .bar-empty {
    flex: 1;
    text-align: center;
    font-size: 11px;
    color: var(--text-secondary);
    line-height: 12px;
    opacity: 0.7;
  }
  .metadata-legend {
    list-style: none;
    padding: 0;
    margin: 0;
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
    gap: 4px 12px;
    width: 100%;
    font-size: 12px;
  }
  .metadata-legend li {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .metadata-legend .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
  }
  .legend-label {
    flex: 1;
    color: var(--text-secondary);
  }
  .legend-count {
    color: var(--text);
    opacity: 0.85;
    font-variant-numeric: tabular-nums;
  }
  .metadata-footer {
    display: flex;
    gap: 0.4rem;
    color: var(--text-secondary);
    font-size: 12px;
  }
  .metadata-footer .sep {
    opacity: 0.5;
  }
  /* --- Slide 3 (Indexing graph) --- */
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
  .indexing-graph {
    width: min(100%, 320px);
    height: auto;
    aspect-ratio: 1 / 1;
  }
  .indexing-graph .node {
    cursor: pointer;
  }
  .indexing-graph .node-label {
    font-size: 10px;
    fill: var(--text);
    opacity: 0.85;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    pointer-events: none;
  }
  .indexing-graph .node.selected circle {
    stroke: var(--pane-focus);
    stroke-width: 2;
  }
  /* Pulsate orange (indexing) nodes so in-flight work stands out
     against static greys + greens. Pure CSS — no JS animation
     state. Slow 2.4 s cycle keeps the motion calm; opacity-only
     so the layout never shifts. */
  .indexing-graph .node.pulsate circle {
    animation: indexing-pulse 2.4s ease-in-out infinite;
  }
  @keyframes indexing-pulse {
    0%, 100% { opacity: 1; }
    50%      { opacity: 0.4; }
  }
  .indexing-legend {
    display: flex;
    gap: 14px;
    font-size: 11px;
    color: var(--text-secondary);
    margin-top: 4px;
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
  .indexing-legend .dot.pulse {
    animation: indexing-pulse 2.4s ease-in-out infinite;
  }
  @media (prefers-reduced-motion: reduce) {
    .indexing-graph .node.pulsate circle,
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
  .carousel:focus-visible {
    box-shadow: inset 0 0 0 2px var(--pane-active-focus, var(--pane-focus));
  }
</style>
