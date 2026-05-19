<script lang="ts">
  // One pane: a horizontal tab strip on top, an editor below.

  import {
    activeLayout,
    closeTab,
    enterPaneMode,
    focusColorForWindow,
    isDirty,
    detachTabToPaneEdge,
    layout,
    markLocalTabDrop,
    moveTab,
    openInPane,
    openBrowserInActivePane,
    openTerminalInPane,
    paneMode,
    paneWobble,
    reorderTab,
    reopenClosedTab,
    setActivePane,
    setWindowFocusColor,
    setTerminalActivity,
    shouldCloseTabAfterDragEnd,
    type FocusColor,
    type LeafNode,
    type PaneDropEdge,
    type Tab,
  } from "../state/tabs.svelte";

  import {
    Check,
    FileText,
    Folder,
    LayoutGrid,
    Moon,
    Network,
    PanelRight,
    Palette,
    Radio,
    RefreshCw,
    Search,
    Settings,
    Sun,
    Terminal,
    User,
  } from "lucide-svelte";
  import EmptyPaneCarousel from "./EmptyPaneCarousel.svelte";
  import FileEditorTab from "./FileEditorTab.svelte";
  import FileBrowserSurface from "./FileBrowserSurface.svelte";
  import GraphPanel from "./GraphPanel.svelte";
  import HamburgerMenu from "./HamburgerMenu.svelte";
  import TerminalTab from "./TerminalTab.svelte";
  import {
    driveDisplayName,
    refreshTree,
    scheduleSessionSave,
    tree,
    ui,
  } from "../state/store.svelte";
  import {
    tabLabel,
    tabLabelInPane,
    tabTooltip,
    truncateTabTitle,
  } from "../state/tabs.svelte";
  import type { BrowserLabelCtx } from "../state/tabs.svelte";
  import {
    SHORTCUTS,
    currentOS,
    currentPlatform,
    formatChord,
  } from "../state/shortcuts";
  import { openTabMenu, tabMenu } from "../state/tabMenu.svelte";
  import { onDestroy, onMount } from "svelte";
  import { applyPageWidthToElement, pageWidth } from "../state/pageWidth.svelte";

  let { pane }: { pane: LeafNode } = $props();

  const active = $derived(pane.tabs.find((t) => t.id === pane.activeTabId) ?? null);

  /// Per-row is_dir lookup for the active tree, keyed by path. Drives
  /// the File-Browser tab title which needs to render "the parent
  /// dir of the selected file" or "the selected directory" — and
  /// the only way to disambiguate file vs dir on a `selected` path
  /// is to consult the tree. Re-derives whenever the tree refreshes.
  const treeIsDir = $derived<Map<string, boolean>>(
    new Map(tree.entries.map((e) => [e.path, e.is_dir])),
  );
  function browserCtxFor(tab: Tab): BrowserLabelCtx {
    if (tab.kind !== "browser") return {};
    const sel = tab.selected ?? undefined;
    const selectedIsDir = sel ? treeIsDir.get(sel) : undefined;
    return { driveName: driveDisplayName(), selectedIsDir };
  }

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

  /// Platform + OS settle once at module init: the chord set (web
  /// vs native) and Mod label (Cmd vs Ctrl) don't change at runtime.
  /// Used by the welcome-menu chord column below; the carousel
  /// owns its own copy + the shortcut table.
  const platform = currentPlatform();
  const os = currentOS();
  const paneFocusColors: FocusColor[] = ["blue", "green", "pink"];

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
  const emptyPaneActions: EmptyMenuRow[] = [
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
    e.preventDefault();
    setActivePane(pane.id);
    closePaneHamburgerMenu();
    emptyPaneMenu?.openAtCursor(e.clientX, e.clientY);
  }

  function onEmptyPaneContextMenu(e: MouseEvent): void {
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

  function onEnterPaneMode(): void {
    closePaneMenus();
    enterPaneMode();
  }

  /// `fullstack-59`: per-Hybrid theme override. Click on the
  /// Hybrid chrome's theme button cycles between "follow global"
  /// (no override) and "override to the opposite of global". One
  /// override slot per side; `flipHybrid()` already swaps the
  /// stored override with the back-side override. The data-theme
  /// attribute on the pane root drives the CSS cascade via the
  /// `:global(.pane[data-theme="..."])` rules in App.svelte.
  function paneEffectiveTheme(): "dark" | "light" {
    return pane.theme ?? ui.theme;
  }

  function paneThemeTooltip(): string {
    if (pane.theme === undefined) {
      return `Theme: follow global (${ui.theme}). Click to override.`;
    }
    return `Theme: ${pane.theme} (per-Hybrid). Click to follow global.`;
  }

  function togglePaneTheme(): void {
    if (pane.theme === undefined) {
      pane.theme = ui.theme === "dark" ? "light" : "dark";
    } else {
      pane.theme = undefined;
    }
    scheduleSessionSave();
  }

  /// Subscribe to the structural-wobble bus. Each splitPane /
  /// closePane / paneModeSwap bumps `paneWobble.versions[pane.id]`;
  /// we mirror that into a class toggle so the CSS animation
  /// re-fires (CSS animations don't replay on a static class —
  /// we briefly drop the class, then re-add it on rAF). The
  /// `onanimationend` handler clears the class so the next event
  /// can re-trigger cleanly.
  const wobbleVersion = $derived(paneWobble.versions[pane.id] ?? 0);
  let wobbleActive = $state(false);
  let lastWobbleVersion = 0;
  $effect(() => {
    if (wobbleVersion === lastWobbleVersion) return;
    lastWobbleVersion = wobbleVersion;
    if (wobbleVersion === 0) return;
    wobbleActive = false;
    requestAnimationFrame(() => {
      wobbleActive = true;
    });
  });

  /// `fullstack-48` Phase C: small flashing dot on the pane chrome
  /// when the OTHER side of the Hybrid has something unread /
  /// active. Designed as a generic signal so future attention
  /// sources (terminal activity per `fullstack-25`, etc.) plug in
  /// here without re-spec. Today the wired sources are:
  ///   - any terminal tab on the hidden side with
  ///     `watcher.unread === true`
  ///   - any terminal tab on the hidden side with
  ///     `terminalActivity === true`
  /// The check is symmetric: the indicator shows on whichever side
  /// is currently visible whenever the hidden side has attention,
  /// and clears the moment the user flips to that side (since
  /// flipping moves those tabs into `pane.tabs`).
  const backHasAttention = $derived.by<boolean>(() => {
    if (!pane.back) return false;
    for (const tab of pane.back.tabs) {
      if (tab.kind !== "terminal") continue;
      if (tab.watcher?.unread) return true;
      if (tab.terminalActivity) return true;
    }
    return false;
  });

  function closePaneMenus(): void {
    paneMenu?.close();
    paneContextMenu?.close();
    emptyPaneMenu?.close();
  }

  function closePaneContextMenus(): void {
    paneContextMenu?.close();
    emptyPaneMenu?.close();
  }

  function closePaneHamburgerMenu(): void {
    paneMenu?.close();
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

  function doSetFocusColor(color: FocusColor): void {
    closePaneMenus();
    setWindowFocusColor(color);
  }

  function openPaneContextAt(e: MouseEvent): void {
    e.preventDefault();
    setActivePane(pane.id);
    closePaneHamburgerMenu();
    paneContextMenu?.openAtCursor(e.clientX, e.clientY);
  }

  /// Fire the same `chan:command` event the keymap layer uses so
  /// every shortcut row routes through the existing dispatcher in
  /// App.svelte. Avoids re-implementing the actions here.
  function dispatchCommand(id: string): void {
    emptyPaneMenu?.close();
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

  // `fullstack-56`: removed `onSave()` + the Cmd+S keystroke
  // interception. Autosave (debounced on idle + tab-close +
  // visibility hooks) is the canonical write path; the explicit
  // shortcut + action don't pull their weight. Cmd+Shift+S
  // strikethrough is owned by the editor and unaffected since the
  // plain-S gate is gone.

  function onKeyDown(e: KeyboardEvent): void {
    if (e.key === "Escape" && (paneMenuOpen || paneContextMenuOpen || emptyPaneMenuOpen)) {
      e.preventDefault();
      closePaneMenus();
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
  class:wobble={wobbleActive}
  data-focus-color={focusColorForWindow()}
  data-theme={pane.theme}
  onmousedown={() => setActivePane(pane.id)}
  onanimationend={(e) => {
    if (e.animationName === "pane-wobble-once") wobbleActive = false;
  }}
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
      if (pane.tabs.length === 0) openEmptyPaneMenuAt(e);
      else openPaneContextAt(e);
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
          if (t.kind === "terminal") setTerminalActivity(t, false);
        }}
        onclick={() => {
          tabMouseDownPrevActive = null;
        }}
        oncontextmenu={(e) => {
          e.preventDefault();
          e.stopPropagation();
          pane.activeTabId = t.id;
          if (t.kind === "terminal") setTerminalActivity(t, false);
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
            if (t.kind === "terminal") setTerminalActivity(t, false);
            layout.activePaneId = pane.id;
            openTabMenu(t.id, {
              left: e.clientX,
              top: e.clientY,
              right: e.clientX,
              bottom: e.clientY,
            });
          }}
        >{truncateTabTitle(tabLabelInPane(t, pane.tabs, browserCtxFor(t)))}</span>
        {#if isDirty(t)}
          <span class="dirty unsaved" title="unsaved changes">●</span>
        {/if}
        {#if t.kind === "terminal" && t.terminalActivity}
          <span
            class="dirty activity"
            title="terminal output since last focus"
            aria-label="terminal output since last focus"
          >●</span>
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
      {#if backHasAttention}
        <!-- fullstack-48 Phase C: back-side-attention indicator.
             Sits just left of the hamburger so it's adjacent to
             the chrome the user is already scanning. Cleared
             automatically when the user flips because the
             attention sources move into the visible-side tabs. -->
        <span
          class="back-attention"
          aria-label="back side has unread activity"
          title="back side has unread activity (Cmd+K Tab to flip)"
        ></span>
      {/if}
      <!-- fullstack-59: per-Hybrid theme override toggle. Icon
           shows the theme the click WILL apply (Sun in dark mode
           offers a switch to light; Moon in light mode offers a
           switch to dark). When the pane already has an override,
           a second click clears it back to "follow global". -->
      <button
        type="button"
        class="pane-theme-toggle"
        class:overridden={pane.theme !== undefined}
        title={paneThemeTooltip()}
        aria-label={paneThemeTooltip()}
        onclick={togglePaneTheme}
      >
        {#if paneEffectiveTheme() === "dark"}
          <Sun size={14} strokeWidth={1.75} aria-hidden="true" />
        {:else}
          <Moon size={14} strokeWidth={1.75} aria-hidden="true" />
        {/if}
      </button>
      <!-- Pane-only controls live inside a single hamburger menu
           to match the file browser / search / graph overlays.
           Split rows hide when the platform doesn't allow any splits
           (iPhone) and grey out when the platform's cap is reached
           (iPad after one split, native desktop / web have no cap). -->
      <HamburgerMenu
        bind:this={paneMenu}
        bind:open={paneMenuOpen}
        width={250}
        height={360}
        onBeforeOpen={closePaneContextMenus}
      >
        <li>
          <button role="menuitem" onclick={onEnterPaneMode}>
            <LayoutGrid size={16} strokeWidth={1.75} aria-hidden="true" />
            <span class="menu-row-label">Enter Hybrid NAV</span>
            <span class="menu-row-chord">{chordLabel("app.pane.mode")}</span>
          </button>
        </li>
        <li class="sep" role="separator"></li>
        <li class="menu-label">
          <Palette size={16} strokeWidth={1.75} aria-hidden="true" />
          <span>Focus border colour</span>
        </li>
        {#each paneFocusColors as color}
          <li>
            <button role="menuitem" onclick={() => doSetFocusColor(color)}>
              <span class={`color-dot ${color}`} aria-hidden="true"></span>
              <span>{color}</span>
              {#if focusColorForWindow() === color}
                <Check size={14} strokeWidth={2} aria-hidden="true" />
              {/if}
            </button>
          </li>
        {/each}
      </HamburgerMenu>
      <HamburgerMenu
        bind:this={paneContextMenu}
        bind:open={paneContextMenuOpen}
        showTrigger={false}
        width={220}
        height={110}
        onBeforeOpen={closePaneHamburgerMenu}
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
      <div class="pane-mode-preview" aria-label="Hybrid NAV preview">
        <div class="pane-mode-title">{active ? truncateTabTitle(tabLabel(active, browserCtxFor(active))) : "Empty pane"}</div>
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
        oncontextmenu={onEmptyPaneContextMenu}
        role="presentation"
      >
        <!-- Single-pane lone-pane case renders the full carousel
             (welcome + metadata + indexing graph). Multi-pane
             empty panes are usually workspace setup ("about to
             drop files here"), so the chrome stays minimal — just
             the chan mark, no carousel. -->
        {#if !multiPane}
          <EmptyPaneCarousel oncontextmenu={onEmptyPaneContextMenu} />
        {:else}
          <div class="placeholder-stack">
            <div class="placeholder-mark"></div>
          </div>
        {/if}
        <!-- Right-click menu. Triggerless: opens only via the
             contextmenu handler above. Same `chan:command` ids as
             the keymap layer so actions stay unified. -->
        <HamburgerMenu
          bind:this={emptyPaneMenu}
          bind:open={emptyPaneMenuOpen}
          showTrigger={false}
          width={280}
          height={260}
          onBeforeOpen={closePaneHamburgerMenu}
        >
          {#each emptyPaneActions as row (row.command)}
            {@const Icon = row.icon}
            <li>
              <button role="menuitem" onclick={() => dispatchCommand(row.command)}>
                <Icon size={16} strokeWidth={1.75} aria-hidden="true" />
                <span class="menu-row-label">{row.label}</span>
                <span class="menu-row-chord">{chordLabel(row.chordId)}</span>
              </button>
            </li>
          {/each}
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
        <TerminalTab
          tab={t}
          paneId={pane.id}
          active={t.id === pane.activeTabId}
          focused={t.id === pane.activeTabId && viewLayout.activePaneId === pane.id}
        />
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
    /* Pane chrome — floating shade. Margin keeps panes off the
       workspace edge and off each other (the split divider is
       4px; with 4px margin on each side the inter-pane gap reads
       as one clean 12px channel). Overflow:hidden lets the
       rounded corners clip the tabs strip + editor body; the
       drop shadow paints inside the margin space (well outside
       the rounded box) so it isn't clipped by the .half wrapper. */
    margin: 4px;
    border-radius: 6px;
    overflow: hidden;
    box-shadow: var(--pane-shadow);
  }
  .pane[data-focus-color="blue"] { --pane-active-focus: var(--pane-focus); }
  .pane[data-focus-color="green"] { --pane-active-focus: #22c55e; }
  .pane[data-focus-color="pink"] { --pane-active-focus: #ff5fb7; }
  /* Inset glow rather than just a border-color swap: 2px reads
     clearly on both light and dark canvases, and the inset stays
     inside the pane's own box so the surrounding layout doesn't
     shift when focus moves between panes. The transparent border on
     `.pane` keeps the box dimensions stable; this shadow paints
     over the inside edge. Composes with the chrome shadow so the
     focused pane keeps both its floating shadow and its focus ring. */
  .pane.focused {
    border-color: var(--pane-active-focus);
    box-shadow:
      inset 0 0 0 2px var(--pane-active-focus),
      var(--pane-shadow);
  }
  /* Single-fire structural wobble. Triggered by tabs.svelte's
     paneWobble bus on split / close / pane-move; the .wobble
     class is toggled off in onanimationend so subsequent events
     can re-fire. Same easeOutBack curve as the tab/style-toolbar
     hover wobble so the motion language stays consistent — just
     a one-shot bounce instead of a hover transition. The 1.012
     scale is deliberately gentler than the tab's 1.04 because
     the pane is several hundred px across. */
  .pane.wobble {
    animation: pane-wobble-once 360ms cubic-bezier(0.34, 1.56, 0.64, 1);
  }
  @keyframes pane-wobble-once {
    0%   { transform: scale(1); }
    40%  { transform: scale(1.012); }
    100% { transform: scale(1); }
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
  .dirty.activity {
    color: var(--warning-text, #d29922);
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
  .actions { margin-left: auto; display: flex; align-items: center; gap: 6px; padding-left: 4px; }
  /* fullstack-48 Phase C — back-side-attention indicator. Same
     warn-text family as the terminal activity marker so a user
     learns one colour-language across the chrome. The 1.5s alpha
     cycle is gentle enough to read as "pulsing" without becoming
     a strobe at the corner of vision. prefers-reduced-motion
     replaces the pulse with a static dot. */
  .back-attention {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--warn-text, #d29922);
    flex-shrink: 0;
    animation: back-attention-pulse 1.5s ease-in-out infinite;
  }
  @keyframes back-attention-pulse {
    0%, 100% { opacity: 1; }
    50%      { opacity: 0.35; }
  }
  @media (prefers-reduced-motion: reduce) {
    .back-attention { animation: none; }
  }
  /* fullstack-59: per-Hybrid theme toggle. Same chrome footprint
     as the hamburger so the two read as a pair; coloured swatch
     ring when the override is active so it's visible at a glance
     that this pane diverges from the global theme. */
  .pane-theme-toggle {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 22px;
    height: 22px;
    padding: 0;
    background: transparent;
    color: var(--text-secondary);
    border: 1px solid transparent;
    border-radius: 4px;
    cursor: pointer;
    flex-shrink: 0;
  }
  .pane-theme-toggle:hover {
    background: var(--hover-bg);
    color: var(--text);
  }
  .pane-theme-toggle.overridden {
    color: var(--link);
    border-color: var(--link);
  }
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
  /* Empty pane shell. Single-pane lone-pane case is hosted by the
     EmptyPaneCarousel component (welcome / metadata / indexing
     slides) which owns its own layout. Multi-pane empty case
     keeps the bare-logo placeholder-stack rhythm since adding
     a full carousel to a setup pane would just clutter. */
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
  @media (prefers-reduced-motion: reduce) {
    .tab,
    .tab:hover {
      transition: background 80ms ease, color 80ms ease;
      transform: none;
    }
  }
</style>
