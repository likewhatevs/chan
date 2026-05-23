<script lang="ts">
  // `fullstack-a-75b`: Infographics tab body. Per @@Alex's
  // `d4a3fc8` route on the slice-1 walk, the rotating carousel
  // moves OUT of the welcome surface (which becomes a static
  // spawn grid via EmptyPaneWelcome.svelte) and lives only
  // INSIDE this tab. The full carousel widget (rotation +
  // play/pause + pagination + 3 slides — Shortcuts / Drive
  // metadata / Indexing graph) renders here.
  //
  // Earlier slice (-a-75 slice 1) shipped this tab as a static
  // ASCII shortcut table; that table is now slide 1 of the
  // carousel below.

  import EmptyPaneCarousel from "./EmptyPaneCarousel.svelte";
</script>

<div class="infographics" aria-label="Infographics">
  <EmptyPaneCarousel />
</div>

<style>
  .infographics {
    flex: 1;
    min-height: 0;
    min-width: 0;
    display: flex;
    flex-direction: column;
    background: var(--bg);
    color: var(--text);
  }
</style>
