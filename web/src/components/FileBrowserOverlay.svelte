<script lang="ts">
  // File browser overlay. The recursive tree on the left, the
  // shared Inspector on the right (file/folder metadata for the
  // current selection), wrapped in OverlayShell so it floats over
  // the workspace pane tree on every platform (web, native desktop,
  // mobile). Replaces the previous `BrowserTab` tab kind and the
  // native-only special-tab WebviewWindow.

  import { tick } from "svelte";
  import FileTree from "./FileTree.svelte";
  import Inspector from "./Inspector.svelte";
  import FileInfoBody from "./FileInfoBody.svelte";
  import ImportContactsModal from "./ImportContactsModal.svelte";
  import OverlayShell from "./OverlayShell.svelte";
  import { isEditableText } from "../state/fileTypes";
  import {
    browserOverlay,
    browserSelection,
    collapseAllFolders,
    expandAllFolders,
    fileOps,
    isFullyExpanded,
    paneWidths,
    persistPaneWidths,
    tree,
    drive,
  } from "../state/store.svelte";
  import { openInActivePane } from "../state/tabs.svelte";

  const visible = $derived(browserOverlay.open);

  /// Drives the expand-all / collapse-all glyph + title. Reads
  /// reactive state directly so the button label flips as soon as
  /// the user toggles a single folder twirl.
  const fullyExpanded = $derived.by(() => {
    void tree.entries;
    return isFullyExpanded();
  });

  function toggleAll(): void {
    if (fullyExpanded) collapseAllFolders();
    else expandAllFolders();
    menuOpen = false;
  }

  /// FileTree exposes `focusTree()` for keyboard nav. Pull it on
  /// every open of the overlay so arrows / Enter are immediately
  /// live. `tick()` waits for the OverlayShell mount so the tree
  /// element exists in the DOM before we focus it.
  let treeRef: { focusTree(): void } | undefined = $state();
  $effect(() => {
    if (browserOverlay.open) {
      void tick().then(() => treeRef?.focusTree());
    }
  });

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

  /// Hamburger popover state. Holds: New file, New folder, Rename
  /// drive, and a click-to-copy row exposing the on-disk drive
  /// folder path. Closes on outside click via the click-capture
  /// handler on the popover root.
  let menuOpen = $state(false);
  let triggerEl: HTMLButtonElement | undefined = $state();
  let popoverPos = $state<{ top: number; left: number }>({ top: 0, left: 0 });

  /// Roughly the rendered size of the popover. We pre-compute layout
  /// before the popover mounts so it never flashes in the wrong
  /// place; these constants approximate the real footprint of the
  /// menu (4 create/action rows + separator + folder readout).
  /// The folder readout can be long but is clipped by overflow:
  /// hidden so the height stays predictable.
  const POPOVER_HEIGHT = 256;
  const POPOVER_WIDTH = 240;

  /// "Copied" flash state for the folder-path row. Reset after
  /// COPIED_FLASH_MS so the indicator doesn't stick.
  let copiedFlash = $state(false);
  let copiedTimer: ReturnType<typeof setTimeout> | null = null;
  const COPIED_FLASH_MS = 1200;

  /// Import-contacts wizard. Opens from the popover; closes back
  /// to the file browser. The wizard refreshes the tree on
  /// success so the new notes show up immediately.
  let importContactsOpen = $state(false);

  function openImportContacts(): void {
    menuOpen = false;
    importContactsOpen = true;
  }

  /// Sensible default destination for the import wizard. If the
  /// user has a folder selected in the tree, suggest that.
  /// Otherwise default to "Contacts" so the typical first-run lands
  /// in a single, named folder rather than scattering at the drive
  /// root.
  function pickInitialFolder(sel: string | null): string {
    if (!sel) return "Contacts";
    const entry = tree.entries.find((e) => e.path === sel);
    if (entry?.is_dir) return entry.path;
    if (entry && !entry.is_dir) {
      const slash = entry.path.lastIndexOf("/");
      return slash > 0 ? entry.path.slice(0, slash) : "";
    }
    return "Contacts";
  }

  /// Toggle the popover. When opening, measure the trigger button
  /// against the viewport and pick "below" or "above" depending on
  /// which side has more room. Without this the menu was clipped
  /// outside the OverlayShell panel whenever the file browser sat
  /// near the bottom of the screen.
  function toggleMenu(): void {
    if (menuOpen) {
      menuOpen = false;
      return;
    }
    if (!triggerEl) {
      menuOpen = true;
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
    menuOpen = true;
  }

  async function newFileHere(): Promise<void> {
    menuOpen = false;
    await fileOps.createFile("");
  }

  async function newDirHere(): Promise<void> {
    menuOpen = false;
    await fileOps.createDir("");
  }

  async function renameDrive(): Promise<void> {
    menuOpen = false;
    await fileOps.renameDrive();
  }

  /// Copy the drive folder path to the system clipboard. Leaves
  /// the menu open so the user sees the "copied" flash without
  /// having to reopen it. Silently no-ops if the clipboard write
  /// rejects (rare on localhost; we don't need to surface it).
  async function copyFolder(): Promise<void> {
    const root = drive.info?.root;
    if (!root) return;
    try {
      await navigator.clipboard.writeText(root);
      copiedFlash = true;
      if (copiedTimer) clearTimeout(copiedTimer);
      copiedTimer = setTimeout(() => {
        copiedFlash = false;
        copiedTimer = null;
      }, COPIED_FLASH_MS);
    } catch {
      // ignore: localhost clipboard writes essentially never fail
    }
  }

  function onWindowMousedown(e: MouseEvent): void {
    if (!menuOpen) return;
    const target = e.target as HTMLElement | null;
    if (target?.closest(".browser-menu, .menu-trigger")) return;
    menuOpen = false;
  }
</script>

<svelte:window onmousedown={onWindowMousedown} />

<OverlayShell id="browser" open={visible} onClose={close}>
  <div class="browser">
    <header>
      <span class="name" title={drive.info?.root}>
        {drive.info?.name ?? "(unnamed)"}
      </span>
      <span class="actions">
        <span class="menu-wrap">
          <button
            bind:this={triggerEl}
            class="hbtn menu-trigger"
            class:on={menuOpen}
            title="Drive actions"
            aria-label="Drive actions"
            aria-haspopup="menu"
            aria-expanded={menuOpen}
            onclick={toggleMenu}
          >⋮</button>
        </span>
        <button
          class="hbtn"
          class:on={browserOverlay.inspectorOpen}
          title={browserOverlay.inspectorOpen ? "hide inspector" : "show inspector"}
          onclick={() => (browserOverlay.inspectorOpen = !browserOverlay.inspectorOpen)}
          aria-label="toggle inspector"
        >◫</button>
      </span>
    </header>
    <div class="body">
      <div class="tree-wrap">
        <FileTree bind:this={treeRef} />
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
            showRefs
            onNavigate={(p) => {
              void openInActivePane(p);
              close();
            }}
          />
        </Inspector>
      {/if}
    </div>
  </div>
</OverlayShell>

<!-- Popover lives outside the OverlayShell panel so it can render
     above OR below the trigger without getting clipped by the
     panel's overflow:hidden. Position is computed in
     `toggleMenu` against the viewport. -->
{#if visible && menuOpen}
  <ul
    class="browser-menu"
    role="menu"
    style="top: {popoverPos.top}px; left: {popoverPos.left}px;"
  >
    <li>
      <button role="menuitem" onclick={newFileHere}>
        <svg viewBox="0 0 16 16" aria-hidden="true">
          <path d="M2 1.75C2 .784 2.784 0 3.75 0h5.586c.464 0 .909.184 1.237.513l2.914 2.914c.329.328.513.773.513 1.237v9.586A1.75 1.75 0 0 1 12.25 16h-8.5A1.75 1.75 0 0 1 2 14.25V1.75zm1.75-.25a.25.25 0 0 0-.25.25v12.5c0 .138.112.25.25.25h8.5a.25.25 0 0 0 .25-.25V6h-2.75A1.75 1.75 0 0 1 8 4.25V1.5H3.75zM9.5 1.5v2.75c0 .138.112.25.25.25h2.5l-2.75-3z" />
        </svg>
        <span>New file</span>
      </button>
    </li>
    <li>
      <button role="menuitem" onclick={newDirHere}>
        <svg viewBox="0 0 16 16" aria-hidden="true">
          <path d="M1.75 1A1.75 1.75 0 0 0 0 2.75v10.5C0 14.216.784 15 1.75 15h12.5A1.75 1.75 0 0 0 16 13.25v-8.5A1.75 1.75 0 0 0 14.25 3H7.5l-1.4-1.55A1.75 1.75 0 0 0 4.81 1H1.75z" />
        </svg>
        <span>New folder</span>
      </button>
    </li>
    <li>
      <button role="menuitem" onclick={openImportContacts}>
        <svg viewBox="0 0 16 16" aria-hidden="true">
          <path d="M8 8a3 3 0 1 0 0-6 3 3 0 0 0 0 6zm-1.5 1.5A4.5 4.5 0 0 0 2 14h12a4.5 4.5 0 0 0-4.5-4.5h-3z"/>
        </svg>
        <span>Import contacts…</span>
      </button>
    </li>
    <li class="sep" role="separator"></li>
    <li>
      <button role="menuitem" onclick={toggleAll}>
        <span class="glyph" aria-hidden="true">⇅</span>
        <span>{fullyExpanded ? "Collapse all folders" : "Expand all folders"}</span>
      </button>
    </li>
    <li class="sep" role="separator"></li>
    <li>
      <button role="menuitem" onclick={renameDrive}>
        <svg viewBox="0 0 16 16" aria-hidden="true">
          <path d="M11.013 1.427a1.75 1.75 0 0 1 2.474 0l1.086 1.086a1.75 1.75 0 0 1 0 2.474l-8.61 8.61c-.21.21-.47.364-.756.445l-3.251.93a.75.75 0 0 1-.927-.928l.929-3.25c.081-.286.235-.547.445-.756l8.61-8.61zm.176 4.823L9.75 4.81l-6.286 6.287a.253.253 0 0 0-.064.108l-.558 1.953 1.953-.558a.253.253 0 0 0 .108-.064l6.286-6.286zM10.811 3.75 12.25 5.189l1.227-1.227a.25.25 0 0 0 0-.354l-1.085-1.085a.25.25 0 0 0-.354 0L10.811 3.75z" />
        </svg>
        <span>Rename drive…</span>
      </button>
    </li>
    <li>
      <!-- Folder readout doubles as the disclosure ("where on disk
           is this drive?") and the copy-to-clipboard action. We
           keep the menu open after copy so the flash is visible. -->
      <button
        role="menuitem"
        class="folder-row"
        onclick={copyFolder}
        title={drive.info?.root}
        disabled={!drive.info?.root}
      >
        <svg viewBox="0 0 16 16" aria-hidden="true">
          <path d="M0 6.75C0 5.784.784 5 1.75 5h1.5a.75.75 0 0 1 0 1.5h-1.5a.25.25 0 0 0-.25.25V13.25c0 .138.112.25.25.25h12.5a.25.25 0 0 0 .25-.25V6.75a.25.25 0 0 0-.25-.25h-1.5a.75.75 0 0 1 0-1.5h1.5c.966 0 1.75.784 1.75 1.75v6.5A1.75 1.75 0 0 1 14.25 15H1.75A1.75 1.75 0 0 1 0 13.25V6.75z M7.25 0a.75.75 0 0 1 .75.75v6.5l1.97-1.97a.749.749 0 1 1 1.06 1.06l-3.25 3.25a.749.749 0 0 1-1.06 0L3.47 6.34a.749.749 0 1 1 1.06-1.06l1.97 1.97V.75A.75.75 0 0 1 7.25 0z" />
        </svg>
        <span class="folder-text">
          <span class="folder-label">Folder</span>
          <span class="folder-path mono">{drive.info?.root ?? ""}</span>
        </span>
        <span class="copied-hint" class:on={copiedFlash}>copied</span>
      </button>
    </li>
  </ul>
{/if}

<ImportContactsModal
  open={importContactsOpen}
  defaultDir={pickInitialFolder(browserSelection.path)}
  onClose={() => (importContactsOpen = false)}
/>

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
    font-size: 15px;
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
  .menu-wrap { position: relative; display: inline-flex; }
  /* Bubble menu, sibling of the tab-menu bubble in FileEditorTab.
     position: fixed against viewport coords computed in script;
     z-index above the overlay scrim (25000) so the menu paints in
     front. Bouncy reveal + hover scale match the tab-menu bubble so
     the two menus read as the same affordance across the app. */
  .browser-menu {
    position: fixed;
    z-index: 25500;
    margin: 0;
    padding: 6px;
    list-style: none;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: 8px;
    box-shadow: 0 6px 20px rgba(0, 0, 0, 0.18);
    min-width: 220px;
    max-width: calc(100vw - 16px);
    max-height: calc(100vh - 24px);
    overflow-y: auto;
    font-size: 13px;
    color: var(--text);
    transform-origin: top left;
    animation: bubble-pop 260ms cubic-bezier(0.34, 1.56, 0.64, 1);
    transition: transform 200ms cubic-bezier(0.34, 1.56, 0.64, 1);
  }
  .browser-menu:hover { transform: scale(1.015); }
  @keyframes bubble-pop {
    0% { opacity: 0; transform: scale(0.92); }
    100% { opacity: 1; transform: scale(1); }
  }
  @media (prefers-reduced-motion: reduce) {
    .browser-menu { animation: none; transition: none; }
    .browser-menu:hover { transform: none; }
  }
  .browser-menu li { margin: 0; }
  .browser-menu li.sep {
    height: 1px;
    background: var(--separator, var(--border));
    margin: 4px 2px;
  }
  .browser-menu button {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    text-align: left;
    background: none;
    border: 0;
    border-radius: 4px;
    color: var(--text);
    padding: 6px 8px;
    cursor: pointer;
    font: inherit;
    font-size: 13px;
  }
  .browser-menu button:hover:not(:disabled) {
    background: var(--hover-bg);
    color: var(--text);
  }
  .browser-menu button:disabled {
    cursor: default;
    opacity: 0.6;
  }
  .browser-menu svg {
    width: 14px;
    height: 14px;
    flex-shrink: 0;
    fill: currentColor;
    color: var(--text-secondary);
  }
  .browser-menu .glyph {
    width: 14px;
    text-align: center;
    color: var(--text-secondary);
    flex-shrink: 0;
    font-size: 14px;
    line-height: 1;
  }
  /* Folder readout: two-line inside one row. Top line is the
     "Folder" label, bottom line is the on-disk path in monospace
     and ellipsised so a long path doesn't blow up the popover. */
  .browser-menu .folder-row { align-items: flex-start; }
  .folder-text {
    display: flex;
    flex-direction: column;
    gap: 1px;
    min-width: 0;
    flex: 1;
  }
  .folder-label {
    font-size: 12px;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .folder-path {
    font-size: 12px;
    color: var(--text-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    direction: rtl; /* keep the basename visible when truncated */
    text-align: left;
  }
  .mono { font-family: ui-monospace, SFMono-Regular, Menlo, monospace; }
  .copied-hint {
    font-size: 12px;
    color: var(--text-secondary);
    opacity: 0;
    transition: opacity 120ms ease-out;
    flex-shrink: 0;
    align-self: center;
  }
  .copied-hint.on { opacity: 1; color: var(--ok, var(--text)); }
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
