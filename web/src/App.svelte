<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import AppStatusBar from "./components/AppStatusBar.svelte";
  import ConfirmModal from "./components/ConfirmModal.svelte";
  import ConflictModal from "./components/ConflictModal.svelte";
  import DisconnectOverlay from "./components/DisconnectOverlay.svelte";
  import DraftCloseModal from "./components/DraftCloseModal.svelte";
  import DriveWarningsModal from "./components/DriveWarningsModal.svelte";
  import SpawnDialog from "./components/SpawnDialog.svelte";
  import TeamDialog from "./components/TeamDialog.svelte";
  import { teamDialogState } from "./state/teamDialog.svelte";
  import FileBrowserSidePane from "./components/FileBrowserSidePane.svelte";
  import MissingTokenOverlay from "./components/MissingTokenOverlay.svelte";
  import PathPromptModal from "./components/PathPromptModal.svelte";
  import PaneModeHelp from "./components/PaneModeHelp.svelte";
  import PromptModal from "./components/PromptModal.svelte";
  import SearchPanel from "./components/SearchPanel.svelte";
  import SearchStatusOverlay from "./components/SearchStatusOverlay.svelte";
  import SettingsPanel from "./components/SettingsPanel.svelte";
  import Workspace from "./components/Workspace.svelte";
  import {
    applyInitialTheme,
    bootstrap,
    installSessionFlushHook,
    browserOverlay,
    browserSelection,
    browserSidePanes,
    closeOverlay,
    drive,
    fileOps,
    graphOverlay,
    openGraph,
    openGraphWithContext,
    openSettings,
    pathPromptState,
    noteDraftCreated,
    persistLayoutToHash,
    promptState,
    reconnectWatcher,
    refreshDrive,
    refreshTree,
    resolveSpawnContext,
    revealAndSelect,
    scheduleSessionSave,
    searchStatusOverlay,
    searchPanel,
    setTransientStatus,
    settingsOverlay,
    syncOverlayStack,
    toggleBrowserSidePane,
    topOverlay,
    watchSystemTheme,
  } from "./state/store.svelte";
  import { confirmState } from "./state/confirm.svelte";
  import {
    activeFileTab,
    activePane,
    closeFind,
    closePane,
    closeTab,
    closeTabsInPane,
    cancelPaneMode,
    commitPaneMode,
    draftCloseState,
    enterPaneMode,
    isWindowFullyReadOnly,
    layout,
    openActiveTerminalRichPrompt,
    openBrowserInActivePane,
    openFind,
    openInActivePane,
    openInfographicsInActivePane,
    openInPane,
    scheduleAutosave,
    flipHybrid,
    selectNextPane,
    selectNextTabInActivePane,
    selectPrevPane,
    selectPrevTabInActivePane,
    selectTabAtIndexInActivePane,
    setActivePane,
    openTerminalInActivePane,
    paneMode,
    paneModeEqualize,
    paneModeMoveFocus,
    paneModeOpenBrowser,
    paneModeOpenGraph,
    paneModeOpenRichPromptTerminal,
    paneModeOpenTerminal,
    paneModeResize,
    paneModeSplit,
    paneModeStageDraftEditor,
    paneModeSwap,
    showOrSpawnRichPromptInFocusedPane,
    toggleActiveFileTabMode,
  } from "./state/tabs.svelte";
  import { applyEditorTheme, DEFAULT_EDITOR_THEME } from "./state/editorTheme";
  import { flushPendingBufferWrites, pruneEditorBuffers } from "./state/editorBuffer";
  import { reloadWindow } from "./api/desktop";
  import { api } from "./api/client";
  import {
    applyInitialPageWidth,
    watchPageWidth,
  } from "./state/pageWidth.svelte";
  import { installIdleTracker, setReadMode } from "./state/idle.svelte";
  import {
    installScreensaverTracker,
    loadScreensaverState,
    lockNow,
  } from "./state/screensaver.svelte";
  import ScreensaverOverlay from "./components/ScreensaverOverlay.svelte";

  // Keep the URL hash in sync with the current layout so reload (and
  // copy-paste of the URL) restores the same panes/tabs. We touch
  // every reactive bit of `layout` in the effect so Svelte tracks
  // mutations to nested arrays/strings: JSON.stringify in
  // serializeLayout() already does that, but the function is called
  // synchronously, and Svelte only tracks reads that happen *during*
  // the effect's run.
  let bootstrapped = $state(false);
  // `fullstack-42`: `h` inside Pane Mode toggles a cheatsheet
  // overlay that lists every Cmd+K binding. The flag stays inside
  // App.svelte because Pane Mode itself is global (one transaction
  // per Cmd+K press) — no per-pane scoping needed.
  let paneModeHelpVisible = $state(false);
  // `fullstack-a-3`: the centre-window "H for help" flash that
  // landed in `fullstack-61` is gone. The status-bar Hybrid
  // label already telegraphs `H help`, and the PaneModeHelp
  // cheatsheet covers discovery — the mid-screen flash was
  // visual noise on every Cmd+K entry.
  $effect(() => {
    // Touch enough of the layout to trip reactivity on common
    // mutations (URL persistence) AND watch every file tab's content
    // for autosave.
    void layout.rootId;
    void layout.activePaneId;
    for (const node of Object.values(layout.nodes)) {
      if (node.kind !== "leaf") continue;
      void node.activeTabId;
      void node.tabs.length;
      for (const t of node.tabs) {
        if (t.kind === "terminal") {
          void t.title;
          void t.broadcastEnabled;
          void t.broadcastTargetIds.length;
          continue;
        }
        if (t.kind !== "file") {
          if (t.kind === "graph") {
            void t.mode;
            void t.scopeId;
            void t.depth;
            void t.inspectorOpen;
            void t.pendingSelectId;
            void t.filters.link;
            void t.filters.tag;
            void t.filters.mention;
            void t.filters.language;
            void t.filters.img;
            void t.filters.folder;
          } else if (t.kind === "browser") {
            void t.inspectorOpen;
          } else {
            // `fullstack-a-75`: infographics tab carries no
            // reactivity-relevant state today (title is
            // immutable + id is stable). Touch the id so the
            // effect still observes tab-list mutations cleanly.
            void t.id;
          }
          continue;
        }
        void t.path;
        void t.mode;
        // Reading t.content here makes the effect rerun on every
        // keystroke, which then debounces the actual save.
        void t.content;
        // Track caret so selection moves bump the URL hash too;
        // without this, the persisted layout never picks up the
        // updated offset and reloads land at doc start.
        void t.caret;
        if (bootstrapped && !t.loading && t.content !== t.saved) {
          scheduleAutosave(node.id, t.id);
        }
      }
    }
    if (bootstrapped) {
      persistLayoutToHash();
      // Same payload, mirrored to the per-window session.json so
      // the layout restores on next launch. Heavier debounce than
      // the URL hash since it touches disk.
      scheduleSessionSave();
    }
  });

  // Mirror overlay open-state + per-overlay knobs into the URL hash
  // (for copy-paste portability) AND the session payload (for
  // close-and-quit restore). The hash captures every visible
  // surface so another browser opening the same URL lands on the
  // identical screen; the session also stores defaults for next
  // launch. Both helpers debounce internally.
  $effect(() => {
    if (!bootstrapped) return;
    void settingsOverlay.open;
    void searchPanel.open;
    void searchPanel.query;
    // search_scope= sibling key in the hash captures this; bumping
    // it here ensures the persistence effect reruns when the user
    // narrows the search to a directory / file / repo.
    void searchPanel.scopeId;
    void searchPanel.inspectorOpen;
    void browserOverlay.open;
    void browserOverlay.inspectorOpen;
    void browserSidePanes.left;
    void browserSidePanes.right;
    void browserSelection.path;
    void graphOverlay.open;
    void graphOverlay.scopeId;
    void graphOverlay.mode;
    void graphOverlay.depth;
    void graphOverlay.filters.link;
    void graphOverlay.filters.tag;
    void graphOverlay.filters.mention;
    void graphOverlay.filters.img;
    // `folder` lives alongside the other graph filter slots; the
    // overlay shows the chip only in filesystem mode, but the URL
    // hash round-trips its state regardless so closing/reopening
    // the overlay restores the user's choice.
    void graphOverlay.filters.folder;
    void graphOverlay.inspectorOpen;
    persistLayoutToHash();
    scheduleSessionSave();
  });

  // Push the active editor theme onto the document root whenever
  // the server-known drive info changes. The CSS in editor/themes/*
  // keys typography + chrome off this attribute.
  $effect(() => {
    const theme = drive.info?.preferences?.editor_theme;
    applyEditorTheme(theme ?? DEFAULT_EDITOR_THEME);
  });

  // Single-writer bridge from per-tab read mode to the window-level
  // readMode flag (which drives the bottom pill's grey state and
  // the idle tracker's read-mode timing window). Computing this in
  // App.svelte means there's exactly one effect mutating the
  // signal regardless of how many panes are open, which avoids the
  // multi-pane fight that the previous per-FileEditorTab effect
  // produced (cleanup-then-set on every toggle, with sibling panes
  // racing to overwrite each other's value).
  $effect(() => {
    setReadMode(isWindowFullyReadOnly());
  });

  // Window-level overlay stack. Each overlay's `.open` flag is the
  // single source of truth for "is it on screen"; this effect mirrors
  // the set of currently-open overlays into `overlayStack.ids` so the
  // most-recently-opened paints on top and Escape can pop one overlay
  // at a time (handler below). Touching each flag explicitly is what
  // ties the effect to their reactivity; the helper doesn't read them
  // back through reactive paths.
  $effect(() => {
    void browserOverlay.open;
    void searchPanel.open;
    void graphOverlay.open;
    void settingsOverlay.open;
    void searchStatusOverlay.open;
    syncOverlayStack();
  });

  onMount(async () => {
    // Apply persisted theme + default editor theme to the document
    // root immediately, before any component renders, to avoid a flash.
    applyInitialTheme();
    applyEditorTheme(DEFAULT_EDITOR_THEME);
    applyInitialPageWidth();
    // While in "system" mode, follow OS-level theme changes live.
    // The listener stays alive for the whole app's lifetime.
    watchSystemTheme();
    // Cross-window sync of the page-width setting via the storage event.
    watchPageWidth();
    // Idle tracker: after 2.5s without scroll/click/keypress, the
    // floating pills fade. Any input flips them back on.
    installIdleTracker();
    // `fullstack-a-77` slice 2: screensaver inactivity
    // tracker. Different cadence (default 5 min, per-drive
    // configurable) + wider event set (keydown + scroll +
    // pointer move; opposite of `installIdleTracker`'s
    // short-window trigger set). Tracker installs the
    // listeners regardless of `enabled` state so a later
    // /api/screensaver/state load doesn't need a re-install
    // pass; the lock fires only when `enabled=true`.
    installScreensaverTracker();
    // Hook pagehide BEFORE bootstrap so a fast reload during the
    // initial load still flushes any in-flight session changes.
    installSessionFlushHook();
    await bootstrap();
    // `fullstack-a-88`: replaced first-boot "spawn FB tab when
    // layout empty" with "boot with docked FB on left by
    // default." The default lives in chan-server's
    // `BrowserSidePanes::default()` so a brand-new
    // preferences.toml ships with `left: true`. SPA respects
    // any user toggle (the load path reads server preferences
    // before this point). Empty pane stays empty; the carousel
    // + shortcut hints carry the empty-state UX.
    bootstrapped = true;
    // `fullstack-a-77` slice 2: fire-and-forget load of the
    // per-drive screensaver state. Tracker is already
    // installed above; the load populates the singleton with
    // the server-side enabled/timeout/pin_set view. Failure
    // is non-fatal (the singleton stays in its default
    // disarmed state).
    void loadScreensaverState();
    // Visibility-change resume hook. Browsers throttle / suspend
    // backgrounded tabs and the WebSocket reconnect can stretch
    // to seconds before the user returns; a manual nudge here
    // lands the connection immediately. Debounced 300 ms so a
    // quick tab-switch flicker doesn't fire the reconnect twice.
    let resumeTimer: ReturnType<typeof setTimeout> | null = null;
    function onVisibility(): void {
      if (document.visibilityState !== "visible") return;
      if (resumeTimer) clearTimeout(resumeTimer);
      resumeTimer = setTimeout(() => {
        resumeTimer = null;
        reconnectWatcher();
        void refreshTree();
        void refreshDrive();
      }, 300);
    }
    document.addEventListener("visibilitychange", onVisibility);
  });

  /// `fullstack-a-32`: context-aware spawn helpers shared by every
  /// chord entry path (top-level chords on `onWindowKey`, Hybrid
  /// NAV cases in `handlePaneModeKey`, and `chan:command` events
  /// fired from chan-desktop's KEY_BRIDGE_JS). Each helper resolves
  /// the focused surface's context (parent dir of a focused doc;
  /// cwd of a focused terminal; scope path of a focused graph)
  /// through `resolveSpawnContext` and threads it into the matching
  /// spawn API. Single source of truth means the four surfaces
  /// (chord / Hybrid Nav / hamburger menu / right-click) all
  /// behave identically.
  function spawnTerminalFromContext(): void {
    const ctx = resolveSpawnContext();
    openTerminalInActivePane({ cwd: ctx.dir });
    scheduleSessionSave();
  }
  function spawnBrowserFromContext(): void {
    const ctx = resolveSpawnContext();
    const select = ctx.file ?? ctx.dir ?? null;
    // Prime the expanded-dirs map + browserSelection so the new
    // tab's tree opens with the context path visible.
    if (select) revealAndSelect(select);
    // `fullstack-a-39`: always spawn a new FB tab. Bypass
    // `openBrowser()`'s `focusExistingBrowserTab` fall-through so
    // the chord stays consistent with the other spawn chords
    // (Cmd+T new terminal every press; Cmd+Shift+M new graph every
    // press). The `select` arg threads the context path into the
    // tab's `selected` field directly so `restoreFromTab`'s mount
    // wipe doesn't clobber the prime.
    openBrowserInActivePane({ select });
    scheduleSessionSave();
  }
  function spawnRichPromptFromContext(): void {
    const ctx = resolveSpawnContext();
    showOrSpawnRichPromptInFocusedPane({ cwd: ctx.dir });
    scheduleSessionSave();
  }
  function spawnGraphFromContext(): void {
    const ctx = resolveSpawnContext();
    openGraphWithContext(ctx);
  }

  /// App-level keyboard shortcuts. Layout follows VS Code where
  /// possible so users carry intuition in from any code editor.
  ///
  /// `fullstack-a-32` spawn-chord family (each context-aware via
  /// `resolveSpawnContext`):
  ///
  ///   Cmd+T          -> Terminal (native; Cmd+Alt+T on web Mac)
  ///   Cmd+O          -> File Browser (native; Cmd+Alt+O on web Mac)
  ///   Cmd+P          -> Rich Prompt (native; Cmd+Alt+P on web Mac)
  ///   Cmd+Shift+M    -> Graph (native + web)
  ///   Mod+. t/o/p/v  -> universal aliases via Hybrid Nav
  ///
  /// Other app chords:
  ///
  ///   Cmd/Ctrl+,             -> Settings (open)
  ///   Alt+Shift+[ / ]        -> previous / next tab       (web fallback)
  ///   Ctrl+Alt+1..9          -> jump to tab N             (web fallback)
  ///
  /// Native (chan-desktop) layers VS Code's browser-reserved chords
  /// on top via its init script: Cmd+T / Cmd+` (terminal), Cmd+W (close tab), Cmd+N (new),
  /// Cmd+Shift+[/] (tab nav), Cmd+1..9 (jump), Cmd+F/G/Shift+G
  /// (find / next / prev — find-on-page lives in chan but no chord
  /// is bound in the browser; users have the browser's native find).
  ///
  /// Mac note: bare-Alt chords are off-limits for letters / digits
  /// because Option is a dead-key for special characters there
  /// (Alt+G prints `©`, Alt+L prints `¬`, Alt+1 prints `¡`, etc.).
  /// All letter / digit chords therefore use Cmd/Ctrl-based combos
  /// or Ctrl+Alt; Alt+Shift+[/] is kept only because we match by
  /// `e.code` (which is layout-independent) and preventDefault
  /// suppresses the typed `«` / `»` before they reach the editor.
  ///
  /// Browser-reserved chord notes:
  ///   - Cmd+P (browser print) -> preventDefault wins in Chrome /
  ///     Safari / Firefox; tolerable cost for matching VS Code.
  ///   - Cmd+W / Cmd+N / Cmd+Shift+[/] / Cmd+1..9 are OS-level
  ///     reserved in browsers — preventDefault doesn't win. Hence
  ///     the Alt+Shift / Ctrl+Alt fallbacks above; native binds
  ///     the VS Code-shaped chords directly.
  function onWindowKey(e: KeyboardEvent): void {
    const meta = e.metaKey || e.ctrlKey;
    if (paneMode.active) {
      e.preventDefault();
      e.stopPropagation();
      handlePaneModeKey(e);
      return;
    }
    // `fullstack-a-7`: swap the Hybrid Nav entry chord from
    // Cmd+K to Cmd+. so Cmd+, can own Settings (macOS
    // app-preferences convention; already wired via
    // `app.settings.toggle` in `shortcuts.ts`). Cmd+. is not
    // browser-reserved on macOS (Safari + Chrome both let JS
    // intercept it), so the same chord works on the web SPA
    // and the desktop shell. Cmd+K no longer triggers Hybrid.
    if (meta && !e.shiftKey && !e.altKey && e.code === "Period") {
      e.preventDefault();
      enterPaneMode();
      return;
    }
    // Escape: pop just the topmost overlay so a stack of open
    // surfaces unwinds one at a time. Previously each OverlayShell
    // owned its own window keydown listener and they all fired in
    // parallel, closing every open overlay on a single press.
    if (e.key === "Escape" && !meta && !e.altKey && !e.shiftKey) {
      const top = topOverlay();
      if (top) {
        e.preventDefault();
        closeOverlay(top);
      return;
    }
  }

  function handlePaneModeKey(e: KeyboardEvent): void {
    const large = e.shiftKey ? 0.1 : 0.02;
    switch (e.key) {
      case "Enter": {
        // `fullstack-72`: prime the module-level browserSelection
        // before commit when a browser spawn is staged so the new
        // tab's tree lands already expanded to + selecting the
        // contextual node. Peek the intent before commitPaneMode
        // clears it. revealAndSelect is a no-op for empty paths.
        const intent = paneMode.spawnIntent;
        if (intent && intent.kind === "browser") {
          if (intent.ctx.file) revealAndSelect(intent.ctx.file);
          else if (intent.ctx.dir) revealAndSelect(intent.ctx.dir);
        }
        // `fullstack-a-68 slice 2`: materialize any staged "new
        // draft editor" intents BEFORE commitPaneMode promotes
        // the draft to live. Each staged entry pins the target
        // paneId at press time; createDraft is async, so we kick
        // off the round-trip in parallel and let each one open
        // the resulting file in its pinned pane. Commit doesn't
        // wait — the draft layout already reflects T / O / P /
        // G additions, and the new-draft files will land in
        // their panes when the round-trips resolve.
        materializeStagedDraftEditors();
        commitPaneMode();
        scheduleSessionSave();
        paneModeHelpVisible = false;
        return;
      }
      case "Escape":
        // `fullstack-a-68 slice 2`: discard staged drafts. The
        // T / O / P / G additions live inside the draft layout
        // and disappear automatically when commitPaneMode does
        // not run. The Esc path bails before
        // materializeStagedDraftEditors fires, so no orphan
        // drafts get created.
        cancelPaneMode();
        paneModeHelpVisible = false;
        return;
      // @@Alex's mental model: arrows navigate (move focus),
      // WASD moves stuff (swap tiles). `fullstack-40` swapped
      // these from the `fullstack-16` defaults.
      case "ArrowUp":
        paneModeMoveFocus("up");
        return;
      case "ArrowLeft":
        paneModeMoveFocus("left");
        return;
      case "ArrowDown":
        paneModeMoveFocus("down");
        return;
      case "ArrowRight":
        paneModeMoveFocus("right");
        return;
      case "w":
      case "W":
        paneModeSwap("up");
        return;
      case "a":
      case "A":
        paneModeSwap("left");
        return;
      // `fullstack-74`: `s` / `S` rejoin the WASD swap-tile group
      // (previously case-sensitive: lowercase opened Search,
      // uppercase swapped). Search moved to `f` / `F` below.
      case "s":
      case "S":
        paneModeSwap("down");
        return;
      case "d":
      case "D":
        paneModeSwap("right");
        return;
      case "[":
        paneModeResize("row", false, large);
        return;
      case "]":
        paneModeResize("row", true, large);
        return;
      case "-":
        paneModeResize("column", false, large);
        return;
      case "=":
        paneModeResize("column", true, large);
        return;
      case "0":
        paneModeEqualize();
        return;
      // `fullstack-a-68 slice 2`: Hybrid Nav T / O / P / G / E
      // chords STAGE additions into the draft layout instead of
      // committing immediately. Multiple presses stack — three
      // T's queue three terminals on the focused pane. Enter
      // materializes the draft; Esc discards. Per addendum-a's
      // "back to transactional mode" framing. Pre-`-a-68 slice
      // 2` behavior (`fullstack-a-32` + `fullstack-50`):
      // immediate commit on T / O / V / P. `v` stays aliased to
      // `g` so muscle memory survives the rename.
      case "t":
      case "T":
        paneModeOpenTerminal(resolveSpawnContext());
        return;
      case "o":
      case "O": {
        const ctx = resolveSpawnContext();
        paneModeOpenBrowser(ctx);
        if (ctx.file) revealAndSelect(ctx.file);
        else if (ctx.dir) revealAndSelect(ctx.dir);
        return;
      }
      case "g":
      case "G":
      case "v":
      case "V":
        paneModeOpenGraph(resolveSpawnContext());
        return;
      // Search lives in an OverlayShell, not a tab type. Open the
      // overlay outside the transaction so it can capture keyboard
      // input cleanly; commit the draft first so any layout edits
      // the user already made don't get dropped. `fullstack-74`:
      // moved from `s` to `f` so WASD (any case) can fully own
      // the swap-tile group; `s` / `S` now both swap-down.
      case "f":
      case "F":
        commitPaneMode();
        scheduleSessionSave();
        searchPanel.open = true;
        return;
      // `h` toggles the Cmd+K help cheatsheet. It does NOT commit
      // the draft — the user is still shaping their layout; the
      // overlay just describes the available keys.
      case "h":
      case "H":
        paneModeHelpVisible = !paneModeHelpVisible;
        return;
      // `fullstack-a-68 slice 2`: `P` now stages a fresh smart-
      // prompt terminal (a terminal tab with the rich-prompt
      // overlay armed open) instead of toggling the overlay on
      // the focused pane's existing terminal. Phase 9 applies
      // the same fresh-terminal rule to top-level Cmd+P.
      case "p":
      case "P":
        paneModeOpenRichPromptTerminal(resolveSpawnContext());
        return;
      // `fullstack-a-68 slice 2`: `N` stages a new draft editor —
      // mirrors the top-level Cmd+N "new draft" chord (the
      // mnemonic the user already has in muscle memory).
      // Drafts are server-side (`api.createDraft()` mints the
      // file), so the intent queues onto
      // `paneMode.stagedDraftEditors` pinned to the pane that
      // was focused at press time. `materializeStagedDraftEditors`
      // resolves the queue on Enter commit; Esc bails the queue
      // before the round-trips fire so no orphan drafts get
      // created.
      case "n":
      case "N":
        paneModeStageDraftEditor();
        return;
      // `fullstack-69`: Cmd+K < and > toggle the docked file
      // browsers. Mapping per @@Alex's verbatim spec — the arrow
      // direction is opposite to the dock side it controls:
      //   `<` (less-than) → right dock toggle
      //   `>` (greater-than) → left dock toggle
      // Same exit semantics as the spawn keys (commit then act).
      case "<":
        commitPaneMode();
        scheduleSessionSave();
        toggleBrowserSidePane("right");
        return;
      case ">":
        commitPaneMode();
        scheduleSessionSave();
        toggleBrowserSidePane("left");
        return;
      // `fullstack-48`: Tab flips the focused Hybrid. Stays inside
      // the pane-mode transaction so Esc can roll the flip back if
      // the user changes their mind. The flipHybrid action targets
      // whichever side is currently visible on the focused pane;
      // calling it twice toggles back to where the user started.
      case "Tab":
        flipHybrid(paneMode.draft?.activePaneId ?? layout.activePaneId);
        return;
      // Split keybinds reuse the right/down constraint from
      // `fullstack-21`'s hamburger menu. New pane lands as the focus
      // so subsequent edits inside the same transaction target it.
      case "/":
        paneModeSplit("row");
        return;
      case "\\":
        paneModeSplit("column");
        return;
      // Close-all / kill-pane reuse the existing affordances and
      // their terminal-confirmation modal. Commit the draft first so
      // the confirmation runs against the layout the user just
      // shaped; the modal needs the normal app keyboard context.
      // `fullstack-77`: kill-pane moved from `k` / `K` to
      // `Backspace` — backspace = delete is the intuitive shape
      // for "delete this pane" and frees `k` for a future binding.
      case "x":
      case "X":
        commitPaneMode();
        closeTabsInActivePane();
        return;
      case "Backspace":
        commitPaneMode();
        killActivePane();
        return;
    }
  }
    // `fullstack-42` pruned every standalone shortcut now covered by
    // Pane Mode (`Cmd+K`): Cmd+P (Files), Cmd+Shift+F (Search),
    // Cmd+Shift+M (Graph), Cmd+Alt+T (Terminal), Cmd+Alt+[ / ]
    // (Prev/Next pane), and Ctrl+Alt+N (New file). Each is reachable
    // via Pane Mode now — the keymap stops shipping two chords for
    // the same action. The native shell's `KEY_BRIDGE_JS` was
    // updated in lockstep.
    if (meta && !e.shiftKey && !e.altKey && e.key === ",") {
      e.preventDefault();
      openSettings();
      return;
    }
    // `fullstack-a-90`: removed the legacy `Alt+Space` rich-prompt
    // chord. Rich prompt is now Cmd+P (native) + Cmd+Alt+P (web Mac
    // fallback) + `Mod+. p` (Hybrid Nav). `-a-32`'s muscle-memory
    // bridge expired.
    // `fullstack-a-32`: spawn-chord family. Each Cmd+Alt+<letter>
    // chord is the macOS web fallback for the matching native
    // Cmd+<letter> chord that browsers reserve at the OS level
    // (Cmd+T new tab, Cmd+O open file, Cmd+P print). Chan-desktop's
    // KEY_BRIDGE_JS intercepts the native chords and replays them
    // as `chan:command` events, which `runCommand` below routes
    // through the same context-aware helpers.
    //
    // Mac-only: require metaKey explicitly (not the `meta` shorthand
    // that includes Ctrl) so Ctrl+Alt+<letter> on Win/Linux stays
    // free for other bindings (`Ctrl+Alt+T` is owned by
    // `app.tab.reopenClosed`; Ctrl+Alt+O / P aren't used yet but
    // we don't want to claim them either). Hybrid Nav `o`/`p`/`v`
    // is the universal fallback on every platform.
    if (e.metaKey && e.altKey && !e.shiftKey && !e.ctrlKey && e.code === "KeyT") {
      e.preventDefault();
      spawnTerminalFromContext();
      return;
    }
    if (e.metaKey && e.altKey && !e.shiftKey && !e.ctrlKey && e.code === "KeyO") {
      e.preventDefault();
      spawnBrowserFromContext();
      return;
    }
    if (e.metaKey && e.altKey && !e.shiftKey && !e.ctrlKey && e.code === "KeyP") {
      e.preventDefault();
      spawnRichPromptFromContext();
      return;
    }
    // `fullstack-a-32`: Cmd+Shift+M spawns a context-aware graph on
    // both web and native. Browsers don't reserve this chord, so
    // no Cmd+Alt+M fallback is needed. KEY_BRIDGE_JS still fires
    // `app.graph.toggle` on native Cmd+Shift+M for parity with
    // the chan-desktop chord catalog.
    if (e.metaKey && !e.altKey && e.shiftKey && !e.ctrlKey && e.code === "KeyM") {
      e.preventDefault();
      spawnGraphFromContext();
      return;
    }
    if (meta && !e.altKey && !e.shiftKey && e.code === "BracketLeft") {
      e.preventDefault();
      selectPrevPane();
      return;
    }
    if (meta && !e.altKey && !e.shiftKey && e.code === "BracketRight") {
      e.preventDefault();
      selectNextPane();
      return;
    }
    if (meta && !e.altKey && !e.shiftKey && e.code === "KeyW") {
      if (closeActiveEmptyPane()) {
        e.preventDefault();
        e.stopPropagation();
        return;
      }
    }
    if (e.altKey && e.shiftKey && !meta) {
      // e.code is layout-and-modifier-independent, so this branch
      // matches even though Option mangles e.key into `«` / `»` on
      // a US Mac layout. preventDefault suppresses the typed
      // character before it reaches the focused editor.
      if (e.code === "BracketLeft") {
        e.preventDefault();
        selectPrevTabInActivePane();
        return;
      }
      if (e.code === "BracketRight") {
        e.preventDefault();
        selectNextTabInActivePane();
        return;
      }
    }
    // Ctrl+Alt+1..9 jump-to-tab. e.code === "Digit<N>" so the
    // comparison survives modifiers changing e.key to a glyph on
    // non-US layouts AND Option mangling it on Mac. metaKey
    // excluded so this is distinct from Cmd+1..9 (which the
    // browser owns for tab switching).
    if (e.ctrlKey && e.altKey && !e.shiftKey && !e.metaKey) {
      const m = e.code.match(/^Digit([1-9])$/);
      if (m) {
        e.preventDefault();
        selectTabAtIndexInActivePane(Number(m[1]) - 1);
        return;
      }
    }
    // `fullstack-a-73`: Cmd+R (Ctrl+R on non-Mac) — window-level
    // reload, mirroring the browser's default Cmd+R. The pane
    // right-click menu's Reload entry calls the same
    // `reloadWindow()` helper. preventDefault suppresses the
    // browser-default reload on web so we don't get a double-fire
    // (SPA handler + browser default). On chan-desktop the
    // serve.rs:1140 Tauri-side binding still routes through the
    // same IPC; the dual path is idempotent.
    if (meta && !e.altKey && !e.shiftKey && !e.ctrlKey && e.code === "KeyR") {
      e.preventDefault();
      void reloadWindow();
      return;
    }
    // `fullstack-a-66`: Cmd+N (Ctrl+N on non-Mac) — New Draft.
    // Calls /api/drafts/new which creates a fresh
    // `Drafts/<untitled-N>/draft.md` via chan-drive's
    // unified-path API + indexes it. The returned unified path
    // opens in the active pane like any other file (post-`-26`
    // the editor's read/write goes through the same Drafts/-
    // prefix-routing). chan-desktop's `-b-27` moved its
    // "New Window" accelerator to Cmd+Shift+N to free plain
    // Cmd+N for this handler.
    //
    // Cmd+Shift+N falls through to chan-desktop's Tauri menu
    // (or browser's New Incognito on web); we only intercept
    // bare Cmd+N here.
    if (meta && !e.altKey && !e.shiftKey && !e.ctrlKey && e.code === "KeyN") {
      e.preventDefault();
      void createDraftAndOpen();
      return;
    }
    // `fullstack-a-77` slice 3: Mod+L → lock screen. On web
    // Mac the browser owns Cmd+L (address bar focus); the
    // chord fires only on platforms where the browser
    // doesn't reserve it. chan-desktop's KEY_BRIDGE_JS
    // intercepts the native Cmd+L + replays as
    // `chan:command app.screensaver.lock`, which the
    // runCommand switch routes through `lockNow()`.
    if (meta && !e.altKey && !e.shiftKey && !e.ctrlKey && e.code === "KeyL") {
      e.preventDefault();
      lockNow();
      return;
    }
    // `fullstack-a-67f` slice 2: Mod+E (Obsidian-style "Show
    // Source Code") flips the active file tab's mode between
    // source and the rendered surface. No-op when the active
    // tab isn't a file tab. chan-desktop's KEY_BRIDGE_JS
    // replays Cmd+E natively as `chan:command
    // app.editor.toggleMode` — both paths converge on the
    // runCommand switch.
    if (meta && !e.altKey && !e.shiftKey && !e.ctrlKey && e.code === "KeyE") {
      e.preventDefault();
      toggleActiveFileTabMode();
      return;
    }
  }

  async function createDraftAndOpen(): Promise<void> {
    try {
      const { path } = await api.createDraft();
      await noteDraftCreated(path);
      await openInActivePane(path);
    } catch (err) {
      console.warn("[chan] createDraft failed", err);
      setTransientStatus(`New draft failed: ${(err as Error).message}`);
    }
  }

  /// `fullstack-a-68 slice 2`: walk the queue of staged "new
  /// draft editor" intents and resolve each one. Snapshot the
  /// queue up-front because `commitPaneMode` clears it (the
  /// callsite calls commit immediately after this returns).
  /// Each round-trip opens the resulting file in the paneId
  /// pinned at press time so a focus change mid-Nav doesn't
  /// redirect the materialization. createDraft + openInPane
  /// failures log + bail per-entry so one bad draft can't
  /// poison the rest of the queue.
  function materializeStagedDraftEditors(): void {
    const queue = paneMode.stagedDraftEditors.slice();
    for (const entry of queue) {
      void (async () => {
        try {
          const { path } = await api.createDraft();
          await noteDraftCreated(path);
          await openInPane(entry.paneId, path);
        } catch (err) {
          console.warn("[chan] paneMode stagedDraftEditor failed", err);
          setTransientStatus(`New draft failed: ${(err as Error).message}`);
        }
      })();
    }
  }
  function leafPaneCount(): number {
    return Object.values(layout.nodes).filter((node) => node.kind === "leaf").length;
  }
  function closeTabsInActivePane(): void {
    const paneId = layout.activePaneId;
    void closeTabsInPane(paneId).then((closed) => {
      if (closed) scheduleSessionSave();
    });
  }
  function killActivePane(opts?: { force?: boolean }): void {
    const paneId = layout.activePaneId;
    void closePane(paneId, opts).then((closed) => {
      if (closed) scheduleSessionSave();
    });
  }
  function closeActiveEmptyPane(): boolean {
    const p = activePane();
    if (p.tabs.length !== 0) return false;
    if (leafPaneCount() <= 1) return false;
    killActivePane({ force: true });
    return true;
  }
  onMount(() => document.addEventListener("keydown", onWindowKey));
  onDestroy(() => document.removeEventListener("keydown", onWindowKey));

  /// Ctrl+D: close the focused non-terminal tab. Per `fullstack-41`,
  /// the keystroke is canonical "close current tab" for Files / Graph
  /// / doc tabs; terminal tabs forward Ctrl+D to the shell as EOF
  /// and we deliberately stay out of that path. The listener fires
  /// at the document's CAPTURE phase so it pre-empts CodeMirror's
  /// default `selectNextOccurrence` multi-cursor binding inside a
  /// focused doc tab — the alternative (bubble) loses the race and
  /// leaves a stale multi-selection behind every close.
  function onCtrlDCapture(e: KeyboardEvent): void {
    if (!e.ctrlKey || e.metaKey || e.shiftKey || e.altKey) return;
    // e.key is lowercase "d" or uppercase "D" depending on
    // caps-lock; e.code === "KeyD" is layout-agnostic and matches
    // both. The keystroke we care about is the literal Ctrl + the
    // physical D key, not a shifted variant or a Cmd-modified one.
    if (e.code !== "KeyD") return;
    // In-house modals + the Cmd+K pane-mode dispatcher own their
    // own keyboard contexts; never close a tab from under them.
    if (promptState.open || pathPromptState.open || confirmState.open || draftCloseState.open) {
      return;
    }
    if (paneMode.active) return;
    const p = activePane();
    const active = p.tabs.find((t) => t.id === p.activeTabId);
    if (!active) {
      if (closeActiveEmptyPane()) {
        e.preventDefault();
        e.stopPropagation();
      }
      return;
    }
    // Terminal: leave the event alone so xterm forwards Ctrl+D
    // (EOF) to the shell. The shell exit collapses the tab through
    // the existing terminal-session lifecycle.
    if (active.kind === "terminal") return;
    // Files / Graph / Doc tabs: pre-empt the default handler and
    // close the tab. stopPropagation prevents CodeMirror's
    // selectNextOccurrence from firing on the same keystroke.
    e.preventDefault();
    e.stopPropagation();
    void closeTab(p.id, active.id);
  }
  onMount(() => document.addEventListener("keydown", onCtrlDCapture, true));
  onDestroy(() => document.removeEventListener("keydown", onCtrlDCapture, true));

  /// Host-driven command bridge. Native wrappers (chan-desktop) and
  /// other embeddings dispatch a `chan:command` window event to
  /// trigger an app action by string id without depending on any
  /// in-app key chord. Names are stable; payload (if any) goes in
  /// `detail`. Unknown ids are a no-op so hosts can ship ahead of
  /// chan adding the command.
  function runCommand(name: string, detail: Record<string, unknown>): void {
    switch (name) {
      case "app.settings.toggle":
        if (settingsOverlay.open) closeOverlay("settings");
        else openSettings();
        return;
      // `fullstack-a-32`: chan-desktop's KEY_BRIDGE_JS fires these
      // ids on native Cmd+T / Cmd+O / Cmd+P / Cmd+Shift+M. Route
      // them through the same context-aware helpers the web
      // chords use so native + web behave identically.
      case "app.files.toggle":
        spawnBrowserFromContext();
        return;
      case "app.search.toggle":
        searchPanel.open = !searchPanel.open;
        return;
      case "app.graph.toggle":
        spawnGraphFromContext();
        return;
      case "app.terminal.toggle":
        spawnTerminalFromContext();
        return;
      case "app.terminal.richPrompt":
        spawnRichPromptFromContext();
        return;
      // `fullstack-a-67` slice 2: New Draft is now a hamburger
      // menu entry too. Route the command through
      // `createDraftAndOpen` so the menu + the Cmd+N chord +
      // chan-desktop's native menu all converge on a single
      // handler.
      case "app.draft.new":
        void createDraftAndOpen();
        return;
      // `fullstack-a-77` slice 3: manual screensaver lock.
      // Routes both the SPA chord (`Mod+L` via onWindowKey
      // when the browser doesn't reserve it) AND the
      // chan-desktop KEY_BRIDGE_JS replay through the same
      // handler.
      case "app.screensaver.lock":
        lockNow();
        return;
      // `fullstack-a-75`: open the new Infographics tab in the
      // active pane. Surface unification: same command from the
      // pane hamburger, the empty-pane right-click menu, and
      // the empty-pane carousel slide-1 button.
      case "app.infographics.open":
        openInfographicsInActivePane();
        return;
      // `fullstack-a-67f` slice 2: Obsidian-style Mod+E "Show
      // Source Code" toggle. The chord fires
      // `dispatchEditorToggleMode()` which finds the active file
      // tab and flips its mode between source and the rendered
      // surface (wysiwyg / pretty / table). No-op when the
      // active tab isn't a file tab — keeps the chord harmless
      // outside the editor.
      case "app.editor.toggleMode":
        toggleActiveFileTabMode();
        return;
      case "app.pane.next":
        selectNextPane();
        return;
      case "app.pane.prev":
        selectPrevPane();
        return;
      case "app.pane.closeTabs":
        closeTabsInActivePane();
        return;
      case "app.pane.kill":
        killActivePane();
        return;
      case "app.tab.next":
        selectNextTabInActivePane();
        return;
      case "app.tab.prev":
        selectPrevTabInActivePane();
        return;
      case "app.tab.jump": {
        const i = Number(detail?.index);
        if (Number.isInteger(i) && i >= 0) selectTabAtIndexInActivePane(i);
        return;
      }
      case "app.tab.close": {
        const p = activePane();
        if (p.activeTabId) closeTab(p.id, p.activeTabId);
        else closeActiveEmptyPane();
        return;
      }
      // `fullstack-56`: dropped `app.save` — autosave covers the
      // write path; the keystroke + action surface is gone.
      case "app.file.new":
        void fileOps.createFile("");
        return;
      case "app.find.open": {
        const t = activeFileTab();
        if (!t) return;
        openFind(t.id);
        return;
      }
      case "app.find.next": {
        const t = activeFileTab();
        if (!t?.find?.open) return;
        const n = t.find.matches.length;
        if (n === 0) return;
        const cur = t.find.currentIndex < 0 ? 0 : t.find.currentIndex;
        t.find.currentIndex = (cur + 1) % n;
        return;
      }
      case "app.find.prev": {
        const t = activeFileTab();
        if (!t?.find?.open) return;
        const n = t.find.matches.length;
        if (n === 0) return;
        const cur = t.find.currentIndex < 0 ? 0 : t.find.currentIndex;
        t.find.currentIndex = (cur - 1 + n) % n;
        return;
      }
      case "app.find.close": {
        const t = activeFileTab();
        if (!t) return;
        closeFind(t.id);
        return;
      }
    }
  }
  function onChanCommand(e: Event): void {
    const detail = (e as CustomEvent).detail ?? {};
    if (typeof detail.name !== "string") return;
    runCommand(detail.name, detail);
  }
  onMount(() => window.addEventListener("chan:command", onChanCommand));
  onDestroy(() => window.removeEventListener("chan:command", onChanCommand));

  // `fullstack-a-72`: prune stale + over-cap editor-buffer
  // entries from localStorage at app load. Background task; the
  // synchronous read/write paths in editorBuffer.svelte.ts also
  // self-prune on quota-exceeded but doing a sweep here keeps
  // localStorage tidy for users with long-lived sessions.
  onMount(() => {
    pruneEditorBuffers();
  });

  // `fullstack-a-74`: synchronously flush any in-flight
  // debounced editor-buffer writes before the page tears down.
  // The pre-`-a-74` mechanism relied on Svelte component-
  // cleanup callbacks to flush, but `window.location.reload()`
  // (Cmd+R / browser refresh / chan-desktop reload IPC) DOES
  // NOT trigger component cleanup — so the last 500ms of edits
  // silently vanished. `beforeunload` + `pagehide` both fire
  // reliably before the page tears down; `pagehide` is the
  // canonical mobile-safe variant + `beforeunload` covers
  // desktop reloads, so we register both.
  //
  // The handlers are deliberately tiny + synchronous — async
  // work in beforeunload is unreliable across browsers, and a
  // synchronous localStorage write is fine.
  function onUnloadFlushBuffers(): void {
    flushPendingBufferWrites();
  }
  onMount(() => {
    window.addEventListener("beforeunload", onUnloadFlushBuffers);
    window.addEventListener("pagehide", onUnloadFlushBuffers);
  });
  onDestroy(() => {
    window.removeEventListener("beforeunload", onUnloadFlushBuffers);
    window.removeEventListener("pagehide", onUnloadFlushBuffers);
  });

  /// `fullstack-a-59` pane-focus-click restore: when chan-desktop is
  /// unfocused and the user clicks back onto the window, the first
  /// click should ALSO select the Hybrid pane under the cursor (not
  /// stay on the previously-focused pane). Critical disambiguation:
  /// only on the mousedown-driven focus restore. Cmd+Tab keyboard
  /// refocus must NOT change pane selection (focus event without an
  /// adjacent mousedown).
  ///
  /// Detection: track the last `window` focus event timestamp +
  /// listen for `mousedown` at the window level. If a mousedown
  /// fires within `FOCUS_CLICK_WINDOW_MS` of a focus event, walk the
  /// target's DOM ancestry looking for the nearest `.pane[data-pane-
  /// id]`. If found, call `setActivePane` on that pane id. Clear
  /// the timestamp after the first matching mousedown so subsequent
  /// clicks fall back to the existing per-pane `onmousedown` handler
  /// in `Pane.svelte` (which already calls `setActivePane`).
  ///
  /// Why this matters: on macOS / Tauri the first mousedown after a
  /// window-focus restore is sometimes consumed by the OS for the
  /// window-activation gesture and doesn't reach the per-pane
  /// handler. Without this top-level catch, the previously-focused
  /// pane stays active even though the user just clicked elsewhere.
  const FOCUS_CLICK_WINDOW_MS = 50;
  let focusRestoreAt = 0;
  function onWindowFocus(): void {
    focusRestoreAt = Date.now();
  }
  function onWindowMouseDown(e: MouseEvent): void {
    if (focusRestoreAt === 0) return;
    if (Date.now() - focusRestoreAt > FOCUS_CLICK_WINDOW_MS) {
      focusRestoreAt = 0;
      return;
    }
    focusRestoreAt = 0;
    const target = e.target;
    if (!(target instanceof Element)) return;
    const paneEl = target.closest<HTMLElement>(".pane[data-pane-id]");
    if (!paneEl) return;
    const paneId = paneEl.dataset.paneId;
    if (!paneId) return;
    setActivePane(paneId);
  }
  onMount(() => {
    window.addEventListener("focus", onWindowFocus);
    window.addEventListener("mousedown", onWindowMouseDown, true);
  });
  onDestroy(() => {
    window.removeEventListener("focus", onWindowFocus);
    window.removeEventListener("mousedown", onWindowMouseDown, true);
  });
</script>

<div class="app" class:pane-mode={paneMode.active}>
  {#if browserSidePanes.left}
    <FileBrowserSidePane side="left" />
  {/if}
  <main>
    <Workspace />
  </main>
  {#if browserSidePanes.right}
    <FileBrowserSidePane side="right" />
  {/if}
</div>
<!-- Bottom-left ambient status bar: indexer state, import
     progress, transient ui.status messages. Window-level and
     lifted above every overlay so users keep visibility on
     long-running work no matter which panel they're in. -->
<AppStatusBar />
<!-- Pane Mode (Cmd+K) cheatsheet, toggled with `h` while pane mode
     is active. Gated on the live `paneMode.active` so it auto-hides
     the moment the transaction commits / discards. -->
{#if paneMode.active && paneModeHelpVisible}
  <PaneModeHelp />
{/if}
<!-- Window-level overlays. Mounted once. -->
<PromptModal />
<PathPromptModal />
<ConfirmModal />
<DraftCloseModal />
<DriveWarningsModal />
<SearchPanel />
<SearchStatusOverlay />
<SettingsPanel />
<!-- CAS conflict prompt: surfaces when a save returns 409. Mounted
     once per window so any pane can trigger it; the dialog itself
     keys off `conflictDialog.tabId`. -->
<ConflictModal />
<!-- `fullstack-a-4`: Spawn agent dialog. Mounted at the App
     root rather than inside the rich prompt so its `position:
     fixed` backdrop is never clipped by an ancestor's stacking
     context (the pane has `overflow: hidden`; Hybrid Nav adds
     a `filter` to unfocused panes; the rich prompt itself is a
     positioned z-index: 20 stacking context). -->
<SpawnDialog />
<!-- `fullstack-a-78`: Spawn agents dialog mounted at App root for
     the same stacking-context reasons as SpawnDialog. Renders
     only when a request is pending; closes itself on
     Bootstrap / Cancel / Escape / backdrop click. -->
{#if teamDialogState.request}
  <TeamDialog request={teamDialogState.request} />
{/if}
<!-- Disconnect overlay applies in every mode: any window is just
     as broken when the watcher dies, regardless of layout. -->
<DisconnectOverlay />
<!-- Missing-token overlay: surfaces when the user landed on the
     SPA shell without the launch token, so /api 401s and the app
     is unusable until they reopen the original URL. -->
<MissingTokenOverlay />
<!-- `fullstack-a-77` slice 2: screensaver cover. Mounts at App
     root so z-index sits above every chan overlay
     (`screensaver-backdrop` uses z=2000). The component
     renders nothing while `screensaver.locked === false`;
     when locked it covers the SPA + accepts PIN entry. -->
<ScreensaverOverlay />

<style>
  /* Theme palette. Defaults to dark; [data-theme="light"] overrides.
     The neutrals mirror Apple's Notes / system grays so chan reads
     as "the markdown notes app" rather than "GitHub Dark with our
     stuff in it"; functional colors (link blue, accent green, warn
     amber, pane focus) are kept distinct. */
  /* Surface-scoped theme overrides re-apply the same token block at
     `[data-theme="..."]` so a Hybrid body theme pick cascades only
     through that surface subtree without touching the root. */
  :global(:root),
  :global([data-theme="dark"]) {
    --bg: #1c1c1e;
    --bg-card: #232325;
    --bg-elev: #2a2a2c;
    --border: #3a3a3c;
    /* Softened dark-mode text. Pure white #f5f5f7 on near-black
       #1c1c1e measures ~17:1 contrast, far above Apple's spec for
       primary label on system dark (which lands ~13:1). #ebebf0
       is Apple's "primary label" off-white; --text-heading dimmed
       a notch further so H1/H2 don't fight the body. Light mode
       is intentionally untouched. */
    --text: #ebebf0;
    --text-secondary: #8e8e93;
    --text-heading: #d8d8de;
    --link: #58a6ff;
    /* Code background needs a clear step away from --bg (#1c1c1e) so
       fenced blocks and inline `code` read as a slab; #232325 collided
       with --bg-card. Light-mode value (below) does the same against
       the white canvas. */
    --code-bg: #2a2a2c;
    --btn-bg: #2a2a2c;
    --btn-border: #3a3a3c;
    --btn-hover: #98989d;
    --accent: #3fb950;
    --warn-text: #e3b341;
    --danger-text: #f85149;
    /* "Unsaved buffer" color used by the dirty-dot in the file
       tree and tab strip. */
    --info-text: #4ade80;
    --hover-bg: rgba(255, 255, 255, 0.06);
    --selection-bg: rgba(56, 139, 253, 0.4);
    /* Subtle tint on the off-page area when the page-width cap is
       active. Sits below --hover-bg so the page itself still pops. */
    --page-shade: rgba(255, 255, 255, 0.025);
    /* Subtle alternating row tint for tree views (file browser).
       Sits well below --hover-bg in opacity so hover/selection still
       wins over zebra striping at the cascade level. */
    --zebra-bg: rgba(255, 255, 255, 0.025);
    /* Right-side inspector panel background. Distinct from --bg-card
       so the inspector visually separates from the editor canvas
       (especially in light mode where --bg-card is close to --bg). */
    --inspector-bg: #232325;
    /* Resize-handle bar color. Brighter than --border so the 4px bar
       between inspector and canvas is findable in light mode (where
       --border can blend with bg). */
    --separator: #4a4a4d;
    --separator-hover: #98989d;
    --tab-active-bg: #1c1c1e;
    --tab-inactive-bg: #232325;
    --smart-bg: rgba(88, 166, 255, 0.18);
    --pane-focus: #388bfd;
    --bubble-bg: #2a2a2c;
    --bubble-right-bg: rgba(88, 166, 255, 0.28);
    /* Graph palette: only doc / image / tag are rendered now,
       matching the chan brand's warm orange primary plus a
       hue-separated supporting pair (purple for images, green
       for tag hashtag labels). */
    --g-doc: #ff8a3d;
    --g-img: #b07dff;
    --g-tag: #6cd07a;
    --chan-color-language: #ff4db8;
    --chan-color-code: var(--chan-color-language);
    --g-language: var(--chan-color-language);
    /* `fullstack-a-51` G6 colour scheme: markdown / source / binary
       / media split.
       * `--g-doc` (above) → orange for markdown (.md, .txt).
       * `--g-source` (royalblue) → code + config text files (.rs,
         .py, .ts, etc.). Pre-`-a-51` had this hue assigned to
         `--g-binary`; the rename clarifies the bucket.
       * `--g-binary` (grey, darker than `--g-folder`) → opaque files
         (archives, executables, fonts, etc.).
       * `--g-img` (above) → purple for media (image / pdf).
       * `--g-folder` (medium grey) → directory nodes; distinct from
         binary's darker grey so the two don't visually collapse. */
    --g-source: #4169e1;
    --g-binary: #5e5e62;
    --g-folder: #8e8e93;
    /* `fullstack-a-66b`: Drafts folder distinct yellow tone.
       Sits at the top of the FB tree as the synthetic "Drafts"
       entry. Background uses a low-alpha tint so the row reads
       as a category marker without dominating the panel. */
    --fb-drafts-fg: #e3b341;
    --fb-drafts-bg: rgba(227, 179, 65, 0.10);
    /* Inline editor pills (wiki link, image, tag, contact, date,
       broken). Hues track the canonical concept palette so the
       same item reads with the same color across the graph, the
       file tree, the info panel, and the editor: document ->
       orange, media -> purple, tag -> green, contact -> yellow,
       date/time -> grey, broken -> red. See web/src/design.md
       for the full cross-surface table. Backgrounds are alpha
       tints of the foreground so each pill reads as a badge
       rather than a button. */
    --pill-wiki-fg: var(--text);
    --pill-wiki-bg: rgba(255, 138, 61, 0.18);
    --pill-wiki-bg-hover: rgba(255, 138, 61, 0.28);
    --pill-image-fg: var(--text);
    --pill-image-bg: rgba(176, 125, 255, 0.20);
    --pill-tag-fg: var(--text);
    --pill-tag-bg: rgba(108, 208, 122, 0.18);
    --pill-tag-bg-hover: rgba(108, 208, 122, 0.28);
    --pill-contact-fg: var(--text);
    --pill-contact-bg: rgba(227, 179, 65, 0.18);
    --pill-date-fg: var(--text);
    --pill-date-bg: rgba(152, 152, 157, 0.15);
    --pill-broken-fg: var(--text);
    --pill-broken-bg: rgba(255, 80, 75, 0.20);
    /* Floating-pane drop shadow. Dark mode reads against a
       near-black canvas, so the shadow is a subtle white-ish
       glow rather than a darker tone (which would disappear
       into the background). */
    --pane-shadow: 0 1px 6px rgba(255, 255, 255, 0.08);
  }
  :global([data-theme="light"]) {
    --bg: #ffffff;
    --bg-card: #f5f5f7;
    --bg-elev: #ffffff;
    --border: #d1d1d6;
    --text: #1c1c1e;
    --text-secondary: #6c6c70;
    --text-heading: #1c1c1e;
    --link: #0969da;
    --code-bg: #e8e8ec;
    --btn-bg: #f2f2f4;
    --btn-border: #d1d1d6;
    --btn-hover: #6c6c70;
    --accent: #1a7f37;
    --warn-text: #9a6700;
    --danger-text: #cf222e;
    --info-text: #16a34a;
    --hover-bg: rgba(0, 0, 0, 0.05);
    --selection-bg: rgba(9, 105, 218, 0.18);
    --page-shade: rgba(0, 0, 0, 0.035);
    --zebra-bg: rgba(0, 0, 0, 0.025);
    /* Light mode: pull the inspector several shades off white so the
       resize handle's seam reads as a real boundary instead of a
       hairline at the same brightness as the editor canvas. */
    --inspector-bg: #ececef;
    --separator: #b0b0b6;
    --separator-hover: #6c6c70;
    --tab-active-bg: #ffffff;
    --tab-inactive-bg: #ececec;
    --smart-bg: rgba(80, 120, 200, 0.12);
    --pane-focus: #7aa6e0;
    --bubble-bg: #ececef;
    --bubble-right-bg: #cfe1fb;
    /* Light-mode graph palette: deeper, less saturated than dark
       mode so the same node hues stay legible against light bg
       without glaring. */
    --g-doc: #c25a1f;
    --g-img: #7a4cd8;
    --g-tag: #2f9444;
    --chan-color-language: #c71585;
    --chan-color-code: var(--chan-color-language);
    --g-language: var(--chan-color-language);
    /* `fullstack-a-51` G6 light-mode counterparts: deeper hues
       balanced against the bright bg. */
    --g-source: #2851c4;
    --g-binary: #4e4e54;
    --g-folder: #6c6c70;
    /* `fullstack-a-66b` light-mode counterparts — deeper yellow
       for contrast against the bright page bg. */
    --fb-drafts-fg: #9a6700;
    --fb-drafts-bg: rgba(154, 103, 0, 0.08);
    /* Light-mode pill palette. Same canonical mapping as dark
       (document orange, media purple, tag green, contact yellow,
       date grey, broken red), but deeper foreground hues and
       lower-alpha backgrounds so the pills still read as badges
       against the bright canvas. */
    --pill-wiki-fg: #c25a1f;
    --pill-wiki-bg: rgba(255, 138, 61, 0.14);
    --pill-wiki-bg-hover: rgba(255, 138, 61, 0.22);
    --pill-image-fg: #7a4cd8;
    --pill-image-bg: rgba(122, 76, 216, 0.10);
    --pill-tag-fg: #2f9444;
    --pill-tag-bg: rgba(47, 148, 68, 0.12);
    --pill-tag-bg-hover: rgba(47, 148, 68, 0.20);
    --pill-contact-fg: #9a6700;
    --pill-contact-bg: rgba(154, 103, 0, 0.10);
    --pill-date-fg: #6c6c70;
    --pill-date-bg: rgba(108, 108, 112, 0.10);
    --pill-broken-fg: #c93232;
    --pill-broken-bg: rgba(255, 59, 48, 0.12);
    /* Standard soft drop-shadow against the light canvas. */
    --pane-shadow: 0 1px 6px rgba(0, 0, 0, 0.14);
  }

  :global(html), :global(body), :global(#app) {
    height: 100%; margin: 0; padding: 0;
  }
  :global(body) {
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
    /* Baseline; chrome / file tree / overlays inherit this where
       no explicit size is set. Component-level font-sizes are
       in the 12-16 px range; baseline 15 keeps the small ones
       readable on 4k panels at 100% scale. */
    font-size: 15px;
    color: var(--text);
    background: var(--bg);
    overflow: hidden;
  }
  :global(.md-bubble .md-bubble-status.md-bubble-status-empty) {
    border-top: 0;
    margin-top: 0;
    padding: 0;
  }
  :global(.md-bubble .md-bubble-empty-state) {
    padding: 7px 8px;
    color: var(--text-secondary, #888);
    gap: 2px;
  }
  :global(.md-bubble .md-bubble-empty-primary) {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--text);
  }
  :global(.md-bubble .md-bubble-empty-secondary) {
    font-size: 12px;
  }
  :global(.md-bubble .md-bubble-spinner) {
    width: 10px;
    height: 10px;
    border: 2px solid var(--border, #ddd);
    border-top-color: var(--accent, #2563b8);
    border-radius: 999px;
    animation: md-bubble-spin 700ms linear infinite;
  }
  @keyframes md-bubble-spin {
    to { transform: rotate(360deg); }
  }
  .app {
    display: flex;
    height: 100vh;
    width: 100vw;
  }
  .app.pane-mode :global(.pane) {
    position: relative;
    transition:
      opacity 90ms ease,
      box-shadow 90ms ease,
      filter 90ms ease;
  }
  .app.pane-mode :global(.pane:not(.focused)) {
    opacity: 0.72;
    filter: saturate(0.8);
  }
  .app.pane-mode :global(.pane.focused) {
    box-shadow:
      inset 0 0 0 2px var(--pane-active-focus),
      0 0 0 1px color-mix(in srgb, var(--pane-active-focus) 40%, transparent);
  }
  main {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    position: relative;
    /* Workspace backdrop sits one step off the pane background so
       the pane chrome (rounded corners + drop shadow) reads as a
       floating card. Without this contrast the shadow has nothing
       to fall onto and the rounded corners hug the same color on
       both sides. */
    background: var(--bg-card);
  }

</style>
