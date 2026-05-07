<script lang="ts">
  // File browser overlay. The recursive tree on the left, the
  // shared Inspector on the right (file/folder metadata for the
  // current selection), wrapped in OverlayShell so it floats over
  // the workspace pane tree on every platform (web, native desktop,
  // mobile). Replaces the previous `BrowserTab` tab kind and the
  // native-only special-tab WebviewWindow.

  import FileTree from "./FileTree.svelte";
  import Inspector from "./Inspector.svelte";
  import FileInfoBody from "./FileInfoBody.svelte";
  import OverlayShell from "./OverlayShell.svelte";
  import { isEditableText } from "../state/fileTypes";
  import {
    browserOverlay,
    browserSelection,
    fileOps,
    paneWidths,
    persistPaneWidths,
    tree,
    drive,
  } from "../state/store.svelte";
  import { openInActivePane } from "../state/tabs.svelte";

  const visible = $derived(browserOverlay.open);

  function close(): void {
    browserOverlay.open = false;
  }

  function openSelected(): void {
    const path = browserSelection.path;
    if (!path) return;
    const entry = tree.entries.find((e) => e.path === path);
    if (entry && !entry.is_dir && isEditableText(entry.path)) {
      void openInActivePane(entry.path);
      // Auto-close after opening a file: the user wanted to read /
      // edit it, not keep the picker hovering.
      close();
    }
  }

  function clearSelection(): void {
    browserSelection.path = null;
  }

  /// Auto-open the inspector when the user picks a row in the tree.
  /// Same behaviour as the previous browser tab: every click on a
  /// file or folder surfaces its metadata, so an explicitly-closed
  /// inspector is not a "stay closed" sticky state; the next click
  /// re-opens it.
  $effect(() => {
    if (browserSelection.path) browserOverlay.inspectorOpen = true;
  });

  /// "+" popover state. Closes on outside click via the
  /// click-capture handler on the popover root.
  let newMenuOpen = $state(false);
  let triggerEl: HTMLButtonElement | undefined = $state();
  let popoverPos = $state<{ top: number; left: number }>({ top: 0, left: 0 });

  /// Roughly the rendered size of the popover. We pre-compute layout
  /// before the popover mounts so it never flashes in the wrong
  /// place; these constants approximate the real footprint of two
  /// list rows with the icon + label inside.
  const POPOVER_HEIGHT = 76;
  const POPOVER_WIDTH = 156;

  /// Toggle the popover. When opening, measure the trigger button
  /// against the viewport and pick "below" or "above" depending on
  /// which side has more room. Without this the menu was clipped
  /// outside the OverlayShell panel whenever the file browser sat
  /// near the bottom of the screen.
  function toggleNewMenu(): void {
    if (newMenuOpen) {
      newMenuOpen = false;
      return;
    }
    if (!triggerEl) {
      newMenuOpen = true;
      return;
    }
    const r = triggerEl.getBoundingClientRect();
    const viewportH =
      typeof window !== "undefined" && window.visualViewport
        ? window.visualViewport.height
        : window.innerHeight;
    const viewportW = window.innerWidth;
    const gap = 6;
    const margin = 8;
    const spaceBelow = viewportH - r.bottom;
    const spaceAbove = r.top;
    const top =
      spaceBelow >= POPOVER_HEIGHT + gap || spaceBelow >= spaceAbove
        ? r.bottom + gap
        : r.top - POPOVER_HEIGHT - gap;
    // Anchor the popover's right edge to the trigger's right edge,
    // then clamp horizontally so a trigger near either viewport
    // edge doesn't shoot off-screen.
    let left = r.right - POPOVER_WIDTH;
    if (left < margin) left = margin;
    if (left + POPOVER_WIDTH > viewportW - margin) {
      left = viewportW - margin - POPOVER_WIDTH;
    }
    popoverPos = { top, left };
    newMenuOpen = true;
  }

  async function newFileHere(): Promise<void> {
    newMenuOpen = false;
    await fileOps.createFile("");
  }

  async function newDirHere(): Promise<void> {
    newMenuOpen = false;
    await fileOps.createDir("");
  }

  function onWindowMousedown(e: MouseEvent): void {
    if (!newMenuOpen) return;
    const target = e.target as HTMLElement | null;
    if (target?.closest(".new-popover, .new-trigger")) return;
    newMenuOpen = false;
  }
</script>

<svelte:window onmousedown={onWindowMousedown} />

<OverlayShell open={visible} onClose={close}>
  <div class="browser">
    <header>
      <span class="name" title={drive.info?.root}>
        {drive.info?.name ?? "(unnamed)"}
      </span>
      <span class="actions">
        <span class="new-wrap">
          <button
            bind:this={triggerEl}
            class="hbtn new-trigger"
            class:on={newMenuOpen}
            title="New file or folder"
            aria-label="New"
            onclick={toggleNewMenu}
          >+</button>
        </span>
        <button
          class="hbtn"
          class:on={browserOverlay.inspectorOpen}
          title={browserOverlay.inspectorOpen ? "hide inspector" : "show inspector"}
          onclick={() => (browserOverlay.inspectorOpen = !browserOverlay.inspectorOpen)}
          aria-label="toggle inspector"
        >≡</button>
      </span>
    </header>
    <div class="body">
      <div class="tree-wrap">
        <FileTree />
      </div>
      {#if browserOverlay.inspectorOpen}
        <Inspector
          title="Details"
          bind:width={paneWidths.browser}
          onResize={persistPaneWidths}
        >
          <FileInfoBody
            path={browserSelection.path}
            onOpen={openSelected}
            onClose={clearSelection}
          />
        </Inspector>
      {/if}
    </div>
  </div>
</OverlayShell>

<!-- Popover lives outside the OverlayShell panel so it can render
     above OR below the trigger without getting clipped by the
     panel's overflow:hidden. Position is computed in
     `toggleNewMenu` against the viewport. -->
{#if visible && newMenuOpen}
  <ul
    class="new-popover"
    role="menu"
    style="top: {popoverPos.top}px; left: {popoverPos.left}px;"
  >
    <li>
      <button onclick={newFileHere}>
        <svg viewBox="0 0 16 16" aria-hidden="true">
          <path d="M2 1.75C2 .784 2.784 0 3.75 0h5.586c.464 0 .909.184 1.237.513l2.914 2.914c.329.328.513.773.513 1.237v9.586A1.75 1.75 0 0 1 12.25 16h-8.5A1.75 1.75 0 0 1 2 14.25V1.75zm1.75-.25a.25.25 0 0 0-.25.25v12.5c0 .138.112.25.25.25h8.5a.25.25 0 0 0 .25-.25V6h-2.75A1.75 1.75 0 0 1 8 4.25V1.5H3.75zM9.5 1.5v2.75c0 .138.112.25.25.25h2.5l-2.75-3z" />
        </svg>
        <span>New file</span>
      </button>
    </li>
    <li>
      <button onclick={newDirHere}>
        <svg viewBox="0 0 16 16" aria-hidden="true">
          <path d="M1.75 1A1.75 1.75 0 0 0 0 2.75v10.5C0 14.216.784 15 1.75 15h12.5A1.75 1.75 0 0 0 16 13.25v-8.5A1.75 1.75 0 0 0 14.25 3H7.5l-1.4-1.55A1.75 1.75 0 0 0 4.81 1H1.75z" />
        </svg>
        <span>New folder</span>
      </button>
    </li>
  </ul>
{/if}

<style>
  .browser {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
    min-width: 0;
    background: var(--bg);
    color: var(--text);
  }
  header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.4rem 0.6rem;
    border-bottom: 1px solid var(--border);
    background: var(--bg-card);
    font-weight: 600;
    font-size: 13px;
    color: var(--text-heading);
    flex-shrink: 0;
  }
  .name { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .actions { display: flex; gap: 2px; }
  .hbtn {
    background: none;
    border: 1px solid transparent;
    border-radius: 3px;
    cursor: pointer;
    color: var(--text-secondary);
    font: inherit;
    padding: 0 5px;
    line-height: 18px;
    height: 20px;
  }
  .hbtn:hover { color: var(--text); border-color: var(--btn-border); }
  .hbtn.on {
    color: var(--text);
    border-color: var(--btn-hover);
    background: var(--hover-bg);
  }
  .new-wrap { position: relative; display: inline-flex; }
  /* position: fixed against viewport coords computed in script;
     z-index above the overlay scrim (25000) so the menu paints in
     front. */
  .new-popover {
    position: fixed;
    z-index: 25500;
    margin: 0;
    padding: 4px 0;
    list-style: none;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: 6px;
    box-shadow: 0 6px 16px rgba(0, 0, 0, 0.25);
    min-width: 140px;
    font-size: 13px;
  }
  .new-popover li { margin: 0; }
  .new-popover button {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    text-align: left;
    background: none;
    border: 0;
    color: var(--text);
    padding: 6px 10px;
    cursor: pointer;
    font: inherit;
  }
  .new-popover button:hover {
    background: var(--hover-bg);
    color: var(--text);
  }
  .new-popover svg {
    width: 14px;
    height: 14px;
    flex-shrink: 0;
    fill: currentColor;
    color: var(--text-secondary);
  }
  .body {
    flex: 1;
    display: flex;
    min-height: 0;
    min-width: 0;
  }
  .tree-wrap {
    flex: 1;
    overflow-y: auto;
    padding: 0.25rem 0;
  }
</style>
