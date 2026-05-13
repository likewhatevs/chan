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
  import { ArrowRight } from "lucide-svelte";
  import ResizeHandle from "./ResizeHandle.svelte";

  let {
    title,
    width = $bindable(220),
    onResize,
    onClose,
    children,
  }: {
    title: string;
    width?: number;
    onResize?: () => void;
    onClose?: () => void;
    children?: Snippet;
  } = $props();
</script>

<ResizeHandle bind:width onChange={onResize} />
<aside class="inspector" style="width: {width}px">
  <div class="title">
    <span class="title-text">{title}</span>
    {#if onClose}
      <button
        class="close"
        type="button"
        title="Close"
        aria-label="Close {title}"
        onclick={onClose}
      >
        <!-- Right-pointing arrow matches the panel's collapse
             direction (the inspector lives on the right edge and
             slides off to the right when closed). Visual idiom
             copied from Google Docs' left-panel close (which uses
             a left-pointing arrow because that panel sits on the
             left). -->
        <ArrowRight size={16} strokeWidth={1.75} aria-hidden="true" />
      </button>
    {/if}
  </div>
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
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }
  .title-text {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .close {
    flex-shrink: 0;
    background: transparent;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    padding: 0.15rem 0.25rem;
    border-radius: 3px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    line-height: 0;
  }
  .close:hover {
    background: var(--hover-bg, rgba(127, 127, 127, 0.15));
    color: var(--text);
  }
  .body {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
</style>
