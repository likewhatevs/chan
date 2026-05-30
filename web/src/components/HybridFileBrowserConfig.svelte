<script lang="ts">
  // File Browser back-side. After the phase-15 Dashboard redesign the
  // Search / embedding-model / chan-reports controls that used to live
  // here moved into the Dashboard's per-slot backs (Search + Workspace
  // slots). This surface keeps no settings of its own; it stays as the
  // canonical File Browser flip-back so Cmd+, on a File Browser still
  // lands on a config shell (title band + per-Hybrid Dark/Light theme
  // override + OK).
  import HybridSurfaceConfigShell from "./HybridSurfaceConfigShell.svelte";

  let { onDone }: { onDone?: () => void } = $props();
</script>

<HybridSurfaceConfigShell
  title="Hybrid File Browser"
  surface="browser"
  ariaLabel="File Browser settings"
  {onDone}
>
  <section>
    <p class="placeholder-hint">
      No settings here, cheers. Search, the embedding model and
      chan-reports moved to the Dashboard's Search and Workspace slots.
    </p>
  </section>
</HybridSurfaceConfigShell>

<style>
  .placeholder-hint {
    margin: 0;
    color: var(--text-secondary);
    font-size: 13px;
  }
</style>
