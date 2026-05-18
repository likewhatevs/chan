<script lang="ts">
  import { tick } from "svelte";
  import {
    ArrowLeft,
    ArrowRight,
    FilePlus,
    FolderOpen,
    FolderPlus,
    Maximize2,
    Minimize2,
    Network,
    PanelLeftOpen,
    PanelRightOpen,
    Pencil,
    Search,
    Settings,
    Users,
    X,
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
  import { chordFor } from "../state/shortcuts";
  import { isEditableText } from "../state/fileTypes";
  import {
    browserOverlay,
    browserSelection,
    browserSidePanes,
    collapseAllFolders,
    expandAllFolders,
    fileOps,
    isFullyExpanded,
    openFsGraphForDirectory,
    openFsGraphForFile,
    openGraphForDrive,
    openSettings,
    paneWidths,
    persistPaneWidths,
    refreshTree,
    searchPanel,
    setBrowserSidePane,
    toggleBrowserSidePane,
    tree,
    drive,
  } from "../state/store.svelte";
  import { openBrowserInActivePane, openInActivePane } from "../state/tabs.svelte";
  import type { BrowserTab } from "../state/tabs.svelte";
  import { fileBrowserTitlePath } from "../terminal/fromHere";

  type Variant = "overlay" | "dock" | "tab";
  type Side = "left" | "right";

  let {
    variant = "overlay",
    side,
    tab,
    onClose,
  }: {
    variant?: Variant;
    side?: Side;
    tab?: BrowserTab;
    onClose?: () => void;
  } = $props();

  const isOverlay = $derived(variant === "overlay");
  const isTab = $derived(variant === "tab");
  const isWideSurface = $derived(isOverlay || isTab);
  const browserState = $derived(tab ?? browserOverlay);
  const browserTitle = $derived(
    fileBrowserTitlePath(browserSelection.path, drive.info?.root ?? drive.info?.name ?? "drive"),
  );
  const fullyExpanded = $derived.by(() => {
    void tree.entries;
    return isFullyExpanded();
  });

  interface TreeRef {
    focusTree(): void;
    setFindQuery(q: string, onCount?: (total: number, current: number) => void): void;
    findStep(direction: 1 | -1): void;
    clearFind(): void;
  }

  let treeRef: TreeRef | undefined = $state();
  let findOpen = $state(false);
  let findQuery = $state("");
  let findCount = $state({ total: 0, current: -1 });
  let findInputEl: HTMLInputElement | undefined = $state();
  let menu: HamburgerMenu | undefined = $state();
  let menuOpen = $state(false);
  let importContactsOpen = $state(false);

  const POPOVER_HEIGHT = 420;
  const POPOVER_WIDTH = 240;

  $effect(() => {
    if ((isOverlay && browserOverlay.open) || isTab) {
      void tick().then(() => treeRef?.focusTree());
    }
  });

  function closeSurface(): void {
    onClose?.();
  }

  function openFind(): void {
    findOpen = true;
    void tick().then(() => findInputEl?.focus());
  }

  function closeFind(): void {
    findOpen = false;
    findQuery = "";
    treeRef?.clearFind();
    treeRef?.focusTree();
  }

  $effect(() => {
    if (!findOpen) return;
    treeRef?.setFindQuery(findQuery, (total, current) => {
      findCount = { total, current };
    });
  });

  function onFindKeydown(e: KeyboardEvent): void {
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      closeFind();
      return;
    }
    if (e.key === "Enter") {
      e.preventDefault();
      if (findCount.total === 0) return;
      treeRef?.findStep(e.shiftKey ? -1 : 1);
    }
  }

  function onBrowserKeydown(e: KeyboardEvent): void {
    if (e.key !== "f" && e.key !== "F") return;
    if (!(e.metaKey || e.ctrlKey)) return;
    if (e.altKey || e.shiftKey) return;
    e.preventDefault();
    openFind();
  }

  function toggleAll(): void {
    if (fullyExpanded) collapseAllFolders();
    else expandAllFolders();
    menu?.close();
  }

  function openSelected(): void {
    const path = browserSelection.path;
    if (!path) return;
    const entry = tree.entries.find((e) => e.path === path);
    if (entry && !entry.is_dir && isEditableText(entry.path)) {
      void openInActivePane(entry.path);
      if (isOverlay) closeSurface();
    }
  }

  function clearSelection(): void {
    browserSelection.path = null;
  }

  function graphSelection(): void {
    const path = browserSelection.path;
    if (path === null) return;
    const entry = tree.entries.find((e) => e.path === path);
    if (entry?.is_dir) openFsGraphForDirectory(path);
    else openFsGraphForFile(path);
  }

  function onBrowserContextMenu(e: MouseEvent): void {
    e.preventDefault();
    menu?.openAtCursor(e.clientX, e.clientY);
  }

  export function openMenuAtCursor(x: number, y: number): void {
    menu?.openAtCursor(x, y);
  }

  function toggleInspector(): void {
    browserState.inspectorOpen = !browserState.inspectorOpen;
    menu?.close();
  }

  function doToggleOverlayMaximized(): void {
    setOverlayMaximized(!overlayMaximized.on);
    menu?.close();
  }

  function toggleStick(target: Side): void {
    toggleBrowserSidePane(target);
    menu?.close();
  }

  function unstick(): void {
    if (!side) return;
    setBrowserSidePane(side, false);
  }

  function openOverlay(): void {
    openBrowserInActivePane();
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

  function graphDrive(): void {
    menu?.close();
    openGraphForDrive();
  }

  function searchDrive(): void {
    menu?.close();
    searchPanel.scopeId = "drive";
    searchPanel.open = true;
  }

  async function renameDrive(): Promise<void> {
    menu?.close();
    await fileOps.renameDrive();
  }

  function doOpenSettings(): void {
    menu?.close();
    openSettings();
  }

  function showDriveInfo(): void {
    menu?.close();
    browserSelection.path = null;
    browserSelection.showDrive = true;
    browserState.inspectorOpen = true;
  }

  function openImportContacts(): void {
    menu?.close();
    importContactsOpen = true;
  }

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
</script>

<div
  class="browser"
  class:dock={variant === "dock"}
  oncontextmenu={onBrowserContextMenu}
  onkeydown={onBrowserKeydown}
  role="presentation"
>
  <header>
    {#if isOverlay}
      <button
        type="button"
        class="chrome-btn"
        onclick={doToggleOverlayMaximized}
        title={overlayMaximized.on ? "Restore size" : "Maximize"}
        aria-label={overlayMaximized.on ? "Restore size" : "Maximize"}
      >
        {#if overlayMaximized.on}
          <Minimize2 size={14} strokeWidth={1.75} aria-hidden="true" />
        {:else}
          <Maximize2 size={14} strokeWidth={1.75} aria-hidden="true" />
        {/if}
      </button>
    {:else if variant === "dock"}
      <button
        type="button"
        class="chrome-btn"
        onclick={unstick}
        title={side === "right" ? "Unstick right" : "Unstick left"}
        aria-label={side === "right" ? "Unstick right" : "Unstick left"}
      >
        {#if side === "right"}
          <ArrowRight size={14} strokeWidth={1.75} aria-hidden="true" />
        {:else}
          <ArrowLeft size={14} strokeWidth={1.75} aria-hidden="true" />
        {/if}
      </button>
    {/if}
    <span class="name" title={browserTitle}>{browserTitle}</span>
    <HamburgerMenu
      bind:this={menu}
      bind:open={menuOpen}
      width={POPOVER_WIDTH}
      height={POPOVER_HEIGHT}
    >
      {@render menuItems()}
    </HamburgerMenu>
    {#if isWideSurface}
      <button
        type="button"
        class="chrome-btn close"
        onclick={closeSurface}
        title="Close"
        aria-label="Close"
      >
        <X size={14} strokeWidth={1.75} aria-hidden="true" />
      </button>
    {/if}
  </header>
  <div class="body">
    <div class="tree-wrap">
      {#if findOpen}
        <div class="find-bar" role="search" aria-label="find in file browser">
          <input
            bind:this={findInputEl}
            bind:value={findQuery}
            onkeydown={onFindKeydown}
            class="find-input"
            class:no-matches={findQuery !== "" && findCount.total === 0}
            type="text"
            placeholder="Find in visible entries"
            aria-label="find query"
            spellcheck="false"
            autocomplete="off"
          />
          <span class="find-counter" aria-live="polite">
            {#if findQuery === ""}
              {""}
            {:else if findCount.total === 0}
              0 of 0
            {:else}
              {findCount.current + 1} of {findCount.total}
            {/if}
          </span>
          <button
            type="button"
            class="find-btn"
            onclick={() => treeRef?.findStep(-1)}
            disabled={findCount.total === 0}
            title="previous match (Shift+Enter)"
            aria-label="previous match"
          >▲</button>
          <button
            type="button"
            class="find-btn"
            onclick={() => treeRef?.findStep(1)}
            disabled={findCount.total === 0}
            title="next match (Enter)"
            aria-label="next match"
          >▼</button>
          <button
            type="button"
            class="find-btn"
            onclick={closeFind}
            title="close (Esc)"
            aria-label="close find"
          >×</button>
        </div>
      {/if}
      <FileTree bind:this={treeRef} />
    </div>
    {#if isWideSurface && browserState.inspectorOpen}
      <Inspector
        title="Details"
        bind:width={paneWidths.browser}
        onResize={persistPaneWidths}
        onClose={() => (browserState.inspectorOpen = false)}
      >
        {#if browserSelection.showDrive && !browserSelection.path}
          <DriveInfoBody />
        {:else}
          <FileInfoBody
            path={browserSelection.path}
            onOpen={openSelected}
            onClose={clearSelection}
            onSetAsScope={graphSelection}
            showRefs
            onNavigate={(p) => {
              void openInActivePane(p);
              if (isOverlay) closeSurface();
            }}
          />
        {/if}
      </Inspector>
    {/if}
  </div>
</div>

{#snippet menuItems()}
  {#if variant === "dock"}
    <li>
      <button role="menuitem" onclick={openOverlay}>
        <Maximize2 size={16} strokeWidth={1.75} aria-hidden="true" />
        <span class="menu-row-label">Open overlay</span>
        <span class="menu-row-chord">{chordFor("app.files.toggle") ?? ""}</span>
      </button>
    </li>
  {/if}
  <li>
    <button role="menuitem" onclick={() => toggleStick("left")}>
      <PanelLeftOpen size={16} strokeWidth={1.75} aria-hidden="true" />
      <span class="menu-row-label">
        {browserSidePanes.left ? "Unstick left" : "Stick to left"}
      </span>
      <span class="menu-row-chord"></span>
    </button>
  </li>
  <li>
    <button role="menuitem" onclick={() => toggleStick("right")}>
      <PanelRightOpen size={16} strokeWidth={1.75} aria-hidden="true" />
      <span class="menu-row-label">
        {browserSidePanes.right ? "Unstick right" : "Stick to right"}
      </span>
      <span class="menu-row-chord"></span>
    </button>
  </li>
  <li class="sep" role="separator"></li>
  {#if isWideSurface}
    <li>
      <button role="menuitem" onclick={toggleInspector}>
        {#if browserState.inspectorOpen}
          <ArrowRight size={16} strokeWidth={1.75} aria-hidden="true" />
        {:else}
          <ArrowLeft size={16} strokeWidth={1.75} aria-hidden="true" />
        {/if}
        <span class="menu-row-label">
          {browserState.inspectorOpen ? "Hide Details" : "Show Details"}
        </span>
        <span class="menu-row-chord"></span>
      </button>
    </li>
    <li class="sep" role="separator"></li>
  {/if}
  <li>
    <button role="menuitem" onclick={newFileHere}>
      <FilePlus size={16} strokeWidth={1.75} aria-hidden="true" />
      <span class="menu-row-label">New file</span>
      <span class="menu-row-chord">{chordFor("app.file.new") ?? ""}</span>
    </button>
  </li>
  <li>
    <button role="menuitem" onclick={newDirHere}>
      <FolderPlus size={16} strokeWidth={1.75} aria-hidden="true" />
      <span class="menu-row-label">New directory</span>
      <span class="menu-row-chord"></span>
    </button>
  </li>
  <li>
    <button role="menuitem" onclick={openImportContacts}>
      <Users size={16} strokeWidth={1.75} aria-hidden="true" />
      <span class="menu-row-label">Import contacts...</span>
      <span class="menu-row-chord"></span>
    </button>
  </li>
  <li>
    <button role="menuitem" onclick={graphDrive}>
      <Network size={16} strokeWidth={1.75} aria-hidden="true" />
      <span class="menu-row-label">Graph from here</span>
      <span class="menu-row-chord"></span>
    </button>
  </li>
  <li>
    <button role="menuitem" onclick={searchDrive}>
      <Search size={16} strokeWidth={1.75} aria-hidden="true" />
      <span class="menu-row-label">Search this</span>
      <span class="menu-row-chord"></span>
    </button>
  </li>
  <li class="sep" role="separator"></li>
  <li>
    <button role="menuitem" onclick={toggleAll}>
      <span class="glyph" aria-hidden="true">⇅</span>
      <span class="menu-row-label">
        {fullyExpanded ? "Collapse all directories" : "Expand all directories"}
      </span>
      <span class="menu-row-chord"></span>
    </button>
  </li>
  <li>
    <button role="menuitem" onclick={reloadTree}>
      <span class="glyph" aria-hidden="true">↻</span>
      <span class="menu-row-label">Reload</span>
      <span class="menu-row-chord"></span>
    </button>
  </li>
  <li class="sep" role="separator"></li>
  <li>
    <button role="menuitem" onclick={renameDrive}>
      <Pencil size={16} strokeWidth={1.75} aria-hidden="true" />
      <span class="menu-row-label">Rename drive...</span>
      <span class="menu-row-chord"></span>
    </button>
  </li>
  <li>
    <button
      role="menuitem"
      class="folder-row"
      onclick={showDriveInfo}
      title={drive.info?.root}
      disabled={!drive.info?.root}
    >
      <FolderOpen size={16} strokeWidth={1.75} aria-hidden="true" />
      <span class="folder-text">
        <span class="folder-label">Directory</span>
        <span class="folder-path mono">{drive.info?.root ?? ""}</span>
      </span>
    </button>
  </li>
  <li class="sep" role="separator"></li>
  <li>
    <button role="menuitem" onclick={doOpenSettings}>
      <Settings size={14} strokeWidth={1.75} aria-hidden="true" />
      <span class="menu-row-label">Settings</span>
      <span class="menu-row-chord">{chordFor("app.settings.toggle") ?? ""}</span>
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
  .dock {
    border-inline: 1px solid var(--border);
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
  .dock header {
    padding-inline: 0.45rem;
  }
  .name { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .chrome-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 26px;
    height: 24px;
    padding: 0;
    background: var(--bg);
    color: var(--text-secondary);
    border: 1px solid var(--border);
    border-radius: 4px;
    cursor: pointer;
    transition: color 0.15s ease, border-color 0.15s ease;
    flex-shrink: 0;
  }
  .chrome-btn:hover {
    color: var(--text);
    border-color: var(--btn-hover);
  }
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
    direction: rtl;
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
    position: relative;
  }
  .find-bar {
    position: sticky;
    top: 0;
    z-index: 5;
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 4px 6px;
    background: var(--bg-card);
    border-bottom: 1px solid var(--border);
    font-size: 13px;
    color: var(--text);
  }
  .find-input {
    flex: 1;
    min-width: 0;
    background: var(--bg);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 4px 6px;
    font: inherit;
    outline: none;
  }
  .find-input:focus {
    border-color: var(--accent, var(--btn-hover));
  }
  .find-input.no-matches {
    box-shadow: 0 0 0 1px #d33 inset;
  }
  .find-counter {
    min-width: 56px;
    text-align: right;
    font-variant-numeric: tabular-nums;
    color: var(--text-secondary);
    font-size: 12px;
    padding: 0 2px;
  }
  .find-btn {
    background: none;
    border: 1px solid transparent;
    border-radius: 4px;
    color: var(--text-secondary);
    cursor: pointer;
    font: inherit;
    font-size: 13px;
    line-height: 1;
    padding: 3px 6px;
  }
  .find-btn:hover:not(:disabled) {
    background: var(--hover-bg);
    color: var(--text);
  }
  .find-btn:disabled {
    opacity: 0.4;
    cursor: default;
  }
</style>
