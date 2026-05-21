<script lang="ts">
  // `fullstack-a-43` Task A stub. Title band only; the body will
  // host the node-type legend grid (Dir, File (Regular, Code,
  // Document, Contact), Hashtag, Mention, Language(Code)) with
  // each row pairing a node icon and its colour swatch. Task D
  // populates from the existing graph colour-scheme map.
</script>

<section class="hybrid-config" aria-label="Hybrid Graph configuration">
  <header class="config-header">
    <h2 class="config-title">Hybrid Graph</h2>
  </header>
  <div class="config-body">
    <!-- Empty body until Task D builds the legend grid. -->
  </div>
</section>

<style>
  .hybrid-config {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
    min-height: 0;
  }
  .config-header {
    padding: 16px 20px;
    border-bottom: 1px solid var(--border);
  }
  .config-title {
    margin: 0;
    font-size: 18px;
    font-weight: 600;
    color: var(--text);
  }
  .config-body {
    flex: 1;
    overflow: auto;
    padding: 16px 20px;
  }
</style>
