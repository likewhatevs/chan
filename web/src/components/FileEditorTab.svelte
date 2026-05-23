<script lang="ts">
  // Body of a file tab. The previous top "tab-bar" (Aa, page-width,
  // formatting group, reveal-in-browser, mode toggle, outline toggle)
  // is split across two surfaces: a per-tab popover anchored to the
  // tab title click (page width + duplicate / reveal / mode / outline /
  // show-style-toolbar actions), plus a floating style toolbar pinned
  // to the top-left of the editor canvas (block kind + B/I/S/code/link/
  // lists/HR/image). The bubble's formatting row used to sit inside
  // the popover; lifting it out of there reduces the chrome users have
  // to twirl through and keeps formatting one mouse-move away.

  import Wysiwyg from "../editor/Wysiwyg.svelte";
  import Source from "../editor/Source.svelte";
  import {
    cancelPendingBufferWrite,
    clearEditorBuffer,
    divergentBufferOrNull,
    queueBufferWrite,
    type EditorBuffer,
  } from "../state/editorBuffer";
  import JsonPretty from "../editor/JsonPretty.svelte";
  import CsvTable from "../editor/CsvTable.svelte";
  import {
    ArrowLeft,
    ArrowRight,
    Braces,
    Code2,
    Copy,
    Eraser,
    FilePlus,
    Folder,
    History,
    Highlighter,
    Network,
    Pencil,
    Pilcrow,
    RotateCw,
    Search as SearchIcon,
    Settings2,
    Square,
    Table2,
    Terminal as TerminalIcon,
    Type,
    X,
  } from "lucide-svelte";
  import {
    SHORTCUTS,
    currentOS,
    currentPlatform,
    formatChord,
  } from "../state/shortcuts";
  import FindBar from "./FindBar.svelte";
  import Inspector from "./Inspector.svelte";
  import OutlineBody, { type Heading } from "./OutlineBody.svelte";
  import FileInfoBody from "./FileInfoBody.svelte";
  import StyleToolbar from "./StyleToolbar.svelte";
  import { clampMenu } from "./menuClamp";
  import {
    layout,
    attemptInPlaceReopen,
    beginMissingFileReopen,
    canReopenClosedTab,
    closeTab,
    flipHybrid,
    openFind,
    reopenClosedTab,
    setMode,
    setTabCaret,
    setTabInspectorOpen,
    setTabCodeBlocksCollapsed,
    setTabHighlightTrailingWhitespace,
    setTabOutlineOpen,
    setTabStyleToolbarOpen,
    setTabSyntaxHighlight,
    type FileTab,
  } from "../state/tabs.svelte";
  import WikiStatusBar from "./WikiStatusBar.svelte";
  import {
    renderedCaretForSourceCaret,
    sourceCaretForRenderedCaret,
  } from "../editor/caret_mapping";
  import { stripTrailingWhitespaceText } from "../editor/tools";

  import {
    fileOps,
    openFsGraphForFile,
    openGraphForFile,
    openGraphForTag,
    searchPanel,
    openGraphAtNode,
    paneWidths,
    persistPaneWidths,
    revealPathInBrowser,
    setTransientStatus,
    ui,
  } from "../state/store.svelte";
  import {
    openInActivePane,
    openTerminalInPane,
    tabFocusPulse,
  } from "../state/tabs.svelte";
  import { terminalFromHereTarget } from "../terminal/fromHere";
  import { csvDelimiter, isCsv, isJson } from "../state/fileTypes";
  import { api } from "../api/client";
  import {
    PAGE_WIDTH_MAX_PCT,
    PAGE_WIDTH_MIN_PCT,
    PAGE_WIDTH_STEP_PCT,
    pageWidth,
    setPageWidth,
  } from "../state/pageWidth.svelte";
  import {
    tabMenu,
    closeTabMenu,
    openTabMenu,
  } from "../state/tabMenu.svelte";
  // `fullstack-a-67f`: dropped `isTauriDesktop` / `openWebInspector`
  // / `reloadWindow` + the `notify` import. The Editor right-click
  // menu no longer carries the `-b-26` Reload / Open Inspector
  // tail entries per addendum-a's verbatim spec; Cmd+R + the pane
  // hamburger remain the canonical surfaces. Same drop pattern as
  // `-a-67d` (Terminal).

  let { tab }: { tab: FileTab } = $props();

  // Editor refs so the outline body can call scrollToHeading /
  // scrollToLine on whichever editor variant is showing, and so
  // the toolbar can call into the Wysiwyg formatting API.
  let wysiwygRef: Wysiwyg | undefined = $state();
  let sourceRef: Source | undefined = $state();

  // `fullstack-a-64`: when the user switches into this editor tab
  // via Cmd+Shift+[ / Cmd+Shift+] / Ctrl+Alt+1..9, the chord
  // handler bumps `tabFocusPulse`. FileEditorTab only mounts when
  // it's the active tab, so any pulse increment during our
  // lifetime means we just became the focus target. Re-focus the
  // appropriate editor ref via the `focus()` export we added to
  // Source/Wysiwyg in this task. queueMicrotask defers past the
  // synchronous chord-handler stack so the editor view has had a
  // tick to take the activeElement back from `<body>` (which
  // `bumpTabFocusPulse` parks us on by blurring the prior focus).
  $effect(() => {
    tabFocusPulse.value;
    queueMicrotask(() => {
      if (tab.mode === "wysiwyg") wysiwygRef?.focus();
      else sourceRef?.focus();
    });
  });

  // `fullstack-a-72` hang-recovery via localStorage; `-a-74`
  // moved the debounce machinery into editorBuffer.ts so a
  // `beforeunload` / `pagehide` flush can persist pending
  // writes before the page tears down (force-reload skips
  // Svelte component cleanups, which had been the previous
  // flush trigger):
  //
  // * On mount: compare any pre-existing buffer to the
  //   just-loaded disk content (`tab.saved`). If divergent,
  //   surface `recoveredBuffer` so the banner renders + the
  //   user can restore or discard.
  // * On every content mutation: queue a debounced (500ms)
  //   write via `queueBufferWrite` — the registry lives in
  //   editorBuffer.ts so App.svelte's unload listeners can
  //   flush all pending writes synchronously.
  // * On save success / discard / clean transition / unmount:
  //   cancel any pending write + clear the persisted buffer
  //   via the appropriate helper.
  let recoveredBuffer: EditorBuffer | null = $state(null);

  $effect(() => {
    // Mount-time divergence check. Use the AS-LOADED disk
    // content (`tab.saved`) — not `tab.content` — since the
    // user may have already started editing this tick and we
    // need to compare against what the FILE has, not what the
    // BUFFER would have.
    //
    // `fullstack-a-82`: key the buffer on `tab.path` (not
    // `tab.id`). Tab ids are module-counter-generated and
    // reset on every page load — so a pre-`-a-82` buffer
    // written as `chan:editor-buffer:<tabId>` became a dead
    // reference after the user force-reloaded. Path keys
    // survive the reload + the tab-id regeneration.
    const disk = tab.saved ?? tab.content;
    recoveredBuffer = divergentBufferOrNull(tab.path, tab.path, disk);
    return () => {
      // On graceful unmount (Cmd+W close / tab swap), cancel
      // any pending write. Force-reload doesn't run this path;
      // App.svelte's `beforeunload` / `pagehide` listeners
      // call `flushPendingBufferWrites` to handle that.
      cancelPendingBufferWrite(tab.path);
    };
  });

  $effect(() => {
    // Persistence on content mutation. Track `tab.content` +
    // `tab.saved` as reactive deps so the effect re-runs on
    // every mutation. Skip the write when the content matches
    // the saved-on-disk content (clean state — nothing to
    // recover; clear any leftover buffer + cancel any pending
    // write so a stale recovered-state doesn't surface on
    // next reload).
    //
    // `fullstack-a-82`: ALSO skip when `tab.saved` is undefined
    // — that means the disk content hasn't finished loading
    // yet (the openInActivePane initialiser sets
    // `content: ""` + the async fetch fills in `tab.saved`
    // later). Without this guard the mount-time effect would
    // queue an empty `""` write that races the file fetch +
    // could clobber the freshly-restored buffer with empty
    // content after the 500ms debounce.
    //
    // `fullstack-a-83`: when a recovery banner is currently
    // surfacing, DO NOT clear the buffer in the clean-state
    // branch. This guards the effect-ordering race
    // @@WebtestA caught: the mount-time effect + the
    // persistence effect both fire in the same tick once
    // `tab.saved` arrives;
    // depending on the microtask order, the persistence
    // effect's `clearEditorBuffer` can wipe the localStorage
    // buffer between the mount effect's read and the user's
    // chance to act on it. Banner stays on screen only until
    // the user clicks Restore (which replaces `tab.content`)
    // or Discard (which explicitly clears the buffer). Both
    // paths drop `recoveredBuffer` to null + the next
    // persistence-effect run will reach the clean-state
    // branch + clear normally.
    const content = tab.content;
    const saved = tab.saved;
    if (saved === undefined) return;
    if (content === saved) {
      if (recoveredBuffer !== null) {
        // Banner is up — leave the buffer in place so the
        // user can still click Restore. Skip queue cancel
        // too; queueBufferWrite is a no-op in the clean
        // state anyway.
        return;
      }
      cancelPendingBufferWrite(tab.path);
      clearEditorBuffer(tab.path);
      return;
    }
    queueBufferWrite(tab.path, content, tab.path);
  });

  function restoreFromBuffer(): void {
    if (!recoveredBuffer) return;
    tab.content = recoveredBuffer.content;
    recoveredBuffer = null;
    // The newly-set content is now divergent from `tab.saved`,
    // so the debounced write effect will re-persist it on the
    // next tick. No need to clear storage here.
  }

  function discardBuffer(): void {
    // `fullstack-a-83` follow-up to `-a-82`: discard must use
    // `tab.path` to match the path-keyed storage. The
    // pre-`-a-83` `tab.id` form was a stale relic from before
    // `-a-82` re-keyed the buffer — it silently no-op'd
    // (tab.id changes on every reload, so `clearEditorBuffer`
    // was called against a non-existent key). The banner
    // would clear from `recoveredBuffer = null` but the
    // localStorage entry would linger until natural
    // expiration.
    clearEditorBuffer(tab.path);
    recoveredBuffer = null;
  }

  /// Read-only mode for this tab. The status bar's lamp toggle
  /// drives `tab.readMode` directly; an OS-level read-only file
  /// (no user-write bit) is reflected through `tab.fsWritable`
  /// and overrides the lamp so the user can't try to write.
  /// Per-tab so multi-pane layouts can mix read/write without
  /// a global signal fighting between panes.
  const readOnly = $derived(tab.readMode || !tab.fsWritable);

  /// 0-indexed source line under the caret. Drives the outline's
  /// active-heading marker (Google-Docs-style "you are here" bar
  /// on the guide line). Counts newlines up to tab.caret.from in
  /// O(n) which is fine for the buffer sizes chan deals with.
  const caretLine = $derived.by((): number | null => {
    if (!tab.caret) return null;
    const upto = tab.content.slice(0, tab.caret.from);
    let n = 0;
    for (let i = 0; i < upto.length; i++) if (upto.charCodeAt(i) === 10) n++;
    return n;
  });

  // Bumped on every selection / doc change in the WYSIWYG editor so
  // the StyleToolbar's active-mark / current-block derivations re-run.
  // The value itself doesn't matter; the dependency does. Toolbar
  // lives in a child component now; we still own the signal so any
  // sibling (status bar, outline) can hook into it later.
  let selVer = $state(0);

  function jumpTo(h: Heading): void {
    if (tab.mode === "wysiwyg") wysiwygRef?.scrollToHeading(h.index);
    else sourceRef?.scrollToLine(h.line);
  }

  // Find-on-page adapter for whichever editor is mounted. Both
  // editors expose `findAdapter` (see editor/find.ts FindAdapter)
  // with the same shape; FindBar.svelte drives it. We re-derive
  // on mode flip so a Wysiwyg <-> Source toggle while the bar is
  // open re-paints highlights against the new view.
  const findAdapter = $derived(
    tab.mode === "wysiwyg" ? wysiwygRef?.findAdapter : sourceRef?.findAdapter,
  );

  /// Reveal the open file in a File Browser tab. Expand every
  /// ancestor directory so the row is visible, set the browser
  /// selection to this file, then focus/create the tab. Mirrors the
  /// post-create/move "land next to the thing you just produced"
  /// flow in `revealAndSelect`.
  function revealInBrowser(): void {
    revealPathInBrowser(tab.path, { inspectorOpen: true });
    closeTabMenu();
  }

  // In-tab find was removed; the browser's native ⌘F applies. The
  // editor's selectable text (WYSIWYG and source) is plain DOM, so
  // browser find lights up matches the way users already expect.

  /// True while the popover for THIS tab is open. The tab-menu state
  /// is shared so the trigger button (in Pane.svelte's tab strip) can
  /// open it; the bubble itself renders here so it has direct access
  /// to `wysiwygRef` + the reactive `selVer` signal.
  const menuOpen = $derived(tabMenu.openForTabId === tab.id);

  /// Bubble positioning. The desired anchor is the trigger's bottom-
  /// left; the actual placement runs through `clampMenu` so a tab
  /// docked near the right or bottom edge still renders fully on-
  /// screen (clamp re-flips to the left / above as needed).
  const menuPos = $derived.by(() => {
    const a = tabMenu.anchor;
    if (!a) return { x: 0, y: 0 };
    return { x: Math.round(a.left), y: Math.round(a.bottom + 4) };
  });

  function onMenuKeydown(e: KeyboardEvent): void {
    if (e.key === "Escape" && menuOpen) {
      e.preventDefault();
      closeTabMenu();
    }
  }

  /// Dismiss when the click lands outside the bubble AND outside any
  /// tab row (the row's own click handler already toggles the state,
  /// so we ignore clicks that bubble up from it — without this guard
  /// the global handler closes the menu before the row handler has a
  /// chance to reopen it, and a second click on the active tab feels
  /// dead).
  function onDocPointerDown(e: PointerEvent): void {
    if (!menuOpen) return;
    const t = e.target as Node | null;
    if (!t) return;
    const bubble = document.querySelector(".tab-menu-bubble");
    if (bubble && bubble.contains(t)) return;
    const trigger = (t as Element).closest?.(".tab");
    if (trigger) return;
    closeTabMenu();
  }

  function onPageWidthSlider(e: Event): void {
    const pct = Number((e.currentTarget as HTMLInputElement).value);
    setPageWidth(pct / 100);
  }

  function doNewFile(): void {
    closeTabMenu();
    void fileOps.createFile(parentPath(tab.path));
  }

  function doReopenClosedTab(): void {
    closeTabMenu();
    reopenClosedTab();
  }

  function paneIdForTab(): string | null {
    for (const [paneId, node] of Object.entries(layout.nodes)) {
      if (node.kind === "leaf" && node.tabs.some((candidate) => candidate.id === tab.id)) {
        return paneId;
      }
    }
    return null;
  }

  function doCloseTab(): void {
    closeTabMenu();
    const paneId = paneIdForTab();
    if (paneId) void closeTab(paneId, tab.id);
  }

  /// `fullstack-a-67f`: dropped `doCloseOthers` + `doCloseAll`.
  /// Addendum-a's Editor menu spec lists only "Close" + "Reopen
  /// last tab" in the foot. Close-others / close-all aren't
  /// listed; trivial to restore if Alex routes a follow-up.

  function parentPath(path: string): string {
    const slash = path.lastIndexOf("/");
    return slash < 0 ? "" : path.slice(0, slash);
  }

  async function doCopyPath(): Promise<void> {
    closeTabMenu();
    try {
      await navigator.clipboard?.writeText(tab.path);
      // `fullstack-a-86`: success toast auto-dismisses (3s)
      // — same shape as `-a-85`'s move-success fix.
      setTransientStatus("Copied file path");
    } catch (err) {
      ui.status = `copy failed: ${(err as Error).message}`;
    }
  }

  function doDuplicate(): void {
    closeTabMenu();
    void fileOps.duplicateFile(tab.path);
  }

  // `fullstack-a-35`: inline-rename header band. Replaces the
  // modal-driven rename (`fileOps.rename`) with the terminal
  // tab's "trigger from the right-click menu, commit inline"
  // shape. The band sits above the editor's page-width-capped
  // content (no `--chan-page-max-width` constraint), Enter commits,
  // Esc cancels.
  /// `fullstack-a-67f`: in-menu inline rename per addendum-a's
  /// "Name, editable like Terminal's" spec. The legacy `-a-35`
  /// full-width inline rename band above the editor body is
  /// dropped; the menu input replaces it. Commits on Enter +
  /// blur (NOT every keystroke — file rename is destructive +
  /// cross-tree, can't fire on each character). Uses
  /// `fileOps.renameInPlace` which handles path traversal
  /// (`../other/dir/`), extension preservation, and link
  /// rewriting.
  let nameDraft = $state("");
  $effect(() => {
    // Sync the draft to the tab path when the underlying file
    // changes (e.g. external rename, or another menu surface).
    // Fires on mount so the input starts seeded; fires again on
    // tab.path mutation so external renames stay reflected.
    nameDraft = tab.path;
  });
  async function commitTabName(): Promise<void> {
    const next = nameDraft.trim();
    if (!next || next === tab.path) {
      nameDraft = tab.path;
      return;
    }
    await fileOps.renameInPlace(tab.path, next, false);
  }
  function onTabNameKey(e: KeyboardEvent): void {
    if (e.key === "Enter") {
      e.preventDefault();
      (e.currentTarget as HTMLInputElement).blur();
    } else if (e.key === "Escape") {
      e.preventDefault();
      nameDraft = tab.path;
      (e.currentTarget as HTMLInputElement).blur();
    }
  }

  /// True for tabs that have a structured render mode alongside
  /// source mode. Markdown (wysiwyg), JSON (pretty), CSV/TSV (table).
  /// Arbitrary text tabs do not (source is the only sensible
  /// surface for a .py / .toml / Makefile).
  const hasRenderedMode = $derived(
    tab.fileKind !== "text" || isJson(tab.path) || isCsv(tab.path),
  );

  /// Which render mode this tab pairs with source mode. Drives the
  /// toggle button copy + the icon picker below.
  const renderedModeForTab = $derived<"wysiwyg" | "pretty" | "table">(
    isJson(tab.path) ? "pretty" : isCsv(tab.path) ? "table" : "wysiwyg",
  );

  function doToggleMode(): void {
    if (!hasRenderedMode) return;
    const rendered = renderedModeForTab;
    const next = tab.mode === "source" ? rendered : "source";
    if (tab.caret && rendered === "wysiwyg") {
      const mapped =
        next === "wysiwyg"
          ? renderedCaretForSourceCaret(tab.content, tab.caret)
          : sourceCaretForRenderedCaret(tab.content, tab.caret);
      setTabCaret(tab, mapped.from, mapped.to);
    }
    setMode(tab, next);
    closeTabMenu();
  }

  function doToggleSyntaxHighlight(): void {
    setTabSyntaxHighlight(tab, !tab.syntaxHighlight);
    closeTabMenu();
  }

  const markdownToolsEnabled = $derived(tab.fileKind !== "text");

  function doToggleTrailingWhitespace(): void {
    setTabHighlightTrailingWhitespace(tab, !tab.highlightTrailingWhitespace);
    closeTabMenu();
  }

  function doToggleCodeBlocks(): void {
    if (!markdownToolsEnabled) return;
    const changed =
      tab.mode === "wysiwyg"
        ? wysiwygRef?.toggleCodeBlocksInEditor()
        : sourceRef?.toggleCodeBlocksInEditor();
    if (changed) setTabCodeBlocksCollapsed(tab, !tab.codeBlocksCollapsed);
    closeTabMenu();
  }

  function doRemoveTrailingWhitespace(): void {
    const changed =
      tab.mode === "wysiwyg"
        ? wysiwygRef?.removeTrailingWhitespaceInEditor()
        : sourceRef?.removeTrailingWhitespaceInEditor();
    if (!changed) {
      const stripped = stripTrailingWhitespaceText(tab.content);
      if (stripped !== tab.content) tab.content = stripped;
    }
    closeTabMenu();
  }

  // ---- right-click context menu --------------------------------------
  // Re-uses the existing tab menu bubble (the same one that opens
  // from the tab dot). The bubble carries Duplicate / Rename /
  // mode-toggle / outline / style-toolbar plus our three new
  // actions (Reload / Search / Graph from here). Anchored at
  // the click coords by synthesizing a zero-size rect.

  function onEditorContext(e: MouseEvent): void {
    e.preventDefault();
    e.stopPropagation();
    openTabMenu(tab.id, {
      left: e.clientX,
      top: e.clientY,
      right: e.clientX,
      bottom: e.clientY,
    });
  }

  async function doReload(): Promise<void> {
    closeTabMenu();
    try {
      const res = await api.read(tab.path);
      tab.content = res.content;
      tab.saved = res.content;
      tab.savedMtime = res.mtime;
    } catch (err) {
      console.error("[chan] reload failed", err);
    }
  }

  /// `fullstack-a-67f`: dropped `doReloadWindow` + `doOpenInspector`
  /// helpers. Same drop pattern as `-a-67d` (Terminal). Cmd+R + the
  /// pane hamburger remain the canonical surfaces.

  /// `fullstack-a-67f`: replaced direct `openSettings()` (global
  /// Settings overlay) with `flipToSettings()` (per-tab back-side
  /// flip). Mirrors the addendum-a spec ("Settings (toggle that
  /// flips the terminal to show settings)") which the
  /// FBSurface (`-a-67e`) + TerminalTab (`-a-67d`) menus already
  /// adopted.
  function flipToSettings(): void {
    closeTabMenu();
    const paneId = paneIdForTab();
    if (paneId) flipHybrid(paneId);
  }

  /// `fullstack-a-67f`: "From $CWD" spawn entries. Mirror of the
  /// `-a-67d` Terminal pattern — each closes the menu + fires the
  /// canonical `chan:command` event so the chord layer + the
  /// empty-pane carousel + this menu converge on one handler.
  function dispatchChanCommand(id: string): void {
    window.dispatchEvent(
      new CustomEvent("chan:command", { detail: { name: id } }),
    );
  }
  function doNewTerminal(): void {
    closeTabMenu();
    dispatchChanCommand("app.terminal.toggle");
  }
  function doNewFileBrowser(): void {
    closeTabMenu();
    dispatchChanCommand("app.files.toggle");
  }
  function doNewGraph(): void {
    closeTabMenu();
    dispatchChanCommand("app.graph.toggle");
  }

  /// `fullstack-a-67f`: "Copy path to $CWD" — addendum-a wants
  /// the editor menu to expose both the file path and the
  /// parent-directory path. Parent dir is `tab.path` up to the
  /// last `/`; for root-level files the CWD is the drive root.
  async function doCopyCwdPath(): Promise<void> {
    closeTabMenu();
    const slash = tab.path.lastIndexOf("/");
    const cwd = slash > 0 ? tab.path.slice(0, slash) : "";
    try {
      await navigator.clipboard?.writeText(cwd);
    } catch (err) {
      ui.status = `copy failed: ${(err as Error).message ?? err}`;
    }
  }

  /// `fullstack-a-67f`: Find — opens the per-tab find bar via the
  /// existing `openFind(tabId)` helper that the `app.find.open`
  /// chord already routes to.
  function doFind(): void {
    closeTabMenu();
    openFind(tab.id);
  }

  function doOpenSearch(): void {
    closeTabMenu();
    // SearchPanel's open-effect calls extractSearchSeed() on the
    // open transition, which reads window.getSelection() and pre-
    // fills `searchPanel.query` when the user had text highlighted.
    // Same flow as the Mod+Shift+F chord — no extra wiring needed
    // beyond setting the open bit here.
    searchPanel.open = true;
  }

  async function doReopenMissing(): Promise<void> {
    // First try to restore the SAME file at its original path —
    // covers the false-positive case where the panel surfaced
    // briefly because of an atomic-write race that's since
    // resolved. If the file is genuinely gone, fall through to
    // the FB-navigation flow so the user can pick the moved file
    // manually.
    if (await attemptInPlaceReopen(tab.id)) return;
    const parent = parentPath(tab.path);
    beginMissingFileReopen(tab.id);
    revealPathInBrowser(parent || tab.path, { inspectorOpen: true });
    ui.status = "Choose the moved file in Files to re-open this tab";
  }

  function doReopenAtSuggested(): void {
    const suggested = tab.fileMissing?.suggestedPath;
    if (!suggested) return;
    beginMissingFileReopen(tab.id);
    void openInActivePane(suggested);
  }

  function doFindMissing(): void {
    const fragment = tab.fileMissing?.fragment;
    if (fragment) searchPanel.query = fragment;
    searchPanel.open = true;
  }

  function doOpenGraph(): void {
    closeTabMenu();
    // "Graph from here" from a file's menu scopes the graph to that
    // file (file:<path>), not the whole drive. Hashtags etc. still
    // route through openGraphAtNode at drive scope.
    openGraphForFile(tab.path);
  }

  function doTerminalFromHere(): void {
    closeTabMenu();
    openTerminalInPane(layout.activePaneId, terminalFromHereTarget(tab.path, false));
  }

  function doToggleOutline(): void {
    setTabOutlineOpen(tab, !tab.outlineOpen);
    closeTabMenu();
  }

  function doToggleDetails(): void {
    setTabInspectorOpen(tab, !tab.inspectorOpen);
    closeTabMenu();
  }

  function doToggleStyleToolbar(): void {
    setTabStyleToolbarOpen(tab, !tab.styleToolbarOpen);
    closeTabMenu();
  }

  /// Chord lookup mirrors the empty-pane menu in Pane.svelte: SHORTCUTS
  /// is keyed by command id; render the platform-specific chord and
  /// format it for the current OS. Rows without a registered chord
  /// render an empty cell so the right column stays aligned.
  const menuPlatform = currentPlatform();
  const menuOs = currentOS();
  function chordLabel(id: string | undefined): string {
    if (!id) return "";
    const s = SHORTCUTS.find((x) => x.id === id);
    if (!s) return "";
    const chord = s[menuPlatform];
    if (!chord) return "";
    return formatChord(chord, menuOs);
  }
</script>

<svelte:window onkeydown={onMenuKeydown} onpointerdown={onDocPointerDown} />

<div class="editor-tab">
  {#if recoveredBuffer}
    <!-- `fullstack-a-72`: hang-recovery banner. Surfaces when
         localStorage has unsaved content for this tab that
         diverges from the on-disk content. User picks Restore
         (replace editor content with the buffer) or Discard
         (clear the buffer and keep the disk content). Banner
         clears itself either way. -->
    <div class="recovery-banner" role="alert">
      <span class="recovery-banner-text">
        Unsaved changes from a previous session were found.
      </span>
      <button
        type="button"
        class="recovery-banner-btn recovery-banner-restore"
        onclick={restoreFromBuffer}
      >
        Restore
      </button>
      <button
        type="button"
        class="recovery-banner-btn"
        onclick={discardBuffer}
      >
        Discard
      </button>
    </div>
  {/if}
  {#if menuOpen}
    <!-- Tab menu bubble. Anchored to the tab title in the pane's
         tab strip; rendered here so it has direct access to the
         live Wysiwyg ref + selVer signal that drives the
         formatting buttons' "on" states. -->
    <div
      class="tab-menu-bubble"
      role="menu"
      tabindex="-1"
      aria-label="tab menu"
      use:clampMenu={menuPos}
      onmousedown={(e) => e.stopPropagation()}
    >
      <!-- `fullstack-a-67f`: Editor right-click menu reshape per
           addendum-a's verbatim spec. Header: editable Name
           input. Body: Show Source Code + Collapse Code Blocks
           band, then a View-toggles band (kept against spec
           since dropping the side-panel toggles + cleanup
           utilities would orphan the features — flagged in the
           journal), find/copy band, From-$CWD spawn band.
           Foot: Settings (flipHybrid) + Reopen Closed Tab +
           Close. Reload Window / Open Inspector tail dropped
           per the addendum spec; Cmd+R + pane hamburger still
           cover them. -->
      <div class="action-list">
        <!-- Editable Name input (addendum: "editable like
             Terminal's"). Commits on Enter/blur via
             fileOps.renameInPlace which handles path traversal,
             extension preservation, and link rewriting. -->
        <label class="name-row">
          <span class="name-label">
            <Pencil size={15} strokeWidth={1.75} aria-hidden="true" />
            <span>Name</span>
          </span>
          <input
            class="name-input"
            bind:value={nameDraft}
            spellcheck="false"
            autocomplete="off"
            autocorrect="off"
            autocapitalize="off"
            onkeydown={onTabNameKey}
            onblur={commitTabName}
            aria-label="file path"
          />
        </label>
        <div class="msep" role="separator"></div>
        <!-- Page-width slider: ratio of the current window width.
             100 % is the "no cap" sentinel (drag all the way right).
             Stored as a ratio so window resize and browser zoom both
             keep the cap proportional to the viewport. -->
        <div class="page-width-row">
          <span class="page-width-label">Page width</span>
          <input
            class="page-width-slider"
            type="range"
            min={PAGE_WIDTH_MIN_PCT}
            max={PAGE_WIDTH_MAX_PCT}
            step={PAGE_WIDTH_STEP_PCT}
            value={Math.round(pageWidth.ratio * 100)}
            oninput={onPageWidthSlider}
            onmousedown={(e) => e.stopPropagation()}
            aria-label="page width"
          />
          <span class="page-width-value">{Math.round(pageWidth.ratio * 100)}%</span>
        </div>
        <!-- Show Source Code: addendum spec's primary toggle.
             Hidden for plain text tabs that have no structured
             renderer. -->
        {#if hasRenderedMode}
          {@const inSource = tab.mode === "source"}
          {@const renderedLabel =
            renderedModeForTab === "pretty"
              ? "Show Pretty Tree"
              : renderedModeForTab === "table"
                ? "Show Table"
                : "Show Rendered"}
          <button class="mbtn" onclick={doToggleMode}>
            <span class="mbtn-icon">
              {#if inSource && renderedModeForTab === "pretty"}
                <Braces size={16} strokeWidth={1.75} aria-hidden="true" />
              {:else if inSource && renderedModeForTab === "table"}
                <Table2 size={16} strokeWidth={1.75} aria-hidden="true" />
              {:else if inSource}
                <Pilcrow size={16} strokeWidth={1.75} aria-hidden="true" />
              {:else}
                <Code2 size={16} strokeWidth={1.75} aria-hidden="true" />
              {/if}
            </span>
            <span class="mbtn-label">
              {inSource ? renderedLabel : "Show Source Code"}
            </span>
            <span class="mbtn-chord">{chordLabel("app.editor.toggleMode")}</span>
          </button>
        {/if}
        {#if markdownToolsEnabled}
          <button
            class="mbtn"
            onclick={doToggleCodeBlocks}
            class:on={tab.codeBlocksCollapsed}
          >
            <span class="mbtn-icon">
              <Code2 size={16} strokeWidth={1.75} aria-hidden="true" />
            </span>
            <span class="mbtn-label">
              {tab.codeBlocksCollapsed ? "Expand Code Blocks" : "Collapse Code Blocks"}
            </span>
            <span class="mbtn-chord"></span>
          </button>
        {/if}
        <!-- `fullstack-a-67f`: View toggles + cleanup utilities
             kept against spec. Addendum-a's Editor menu does
             NOT list Outline / Details / Style Toolbar /
             Syntax Highlight / Trailing Whitespace toggles +
             the destructive Remove-TW button. Dropping them
             without a chord alternative would orphan the
             features. Flagged for Alex review in the journal;
             trivial to drop if requested. -->
        <div class="msep" role="separator"></div>
        <button class="mbtn" onclick={doToggleOutline} class:on={tab.outlineOpen}>
          <span class="mbtn-icon">
            {#if tab.outlineOpen}
              <ArrowLeft size={16} strokeWidth={1.75} aria-hidden="true" />
            {:else}
              <ArrowRight size={16} strokeWidth={1.75} aria-hidden="true" />
            {/if}
          </span>
          <span class="mbtn-label">
            {tab.outlineOpen ? "Hide Outline" : "Show Outline"}
          </span>
          <span class="mbtn-chord"></span>
        </button>
        <button class="mbtn" onclick={doToggleDetails} class:on={tab.inspectorOpen}>
          <span class="mbtn-icon">
            {#if tab.inspectorOpen}
              <ArrowRight size={16} strokeWidth={1.75} aria-hidden="true" />
            {:else}
              <ArrowLeft size={16} strokeWidth={1.75} aria-hidden="true" />
            {/if}
          </span>
          <span class="mbtn-label">
            {tab.inspectorOpen ? "Hide Details" : "Show Details"}
          </span>
          <span class="mbtn-chord"></span>
        </button>
        <button
          class="mbtn"
          onclick={doToggleStyleToolbar}
          class:on={tab.styleToolbarOpen}
        >
          <span class="mbtn-icon">
            <Type size={16} strokeWidth={1.75} aria-hidden="true" />
          </span>
          <span class="mbtn-label">
            {tab.styleToolbarOpen ? "Hide Style Toolbar" : "Show Style Toolbar"}
          </span>
          <span class="mbtn-chord"></span>
        </button>
        {#if tab.mode === "source"}
          <button
            class="mbtn"
            onclick={doToggleSyntaxHighlight}
            class:on={tab.syntaxHighlight}
          >
            <span class="mbtn-icon">
              <Highlighter size={16} strokeWidth={1.75} aria-hidden="true" />
            </span>
            <span class="mbtn-label">
              {tab.syntaxHighlight ? "Disable Syntax Highlight" : "Enable Syntax Highlight"}
            </span>
            <span class="mbtn-chord"></span>
          </button>
        {/if}
        <button
          class="mbtn"
          onclick={doToggleTrailingWhitespace}
          class:on={tab.highlightTrailingWhitespace}
        >
          <span class="mbtn-icon">
            <Highlighter size={16} strokeWidth={1.75} aria-hidden="true" />
          </span>
          <span class="mbtn-label">Highlight trailing whitespace</span>
          <span class="mbtn-chord"></span>
        </button>
        <button class="mbtn" onclick={doRemoveTrailingWhitespace}>
          <span class="mbtn-icon">
            <Eraser size={16} strokeWidth={1.75} aria-hidden="true" />
          </span>
          <span class="mbtn-label">Remove trailing whitespace</span>
          <span class="mbtn-chord"></span>
        </button>
        <div class="msep" role="separator"></div>
        <button class="mbtn" onclick={doOpenSearch}>
          <span class="mbtn-icon">
            <SearchIcon size={16} strokeWidth={1.75} aria-hidden="true" />
          </span>
          <span class="mbtn-label">Search</span>
          <span class="mbtn-chord">{chordLabel("app.search.toggle")}</span>
        </button>
        <button class="mbtn" onclick={doFind}>
          <span class="mbtn-icon">
            <SearchIcon size={16} strokeWidth={1.75} aria-hidden="true" />
          </span>
          <span class="mbtn-label">Find</span>
          <span class="mbtn-chord">{chordLabel("app.find.open")}</span>
        </button>
        <button class="mbtn" onclick={doCopyPath}>
          <span class="mbtn-icon">
            <Copy size={16} strokeWidth={1.75} aria-hidden="true" />
          </span>
          <span class="mbtn-label">Copy path to file</span>
          <span class="mbtn-chord"></span>
        </button>
        <button class="mbtn" onclick={doCopyCwdPath}>
          <span class="mbtn-icon">
            <Copy size={16} strokeWidth={1.75} aria-hidden="true" />
          </span>
          <span class="mbtn-label">Copy path to $CWD</span>
          <span class="mbtn-chord"></span>
        </button>
        <button class="mbtn" onclick={doReload}>
          <span class="mbtn-icon">
            <RotateCw size={16} strokeWidth={1.75} aria-hidden="true" />
          </span>
          <span class="mbtn-label">Reload from Disk</span>
          <span class="mbtn-chord"></span>
        </button>
        <div class="msep" role="separator"></div>
        <!-- `fullstack-a-67f`: From-$CWD spawn band per addendum-a.
             Mirror of the `-a-67d` Terminal pattern: New File
             uses the existing dialog; New Terminal / New File
             Browser / New Graph fire chan:command events. -->
        <div class="from-cwd-label">From $CWD</div>
        <button class="mbtn" onclick={doDuplicate}>
          <span class="mbtn-icon">
            <Copy size={16} strokeWidth={1.75} aria-hidden="true" />
          </span>
          <span class="mbtn-label">Duplicate File</span>
          <span class="mbtn-chord"></span>
        </button>
        <button class="mbtn" onclick={doNewFile}>
          <span class="mbtn-icon">
            <FilePlus size={16} strokeWidth={1.75} aria-hidden="true" />
          </span>
          <span class="mbtn-label">New File</span>
          <span class="mbtn-chord">{chordLabel("app.file.new")}</span>
        </button>
        <button class="mbtn" onclick={doNewTerminal}>
          <span class="mbtn-icon">
            <TerminalIcon size={16} strokeWidth={1.75} aria-hidden="true" />
          </span>
          <span class="mbtn-label">New Terminal</span>
          <span class="mbtn-chord">{chordLabel("app.terminal.toggle")}</span>
        </button>
        <button class="mbtn" onclick={doNewFileBrowser}>
          <span class="mbtn-icon">
            <Folder size={16} strokeWidth={1.75} aria-hidden="true" />
          </span>
          <span class="mbtn-label">New File Browser</span>
          <span class="mbtn-chord">{chordLabel("app.files.toggle")}</span>
        </button>
        <button class="mbtn" onclick={doNewGraph}>
          <span class="mbtn-icon">
            <Network size={16} strokeWidth={1.75} aria-hidden="true" />
          </span>
          <span class="mbtn-label">New Graph</span>
          <span class="mbtn-chord">{chordLabel("app.graph.toggle")}</span>
        </button>
        <div class="msep" role="separator"></div>
        <button class="mbtn" onclick={flipToSettings}>
          <span class="mbtn-icon">
            <Settings2 size={16} strokeWidth={1.75} aria-hidden="true" />
          </span>
          <span class="mbtn-label">Settings</span>
          <span class="mbtn-chord"></span>
        </button>
        <div class="msep" role="separator"></div>
        <button
          class="mbtn"
          disabled={!canReopenClosedTab()}
          onclick={doReopenClosedTab}
        >
          <span class="mbtn-icon">
            <History size={16} strokeWidth={1.75} aria-hidden="true" />
          </span>
          <span class="mbtn-label">Reopen Closed Tab</span>
          <span class="mbtn-chord">{chordLabel("app.tab.reopenClosed")}</span>
        </button>
        <button class="mbtn" onclick={doCloseTab}>
          <span class="mbtn-icon">
            <X size={16} strokeWidth={1.75} aria-hidden="true" />
          </span>
          <span class="mbtn-label">Close</span>
          <span class="mbtn-chord">{chordLabel("app.tab.close")}</span>
        </button>
        <!-- `fullstack-a-67f`: Reload Window + Open Inspector
             tail entries dropped per spec consistency with
             `-a-67d`. Cmd+R + the pane hamburger still surface
             window-level reload + devtools. -->
      </div>
    </div>
  {/if}

  <!-- `fullstack-a-67f`: dropped the `-a-35` inline rename band.
       Addendum-a moves the rename surface into the menu top as
       a "Name, editable like Terminal's" input. The new in-menu
       input commits on Enter/blur via `fileOps.renameInPlace`
       (same chan-drive rename + link-rewrite pass as before). -->
  {#if tab.fileMissing}
    <div class="editor-toolbar missing-toolbar">
      <span>File moved or deleted</span>
    </div>
  {:else if tab.error}
    <div class="editor-toolbar">
      <span class="error">{tab.error}</span>
    </div>
  {/if}
  {#key tab.id}
    {#if tab.loading}
      <div class="placeholder">loading…</div>
    {:else if tab.fileMissing}
      <div class="missing-file-state">
        <div class="missing-title">File moved or deleted</div>
        <div class="missing-path">{tab.fileMissing.path}</div>
        {#if tab.fileMissing.suggestedPath}
          <div class="missing-suggest">
            Looks like it moved to
            <code>{tab.fileMissing.suggestedPath}</code>
          </div>
        {/if}
        <div class="missing-actions">
          {#if tab.fileMissing.suggestedPath}
            <button
              type="button"
              class="suggest-reopen"
              onclick={doReopenAtSuggested}
            >
              <Folder size={15} strokeWidth={1.75} aria-hidden="true" />
              <span>Re-open there</span>
            </button>
          {/if}
          <button type="button" onclick={doReopenMissing}>
            <Folder size={15} strokeWidth={1.75} aria-hidden="true" />
            <span>Re-open</span>
          </button>
          <button type="button" onclick={doFindMissing}>
            <SearchIcon size={15} strokeWidth={1.75} aria-hidden="true" />
            <span>Find</span>
          </button>
          <button type="button" onclick={doCloseTab}>
            <X size={15} strokeWidth={1.75} aria-hidden="true" />
            <span>Close</span>
          </button>
        </div>
      </div>
    {:else if tab.error}
      <div class="placeholder error-placeholder">{tab.error}</div>
    {:else}
      <div class="editor-inspector-row">
      {#if tab.outlineOpen}
        <Inspector
          title="Outline"
          side="left"
          bind:width={
            () => tab.outlineWidth ?? paneWidths.outline,
            (v) => (tab.outlineWidth = v)
          }
          onResize={persistPaneWidths}
          onClose={() => setTabOutlineOpen(tab, false)}
        >
          <OutlineBody content={tab.content} {caretLine} onSelect={jumpTo} />
        </Inspector>
      {/if}
      {#if tab.mode === "wysiwyg"}
        <!-- Wysiwyg + floating style toolbar share a positioned
             host so the toolbar can pin to the top-left of the
             editor canvas. Without `position: relative` here the
             absolute toolbar would escape to the next ancestor
             (the pane) and end up over the tab strip. The find
             bar shares the same host so it can pin to the
             top-right of the same canvas. -->
        <div
          class="editor-host"
          style:--editor-top-pad={tab.styleToolbarOpen ? "2.5rem" : "0.5rem"}
          oncontextmenu={onEditorContext}
          role="presentation"
        >
          <Wysiwyg
            bind:this={wysiwygRef}
            bind:value={tab.content}
            readonly={readOnly}
            highlightTrailingWhitespace={tab.highlightTrailingWhitespace}
            initialCaret={tab.caret ?? null}
            onCaretChange={(from, to) => setTabCaret(tab, from, to)}
            onSelectionChange={() => (selVer = selVer + 1)}
            wikiPickerPrefix={tab.repoRoot}
            currentPath={tab.path}
            onWikiClick={(args) => {
              // Navigation: click on a wikilink pill opens the
              // target in the active pane (or a new pane on Cmd /
              // Ctrl click).
              void openInActivePane(args.target);
            }}
            onTagClick={(name) => openGraphForTag(`#${name}`, name)}
            onMentionClick={(args) => {
              // Mention widget resolved the contact via api.contacts
              // and (in read-only contexts) already opened the preview
              // popover. We get here on commit (Cmd/Ctrl+Enter from
              // the popover) or on a writable plain click. Open the
              // resolved contact file; the widget passes a null path
              // when the name didn't match any contact (silent no-op
              // for now — could surface a status hint later).
              if (args.path) void openInActivePane(args.path);
            }}
          />
          {#if tab.styleToolbarOpen}
            <!-- `fullstack-a-26`: parity with the rich-prompt
                 toolbar — separator + rendered/source toggle next
                 to the formatting buttons. `mode` + `onModeToggle`
                 are passed through to the shared StyleToolbar
                 component (which already supports the toggle, gated
                 on these props being defined). The toggle calls
                 `doToggleMode()` which swaps between source and the
                 tab's rendered mode (wysiwyg / pretty / table). -->
            <StyleToolbar
              wysiwyg={wysiwygRef}
              selVer={selVer}
              disabled={readOnly}
              mode="wysiwyg"
              onModeToggle={hasRenderedMode ? () => doToggleMode() : undefined}
            />
          {/if}
          {#if tab.find?.open}
            <FindBar
              find={tab.find}
              adapter={findAdapter}
              docText={tab.content}
              tabId={tab.id}
            />
          {/if}
        </div>
      {:else if tab.mode === "pretty"}
        <!-- Pretty / structured renderer (JSON tree today). The
             buffer stays authoritative; we don't mount FindBar
             here because the renderer is read-only -- edits happen
             in source mode. -->
        <div
          class="editor-host"
          oncontextmenu={onEditorContext}
          role="presentation"
        >
          <JsonPretty value={tab.content} />
        </div>
      {:else if tab.mode === "table"}
        <!-- Tabular renderer (CSV / TSV). Cell commits flow back
             through the bound value prop; the autosave debouncer
             picks them up like any other text edit. -->
        <div
          class="editor-host"
          oncontextmenu={onEditorContext}
          role="presentation"
        >
          <CsvTable
            bind:value={tab.content}
            delimiter={csvDelimiter(tab.path)}
          />
        </div>
      {:else}
        <!-- Source mode gets its own positioned host so FindBar
             can pin to the same top-right spot it occupies in the
             Wysiwyg view. -->
        <div
          class="editor-host"
          oncontextmenu={onEditorContext}
          role="presentation"
        >
          <Source
            bind:this={sourceRef}
            bind:value={tab.content}
            path={tab.path}
            syntaxHighlight={tab.syntaxHighlight}
            highlightTrailingWhitespace={tab.highlightTrailingWhitespace}
            initialCaret={tab.caret ?? null}
            onCaretChange={(from, to) => setTabCaret(tab, from, to)}
          />
          {#if tab.styleToolbarOpen && hasRenderedMode}
            <!-- `fullstack-a-26`: also mount the StyleToolbar in
                 source mode so the rendered/source toggle stays
                 reachable from inside source mode. `disabled` is
                 on (the formatting row collapses) but the toggle
                 sits OUTSIDE the formatting row (per the
                 StyleToolbar's own design comment around its
                 `.fbtn-row`) and stays clickable. Only mount for
                 tabs with a rendered mode (markdown / JSON /
                 CSV) — plain `.py` / `.toml` source has no
                 rendered counterpart, so there's no useful
                 toggle direction. -->
            <StyleToolbar
              wysiwyg={undefined}
              selVer={selVer}
              disabled={true}
              mode="source"
              onModeToggle={() => doToggleMode()}
            />
          {/if}
          {#if tab.find?.open}
            <FindBar
              find={tab.find}
              adapter={findAdapter}
              docText={tab.content}
              tabId={tab.id}
            />
          {/if}
        </div>
      {/if}
      {#if tab.inspectorOpen}
        <Inspector
          title="Details"
          bind:width={
            () => tab.inspectorWidth ?? paneWidths.inspector,
            (v) => (tab.inspectorWidth = v)
          }
          onResize={persistPaneWidths}
          onClose={() => setTabInspectorOpen(tab, false)}
        >
          <FileInfoBody
            path={tab.path}
            showRefs
            onNavigate={(p) => void openInActivePane(p)}
            onReveal={revealInBrowser}
            onSetAsScope={() => openFsGraphForFile(tab.path)}
          />
        </Inspector>
      {/if}
      </div>
      {#if tab.mode === "wysiwyg"}
        <WikiStatusBar
          path={tab.path}
          content={tab.content}
          fsWritable={tab.fsWritable}
          bind:readMode={tab.readMode}
        />
      {/if}
    {/if}
  {/key}
</div>


<style>
  .editor-tab {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
    min-width: 0;
    background: var(--bg);
    color: var(--text);
  }
  /* `fullstack-a-72` hang-recovery banner. Sits at the very top
     of the editor-tab body, above the menu bubble + the editor
     host. Uses the existing `--warn-text` palette so it reads as
     an attention-needed affordance without competing with the
     editor's content area below. Stays compact (single row) so
     it doesn't push the document down dramatically. */
  .recovery-banner {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.5rem 1rem;
    background: color-mix(in srgb, var(--warn-text) 12%, var(--bg));
    border-bottom: 1px solid var(--border);
    color: var(--text);
    font-size: 0.875rem;
  }
  .recovery-banner-text {
    flex: 1;
    min-width: 0;
  }
  .recovery-banner-btn {
    padding: 0.25rem 0.75rem;
    background: var(--btn-bg);
    border: 1px solid var(--btn-border);
    border-radius: 4px;
    color: var(--text);
    font-size: 0.875rem;
    cursor: pointer;
  }
  .recovery-banner-btn:hover {
    border-color: var(--btn-hover);
  }
  .recovery-banner-restore {
    background: var(--warn-text);
    color: var(--bg);
    border-color: var(--warn-text);
  }
  .recovery-banner-restore:hover {
    opacity: 0.9;
    border-color: var(--warn-text);
  }
  /* Tab menu bubble. Fixed-position so it anchors to the trigger
     button regardless of which pane the user clicked in; the
     translateX clamp keeps it on-screen for tabs that sit near the
     right edge. Width matches the longest action label plus the
     icon column so the buttons read as a tidy list.

     Bouncy reveal: the bubble enters via a `bubble-pop` keyframe
     using the same easeOutBack curve as the BottomPill (small
     overshoot on the way in so the menu reads as alive rather than
     mechanical). Hover gives a tiny scale-up for the same reason. */
  .tab-menu-bubble {
    position: fixed;
    z-index: 50;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: 8px;
    box-shadow: 0 6px 20px rgba(0, 0, 0, 0.18);
    padding: 6px;
    min-width: 220px;
    max-width: calc(100vw - 16px);
    /* Pull back if the anchor pushes us past the viewport. */
    max-height: calc(100vh - 24px);
    overflow-y: auto;
    color: var(--text);
    font-size: 13px;
    /* Anchor the bouncy reveal to the top-left of the bubble so the
       overshoot grows away from the trigger button rather than from
       the center of the popover (which would visually drift it
       sideways during the bounce). */
    transform-origin: top left;
    animation: bubble-pop 260ms cubic-bezier(0.34, 1.56, 0.64, 1);
    transition: transform 200ms cubic-bezier(0.34, 1.56, 0.64, 1);
  }
  .tab-menu-bubble:hover {
    transform: scale(1.015);
  }
  @keyframes bubble-pop {
    0% {
      opacity: 0;
      transform: scale(0.92);
    }
    100% {
      opacity: 1;
      transform: scale(1);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .tab-menu-bubble {
      animation: none;
      transition: none;
    }
    .tab-menu-bubble:hover {
      transform: none;
    }
  }
  .page-width-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 4px;
    border-bottom: 1px solid var(--separator);
  }
  .page-width-label {
    color: var(--text-secondary);
    font-size: 12px;
    min-width: 64px;
  }
  .page-width-slider {
    flex: 1;
    accent-color: var(--btn-hover);
  }
  .page-width-value {
    min-width: 40px;
    text-align: right;
    color: var(--text-secondary);
    font-size: 12px;
    font-variant-numeric: tabular-nums;
  }

  .action-list {
    display: flex;
    flex-direction: column;
    padding-top: 4px;
  }
  .mbtn {
    display: flex;
    align-items: center;
    gap: 8px;
    background: none;
    border: 0;
    border-radius: 4px;
    cursor: pointer;
    color: var(--text);
    font: inherit;
    font-size: 13px;
    padding: 6px 8px;
    text-align: left;
  }
  .mbtn:hover { background: var(--hover-bg); }
  .mbtn.on { color: var(--text); background: var(--hover-bg); }
  .mbtn-icon {
    width: 18px;
    text-align: center;
    color: var(--text);
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }
  .mbtn-label { flex: 1; }
  /* Chord column on the right edge. Matches the empty-pane menu's
     `.empty-pane-menu-chord` so the file-tab bubble and the
     empty-pane right-click menu read as one family. Empty cells
     still occupy the slot so the column stays aligned even on
     rows that don't have a registered shortcut. */
  .mbtn-chord {
    margin-left: 1.5rem;
    color: var(--text-secondary);
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 11.5px;
  }
  /* Group separator inside the action list. Same shape as the
     hamburger menu's `li.sep` so the overlay menus and the file
     tab menu read alike. */
  .msep {
    height: 1px;
    background: var(--separator, var(--border));
    margin: 4px 2px;
  }

  /* One-off error surfacing for the active tab. Save is implicit
     via Cmd/Ctrl+S handled at the pane level. */
  .editor-toolbar {
    padding: 0.25rem 0.5rem;
    background: var(--bg-card);
    border-bottom: 1px solid var(--border);
    font-size: 14px;
    color: #d33;
  }
  .missing-toolbar {
    color: var(--text-primary);
    font-weight: 600;
  }
  /* `fullstack-a-67f`: the `-a-35` `.rename-band` styles
     dropped along with the full-width inline rename band.
     The menu-top `.name-row` input replaces it (styles
     below). */
  .name-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 0 6px;
  }
  .name-label {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--text-secondary);
    font-size: 12px;
    text-transform: lowercase;
    letter-spacing: 0.02em;
    flex-shrink: 0;
  }
  .name-input {
    flex: 1;
    min-width: 0;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text);
    font: inherit;
    font-size: 13px;
    padding: 3px 6px;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  }
  .name-input:focus {
    outline: none;
    border-color: var(--accent);
  }
  /* `fullstack-a-67f`: "From $CWD" section label. Mirrors
     TerminalTab's `.from-cwd-label` styling. */
  .from-cwd-label {
    padding: 4px 8px 2px;
    color: var(--text-secondary);
    font-size: 11px;
    text-transform: lowercase;
    letter-spacing: 0.02em;
  }
  .placeholder {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-secondary);
    font-style: italic;
  }
  .error-placeholder {
    color: var(--danger-text, #d33);
    padding: 1rem;
    text-align: center;
  }
  .missing-file-state {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 0.75rem;
    padding: 1.25rem;
    text-align: center;
    color: var(--text-secondary);
  }
  .missing-title {
    color: var(--text-primary);
    font-size: 1rem;
    font-weight: 650;
  }
  .missing-path {
    max-width: min(42rem, 90%);
    overflow-wrap: anywhere;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 12px;
  }
  .missing-suggest {
    max-width: min(42rem, 90%);
    overflow-wrap: anywhere;
    font-size: 13px;
    color: var(--text-secondary);
  }
  .missing-suggest code {
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 12px;
    color: var(--text-primary);
  }
  .missing-actions {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: 0.5rem;
  }
  .missing-actions button {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    height: 2rem;
    padding: 0 0.7rem;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg-card);
    color: var(--text-primary);
    font: inherit;
    cursor: pointer;
  }
  .missing-actions button:hover {
    background: var(--bg-hover, var(--bg-card));
  }
  .missing-actions button.suggest-reopen {
    border-color: var(--link);
    background: var(--link);
    color: var(--bg-card);
  }
  .missing-actions button.suggest-reopen:hover {
    filter: brightness(1.08);
  }
  /* Row that holds the editor + (optional) inspector. The Inspector
     component renders a ResizeHandle as its previous sibling so
     this row sees handle + inspector as siblings at the same level. */
  .editor-inspector-row {
    flex: 1;
    display: flex;
    min-height: 0;
    min-width: 0;
  }
  /* Wraps the WYSIWYG editor and its floating style toolbar so the
     toolbar can pin to the top-left of the editor canvas. position:
     relative establishes the toolbar's containing block; flex:1 +
     min-height:0 lets the editor inside take its full slot in the
     surrounding flex row. */
  .editor-host {
    position: relative;
    flex: 1;
    display: flex;
    min-height: 0;
    min-width: 0;
  }
  /* `--editor-top-pad` is read by .md-wysiwyg (Wysiwyg.svelte) to
     set its padding-top. We bump it to 1.5rem while the style
     toolbar is enabled in the tab menu so the first line clears
     the floating toolbar pill (top: 8px, ~30px tall); when the
     toolbar is hidden we reclaim that space back to the 1rem
     baseline so the first line sits at the top of the doc. */
</style>
