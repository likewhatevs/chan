<script lang="ts">
  import { tick, untrack } from "svelte";
  import {
    ArrowLeft,
    ArrowRight,
    FolderOpen,
    HardDrive,
    History,
    Maximize2,
    Minimize2,
    Network,
    PanelLeftOpen,
    PanelRightOpen,
    Settings2,
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
  import WorkspaceInfoBody from "./WorkspaceInfoBody.svelte";
  import HamburgerMenu from "./HamburgerMenu.svelte";
  import ImportContactsModal from "./ImportContactsModal.svelte";
  import { tabMenu } from "../state/tabMenu.svelte";
  import { chordFor } from "../state/shortcuts";
  import { isEditableText } from "../state/fileTypes";
  import {
    browserSelection,
    browserSidePanes,
    collapseAllFoldersForInstance,
    expandAllFoldersForInstance,
    ensureFbTreeInstance,
    fbTreeInstance,
    fbSelectSet,
    fbSelectSingle,
    fileOps,
    isFullyExpandedForInstance,
    openFsGraphForDirectory,
    openFsGraphForFile,
    paneWidths,
    persistPaneWidths,
    persistLayoutToHash,
    persistFbTreeInstanceExpansion,
    refreshTree,
    seedFbTreeInstanceFromReloadSnapshot,
    surfaceThemeOverride,
    toggleBrowserSidePane,
    tree,
    workspace,
  } from "../state/store.svelte";
  import {
    canReopenClosedTab,
    openBrowserInActivePane,
    openInActivePane,
    reopenClosedTab,
  } from "../state/tabs.svelte";
  import type { BrowserTab } from "../state/tabs.svelte";
  import {
    fbWatchRegister,
    fbWatchReconcile,
    fbWatchDispose,
  } from "../state/fbWatch.svelte";
  import { api } from "../api/client";

  type Variant = "overlay" | "dock" | "tab";
  type Side = "left" | "right";

  let {
    variant = "overlay",
    side,
    tab,
    onClose,
    onFlip,
  }: {
    variant?: Variant;
    side?: Side;
    tab?: BrowserTab;
    onClose?: () => void;
    // `fullstack-a-67e`: parent (Pane.svelte) supplies the flip
    // callback for the tab variant. Dock + overlay variants
    // don't pass one — the Settings (flip) entry hides for
    // those variants since there's no Hybrid back-side to flip
    // to.
    onFlip?: () => void;
  } = $props();

  const isOverlay = $derived(variant === "overlay");
  const isTab = $derived(variant === "tab");
  const isDock = $derived(variant === "dock");
  const isWideSurface = $derived(isOverlay || isTab);
  /// The dock variant does NOT render the inspector (`isWideSurface` is
  /// false for docks), so its inspectorOpen / inspectorWidth are
  /// write-only and unread; it uses this minimal local state. The tab
  /// variant uses its tab. The browser overlay variant + its shared
  /// `browserOverlay` singleton were retired by the scope-concept wipe
  /// (A5/W6), so non-tab surfaces fall back to the local dock state.
  const dockBrowserState = $state<{ inspectorOpen: boolean; inspectorWidth?: number }>(
    { inspectorOpen: false },
  );
  const browserState = $derived(tab ?? dockBrowserState);
  const fullyExpanded = $derived.by(() => {
    void tree.entries;
    // READ the per-instance map so a dirty re-eval picks up THIS surface's
    // expansion, not a sibling's. Must NOT call ensureFbTreeInstance here
    // (it mutates $state -> state_unsafe_mutation inside a $derived); the
    // instance is created by the subscription-reconcile effect below.
    void fbTreeInstance(instanceId)?.expanded;
    return isFullyExpandedForInstance(instanceId);
  });

  // ---- per-instance scoped /ws subscriptions (phase-11 Slice E) ----------
  //
  // Each File Browser surface is one watcher-scope instance. A stable id
  // keys its subscription bookkeeping in the `fbTreeInstances` registry:
  // the tab variant uses its tab id; the overlay and the two dock sides
  // are singletons. On mount the instance subscribes to the workspace root
  // (so root-level fs changes broadcast to it); as directories expand /
  // collapse it subscribes / unsubscribes the matching dir scopes, with
  // the LAST instance to drop a dir tearing the server watcher down. The
  // expanded-dir set is read from the instance's OWN per-instance
  // `expanded` map (Slice E), which is now also the render source: the
  // tree and the subscription bookkeeping share one per-instance map, so
  // expanding a dir in one surface no longer fans out to the others.
  const instanceId = $derived(
    isTab && tab ? `fb-tab-${tab.id}` : isDock ? `fb-dock-${side ?? "left"}` : "fb-overlay",
  );

  $effect(() => {
    const id = instanceId;
    untrack(() => fbWatchRegister(id));
    // The dock / overlay surfaces have no layout home for their expansion,
    // so seed from the per-instance reload snapshot on register. The tab
    // variant seeds from `tab.expanded` in `restoreFromTab` instead
    // (authoritative across app restart, not just reload).
    if (!isTab) untrack(() => seedFbTreeInstanceFromReloadSnapshot(id));
    return () => untrack(() => fbWatchDispose(id));
  });

  // Reconcile this instance's dir subscriptions against the directories
  // currently expanded in the tree it renders. The per-instance `expanded`
  // map is the reactive source; recompute the expanded-dir list (root
  // excluded) and let the manager diff it against what this instance holds.
  $effect(() => {
    const id = instanceId;
    const map = ensureFbTreeInstance(id).expanded;
    const dirs = Object.keys(map).filter((p) => p.length > 0 && map[p]);
    untrack(() => fbWatchReconcile(id, dirs));
  });

  interface TreeRef {
    focusTree(): void;
    setFindQuery(q: string, onCount?: (total: number, current: number) => void): void;
    findStep(direction: 1 | -1): void;
    clearFind(): void;
  }

  let treeRef: TreeRef | undefined = $state();
  let treeWrapEl: HTMLDivElement | undefined = $state();
  let findOpen = $state(false);
  let findQuery = $state("");
  let findCount = $state({ total: 0, current: -1 });
  let findInputEl: HTMLInputElement | undefined = $state();
  let menu: HamburgerMenu | undefined = $state();
  let menuOpen = $state(false);
  let importContactsOpen = $state(false);

  /// `fullstack-58`: per-tab File Browser view state.
  /// When this surface renders for a tab (variant === "tab"), the
  /// active tab "owns" the module-level `browserSelection` singleton +
  /// the treeWrap scroll, and (Slice E) its OWN per-instance expansion
  /// map keyed by `fb-tab-<id>`. On tab swap the surface unmounts, which
  /// disposes the tab's instance; on (re)activation we snapshot the live
  /// state onto the tab record and restore it back, so the activating
  /// tab's expansion is reseeded from `tab.expanded` into a fresh
  /// instance. The dock + overlay variants own their own instance maps
  /// independently.
  function snapshotIntoTab(target: BrowserTab): void {
    target.selected = browserSelection.path ?? undefined;
    // The multi-selection travels with the tab too (FB capabilities).
    // Only persist when it's a genuine multi-set; a single/empty
    // selection is fully described by `selected` and restores from it.
    const multi = browserSelection.paths;
    target.selectedPaths = multi.length > 1 ? [...multi] : undefined;
    target.showWorkspace = browserSelection.showWorkspace ? true : undefined;
    const map = ensureFbTreeInstance(`fb-tab-${target.id}`).expanded;
    const expanded = Object.keys(map).filter((p) => p.length > 0 && map[p]);
    target.expanded = expanded.length > 0 ? expanded : undefined;
    const scroll = treeWrapEl?.scrollTop ?? 0;
    target.scroll = scroll > 0 ? Math.round(scroll) : undefined;
  }

  function restoreFromTab(source: BrowserTab): void {
    const active = source.selected ?? null;
    const multi = source.selectedPaths;
    if (multi && multi.length > 1) {
      // Restore the full multi-set with the saved active entry as cursor.
      fbSelectSet(multi, active ?? undefined);
    } else {
      fbSelectSingle(active);
    }
    browserSelection.showWorkspace = source.showWorkspace ?? false;
    // Seed THIS tab's per-instance expansion from its persisted
    // `tab.expanded` (round-tripped through the layout hash + session.json).
    const inst = ensureFbTreeInstance(`fb-tab-${source.id}`);
    for (const k of Object.keys(inst.expanded)) {
      if (k !== "") delete inst.expanded[k];
    }
    inst.expanded[""] = true;
    for (const p of source.expanded ?? []) inst.expanded[p] = true;
    const target = source.scroll ?? 0;
    queueMicrotask(() => {
      if (treeWrapEl) treeWrapEl.scrollTop = target;
    });
  }

  $effect(() => {
    if (!isTab || !tab) return;
    const captured = tab;
    void captured.id;
    untrack(() => restoreFromTab(captured));
    return () => untrack(() => snapshotIntoTab(captured));
  });

  $effect(() => {
    if (!isTab || !tab) return;
    const captured = tab;
    const path = browserSelection.path;
    const showWorkspace = browserSelection.showWorkspace;
    const multi = browserSelection.paths;
    untrack(() => {
      captured.selected = path ?? undefined;
      captured.selectedPaths = multi.length > 1 ? [...multi] : undefined;
      captured.showWorkspace = showWorkspace ? true : undefined;
    });
  });

  $effect(() => {
    if (!isTab || !tab) return;
    const captured = tab;
    const map = ensureFbTreeInstance(`fb-tab-${captured.id}`).expanded;
    const expanded = Object.keys(map).filter((p) => p.length > 0 && map[p]);
    untrack(() => {
      captured.expanded = expanded.length > 0 ? expanded : undefined;
      persistLayoutToHash();
    });
  });

  function onTreeWrapScroll(ev: Event): void {
    if (!isTab || !tab) return;
    const top = (ev.currentTarget as HTMLElement).scrollTop;
    tab.scroll = top > 0 ? Math.round(top) : undefined;
  }

  // `fullstack-67`: in tab variant the on-surface header is gone, so
  // the FB hamburger has no visible trigger. Tab-strip right-click in
  // `Pane.svelte` sets `tabMenu.openForTabId` + `tabMenu.anchor`;
  // this effect mirrors that signal back into `menu.openAtCursor()`
  // so the FB-specific menu items still render at the cursor for
  // active Files tabs. Dock + overlay variants ignore the effect
  // (they have on-surface headers per `-54`).
  $effect(() => {
    if (!isTab || !tab) return;
    const open = tabMenu.openForTabId;
    const anchor = tabMenu.anchor;
    if (open !== tab.id || !anchor) return;
    queueMicrotask(() => menu?.openAtCursor(anchor.left, anchor.top));
  });

  const POPOVER_HEIGHT = 420;
  const POPOVER_WIDTH = 240;

  $effect(() => {
    if (isTab) {
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
    // Expand / collapse all targets THIS surface's instance only, so the
    // dock and a tab don't toggle each other.
    if (fullyExpanded) collapseAllFoldersForInstance(instanceId);
    else expandAllFoldersForInstance(instanceId);
    persistFbTreeInstanceExpansion(instanceId);
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

  // `fullstack-80`: tab + overlay variants auto-open the DETAILS
  // inspector on row click; dock variants do not (the dock has no
  // inspector pane anyway, and `isWideSurface` is false there).
  function onRowClicked(_path: string): void {
    if (isTab || isOverlay) browserState.inspectorOpen = true;
  }

  function doToggleOverlayMaximized(): void {
    setOverlayMaximized(!overlayMaximized.on);
    menu?.close();
  }

  function toggleStick(target: Side): void {
    toggleBrowserSidePane(target);
    menu?.close();
  }

  async function reloadTree(): Promise<void> {
    menu?.close();
    await refreshTree();
  }

  /// `fullstack-a-67e`: dropped `newFileHere` / `newDirHere` /
  /// `graphWorkspace` / `renameWorkspace` (modal) — the addendum-a spec
  /// moves New File / New Dir to the selection menu (where they
  /// can root under the selected directory) and replaces the
  /// modal "Rename workspace..." entry with a path row in the menu
  /// header. `openGraphForWorkspace` is still
  /// reachable via the empty-pane spawn grid + Cmd+Shift+M.

  function showWorkspaceInfo(): void {
    if (isDock) {
      openCurrentInFileBrowser();
      return;
    }
    menu?.close();
    browserSelection.path = null;
    browserSelection.showWorkspace = true;
    browserState.inspectorOpen = true;
  }

  function expandedAncestors(path: string): string[] {
    const parts = path.split("/");
    const ancestors: string[] = [];
    let acc = "";
    for (let i = 0; i < parts.length - 1; i++) {
      acc = acc ? `${acc}/${parts[i]}` : parts[i];
      if (acc) ancestors.push(acc);
    }
    return ancestors;
  }

  function openCurrentInFileBrowser(): void {
    menu?.close();
    const path = browserSelection.path;
    const tab = openBrowserInActivePane(path ? { select: path } : {});
    tab.inspectorOpen = true;
    if (path) {
      // The new tab's surface seeds its own per-instance expansion from
      // `tab.expanded` on mount, so no global singleton to prime here.
      const ancestors = expandedAncestors(path);
      tab.showWorkspace = false;
      tab.expanded = ancestors.length > 0 ? ancestors : undefined;
      browserSelection.path = path;
      browserSelection.showWorkspace = false;
      return;
    }
    tab.showWorkspace = true;
    browserSelection.path = null;
    browserSelection.showWorkspace = true;
  }

  /// `fullstack-a-67e`: flip to back-side config view. Routes
  /// through the `onFlip` callback the tab variant's parent
  /// (Pane.svelte) supplies. The menu entry is gated on
  /// `isTab && onFlip` so dock + overlay variants don't render
  /// a Settings entry that would no-op.
  function flipToSettings(): void {
    menu?.close();
    onFlip?.();
  }

  /// `fullstack-a-67e`: Reopen Closed Tab — parity with the
  /// terminal + editor menus. Available regardless of variant
  /// since the closed-tab stack is window-global; the entry
  /// disables when the stack is empty.
  function doReopenClosedTab(): void {
    menu?.close();
    reopenClosedTab();
  }

  /// `fullstack-a-67e`: Close — only renders in the tab variant
  /// where there's a tab to close. Routes through `onClose`
  /// (which Pane.svelte wires to `closeTab(pane.id, tab.id)`).
  function closeFromMenu(): void {
    menu?.close();
    onClose?.();
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
  data-theme={isTab ? surfaceThemeOverride("browser") : undefined}
  oncontextmenu={onBrowserContextMenu}
  onkeydown={onBrowserKeydown}
  role="presentation"
>
  {#if isOverlay}
    <header>
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
      <span class="header-spacer" aria-hidden="true"></span>
      <HamburgerMenu
        bind:this={menu}
        bind:open={menuOpen}
        width={POPOVER_WIDTH}
        height={POPOVER_HEIGHT}
      >
        {@render menuItems()}
      </HamburgerMenu>
    </header>
  {:else}
    <!-- `fullstack-67`/`fullstack-71`: tab + dock variants both drop
         the on-surface header. Tab variant relies on the pane Hybrid
         kebab (right-click on the Files tab → tabMenu state →
         menu.openAtCursor via the $effect above). Dock variant
         relies on the `oncontextmenu={onBrowserContextMenu}` handler
         on the `.browser` root, which calls `menu.openAtCursor`
         directly. Both share the same triggerless HamburgerMenu
         mounted here. -->
    <HamburgerMenu
      bind:this={menu}
      bind:open={menuOpen}
      showTrigger={false}
      width={POPOVER_WIDTH}
      height={POPOVER_HEIGHT}
    >
      {@render menuItems()}
    </HamburgerMenu>
  {/if}
  <div class="body">
    <div class="tree-wrap" bind:this={treeWrapEl} onscroll={onTreeWrapScroll}>
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
      <FileTree
        bind:this={treeRef}
        {instanceId}
        dockSide={variant === "dock" ? side : undefined}
        onClickRow={onRowClicked}
        onFlip={isTab ? onFlip : undefined}
      />
    </div>
    {#if isWideSurface && browserState.inspectorOpen}
      <Inspector
        title="Details"
        bind:width={
          () => browserState.inspectorWidth ?? paneWidths.browser,
          (v) => (browserState.inspectorWidth = v)
        }
        onResize={persistPaneWidths}
        onClose={() => (browserState.inspectorOpen = false)}
      >
        {#if browserSelection.showWorkspace && !browserSelection.path}
          <!-- `fullstack-73`: parity with the file/dir inspector
               surfaces. Click spawns a new Graph tab scoped to
               workspace root via `openFsGraphForDirectory("")` (matches
               the convention `graphSelection()` uses for non-workspace
               selections — `openFsGraphForDirectory` / `openFsGraphForFile`
               both spawn a fresh tab, never re-scope). -->
          <WorkspaceInfoBody onSetAsScope={() => openFsGraphForDirectory("")} />
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
  <!-- File Browser menu.
       Header: path-derived workspace label + full-path row (workspace icon,
       grey, fade-on-overflow, click -> workspace inspector). Body:
       dock toggles, expand/collapse +
       reload, import contacts. Foot: Settings (flipHybrid)
       + Reopen Closed Tab + Close.
       Selection menu (rename/delete/etc.) lives on FileTree's
       row right-click; this menu is the FB tab right-click +
       hamburger. New file / New directory entries moved to the
       selection menu where they're CWD-aware. -->
  <li class="workspace-label-row" role="none" title={workspace.info?.root}>
    <HardDrive size={16} strokeWidth={1.75} aria-hidden="true" />
    <span class="workspace-label-text">{workspace.info?.label ?? ""}</span>
  </li>
  <li>
    <button
      role="menuitem"
      class="workspace-path-row"
      onclick={showWorkspaceInfo}
      title={workspace.info?.root}
      disabled={!workspace.info?.root}
    >
      <HardDrive size={16} strokeWidth={1.75} aria-hidden="true" />
      <span class="workspace-path-text">{workspace.info?.root ?? ""}</span>
    </button>
  </li>
  {#if isDock}
    <li>
      <button role="menuitem" onclick={openCurrentInFileBrowser}>
        <FolderOpen size={16} strokeWidth={1.75} aria-hidden="true" />
        <span class="menu-row-label">Open in File Browser</span>
        <span class="menu-row-chord"></span>
      </button>
    </li>
  {/if}
  <li class="sep" role="separator"></li>
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
    <button role="menuitem" onclick={openImportContacts}>
      <Users size={16} strokeWidth={1.75} aria-hidden="true" />
      <span class="menu-row-label">Import contacts...</span>
      <span class="menu-row-chord"></span>
    </button>
  </li>
  {#if isTab && onFlip}
    <li class="sep" role="separator"></li>
    <li>
      <button role="menuitem" onclick={flipToSettings}>
        <Settings2 size={16} strokeWidth={1.75} aria-hidden="true" />
        <span class="menu-row-label">Settings</span>
        <span class="menu-row-chord"></span>
      </button>
    </li>
  {/if}
  {#if isTab}
    <li class="sep" role="separator"></li>
    <li>
      <button
        role="menuitem"
        onclick={doReopenClosedTab}
        disabled={!canReopenClosedTab()}
      >
        <History size={16} strokeWidth={1.75} aria-hidden="true" />
        <span class="menu-row-label">Reopen Closed Tab</span>
        <span class="menu-row-chord">{chordFor("app.tab.reopenClosed") ?? ""}</span>
      </button>
    </li>
    <li>
      <button role="menuitem" onclick={closeFromMenu}>
        <X size={16} strokeWidth={1.75} aria-hidden="true" />
        <span class="menu-row-label">Close</span>
        <span class="menu-row-chord">{chordFor("app.tab.close") ?? ""}</span>
      </button>
    </li>
  {/if}
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
  .header-spacer { flex: 1; }
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
  /* Workspace label + path row at the head of the FB tab right-click
     menu.
     The :global wrapper drops the `<li>` selectors through to
     the portal'd menu, which renders into <body>. */
  :global(.hamburger-menu li.workspace-label-row) {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 8px;
    color: var(--text-secondary);
    font-size: 13px;
  }
  :global(.hamburger-menu .workspace-label-text) {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text);
  }
  :global(.hamburger-menu .workspace-path-row) {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    background: none;
    border: 0;
    color: var(--text-secondary);
    cursor: pointer;
    padding: 6px 8px;
    text-align: left;
  }
  :global(.hamburger-menu .workspace-path-row:hover) {
    color: var(--text);
  }
  :global(.hamburger-menu .workspace-path-row:disabled) {
    cursor: default;
  }
  :global(.hamburger-menu .workspace-path-text) {
    flex: 1;
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    font-size: 12px;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    mask-image: linear-gradient(to right, black calc(100% - 1.25rem), transparent);
    -webkit-mask-image: linear-gradient(to right, black calc(100% - 1.25rem), transparent);
  }
  /* `fullstack-a-67e`: `.folder-text` / `.folder-label` /
     `.folder-path` / `.mono` selectors dropped along with the
     "Rename workspace..." + "Directory" rows they styled. The current
     workspace label + path rows have their own selectors above. */
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
