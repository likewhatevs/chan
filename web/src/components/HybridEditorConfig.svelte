<script lang="ts">
  // `fullstack-a-43` Task A stub. Title band only; the body will
  // host the Editor settings currently in `SettingsPanel.svelte`
  // (Theme, Layout, Date Pills, On Save per `-a-25`). Task C
  // migrates the section out of Settings.
</script>

<section class="hybrid-config" aria-label="Hybrid Editor configuration">
  <header class="config-header">
    <h2 class="config-title">Hybrid Editor</h2>
  </header>
  <div class="config-body">
    <!-- Empty body until Task C migrates the Editor settings. -->
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
