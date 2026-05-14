<script lang="ts">
  // File browser overlay. The recursive tree on the left, the
  // shared Inspector on the right (file/folder metadata for the
  // current selection), wrapped in OverlayShell so it floats over
  // the workspace pane tree on every platform (web, native desktop,
  // mobile). Replaces the previous `BrowserTab` tab kind and the
  // native-only special-tab WebviewWindow.

  import { tick } from "svelte";
  import {
    ArrowLeft,
    ArrowRight,
    FilePlus,
    FolderOpen,
    FolderPlus,
    Maximize2,
    Minimize2,
    Pencil,
    Settings,
    Users,
  } from "lucide-svelte";
  import {
    overlayMaximized,
    setOverlayMaximized,
  } from "../state/pageWidth.svelte";
  import FileTree from "./FileTree.svelte";
  import Inspector from "./Inspector.svelte";
  import FileInfoBody from "./FileInfoBody.svelte";
  import DriveInfoBody from "./DriveInfoBody.svelte";
  import HamburgerMenu from "./HamburgerMenu.svelte";
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
    openSettings,
    paneWidths,
    persistPaneWidths,
    refreshTree,
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
    menu?.close();
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

  /// Hamburger menu handle. The shared HamburgerMenu component owns
  /// trigger placement, viewport clamping, and outside-click dismiss
  /// (matched to the other overlays — search, graph). We just hold
  /// the ref so the contextmenu handler can re-open at the cursor.
  let menu: HamburgerMenu | undefined = $state();
  let menuOpen = $state(false);
  const POPOVER_HEIGHT = 360;
  const POPOVER_WIDTH = 240;

  /// Import-contacts wizard. Opens from the popover; closes back
  /// to the file browser. The wizard refreshes the tree on
  /// success so the new notes show up immediately.
  let importContactsOpen = $state(false);

  function openImportContacts(): void {
    menu?.close();
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

  /// Right-click handler on the browser body. FileTree's per-row
  /// context menu calls stopPropagation, so this only fires on empty
  /// space (the gutter below the last row, the header background,
  /// the inspector pane). Opens the same drive-actions menu the
  /// hamburger does, anchored at the cursor.
  function onBrowserContextMenu(e: MouseEvent): void {
    e.preventDefault();
    menu?.openAtCursor(e.clientX, e.clientY);
  }

  function toggleInspector(): void {
    browserOverlay.inspectorOpen = !browserOverlay.inspectorOpen;
    menu?.close();
  }

  function doToggleOverlayMaximized(): void {
    setOverlayMaximized(!overlayMaximized.on);
    menu?.close();
  }

  async function reloadTree(): Promise<void> {
    menu?.close();
    await refreshTree();
  }

  async function newFileHere(): Promise<void> {
    menu?.close();
    await fileOps.createFile("");
  }

  async function newDirHere(): Promise<void> {
    menu?.close();
    await fileOps.createDir("");
  }

  async function renameDrive(): Promise<void> {
    menu?.close();
    await fileOps.renameDrive();
  }

  function doOpenSettings(): void {
    menu?.close();
    openSettings();
  }

  /// Pop the drive-info inspector body. Clears the file selection
  /// (file vs. drive view is exclusive) and force-opens the
  /// inspector if it was hidden so the click doesn't no-op.
  function showDriveInfo(): void {
    menu?.close();
    browserSelection.path = null;
    browserSelection.showDrive = true;
    browserOverlay.inspectorOpen = true;
  }

</script>

<OverlayShell id="browser" open={visible} onClose={close}>
  <div class="browser" oncontextmenu={onBrowserContextMenu} role="presentation">
    <header>
      <span class="name" title={drive.info?.root}>
        {drive.info?.name ?? "(unnamed)"}
      </span>
      <HamburgerMenu
        bind:this={menu}
        bind:open={menuOpen}
        width={POPOVER_WIDTH}
        height={POPOVER_HEIGHT}
      >
        {@render menuItems()}
      </HamburgerMenu>
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
          onClose={() => (browserOverlay.inspectorOpen = false)}
        >
          {#if browserSelection.showDrive && !browserSelection.path}
            <DriveInfoBody />
          {:else}
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
          {/if}
        </Inspector>
      {/if}
    </div>
  </div>
</OverlayShell>

<!-- Popover lives outside the OverlayShell panel so it can render
     above OR below the trigger without getting clipped by the
     panel's overflow:hidden. Position is computed in
     `toggleMenu` against the viewport. -->
{#snippet menuItems()}
  <!-- Order across all overlay menus: view toggles, create/action,
       view ops (expand / reload / sliders), identity. Each group
       gets a separator above the next. -->
  <li>
    <button role="menuitem" onclick={toggleInspector}>
      {#if browserOverlay.inspectorOpen}
        <ArrowRight size={16} strokeWidth={1.75} aria-hidden="true" />
      {:else}
        <ArrowLeft size={16} strokeWidth={1.75} aria-hidden="true" />
      {/if}
      <span>{browserOverlay.inspectorOpen ? "Hide Details" : "Show Details"}</span>
    </button>
  </li>
  <li>
    <button role="menuitem" onclick={doToggleOverlayMaximized}>
      {#if overlayMaximized.on}
        <Minimize2 size={14} strokeWidth={1.75} aria-hidden="true" />
        <span>Restore size</span>
      {:else}
        <Maximize2 size={14} strokeWidth={1.75} aria-hidden="true" />
        <span>Maximize</span>
      {/if}
    </button>
  </li>
  <li class="sep" role="separator"></li>
  <li>
    <button role="menuitem" onclick={newFileHere}>
      <FilePlus size={16} strokeWidth={1.75} aria-hidden="true" />
      <span>New file</span>
    </button>
  </li>
  <li>
    <button role="menuitem" onclick={newDirHere}>
      <FolderPlus size={16} strokeWidth={1.75} aria-hidden="true" />
      <span>New folder</span>
    </button>
  </li>
  <li>
    <button role="menuitem" onclick={openImportContacts}>
      <Users size={16} strokeWidth={1.75} aria-hidden="true" />
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
  <li>
    <button role="menuitem" onclick={reloadTree}>
      <span class="glyph" aria-hidden="true">↻</span>
      <span>Reload</span>
    </button>
  </li>
  <li class="sep" role="separator"></li>
  <li>
    <button role="menuitem" onclick={renameDrive}>
      <Pencil size={16} strokeWidth={1.75} aria-hidden="true" />
      <span>Rename drive…</span>
    </button>
  </li>
  <li>
    <!-- Folder readout doubles as the disclosure ("where on disk
         is this drive?") and the entry point into the drive
         inspector (search index status, notes folders config). -->
    <button
      role="menuitem"
      class="folder-row"
      onclick={showDriveInfo}
      title={drive.info?.root}
      disabled={!drive.info?.root}
    >
      <FolderOpen size={16} strokeWidth={1.75} aria-hidden="true" />
      <span class="folder-text">
        <span class="folder-label">Folder</span>
        <span class="folder-path mono">{drive.info?.root ?? ""}</span>
      </span>
    </button>
  </li>
  <li class="sep" role="separator"></li>
  <li>
    <button role="menuitem" onclick={doOpenSettings}>
      <Settings size={14} strokeWidth={1.75} aria-hidden="true" />
      <span>Settings</span>
    </button>
  </li>
{/snippet}

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
  /* Folder readout row: two-line inside the menu's standard <li>.
     The shared HamburgerMenu owns the rest of the popover chrome. */
  :global(.hamburger-menu .folder-row) { align-items: flex-start; }
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
