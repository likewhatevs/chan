<script lang="ts">
  // Chrome-only host for the right-side inspector pane. Renders the
  // resize handle on the left edge, an aside container, and a title
  // bar; the body is supplied by the caller via children. Used by:
  //   - FileEditorTab (body: outline + collapsible file info)
  //   - FileBrowserTab (body: file/folder metadata for the current
  //     selection)
  //   - future: settings tab info, graph node info
  //
  // Width is bound by the caller so each surface can persist into
  // its own preference slot (file editor uses paneWidths.inspector;
  // file browser uses paneWidths.browser; defaults differ).

  import type { Snippet } from "svelte";
  import ResizeHandle from "./ResizeHandle.svelte";

  let {
    title,
    width = $bindable(220),
    onResize,
    children,
  }: {
    title: string;
    width?: number;
    onResize?: () => void;
    children?: Snippet;
  } = $props();
</script>

<ResizeHandle bind:width onChange={onResize} />
<aside class="inspector" style="width: {width}px">
  <div class="title">{title}</div>
  <div class="body">
    {#if children}
      {@render children()}
    {/if}
  </div>
</aside>

<style>
  .inspector {
    /* width is set inline by the parent so the resize handle's
       drag updates apply without a CSS rule rewrite. */
    flex-shrink: 0;
    border-left: 1px solid var(--separator);
    background: var(--inspector-bg);
    color: var(--text);
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    font-size: 15px;
  }
  .title {
    padding: 0.4rem 0.6rem;
    font-size: 14px;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    border-bottom: 1px solid var(--separator);
    flex-shrink: 0;
  }
  .body {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
</style>
