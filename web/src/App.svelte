<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import AppStatusBar from "./components/AppStatusBar.svelte";
  // import BottomPill from "./components/BottomPill.svelte";
  import ConfirmModal from "./components/ConfirmModal.svelte";
  import ConflictModal from "./components/ConflictModal.svelte";
  import DisconnectOverlay from "./components/DisconnectOverlay.svelte";
  import FileBrowserOverlay from "./components/FileBrowserOverlay.svelte";
  import GraphPanel from "./components/GraphPanel.svelte";
  import InlineAssist from "./components/InlineAssist.svelte";
  import MissingTokenOverlay from "./components/MissingTokenOverlay.svelte";
  import PathPromptModal from "./components/PathPromptModal.svelte";
  import PromptModal from "./components/PromptModal.svelte";
  import ScopeHistoryOverlay from "./components/ScopeHistoryOverlay.svelte";
  import SearchPanel from "./components/SearchPanel.svelte";
  import SearchStatusOverlay from "./components/SearchStatusOverlay.svelte";
  import SettingsPanel from "./components/SettingsPanel.svelte";
  import Workspace from "./components/Workspace.svelte";
  import {
    applyInitialTheme,
    assistantOverlay,
    bootstrap,
    installSessionFlushHook,
    browserOverlay,
    browserSelection,
    closeOverlay,
    drive,
    fileOps,
    graphOverlay,
    openAssistant,
    openBrowser,
    openGraph,
    openSettings,
    persistLayoutToHash,
    reconnectWatcher,
    refreshDrive,
    refreshTree,
    scheduleSessionSave,
    scopeHistoryOverlay,
    searchStatusOverlay,
    searchPanel,
    settingsOverlay,
    syncOverlayStack,
    topOverlay,
    watchBubbleDisplayMode,
    watchSystemTheme,
  } from "./state/store.svelte";
  import {
    activeFileTab,
    activePane,
    closeFind,
    closeTab,
    isWindowFullyReadOnly,
    layout,
    openFind,
    openInActivePane,
    saveTab,
    scheduleAutosave,
    selectNextTabInActivePane,
    selectPrevTabInActivePane,
    selectTabAtIndexInActivePane,
  } from "./state/tabs.svelte";
  import { applyEditorTheme, DEFAULT_EDITOR_THEME } from "./state/editorTheme";
  import {
    applyInitialPageWidth,
    watchPageWidth,
  } from "./state/pageWidth.svelte";
  import { installIdleTracker, setReadMode } from "./state/idle.svelte";

  // Keep the URL hash in sync with the current layout so reload (and
  // copy-paste of the URL) restores the same panes/tabs. We touch
  // every reactive bit of `layout` in the effect so Svelte tracks
  // mutations to nested arrays/strings: JSON.stringify in
  // serializeLayout() already does that, but the function is called
  // synchronously, and Svelte only tracks reads that happen *during*
  // the effect's run.
  let bootstrapped = $state(false);
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
    void searchPanel.scopeId;
    void searchPanel.inspectorOpen;
    void assistantOverlay.open;
    void assistantOverlay.contextId;
    void assistantOverlay.prompt;
    void assistantOverlay.inspectorOpen;
    void browserOverlay.open;
    void browserOverlay.inspectorOpen;
    void browserSelection.path;
    void graphOverlay.open;
    void graphOverlay.scopeId;
    void graphOverlay.mode;
    void graphOverlay.depth;
    void graphOverlay.filters.link;
    void graphOverlay.filters.tag;
    void graphOverlay.filters.mention;
    void graphOverlay.filters.img;
    void graphOverlay.inspectorOpen;
    void scopeHistoryOverlay.open;
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
    void assistantOverlay.open;
    void settingsOverlay.open;
    void scopeHistoryOverlay.open;
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
    // Same storage-event sync for the assistant chat's bubble
    // render mode so flipping it in one window propagates to
    // every other open window live.
    watchBubbleDisplayMode();
    // Idle tracker: after 2.5s without scroll/click/keypress, the
    // floating pills fade. Any input flips them back on.
    installIdleTracker();
    // Hook pagehide BEFORE bootstrap so a fast reload during the
    // initial load still flushes any in-flight session changes.
    installSessionFlushHook();
    await bootstrap();
    // Boot-time: if no tabs were restored anywhere in the layout,
    // pop the file browser so the user has a launch surface instead
    // of staring at the empty-pane logo. Subsequent tab closes leave
    // the empty pane intact (the logo + shortcut hints take over).
    const hasAnyTab = Object.values(layout.nodes).some(
      (n) => n.kind === "leaf" && n.tabs.length > 0,
    );
    if (!hasAnyTab) openBrowser();
    bootstrapped = true;
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

  /// App-level keyboard shortcuts. Layout follows VS Code where
  /// possible so users carry intuition in from any code editor.
  ///
  ///   Cmd/Ctrl+,             -> Settings (open)
  ///   Cmd/Ctrl+P             -> Files (toggle)            [VS Code Quick Open]
  ///   Cmd/Ctrl+I             -> Assistant (toggle)        [VS Code Copilot inline]
  ///   Cmd/Ctrl+Shift+F       -> Search across files       [VS Code Find in Files]
  ///   Cmd/Ctrl+Shift+M       -> Graph (toggle)
  ///   Alt+Shift+[ / ]        -> previous / next tab       (web fallback)
  ///   Ctrl+Alt+1..9          -> jump to tab N             (web fallback)
  ///   Ctrl+Alt+N             -> new file                  (web fallback)
  ///
  /// Native (chan-desktop) layers VS Code's browser-reserved chords
  /// on top via its init script: Cmd+W (close tab), Cmd+N (new),
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
    if (meta && !e.shiftKey && !e.altKey && e.key === ",") {
      e.preventDefault();
      openSettings();
      return;
    }
    if (meta && !e.shiftKey && !e.altKey && e.code === "KeyP") {
      e.preventDefault();
      browserOverlay.open = !browserOverlay.open;
      if (browserOverlay.open) openBrowser();
      return;
    }
    if (meta && !e.shiftKey && !e.altKey && e.code === "KeyI") {
      // Assistant master switch: when disabled in Settings, the
      // chord falls through and the user gets no visible response,
      // matching the hidden bottom-pill button.
      if (!(drive.info?.preferences.assistant.effective_enabled ?? false)) return;
      e.preventDefault();
      if (assistantOverlay.open) {
        assistantOverlay.open = false;
      } else {
        openAssistant();
      }
      return;
    }
    if (meta && e.shiftKey && !e.altKey && e.code === "KeyF") {
      e.preventDefault();
      searchPanel.open = !searchPanel.open;
      return;
    }
    if (meta && e.shiftKey && !e.altKey && e.code === "KeyM") {
      e.preventDefault();
      if (graphOverlay.open) {
        graphOverlay.open = false;
      } else {
        openGraph();
      }
      return;
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
      // Ctrl+Alt+N: open the same "new file" prompt the file
      // browser menu uses. `fileOps.createFile` runs the path
      // dialog, creates the file, and opens it in the active pane.
      // e.code so Option mangling on macOS doesn't bury the chord.
      if (e.code === "KeyN") {
        e.preventDefault();
        void fileOps.createFile("");
      }
    }
  }
  onMount(() => document.addEventListener("keydown", onWindowKey));
  onDestroy(() => document.removeEventListener("keydown", onWindowKey));

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
      case "app.files.toggle":
        if (browserOverlay.open) closeOverlay("browser");
        else openBrowser();
        return;
      case "app.assistant.toggle":
        if (!(drive.info?.preferences.assistant.effective_enabled ?? false)) return;
        if (assistantOverlay.open) closeOverlay("assistant");
        else openAssistant();
        return;
      case "app.search.toggle":
        searchPanel.open = !searchPanel.open;
        return;
      case "app.graph.toggle":
        if (graphOverlay.open) closeOverlay("graph");
        else openGraph();
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
        return;
      }
      case "app.save": {
        const p = activePane();
        const t = p.tabs.find((x) => x.id === p.activeTabId);
        if (t) void saveTab(t);
        return;
      }
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
</script>

<div class="app">
  <main>
    <Workspace />
  </main>
</div>
<!-- Floating navigation pill: every overlay (files / search /
     graph / settings / assistant) is reachable from anywhere in
     the workspace. Temporarily disabled while we rethink the
     navigation surface; right-click menus + keyboard chords cover
     the same actions in the meantime. -->
<!-- <BottomPill /> -->
<!-- Bottom-left ambient status bar: indexer state, import
     progress, transient ui.status messages. Window-level and
     lifted above every overlay so users keep visibility on
     long-running work no matter which panel they're in. -->
<AppStatusBar />
<!-- Window-level overlays. Mounted once. -->
<PromptModal />
<PathPromptModal />
<ConfirmModal />
<SearchPanel />
<SearchStatusOverlay />
<InlineAssist />
<GraphPanel />
<SettingsPanel />
<FileBrowserOverlay />
<ScopeHistoryOverlay />
<!-- CAS conflict prompt: surfaces when a save returns 409. Mounted
     once per window so any pane can trigger it; the dialog itself
     keys off `conflictDialog.tabId`. -->
<ConflictModal />
<!-- Disconnect overlay applies in every mode: any window is just
     as broken when the watcher dies, regardless of layout. -->
<DisconnectOverlay />
<!-- Missing-token overlay: surfaces when the user landed on the
     SPA shell without the launch token, so /api 401s and the app
     is unusable until they reopen the original URL. -->
<MissingTokenOverlay />

<style>
  /* Theme palette. Defaults to dark; [data-theme="light"] overrides.
     The neutrals mirror Apple's Notes / system grays so chan reads
     as "the markdown notes app" rather than "GitHub Dark with our
     stuff in it"; functional colors (link blue, accent green, warn
     amber, pane focus) are kept distinct from the brand orange so
     they don't fight the assistant accent. */
  :global(:root) {
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
    /* Assistant chat bubbles. The user bubble takes a stronger blue
       tint so it stands out against the panel; the assistant bubble
       sits on a subtle off-bg shade so two adjacent assistant turns
       still read as discrete messages. Dark-mode values keep enough
       contrast over the dark panel; light-mode overrides below pick
       Apple-Messages-style pastels so the bubbles don't disappear
       into white. */
    --assistant-bubble-bg: #2a2a2c;
    --assistant-user-bubble-bg: rgba(88, 166, 255, 0.28);
    /* Brand accent for the assistant button (brand orange, matching
       chan.app). Single source for the ensō tint; light/dark each
       get their own shade so the silhouette reads on both. */
    --assistant-accent: #e58c4d;
    /* Graph palette: only doc / image / tag are rendered now,
       matching the chan brand's warm orange primary plus a
       hue-separated supporting pair (purple for images, green
       for tag hashtag labels). */
    --g-doc: #ff8a3d;
    --g-img: #b07dff;
    --g-tag: #6cd07a;
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
    /* Light-mode bubble tints: assistant on a warmer gray so it
       sits clearly above the white panel; user on Apple-Messages
       blue (#dbeafe-ish, slightly more saturated) so the two
       sides are distinguishable at a glance. */
    --assistant-bubble-bg: #ececef;
    --assistant-user-bubble-bg: #cfe1fb;
    --assistant-accent: #c46a2a;
    /* Light-mode graph palette: deeper, less saturated than dark
       mode so the same node hues stay legible against light bg
       without glaring. */
    --g-doc: #c25a1f;
    --g-img: #7a4cd8;
    --g-tag: #2f9444;
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
  .app {
    display: flex;
    height: 100vh;
    width: 100vw;
  }
  main {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    position: relative;
  }
</style>
