<script lang="ts">
  import FileBrowserSurface from "./FileBrowserSurface.svelte";
  import OverlayShell from "./OverlayShell.svelte";
  import { browserOverlay } from "../state/store.svelte";

  const visible = $derived(browserOverlay.open);
  let surface: FileBrowserSurface | undefined = $state();

  function close(): void {
    browserOverlay.open = false;
  }

  function onBrowserContextMenu(e: MouseEvent): void {
    e.preventDefault();
    surface?.openMenuAtCursor(e.clientX, e.clientY);
  }
</script>

<OverlayShell
  id="browser"
  open={visible}
  onClose={close}
  onBackdropContextMenu={onBrowserContextMenu}
>
  <FileBrowserSurface bind:this={surface} variant="overlay" onClose={close} />
</OverlayShell>
