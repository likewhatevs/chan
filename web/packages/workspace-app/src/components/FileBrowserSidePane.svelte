<script lang="ts">
  import FileBrowserSurface from "./FileBrowserSurface.svelte";
  import ResizeHandle from "./ResizeHandle.svelte";
  import { paneWidths, persistPaneWidths } from "../state/store.svelte";

  let { side }: { side: "left" | "right" } = $props();
</script>

<aside
  class="browser-side-pane"
  class:left={side === "left"}
  class:right={side === "right"}
  style:width={`${paneWidths.browser}px`}
  aria-label={side === "left" ? "left file browser" : "right file browser"}
>
  {#if side === "right"}
    <ResizeHandle
      bind:width={paneWidths.browser}
      side="right"
      idleVisible={false}
      onChange={persistPaneWidths}
    />
  {/if}
  <div class="surface">
    <FileBrowserSurface variant="dock" {side} />
  </div>
  {#if side === "left"}
    <ResizeHandle
      bind:width={paneWidths.browser}
      side="left"
      idleVisible={false}
      onChange={persistPaneWidths}
    />
  {/if}
</aside>

<style>
  .browser-side-pane {
    display: flex;
    min-width: 140px;
    max-width: 600px;
    min-height: 0;
    height: 100%;
    flex: 0 0 auto;
    background: var(--bg);
  }
  .surface {
    display: flex;
    flex: 1;
    min-width: 0;
    min-height: 0;
  }
</style>
