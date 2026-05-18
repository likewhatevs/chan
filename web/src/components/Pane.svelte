<script lang="ts">
  // One pane: a horizontal tab strip on top, an editor below.

  import {
    activeLayout,
    canSplit,
    canReopenClosedTab,
    closePane,
    closeTab,
    focusColorForPane,
    isDirty,
    detachTabToPaneEdge,
    layout,
    markLocalTabDrop,
    moveTab,
    openInPane,
    openBrowserInActivePane,
    openTerminalInPane,
    paneMode,
    reorderTab,
    reopenClosedTab,
    saveTab,
    selectNextPane,
    selectPrevPane,
    setActivePane,
    setPaneFocusColor,
    shouldCloseTabAfterDragEnd,
    splitPane,
    type LeafNode,
    type PaneDropEdge,
    type PaneFocusColor,
  } from "../state/tabs.svelte";

  import {
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    Check,
    FilePlus,
    FileText,
    Folder,
    History,
    Network,
    PanelRight,
    Palette,
    Radio,
    RefreshCw,
    Search,
    Settings,
    SquareSplitHorizontal,
    SquareSplitVertical,
    Terminal,
    User,
    X,
  } from "lucide-svelte";
  import FileEditorTab from "./FileEditorTab.svelte";
  import FileBrowserSurface from "./FileBrowserSurface.svelte";
  import GraphPanel from "./GraphPanel.svelte";
  import HamburgerMenu from "./HamburgerMenu.svelte";
  import TerminalTab from "./TerminalTab.svelte";
  import {
    drive,
    indexStatus,
    refreshTree,
    tree,
  } from "../state/store.svelte";
  import { tabLabel, tabLabelInPane, tabTooltip } from "../state/tabs.svelte";
  import {
    SHORTCUTS,
    currentOS,
    currentPlatform,
    formatChord,
    renderTable,
  } from "../state/shortcuts";
  import { openTabMenu, tabMenu } from "../state/tabMenu.svelte";
  import { onDestroy, onMount } from "svelte";
  import { applyPageWidthToElement, pageWidth } from "../state/pageWidth.svelte";

  let { pane }: { pane: LeafNode } = $props();

  const active = $derived(pane.tabs.find((t) => t.id === pane.activeTabId) ?? null);

  // ---- empty-pane dashboard summary --------------------------------------
  // Per request.md, the empty-tab background is now the primary
  // dashboard surface. These derivations give the placeholder a
  // small, factual "your drive" header above the shortcut table
  // without turning the surface into a marketing page.
  const driveSummary = $derived.by(() => {
    const entries = tree.entries;
    let files = 0;
    let folders = 0;
    let contacts = 0;
    for (const e of entries) {
      if (e.is_dir) folders++;
      else {
        files++;
        if (e.kind === "contact") contacts++;
      }
    }
    return { files, folders, contacts };
  });
  const indexLabel = $derived.by<string | null>(() => {
    const s = indexStatus.value;
    if (!s) return null;
    if (s.state === "building") {
      if (s.total > 0) return `indexing ${s.current}/${s.total}`;
      return "indexing…";
    }
    if (s.state === "reindexing") return "reindexing…";
    if (s.state === "error") return "index error";
    return null;
  });

  /// Per-path "is this file a contact?" lookup. Drives the tab-strip
  /// icon (User glyph for contacts, FileText otherwise) so a row of
  /// tabs reads as "person, file, file, person" rather than a wall
  /// of equally-weighted text. The kind comes from the tree listing's
  /// `chan.kind: contact` discriminator; re-derives whenever the
  /// tree refreshes.
  const contactPaths = $derived<Set<string>>(
    new Set(
      tree.entries
        .filter((e) => !e.is_dir && e.kind === "contact")
        .map((e) => e.path),
    ),
  );

  /// ASCII shortcut table painted on the empty-pane background.
  /// Picks the chord set (web vs native) and Mod label (Cmd vs Ctrl)
  /// once at module init — these don't change at runtime.
  const platform = currentPlatform();
  const os = currentOS();
  const shortcutTable = renderTable(platform, os);
  const paneFocusColors: PaneFocusColor[] = ["blue", "green", "pink"];

  /// Empty-pane right-click menu, arranged into the canonical
  /// sections shared by every chan menu: content actions, then
  /// navigation, then pane controls, then Settings as the footer.
  /// Each row carries an icon and (optionally) the keyboard chord
  /// for the same action — the empty pane is also the discovery
  /// surface for shortcuts, so we keep the chord hint visible.
  /// `chordId` is a SHORTCUTS registry id; rows whose chord isn't
  /// registered on the current platform render with a blank chord
  /// column rather than disappearing.
  type EmptyMenuRow = {
    label: string;
    // Lucide icons accept `size` + `strokeWidth` at the call site below.
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    icon: any;
    command: string;
    chordId?: string;
  };
  const emptyPaneContent: EmptyMenuRow[] = [
    {
      label: "Reload",
      icon: RefreshCw,
      command: "pane.reload",
    },
    {
      label: "Toggle Inspector",
      icon: PanelRight,
      command: "pane.inspector.toggle",
    },
    {
      label: "New File",
      icon: FilePlus,
      command: "app.file.new",
      chordId: "app.file.new",
    },
    {
      label: "Reopen Closed Tab",
      icon: History,
      command: "app.tab.reopenClosed",
      chordId: "app.tab.reopenClosed",
    },
  ];
  const emptyPaneNavigation: EmptyMenuRow[] = [
    {
      label: "Files",
      icon: Folder,
      command: "app.files.toggle",
      chordId: "app.files.toggle",
    },
    {
      label: "Search",
      icon: Search,
      command: "app.search.toggle",
      chordId: "app.search.toggle",
    },
    {
      label: "Graph",
      icon: Network,
      command: "app.graph.toggle",
      chordId: "app.graph.toggle",
    },
    {
      label: "Terminal",
      icon: Terminal,
      command: "app.terminal.toggle",
      chordId: "app.terminal.toggle",
    },
  ];
  function chordLabel(id: string | undefined): string {
    if (!id) return "";
    const s = SHORTCUTS.find((x) => x.id === id);
    if (!s) return "";
    const chord = s[platform];
    if (!chord) return "";
    return formatChord(chord, os);
  }

  /// Right-click menu state. The HamburgerMenu component owns the
  /// bubble chrome and outside-click dismiss; we just hold the
  /// handle so the contextmenu handler can open it at the cursor.
  let emptyPaneMenu: HamburgerMenu | undefined = $state();
  let emptyPaneMenuOpen = $state(false);

  function openEmptyPaneMenuAt(e: MouseEvent): void {
    emptyPaneMenu?.openAtCursor(e.clientX, e.clientY);
  }

  function onEmptyPaneContextMenu(e: MouseEvent): void {
    e.preventDefault();
    openEmptyPaneMenuAt(e);
  }

  /// Pane chrome menu: the ⋮ in the tab strip that replaces the
  /// per-button split / close controls.
  let paneMenu: HamburgerMenu | undefined = $state();
  let paneMenuOpen = $state(false);
  let paneContextMenu: HamburgerMenu | undefined = $state();
  let paneContextMenuOpen = $state(false);

  /// Per-pane page-width cap. The ratio (state/pageWidth) is
  /// global, but the px cap is pane-relative so splitting one
  /// pane into two halves correctly halves the cap. A
  /// ResizeObserver on .editor-wrap fires whenever the pane
  /// resizes (split, close, window resize, browser zoom) and
  /// pushes a fresh `--chan-page-max-width` onto the wrapper as
  /// inline style. CSS cascade beats the document-root fallback.
  let editorWrapEl: HTMLDivElement | undefined = $state();
  let editorWrapWidth = $state(0);
  let resizeObs: ResizeObserver | null = null;
  $effect(() => {
    if (!editorWrapEl) return;
    resizeObs?.disconnect();
    const target = editorWrapEl;
    resizeObs = new ResizeObserver((entries) => {
      for (const entry of entries) {
        editorWrapWidth = Math.round(entry.contentRect.width);
      }
    });
    resizeObs.observe(target);
    // Prime synchronously so the first paint already has the cap.
    editorWrapWidth = Math.round(target.getBoundingClientRect().width);
    return () => {
      resizeObs?.disconnect();
      resizeObs = null;
    };
  });
  $effect(() => {
    if (!editorWrapEl) return;
    applyPageWidthToElement(editorWrapEl, editorWrapWidth, pageWidth.ratio);
  });
  onDestroy(() => resizeObs?.disconnect());

  function onSplitRight(): void {
    closePaneMenus();
    splitPane(pane.id, "row", "after");
  }
  function onSplitLeft(): void {
    closePaneMenus();
    splitPane(pane.id, "row", "before");
  }
  function onSplitDown(): void {
    closePaneMenus();
    splitPane(pane.id, "column", "after");
  }
  function onSplitUp(): void {
    closePaneMenus();
    splitPane(pane.id, "column", "before");
  }
  function onClosePane(): void {
    closePaneMenus();
    closePane(pane.id);
  }

  function closePaneMenus(): void {
    paneMenu?.close();
    paneContextMenu?.close();
  }

  function doReloadPane(): void {
    closePaneMenus();
    void refreshTree();
  }

  function doToggleInspector(): void {
    closePaneMenus();
    if (active?.kind === "file") {
      active.inspectorOpen = !active.inspectorOpen;
    } else if (active?.kind === "graph") {
      active.inspectorOpen = !active.inspectorOpen;
    } else if (active?.kind === "browser") {
      active.inspectorOpen = !active.inspectorOpen;
    } else {
      openBrowserInActivePane();
    }
  }

  function doSelectPrevPane(): void {
    closePaneMenus();
    selectPrevPane();
  }

  function doSelectNextPane(): void {
    closePaneMenus();
    selectNextPane();
  }

  function doSetFocusColor(color: PaneFocusColor): void {
    closePaneMenus();
    setPaneFocusColor(pane.id, color);
  }

  function openPaneContextAt(e: MouseEvent): void {
    e.preventDefault();
    setActivePane(pane.id);
    paneContextMenu?.openAtCursor(e.clientX, e.clientY);
  }

  /// Fire the same `chan:command` event the keymap layer uses so
  /// every shortcut row routes through the existing dispatcher in
  /// App.svelte. Avoids re-implementing the actions here.
  function dispatchCommand(id: string): void {
    emptyPaneMenu?.close();
    if (id === "pane.reload") {
      void refreshTree();
      return;
    }
    if (id === "pane.inspector.toggle") {
      doToggleInspector();
      return;
    }
    if (id === "app.tab.reopenClosed" && !canReopenClosedTab()) return;
    window.dispatchEvent(
      new CustomEvent("chan:command", { detail: { name: id } }),
    );
  }
  // Single-pane layouts hide the focus highlight: it's only useful
  // when there's more than one pane to disambiguate. Re-derives on
  // every layout mutation so the highlight reappears the moment a
  // split is added and disappears the moment the second pane closes.
  const viewLayout = $derived(activeLayout());
  const multiPane = $derived.by(() => {
    void viewLayout.rootId;
    return Object.values(viewLayout.nodes).some((n) => n.kind === "split");
  });
  const isFocused = $derived(multiPane && viewLayout.activePaneId === pane.id);
  // Re-derive on every layout mutation so the split buttons grey
  // out the instant a tablet user adds their one allowed split.
  const splitsAllowed = $derived.by(() => {
    void viewLayout.rootId;
    void Object.keys(viewLayout.nodes).length;
    return canSplit();
  });
  // Drag state: highlight the tab strip while another pane's tab is being
  // dragged over it. Keyed by pane id so we don't bleed state between
  // panes that share this Svelte 5 component instance.
  let dropActive = $state(false);
  let bodyDropEdge: PaneDropEdge | null = $state(null);
  // Index where a per-tab drop indicator (a thin vertical bar) is
  // currently shown. -1 means no indicator. We render a visual bar
  // before tab[idx]; idx === pane.tabs.length means "after the last
  // tab".
  let dropIndicator = $state<number>(-1);
  /// Snapshot of `pane.activeTabId` taken in a tab's onmousedown,
  /// read in the same tab's onclick. Lets the click handler tell a
  /// tab-switch (was a different tab) apart from a re-click on the
  /// already-active tab (only the latter pops the menu).
  let tabMouseDownPrevActive: string | null = null;

  async function onSave(): Promise<void> {
    if (!active || active.kind !== "file") return;
    try {
      await saveTab(active);
    } catch (e) {
      active.error = (e as Error).message;
    }
  }

  function onKeyDown(e: KeyboardEvent): void {
    const meta = e.metaKey || e.ctrlKey;
    // Plain Cmd/Ctrl+S only. Cmd/Ctrl+Shift+S is the editor's
    // strikethrough toggle; without the shift gate this handler
    // would swallow strike.
    if (meta && !e.shiftKey && !e.altKey && e.key === "s") {
      e.preventDefault();
      void onSave();
      return;
    }
    if (
      layout.activePaneId === pane.id &&
      e.ctrlKey &&
      e.altKey &&
      !e.shiftKey &&
      !e.metaKey &&
      e.code === "KeyT"
    ) {
      e.preventDefault();
      reopenClosedTab();
    }
  }

  function onChanCommand(e: Event): void {
    if (viewLayout.activePaneId !== pane.id) return;
    const detail = (e as CustomEvent<{ name?: string }>).detail;
    if (detail?.name !== "app.tab.reopenClosed") return;
    reopenClosedTab();
  }
  onMount(() => window.addEventListener("chan:command", onChanCommand));
  onDestroy(() => window.removeEventListener("chan:command", onChanCommand));

  // ----- drag & drop ------------------------------------------------------

  // Custom mimes for the kinds of drops a pane accepts:
  //   - intra-window tab move (drop from another pane's tab strip in
  //     the same window): TAB_DRAG_MIME carries `{fromPaneId, tabId}`,
  //     resolved against the local layout.
  //   - cross-window tab move (drop from another Tauri window on the
  //     same chan-app instance): CROSS_TAB_MIME carries the full
  //     payload needed to reconstruct the tab in the target window.
  //     File tabs only; special tabs are window-bound on native.
  //   - file open (from the file tree): FILE_DRAG_MIME carries
  //     `{path}`.
  //
  // dragstart sets both TAB_DRAG_MIME and CROSS_TAB_MIME for file
  // tabs so the same drag works whether the user drops in the same
  // window (intra) or another (cross). The receiver tries intra
  // first; if `fromPaneId` is not in its layout, it falls through to
  // the cross path. The source's dragend handler removes the tab if
  // the drop happened cross-window (intra moves already mutated the
  // local layout).
  const TAB_DRAG_MIME = "application/x-md-tab";
  const CROSS_TAB_MIME = "application/x-chan-tab+json";
  const FILE_DRAG_MIME = "application/x-md-file";

  function onDragStart(e: DragEvent, tabId: string): void {
    if (!e.dataTransfer) return;
    e.dataTransfer.effectAllowed = "move";
    e.dataTransfer.setData(
      TAB_DRAG_MIME,
      JSON.stringify({ fromPaneId: pane.id, tabId }),
    );
    const t = pane.tabs.find((tab) => tab.id === tabId);
    if (t) {
      e.dataTransfer.setData(
        CROSS_TAB_MIME,
        JSON.stringify(
          t.kind === "file"
            ? {
                kind: "file",
                path: t.path,
                mode: t.mode,
                inspectorOpen: t.inspectorOpen,
              }
            : {
                kind: "terminal",
                title: t.title,
              },
        ),
      );
    }
  }

  /// Fired on the SOURCE element after the drop completes (anywhere).
  /// `dropEffect === "move"` means a target accepted; for an
  /// intra-window move, the layout was already mutated by
  /// moveTab/reorderTab and the tab is no longer here. If the tab is
  /// still in this pane after a successful move, the drop must have
  /// landed in another window — close it locally so the visual
  /// matches the cross-window result.
  function onDragEnd(e: DragEvent, tabId: string): void {
    if (shouldCloseTabAfterDragEnd(pane.id, tabId, e.dataTransfer?.dropEffect)) {
      closeTab(pane.id, tabId);
    }
  }

  /// Open a cross-window tab payload in this pane. Used by the
  /// drop handlers when the intra-window check fails (the source
  /// pane belongs to a different window). Drop position is not
  /// honoured for cross-window drops; the user can reorder within
  /// the strip afterwards.
  function acceptCrossWindowTab(payload: string): boolean {
    let parsed: { kind?: string; path?: string };
    try {
      parsed = JSON.parse(payload);
    } catch {
      return false;
    }
    if (parsed.kind === "terminal") {
      openTerminalInPane(pane.id);
      return true;
    }
    if (!parsed.path) return false;
    void openInPane(pane.id, parsed.path);
    return true;
  }

  function isAcceptedDrag(e: DragEvent): boolean {
    const types = e.dataTransfer?.types;
    if (!types) return false;
    return (
      types.includes(TAB_DRAG_MIME) ||
      types.includes(CROSS_TAB_MIME) ||
      types.includes(FILE_DRAG_MIME)
    );
  }

  function onDragOver(e: DragEvent): void {
    if (!isAcceptedDrag(e)) return;
    e.preventDefault();
    if (e.dataTransfer) {
      const isTabMove =
        e.dataTransfer.types.includes(TAB_DRAG_MIME) ||
        e.dataTransfer.types.includes(CROSS_TAB_MIME);
      e.dataTransfer.dropEffect = isTabMove ? "move" : "copy";
    }
    dropActive = true;
  }

  function onDragLeave(e: DragEvent): void {
    // Only clear the highlight when the cursor truly leaves the tab strip,
    // not when it crosses into a child element.
    const related = e.relatedTarget as Node | null;
    if (!related || !(e.currentTarget as Node).contains(related)) {
      dropActive = false;
      dropIndicator = -1;
    }
  }

  function tabDragPayload(e: DragEvent): { fromPaneId: string; tabId: string } | null {
    const raw = e.dataTransfer?.getData(TAB_DRAG_MIME);
    if (!raw) return null;
    try {
      const parsed = JSON.parse(raw) as { fromPaneId?: string; tabId?: string };
      if (!parsed.fromPaneId || !parsed.tabId) return null;
      return { fromPaneId: parsed.fromPaneId, tabId: parsed.tabId };
    } catch {
      return null;
    }
  }

  function edgeForBodyDrop(e: DragEvent): PaneDropEdge {
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;
    const distances: Array<[PaneDropEdge, number]> = [
      ["left", x],
      ["right", rect.width - x],
      ["top", y],
      ["bottom", rect.height - y],
    ];
    distances.sort((a, b) => a[1] - b[1]);
    return distances[0]![0];
  }

  function onBodyDragOver(e: DragEvent): void {
    const payload = tabDragPayload(e);
    if (!payload || !paneInThisWindow(payload.fromPaneId)) return;
    e.preventDefault();
    e.stopPropagation();
    if (e.dataTransfer) e.dataTransfer.dropEffect = "move";
    bodyDropEdge = edgeForBodyDrop(e);
  }

  function onBodyDragLeave(e: DragEvent): void {
    const related = e.relatedTarget as Node | null;
    if (!related || !(e.currentTarget as Node).contains(related)) {
      bodyDropEdge = null;
    }
  }

  function onBodyDrop(e: DragEvent): void {
    const payload = tabDragPayload(e);
    const edge = bodyDropEdge ?? edgeForBodyDrop(e);
    bodyDropEdge = null;
    if (!payload || !paneInThisWindow(payload.fromPaneId)) return;
    e.preventDefault();
    e.stopPropagation();
    markLocalTabDrop(payload.fromPaneId, payload.tabId);
    detachTabToPaneEdge(payload.fromPaneId, payload.tabId, pane.id, edge);
  }

  /// Compute the insertion index for a drop with cursor position `clientX`
  /// over a tab element. We split the tab in half: drop on the left half
  /// → insert before; right half → insert after.
  function indicatorIndexFor(tabIdx: number, e: DragEvent): number {
    const el = e.currentTarget as HTMLElement;
    const r = el.getBoundingClientRect();
    return e.clientX < r.left + r.width / 2 ? tabIdx : tabIdx + 1;
  }

  /// Is the current drag from a tab in THIS pane? Affects both the
  /// visual indicator and the drop semantics: same-pane drops use a
  /// "drop on tab T = land at slot T" rule (a swap-like reorder, no
  /// half-tab logic), because users naturally expect dropping on a tab
  /// to displace it. Cross-pane drops keep the precise "insert before
  /// or after this tab" semantic since insertion between tabs is the
  /// useful action there.
  function isSamePaneDrag(e: DragEvent): boolean {
    const raw = e.dataTransfer?.getData(TAB_DRAG_MIME);
    if (!raw) return false;
    try {
      return (JSON.parse(raw) as { fromPaneId: string }).fromPaneId === pane.id;
    } catch {
      return false;
    }
  }

  function onTabDragOver(e: DragEvent, tabIdx: number): void {
    if (!isAcceptedDrag(e)) return;
    e.preventDefault();
    e.stopPropagation(); // don't let the strip-level handler also fire
    if (e.dataTransfer) {
      const isTabMove =
        e.dataTransfer.types.includes(TAB_DRAG_MIME) ||
        e.dataTransfer.types.includes(CROSS_TAB_MIME);
      e.dataTransfer.dropEffect = isTabMove ? "move" : "copy";
    }
    dropActive = true;
    // For same-pane drags we show the indicator at the target tab's
    // slot (i.e., "this is where your tab will land"). For cross-pane
    // drags the half-tab heuristic gives precise insertion control.
    dropIndicator = isSamePaneDrag(e) ? tabIdx : indicatorIndexFor(tabIdx, e);
  }

  /// Whether a `fromPaneId` belongs to this window's layout. Cross-
  /// window drops carry a `fromPaneId` that doesn't exist in the
  /// receiving window; we use that to distinguish.
  function paneInThisWindow(paneId: string): boolean {
    const n = layout.nodes[paneId];
    return !!n && n.kind === "leaf";
  }

  function onTabDrop(e: DragEvent, tabIdx: number): void {
    const dt = e.dataTransfer;
    if (!dt) return;
    dropActive = false;
    dropIndicator = -1;
    const tabRaw = dt.getData(TAB_DRAG_MIME);
    if (tabRaw) {
      try {
        const { fromPaneId, tabId } = JSON.parse(tabRaw) as {
          fromPaneId: string;
          tabId: string;
        };
        if (paneInThisWindow(fromPaneId)) {
          e.preventDefault();
          e.stopPropagation();
          markLocalTabDrop(fromPaneId, tabId);
          if (fromPaneId === pane.id) {
            // Same-pane reorder: drop on tab T means source lands at
            // position T in the final array. No half-tab logic; drops
            // on either half of the target produce the same swap.
            reorderTab(pane.id, tabId, tabIdx);
          } else {
            moveTab(fromPaneId, tabId, pane.id, indicatorIndexFor(tabIdx, e));
          }
          return;
        }
        // Fall through to cross-window path: fromPaneId is a stranger.
      } catch {
        // malformed payload; fall through.
      }
    }
    const crossRaw = dt.getData(CROSS_TAB_MIME);
    if (crossRaw) {
      e.preventDefault();
      e.stopPropagation();
      acceptCrossWindowTab(crossRaw);
      return;
    }
    const fileRaw = dt.getData(FILE_DRAG_MIME);
    if (fileRaw) {
      e.preventDefault();
      e.stopPropagation();
      try {
        const { path } = JSON.parse(fileRaw) as { path: string };
        if (path) void openInPane(pane.id, path);
      } catch {
        // ignore
      }
    }
  }

  function onDrop(e: DragEvent): void {
    dropActive = false;
    dropIndicator = -1;
    const dt = e.dataTransfer;
    if (!dt) return;
    // Tab move takes precedence over file open if both are present.
    const tabRaw = dt.getData(TAB_DRAG_MIME);
    if (tabRaw) {
      try {
        const { fromPaneId, tabId } = JSON.parse(tabRaw) as {
          fromPaneId: string;
          tabId: string;
        };
        if (paneInThisWindow(fromPaneId)) {
          e.preventDefault();
          markLocalTabDrop(fromPaneId, tabId);
          if (fromPaneId === pane.id) {
            // Strip-level drop in the same pane (i.e., dropped on the
            // background or actions area, not directly on a tab). Treat
            // it as "move source to the end" so dragging a leftmost tab
            // rightward past the last tab does the obvious thing instead
            // of silently no-op'ing.
            reorderTab(pane.id, tabId, Math.max(0, pane.tabs.length - 1));
          } else {
            moveTab(fromPaneId, tabId, pane.id);
          }
          return;
        }
        // Fall through to cross-window path.
      } catch {
        // malformed payload; fall through.
      }
    }
    const crossRaw = dt.getData(CROSS_TAB_MIME);
    if (crossRaw) {
      e.preventDefault();
      acceptCrossWindowTab(crossRaw);
      return;
    }
    const fileRaw = dt.getData(FILE_DRAG_MIME);
    if (fileRaw) {
      e.preventDefault();
      try {
        const { path } = JSON.parse(fileRaw) as { path: string };
        if (path) void openInPane(pane.id, path);
      } catch {
        // ignore
      }
    }
  }
</script>

<svelte:window onkeydown={onKeyDown} />

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  class="pane"
  class:focused={isFocused}
  data-focus-color={focusColorForPane(pane.id)}
  onmousedown={() => setActivePane(pane.id)}
  role="region"
  aria-label="editor pane"
>
  <!-- svelte-ignore a11y_interactive_supports_focus -->
  <div
    class="tabs"
    class:drop-active={dropActive}
    role="tablist"
    ondragover={onDragOver}
    ondragleave={onDragLeave}
    ondrop={onDrop}
    oncontextmenu={(e) => {
      if ((e.target as Element | null)?.closest(".tab, .actions")) return;
      openPaneContextAt(e);
    }}
  >
    {#each pane.tabs as t, i (t.id)}
      {#if dropIndicator === i}
        <div class="drop-bar" aria-hidden="true"></div>
      {/if}
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <div
        class="tab"
        class:active={t.id === pane.activeTabId}
        onmousedown={() => {
          // Stash the pre-switch active tab id so the onclick handler
          // can tell whether this is a tab-switch (do NOT pop the
          // menu) or a re-click on the already-active tab (DO pop the
          // menu). Cleared in onclick. mousedown fires before click,
          // so this captures the previous value before we overwrite
          // it below.
          tabMouseDownPrevActive = pane.activeTabId;
          pane.activeTabId = t.id;
        }}
        onclick={() => {
          tabMouseDownPrevActive = null;
        }}
        oncontextmenu={(e) => {
          e.preventDefault();
          e.stopPropagation();
          pane.activeTabId = t.id;
          layout.activePaneId = pane.id;
          openTabMenu(t.id, {
            left: e.clientX,
            top: e.clientY,
            right: e.clientX,
            bottom: e.clientY,
          });
        }}
        role="tab"
        tabindex="0"
        title={tabTooltip(t)}
        draggable="true"
        ondragstart={(e) => onDragStart(e, t.id)}
        ondragend={(e) => onDragEnd(e, t.id)}
        ondragover={(e) => onTabDragOver(e, i)}
        ondrop={(e) => onTabDrop(e, i)}
      >
        <!-- Spinner appears while the file is still loading from
             disk; once loaded the tab leads with a kind icon (User
             for contacts, FileText otherwise) so the row reads
             scannably. -->
        {#if t.kind === "file" && t.loading}
          <span class="marker spinner" aria-hidden="true"></span>
        {:else if t.kind === "file"}
          <span class="tab-icon" aria-hidden="true">
            {#if contactPaths.has(t.path)}
              <User size={14} strokeWidth={1.75} />
            {:else}
              <FileText size={14} strokeWidth={1.75} />
            {/if}
          </span>
        {:else if t.kind === "terminal"}
          <span class="tab-icon" aria-hidden="true">
            <Terminal size={14} strokeWidth={1.75} />
          </span>
          {#if t.broadcastEnabled}
            <span
              class="broadcast-marker"
              title={`Broadcasting to ${t.broadcastTargetIds.length} tab(s)`}
              aria-label={`Broadcasting to ${t.broadcastTargetIds.length} tab(s)`}
            >
              <Radio size={13} strokeWidth={1.9} aria-hidden="true" />
            </span>
          {/if}
        {:else if t.kind === "graph"}
          <span class="tab-icon" aria-hidden="true">
            <Network size={14} strokeWidth={1.75} />
          </span>
        {:else if t.kind === "browser"}
          <span class="tab-icon" aria-hidden="true">
            <Folder size={14} strokeWidth={1.75} />
          </span>
        {/if}
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <span
          class="path"
          aria-haspopup="menu"
          aria-expanded={tabMenu.openForTabId === t.id}
          onclick={(e) => {
            e.stopPropagation();
            tabMouseDownPrevActive = null;
          }}
          oncontextmenu={(e) => {
            e.preventDefault();
            e.stopPropagation();
            pane.activeTabId = t.id;
            layout.activePaneId = pane.id;
            openTabMenu(t.id, {
              left: e.clientX,
              top: e.clientY,
              right: e.clientX,
              bottom: e.clientY,
            });
          }}
        >{tabLabelInPane(t, pane.tabs)}</span>
        {#if isDirty(t)}
          <span class="dirty unsaved" title="unsaved changes">●</span>
        {/if}
        {#if t.kind === "terminal" && t.watcher}
          <span
            class="dirty watcher"
            class:blink={t.watcher.unread}
            title="watcher active"
            aria-label="watcher active"
          >●</span>
        {/if}
        <button
          class="close"
          onclick={(e) => {
            e.stopPropagation();
            closeTab(pane.id, t.id);
          }}
          title="close"
        >×</button>
      </div>
    {/each}
    {#if dropIndicator === pane.tabs.length}
      <div class="drop-bar" aria-hidden="true"></div>
    {/if}
    <div class="actions">
      <!-- Pane-only controls live inside a single hamburger menu
           to match the file browser / search / graph overlays.
           Split rows hide when the platform doesn't allow any splits
           (iPhone) and grey out when the platform's cap is reached
           (iPad after one split, native desktop / web have no cap). -->
      <HamburgerMenu
        bind:this={paneMenu}
        bind:open={paneMenuOpen}
        width={220}
        height={110}
      >
        <li>
          <button role="menuitem" onclick={doReloadPane}>
            <RefreshCw size={16} strokeWidth={1.75} aria-hidden="true" />
            <span>Reload</span>
          </button>
        </li>
        <li>
          <button role="menuitem" onclick={doToggleInspector}>
            <PanelRight size={16} strokeWidth={1.75} aria-hidden="true" />
            <span>Toggle Web Inspector</span>
          </button>
        </li>
      </HamburgerMenu>
      <HamburgerMenu
        bind:this={paneContextMenu}
        bind:open={paneContextMenuOpen}
        showTrigger={false}
        width={250}
        height={320}
      >
        {#if splitsAllowed}
          <li>
            <button role="menuitem" onclick={onSplitLeft}>
              <ArrowLeft size={16} strokeWidth={1.75} aria-hidden="true" />
              <span>Split left</span>
            </button>
          </li>
          <li>
            <button role="menuitem" onclick={onSplitRight}>
              <ArrowRight size={16} strokeWidth={1.75} aria-hidden="true" />
              <span>Split right</span>
            </button>
          </li>
          <li>
            <button role="menuitem" onclick={onSplitUp}>
              <ArrowUp size={16} strokeWidth={1.75} aria-hidden="true" />
              <span>Split up</span>
            </button>
          </li>
          <li>
            <button role="menuitem" onclick={onSplitDown}>
              <ArrowDown size={16} strokeWidth={1.75} aria-hidden="true" />
              <span>Split down</span>
            </button>
          </li>
          <li class="sep" role="separator"></li>
        {/if}
        <li>
          <button role="menuitem" onclick={doSelectNextPane}>
            <SquareSplitHorizontal size={16} strokeWidth={1.75} aria-hidden="true" />
            <span class="menu-row-label">Next pane</span>
            <span class="menu-row-chord">{chordLabel("app.pane.next")}</span>
          </button>
        </li>
        <li>
          <button role="menuitem" onclick={doSelectPrevPane}>
            <SquareSplitHorizontal size={16} strokeWidth={1.75} aria-hidden="true" />
            <span class="menu-row-label">Previous pane</span>
            <span class="menu-row-chord">{chordLabel("app.pane.prev")}</span>
          </button>
        </li>
        <li class="sep" role="separator"></li>
        <li class="menu-label">
          <Palette size={16} strokeWidth={1.75} aria-hidden="true" />
          <span>Focus border color</span>
        </li>
        {#each paneFocusColors as color}
          <li>
            <button role="menuitem" onclick={() => doSetFocusColor(color)}>
              <span class={`color-dot ${color}`} aria-hidden="true"></span>
              <span>{color}</span>
              {#if focusColorForPane(pane.id) === color}
                <Check size={14} strokeWidth={2} aria-hidden="true" />
              {/if}
            </button>
          </li>
        {/each}
        <li class="sep" role="separator"></li>
        <li>
          <button role="menuitem" onclick={onClosePane}>
            <X size={16} strokeWidth={1.75} aria-hidden="true" />
            <span>Close pane</span>
          </button>
        </li>
      </HamburgerMenu>
    </div>
  </div>

  <div
    class="editor-wrap"
    class:body-drop-left={bodyDropEdge === "left"}
    class:body-drop-right={bodyDropEdge === "right"}
    class:body-drop-top={bodyDropEdge === "top"}
    class:body-drop-bottom={bodyDropEdge === "bottom"}
    bind:this={editorWrapEl}
    ondragover={onBodyDragOver}
    ondragleave={onBodyDragLeave}
    ondrop={onBodyDrop}
    role="group"
    aria-label="pane content"
  >
    {#if paneMode.active}
      <div class="pane-mode-preview" aria-label="pane mode preview">
        <div class="pane-mode-title">{active ? tabLabel(active) : "Empty pane"}</div>
        <div class="pane-mode-subtitle">
          {active?.kind === "file"
            ? active.path
            : active?.kind === "terminal"
              ? "terminal"
              : active?.kind === "graph"
                ? active.scopeId
                : active?.kind === "browser"
                  ? "file browser"
                  : "no active tab"}
        </div>
      </div>
    {:else if active?.kind === "file"}
      <FileEditorTab tab={active} />
    {:else if active?.kind === "graph"}
      <GraphPanel
        tab={active}
        onClose={() => {
          void closeTab(pane.id, active.id);
        }}
      />
    {:else if active?.kind === "browser"}
      <FileBrowserSurface
        variant="tab"
        tab={active}
        onClose={() => {
          void closeTab(pane.id, active.id);
        }}
      />
    {:else if !active}
      <div
        class="placeholder"
        aria-label="no tab open"
        onclick={() => setActivePane(pane.id)}
        oncontextmenu={openPaneContextAt}
        role="presentation"
      >
        <div class="placeholder-stack">
          <div class="placeholder-mark"></div>
          <!-- Drive header + shortcut table only on the lone-pane
               case. In a multi-pane layout the extra panes are
               workspace setup (the user is about to drop files in),
               so the dashboard chrome just gets in the way; the
               logo alone is enough.

               Per request.md, this surface is the primary
               dashboard. The header keeps the dashboard factual
               (drive name + counts + index state) instead of
               marketing copy; the shortcut table below stays as
               the discovery surface for chords. -->
          {#if !multiPane}
            {#if drive.info}
              <div class="dashboard-header" aria-label="drive summary">
                <div class="dashboard-name">{drive.info.name ?? "(unnamed)"}</div>
                <div class="dashboard-stats">
                  <span>{driveSummary.files} files</span>
                  <span class="sep" aria-hidden="true">·</span>
                  <span>{driveSummary.folders} directories</span>
                  {#if driveSummary.contacts > 0}
                    <span class="sep" aria-hidden="true">·</span>
                    <span>{driveSummary.contacts} contacts</span>
                  {/if}
                  {#if indexLabel}
                    <span class="sep" aria-hidden="true">·</span>
                    <span class="dashboard-index">{indexLabel}</span>
                  {/if}
                </div>
              </div>
            {/if}
            <p class="placeholder-hint">
              Each pane's visible tab is part of the scope<br />
              for Graph.
            </p>
            <pre class="placeholder-shortcuts">{shortcutTable}</pre>
          {/if}
        </div>
        <!-- Right-click menu. Triggerless: opens only via the
             contextmenu handler above. Same `chan:command` ids as
             the keymap layer so actions stay unified. -->
        <HamburgerMenu
          bind:this={emptyPaneMenu}
          bind:open={emptyPaneMenuOpen}
          showTrigger={false}
          width={280}
          height={260}
        >
          <!-- Canonical section order shared with the file-tab and
               overlay menus: content actions, navigation, pane
               controls, Settings footer. -->
          {#each emptyPaneContent as row (row.command)}
            {@const Icon = row.icon}
            <li>
              <button
                role="menuitem"
                disabled={row.command === "app.tab.reopenClosed" && !canReopenClosedTab()}
                onclick={() => dispatchCommand(row.command)}
              >
                <Icon size={16} strokeWidth={1.75} aria-hidden="true" />
                <span class="menu-row-label">{row.label}</span>
                <span class="menu-row-chord">{chordLabel(row.chordId)}</span>
              </button>
            </li>
          {/each}
          <li class="sep" role="separator"></li>
          {#each emptyPaneNavigation as row (row.command)}
            {@const Icon = row.icon}
            <li>
              <button role="menuitem" onclick={() => dispatchCommand(row.command)}>
                <Icon size={16} strokeWidth={1.75} aria-hidden="true" />
                <span class="menu-row-label">{row.label}</span>
                <span class="menu-row-chord">{chordLabel(row.chordId)}</span>
              </button>
            </li>
          {/each}
          {#if splitsAllowed}
            <li class="sep" role="separator"></li>
            <li>
              <button
                role="menuitem"
                onclick={() => {
                  emptyPaneMenu?.close();
                  splitPane(pane.id, "row", "after");
                }}
              >
                <SquareSplitHorizontal size={16} strokeWidth={1.75} aria-hidden="true" />
                <span class="menu-row-label">Split right</span>
                <span class="menu-row-chord"></span>
              </button>
            </li>
            <li>
              <button
                role="menuitem"
                onclick={() => {
                  emptyPaneMenu?.close();
                  splitPane(pane.id, "column", "after");
                }}
              >
                <SquareSplitVertical size={16} strokeWidth={1.75} aria-hidden="true" />
                <span class="menu-row-label">Split down</span>
                <span class="menu-row-chord"></span>
              </button>
            </li>
          {/if}
          <li class="sep" role="separator"></li>
          <li>
            <button
              role="menuitem"
              onclick={() => dispatchCommand("app.settings.toggle")}
            >
              <Settings size={16} strokeWidth={1.75} aria-hidden="true" />
              <span class="menu-row-label">Settings</span>
              <span class="menu-row-chord">{chordLabel("app.settings.toggle")}</span>
            </button>
          </li>
        </HamburgerMenu>
      </div>
    {/if}
    {#if !paneMode.active}
      {#each pane.tabs.filter((t) => t.kind === "terminal") as t (t.id)}
        <TerminalTab tab={t} paneId={pane.id} active={t.id === pane.activeTabId} />
      {/each}
    {/if}
  </div>
</div>

<style>
  .pane {
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    flex: 1;
    border: 1px solid transparent;
    background: var(--bg);
    color: var(--text);
  }
  .pane[data-focus-color="blue"] { --pane-active-focus: var(--pane-focus); }
  .pane[data-focus-color="green"] { --pane-active-focus: #22c55e; }
  .pane[data-focus-color="pink"] { --pane-active-focus: #ff5fb7; }
  /* Inset glow rather than just a border-color swap: 2px reads
     clearly on both light and dark canvases, and the inset stays
     inside the pane's own box so the surrounding layout doesn't
     shift when focus moves between panes. The transparent border on
     `.pane` keeps the box dimensions stable; this shadow paints
     over the inside edge. */
  .pane.focused {
    border-color: var(--pane-active-focus);
    box-shadow: inset 0 0 0 2px var(--pane-active-focus);
  }
  /* iTerm-style strip: a dark bar with no per-tab dividers. The
     active tab is a rounded pill sitting on the bar rather than a
     beveled cap that "merges" with the editor below. */
  .tabs {
    display: flex;
    align-items: center;
    gap: 2px;
    background: var(--bg);
    border-bottom: 1px solid var(--border);
    padding: 4px 6px;
    overflow-x: auto;
    flex-shrink: 0;
    transition: box-shadow 0.1s;
  }
  .tabs.drop-active {
    box-shadow: inset 0 0 0 2px var(--pane-focus);
  }
  .tab[draggable="true"] { -webkit-user-drag: element; }
  /* Vertical bar between tabs that shows where a drop will land. */
  .drop-bar {
    width: 2px;
    align-self: stretch;
    background: var(--pane-focus);
    flex-shrink: 0;
  }
  .tab {
    display: flex;
    gap: 6px;
    align-items: center;
    padding: 3px 10px;
    border-radius: 999px;
    cursor: pointer;
    font-size: 13px;
    color: var(--text-secondary);
    background: transparent;
    user-select: none;
    /* `transform-origin: center bottom` anchors the hover wobble to
       the tab strip's baseline so the tab lifts upward rather than
       drifting sideways. Same easeOutBack curve as the tab-menu
       bubble, bottom pill, and style toolbar so the motion language
       reads as one set. */
    transform-origin: center bottom;
    transition:
      background 80ms ease,
      color 80ms ease,
      transform 260ms cubic-bezier(0.34, 1.56, 0.64, 1);
  }
  .tab:hover {
    color: var(--text);
    transform: scale(1.04);
  }
  .tab.active {
    background: var(--bg-elev);
    color: var(--text);
    font-weight: 500;
    box-shadow: 0 0 0 1px var(--border);
  }
  /* CSS-only spinner shown while a tab's content is loading. Inherits
     color from the tab's text-secondary so it sits at the same
     visual weight as the surrounding label. */
  .tab .marker.spinner {
    width: 10px;
    height: 10px;
    border: 1.5px solid currentColor;
    border-top-color: transparent;
    border-radius: 50%;
    animation: tab-spin 0.8s linear infinite;
    flex-shrink: 0;
  }
  @keyframes tab-spin {
    to { transform: rotate(360deg); }
  }
  /* Close button stays invisible until the row is hovered or the
     tab is active; matches iTerm / Chrome's behavior so the strip
     reads cleanly with many tabs open. */
  .tab .close {
    background: none;
    border: 0;
    cursor: pointer;
    font-size: 14px;
    line-height: 1;
    color: var(--text-secondary);
    padding: 0 2px;
    opacity: 0;
    transition: opacity 80ms ease, color 80ms ease;
    flex-shrink: 0;
  }
  .tab:hover .close,
  .tab.active .close { opacity: 1; }
  .tab .close:hover { color: var(--text); }
  .dirty {
    font-size: 10px;
    line-height: 1;
    color: var(--info-text);
  }
  .dirty.watcher {
    color: var(--success-text, var(--link));
  }
  .dirty.watcher.blink {
    animation: watcher-blink 850ms steps(2, start) infinite;
  }
  @keyframes watcher-blink {
    50% { opacity: .25; }
  }
  .path { white-space: nowrap; }
  /* Per-tab kind icon: User for contact-kind files, FileText
     otherwise. Sized to the tab font and stroked with text-secondary
     so it sits one step below the label. */
  .tab-icon {
    display: inline-flex;
    align-items: center;
    color: var(--text-secondary);
    flex-shrink: 0;
  }
  .tab.active .tab-icon { color: var(--text); }
  .broadcast-marker {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: #ff5fb7;
    width: 15px;
    height: 15px;
    font-weight: 700;
    flex-shrink: 0;
  }
  .actions { margin-left: auto; display: flex; align-items: center; padding-left: 4px; }
  :global(.hamburger-menu .menu-label) {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 8px;
    color: var(--text-secondary);
    font-size: 12px;
  }
  :global(.hamburger-menu .color-dot) {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    box-shadow: 0 0 0 1px var(--border);
  }
  :global(.hamburger-menu .color-dot.blue) { background: var(--pane-focus); }
  :global(.hamburger-menu .color-dot.green) { background: #22c55e; }
  :global(.hamburger-menu .color-dot.pink) { background: #ff5fb7; }
  .editor-wrap {
    position: relative;
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
  .editor-wrap::after {
    content: "";
    position: absolute;
    inset: 0;
    pointer-events: none;
    opacity: 0;
    transition: opacity 80ms ease;
    z-index: 20;
  }
  .editor-wrap.body-drop-left::after,
  .editor-wrap.body-drop-right::after,
  .editor-wrap.body-drop-top::after,
  .editor-wrap.body-drop-bottom::after {
    opacity: 1;
  }
  .editor-wrap.body-drop-left::after {
    border-left: 4px solid var(--pane-focus);
    background: linear-gradient(90deg, color-mix(in srgb, var(--pane-focus) 14%, transparent), transparent 34%);
  }
  .editor-wrap.body-drop-right::after {
    border-right: 4px solid var(--pane-focus);
    background: linear-gradient(270deg, color-mix(in srgb, var(--pane-focus) 14%, transparent), transparent 34%);
  }
  .editor-wrap.body-drop-top::after {
    border-top: 4px solid var(--pane-focus);
    background: linear-gradient(180deg, color-mix(in srgb, var(--pane-focus) 14%, transparent), transparent 34%);
  }
  .editor-wrap.body-drop-bottom::after {
    border-bottom: 4px solid var(--pane-focus);
    background: linear-gradient(0deg, color-mix(in srgb, var(--pane-focus) 14%, transparent), transparent 34%);
  }
  .pane-mode-preview {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
    gap: 6px;
    padding: 20px;
    text-align: center;
    color: var(--text-secondary);
    background: color-mix(in srgb, var(--bg-card) 36%, transparent);
  }
  .pane-mode-title {
    max-width: 100%;
    color: var(--text);
    font-size: 16px;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .pane-mode-subtitle {
    max-width: 100%;
    font-size: 12px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  /* Empty pane: muted chan logo watermark above the keyboard-
     shortcut table, both centered. CSS mask paints the silhouette
     in the current text-secondary color so it adapts to light /
     dark themes; the image itself (web/public/chan-mark.png) is
     alpha-only. The shortcut table comes from
     state/shortcuts.ts so the surface stays in sync with the
     `chan serve --help` text — resync via
     `node web/scripts/shortcuts-table.mjs`. */
  .placeholder {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 2rem 1rem;
    overflow: auto;
  }
  .placeholder-stack {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1.25rem;
    color: var(--text-secondary);
    opacity: 0.6;
  }
  .placeholder-mark {
    width: 160px;
    height: 160px;
    background-color: var(--text-secondary);
    -webkit-mask: url('/chan-mark.png') center / contain no-repeat;
            mask: url('/chan-mark.png') center / contain no-repeat;
    opacity: 0.45;
  }
  .placeholder-shortcuts {
    margin: 0;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 12px;
    line-height: 1.5;
    white-space: pre;
    color: var(--text-secondary);
  }
  .placeholder-hint {
    margin: 0;
    text-align: center;
    color: var(--text-secondary);
    font-size: 13px;
    line-height: 1.4;
    max-width: 360px;
  }
  /* Dashboard header: factual drive summary on the empty-pane
     background. Reads above the shortcut table on the lone-pane
     case only (multi-pane bg keeps the bare logo). Kept compact
     so the surface still scans as a soft empty state rather
     than a marketing splash. */
  .dashboard-header {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    margin-top: -0.5rem;
  }
  .dashboard-name {
    font-size: 18px;
    color: var(--text);
    opacity: 0.85;
    letter-spacing: 0.01em;
  }
  .dashboard-stats {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: 0.4rem;
    color: var(--text-secondary);
    font-size: 12px;
  }
  .dashboard-stats .sep {
    opacity: 0.5;
  }
  /* Index activity pulls toward --warn-text so a building /
     reindexing line draws the eye without becoming an error. */
  .dashboard-index {
    color: var(--warn-text);
  }
  @media (prefers-reduced-motion: reduce) {
    .tab,
    .tab:hover {
      transition: background 80ms ease, color 80ms ease;
      transform: none;
    }
  }
</style>
