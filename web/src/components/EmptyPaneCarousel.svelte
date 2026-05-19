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
  // tree listing. Slide 3 (Indexing graph) is a stub until
  // chan-server ships GET /api/indexing/state — wired separately.

  import { onDestroy } from "svelte";
  import { drive, indexStatus, tree } from "../state/store.svelte";
  import {
    SHORTCUTS,
    currentOS,
    currentPlatform,
    formatChord,
    renderTable,
  } from "../state/shortcuts";
  import { ChevronLeft, ChevronRight } from "lucide-svelte";

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

  const driveSummary = $derived.by(() => {
    let files = 0;
    let folders = 0;
    let contacts = 0;
    for (const e of tree.entries) {
      if (e.is_dir) folders++;
      else {
        files++;
        if (e.kind === "contact") contacts++;
      }
    }
    return { files, folders, contacts };
  });

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

  // ---- carousel state ----------------------------------------------------

  const slideCount = 3;
  let slideIndex = $state(0);
  let hovering = $state(false);
  let focused = $state(false);
  const paused = $derived(hovering || focused);
  let containerEl: HTMLDivElement | undefined = $state();

  /// Auto-rotate while neither hovered nor focused. Reset the
  /// interval on every dependency change (slideIndex bumped by
  /// keyboard nudges, paused toggled by hover/focus) so manual
  /// nav doesn't lose the next-tick budget.
  $effect(() => {
    void slideIndex;
    if (paused) return;
    const handle = window.setInterval(() => {
      slideIndex = (slideIndex + 1) % slideCount;
    }, 5000);
    return () => window.clearInterval(handle);
  });

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
          <div class="dashboard-header" aria-label="drive summary">
            <div class="dashboard-name">{drive.info.name ?? "(unnamed)"}</div>
            <div class="dashboard-stats">
              <span>{driveSummary.files} files</span>
              <span class="sep" aria-hidden="true">·</span>
              <span>{driveSummary.folders} directories</span>
              {#if driveSummary.contacts > 0}
                <span class="sep" aria-hidden="true">·</span>
                <span>{driveSummary.contacts} contacts</span>
              {/if}
              {#if indexLabel}
                <span class="sep" aria-hidden="true">·</span>
                <span class="dashboard-index">{indexLabel}</span>
              {/if}
            </div>
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
      <!-- Slide 3 — Indexing graph (stub). Waiting on the
           GET /api/indexing/state endpoint from @@Systacean; once
           that lands the dir-only graph with grey/orange/green
           node states replaces this placeholder. -->
      <div class="slide slide-indexing" aria-label="Indexing graph">
        <div class="slide-title">Indexing</div>
        <div class="indexing-stub">
          <p>Directory-only indexing graph lands once chan-server
          ships the per-directory state stream.</p>
          {#if indexLabel}
            <p class="indexing-state">currently {indexLabel}</p>
          {:else}
            <p class="indexing-state">idle</p>
          {/if}
        </div>
      </div>
    {/if}
  </div>

  <div class="carousel-controls" aria-hidden="true">
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
  .dashboard-stats {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: 0.4rem;
    color: var(--text-secondary);
    font-size: 12px;
  }
  .dashboard-stats .sep {
    opacity: 0.5;
  }
  .dashboard-index {
    color: var(--warn-text);
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
  /* --- Slide 3 (Indexing stub) --- */
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
  /* --- Controls --- */
  .carousel-controls {
    display: flex;
    align-items: center;
    gap: 8px;
    padding-top: 0.25rem;
  }
  .nav-arrow {
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
  .nav-arrow:hover {
    opacity: 1;
    background: var(--hover-bg);
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
