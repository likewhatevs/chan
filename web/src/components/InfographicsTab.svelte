<script lang="ts">
  // `fullstack-a-75`: Infographics tab body. Hosts the ASCII
  // shortcut table that previously rendered on the empty-pane
  // carousel slide 1; lifting it into its own tab type lets the
  // carousel focus on spawn affordances + drive metadata and
  // gives the shortcut sheet a stable, scoped surface a user can
  // park in a Hybrid pane.
  //
  // Today's body is the shortcut table alone; later slices can
  // grow this into a multi-panel info hub (drive-wide metrics,
  // indexing topology, broadcast routing, etc.). The tab kind +
  // mount branch in Pane.svelte are the persistent affordance;
  // additions land here without touching the layout layer.

  import {
    SHORTCUTS,
    currentOS,
    currentPlatform,
    formatChord,
    renderTable,
  } from "../state/shortcuts";

  // Pick at module init since platform + chord set don't change
  // at runtime. Mirrors EmptyPaneCarousel.svelte's pattern from
  // before the lift.
  const platform = currentPlatform();
  const os = currentOS();
  const shortcutTable = renderTable(platform, os);

  // Inline chord lookup helper for the per-shortcut detail rows.
  // Same shape as EmptyPaneCarousel.svelte's `chordLabel`; lifted
  // alongside the table for cohesion.
  function chordLabel(id: string | undefined): string {
    if (!id) return "";
    const s = SHORTCUTS.find((x) => x.id === id);
    if (!s) return "";
    const chord = s[platform];
    if (!chord) return "";
    return formatChord(chord, os);
  }
  void chordLabel;
</script>

<div class="infographics" aria-label="Infographics">
  <header class="info-header">
    <h2>Shortcuts</h2>
    <p class="info-sub">
      Keys + chords across the chan UI. Platform: {platform} ({os}).
    </p>
  </header>
  <pre class="info-shortcuts">{shortcutTable}</pre>
</div>

<style>
  .infographics {
    flex: 1;
    min-height: 0;
    min-width: 0;
    display: flex;
    flex-direction: column;
    padding: 1.5rem 2rem;
    overflow: auto;
    background: var(--bg);
    color: var(--text);
  }
  .info-header {
    margin-bottom: 1rem;
  }
  .info-header h2 {
    margin: 0;
    font-size: 1.1rem;
    color: var(--text-heading, var(--text));
  }
  .info-sub {
    margin: 0.25rem 0 0;
    font-size: 0.85rem;
    color: var(--text-secondary);
  }
  /* `fullstack-a-75`: monospace shortcut table inherits the
     carousel's `.placeholder-shortcuts` look — ASCII grid that
     reads as a printable cheatsheet. */
  .info-shortcuts {
    margin: 0;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 12px;
    color: var(--text);
    white-space: pre;
    line-height: 1.45;
  }
</style>
