<script lang="ts">
  // `fullstack-a-43` Task A stub. Title band only; the body will
  // host the Search / Indexing settings — semantic search toggle
  // from `-a-21` and the future multi-model picker from Round-3
  // Track 2. Task F migrates the section out of Settings.
  // Rationale: FB is where users see indexed content and where
  // search results land them, so config sits next to the affected
  // surface. The search OVERLAY (`Cmd+K F`) stays a global
  // surface; only the SETTINGS move.
</script>

<section class="hybrid-config" aria-label="Hybrid File Browser configuration">
  <header class="config-header">
    <h2 class="config-title">Hybrid File Browser</h2>
  </header>
  <div class="config-body">
    <!-- Empty body until Task F migrates the Search / Indexing
         settings. -->
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
