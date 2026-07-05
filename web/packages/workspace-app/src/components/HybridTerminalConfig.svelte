<script lang="ts">
  import HybridSurfaceConfigShell from "./HybridSurfaceConfigShell.svelte";

  let { onDone }: { onDone?: () => void } = $props();
</script>

<HybridSurfaceConfigShell title="Hybrid Terminal" {onDone} />
