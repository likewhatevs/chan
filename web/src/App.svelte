<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import BottomPill from "./components/BottomPill.svelte";
  import CloseGuardModal from "./components/CloseGuardModal.svelte";
  import DisconnectOverlay from "./components/DisconnectOverlay.svelte";
  import FileBrowserOverlay from "./components/FileBrowserOverlay.svelte";
  import GraphPanel from "./components/GraphPanel.svelte";
  import InlineAssist from "./components/InlineAssist.svelte";
  import MobileFloatBar from "./components/MobileFloatBar.svelte";
  import PromptModal from "./components/PromptModal.svelte";
  import SearchPanel from "./components/SearchPanel.svelte";
  import SettingsPanel from "./components/SettingsPanel.svelte";
  import Workspace from "./components/Workspace.svelte";
  import { setupCloseGuard } from "./state/closeGuard.svelte";
  import {
    applyInitialTheme,
    assistantOverlay,
    bootstrap,
    browserOverlay,
    drive,
    graphOverlay,
    openAssistant,
    openBrowser,
    openGraph,
    openSettings,
    persistLayoutToHash,
    pruneStaleAssistantGroups,
    reconnectWatcher,
    refreshDrive,
    refreshTree,
    scheduleSessionSave,
    searchPanel,
    settingsOverlay,
    ui,
    watchSystemTheme,
  } from "./state/store.svelte";
  import {
    layout,
    openInActivePane,
    scheduleAutosave,
    selectNextTabInActivePane,
    selectPrevTabInActivePane,
    selectTabAtIndexInActivePane,
  } from "./state/tabs.svelte";
  import { applyFontPrefs, DEFAULT_FONT_PREFS } from "./state/fontPrefs";
  import { installIdleTracker } from "./state/idle.svelte";
  import { loadShared } from "./api/wasm";
  import {
    isMobile,
    isNativeDesktop,
    listenMenuAction,
    listenOpenSettings,
    readAndConsumeOpenFile,
  } from "./api/native";

  /// Mobile shell mode: chan-app's iOS / Android boot appends
  /// `?platform=ios` (or android). The shell is identical to the
  /// desktop one (Workspace + window-level overlays) plus a mobile-
  /// only floating bar with a keyboard-aware position flip. iPhone
  /// stays single-pane; iPad allows one split (capped in
  /// `tabs.svelte.ts::splitActive`).
  const mobile = isMobile();

  /// True in the chan-app desktop shell. The bare workspace
  /// (no top toolbar) is now the single layout; this flag only
  /// gates the Settings entry on the floating bar (web shows it
  /// because there is no native menubar; native hides it because
  /// macOS exposes Settings through Cmd+, on the App submenu).
  const nativeDesktop = isNativeDesktop();

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
      // Drop in-memory assistant group conversations whose set of
      // visible files no longer matches a current option. Runs on
      // every layout-affecting mutation so closing a pane that
      // contributed to a group conversation drops that thread
      // immediately rather than letting it linger as orphan state.
      pruneStaleAssistantGroups();
    }
  });

  // Mirror overlay open-state + per-overlay knobs + mobile recents
  // into the session payload so close-and-quit restores everything
  // on the next launch. Same debounce mechanism as the layout
  // effect; the helpers in store.svelte.ts already coalesce.
  $effect(() => {
    if (!bootstrapped) return;
    void settingsOverlay.open;
    void searchPanel.open;
    void assistantOverlay.open;
    void assistantOverlay.contextId;
    void browserOverlay.open;
    void graphOverlay.open;
    void graphOverlay.scopeId;
    void graphOverlay.depth;
    scheduleSessionSave();
  });

  // Push the latest font preferences into CSS variables whenever the
  // server-known drive info changes. The settings tab also calls
  // applyFontPrefs() locally for live preview between saves.
  $effect(() => {
    const fonts = drive.info?.preferences?.fonts;
    applyFontPrefs(fonts ?? DEFAULT_FONT_PREFS);
  });

  onMount(async () => {
    // Apply persisted theme + default fonts to the document root
    // immediately, before any component renders, to avoid a flash.
    applyInitialTheme();
    applyFontPrefs(DEFAULT_FONT_PREFS);
    // While in "system" mode, follow OS-level theme changes live.
    // The listener stays alive for the whole app's lifetime.
    watchSystemTheme();
    // Idle tracker: after 2.5s without scroll/click/keypress, the
    // floating pills fade. Any input flips them back on.
    installIdleTracker();
    if (mobile) {
      // iOS Safari / WKWebView auto-zooms when tapping into an input
      // whose computed font-size is < 16 px, and the zoom often
      // pushes the bottom action bar and parts of the top bar
      // off-screen. Pinning maximum-scale to 1 disables that
      // behaviour. Apply only on mobile so a desktop browser hitting
      // the same SPA keeps normal pinch-zoom.
      const meta = document.querySelector('meta[name="viewport"]');
      if (meta) {
        meta.setAttribute(
          "content",
          "width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no, viewport-fit=cover",
        );
      }
    }
    await Promise.all([loadShared(), bootstrap()]);
    // First-launch experience: if the URL hash didn't restore any
    // tabs, the pane stays empty and the file browser overlay
    // auto-opens so the user has somewhere to start.
    const hasTabs = Object.values(layout.nodes).some(
      (n) => n.kind === "leaf" && n.tabs.length > 0,
    );
    if (!hasTabs) openBrowser();
    void listenOpenSettings(() => {
      openSettings();
    });
    // Native menubar entries (View > Files / Search / Graph /
    // Assistant) route here. Each maps to the same overlay helpers
    // the floating pill uses, so menu and button trigger identical
    // behaviour.
    void listenMenuAction((action) => {
      switch (action) {
        case "files":
          openBrowser();
          break;
        case "search":
          searchPanel.open = true;
          break;
        case "graph":
          openGraph();
          break;
        case "assistant":
          openAssistant();
          break;
      }
    });
    const pending = readAndConsumeOpenFile();
    if (pending) void openInActivePane(pending);
    bootstrapped = true;
    // Tauri-only hook. No-op in the browser; the Tauri JS API
    // is dynamically imported only when we detect the runtime.
    // Note: window-level shortcuts (Cmd+N, Cmd+O) live on the
    // native menubar in the desktop app; they don't need a JS
    // handler.
    void setupCloseGuard();
    // Visibility-change resume hook. iOS / Android suspend the
    // WebView when the app goes to background, which severs every
    // open WebSocket. The watcher's exponential-backoff reconnect
    // gets stretched to seconds by the time the user returns, so
    // a manual nudge here lands the connection immediately. We
    // also re-fetch the drive info + tree because any filesystem
    // events that arrived during the suspend were dropped on the
    // floor (no live WS to deliver them). Debounced 300 ms so a
    // quick app-switch flicker doesn't fire the reconnect twice.
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

  /// App-level keyboard shortcuts. Lifted out of the deleted
  /// Toolbar component so they keep working without a top bar:
  ///
  ///   Cmd/Ctrl+,                   -> Settings (open)
  ///   Cmd/Ctrl+Shift+E             -> Files (toggle)
  ///   Cmd/Ctrl+H                   -> Assistant (toggle)
  ///   Cmd/Ctrl+Shift+[ / ]         -> previous / next tab (native)
  ///   Alt+Shift+[ / ]              -> previous / next tab (web)
  ///   Alt+1..9                     -> jump to tab N (web + native)
  ///   Cmd/Ctrl+1..9                -> jump to tab N (native only)
  ///
  /// Tab navigation has two chords because the obvious one
  /// (Cmd+Shift+[/]) is reserved by browsers (Safari / Chrome) for
  /// switching browser tabs. Native desktop keeps the natural chord;
  /// web (chan serve) uses Alt+Shift+[/] so the browser's own tab
  /// nav stays untouched. Both wrap around at the edges.
  ///
  /// Cmd+H on macOS is "Hide window" at the OS level. The browser
  /// catches it before the page sees it on web; preventDefault is
  /// best-effort and doesn't always win. Native (Tauri) intercepts
  /// the event at the webview boundary so the binding works there.
  /// Ctrl+H on Linux/Windows browsers shows history; same caveat.
  /// We register the binding regardless and let preventDefault do
  /// what it can; if a particular OS+browser swallows the chord, the
  /// user can still reach the assistant from the bottom pill.
  function onWindowKey(e: KeyboardEvent): void {
    if ((e.metaKey || e.ctrlKey) && !e.shiftKey && e.key === ",") {
      e.preventDefault();
      openSettings();
      return;
    }
    if (
      (e.metaKey || e.ctrlKey) &&
      e.shiftKey &&
      e.key.toLowerCase() === "e"
    ) {
      e.preventDefault();
      browserOverlay.open = !browserOverlay.open;
      if (browserOverlay.open) openBrowser();
      return;
    }
    if (
      (e.metaKey || e.ctrlKey) &&
      !e.shiftKey &&
      !e.altKey &&
      e.key.toLowerCase() === "h"
    ) {
      e.preventDefault();
      if (assistantOverlay.open) {
        assistantOverlay.open = false;
      } else {
        openAssistant();
      }
      return;
    }
    // Tab nav: pick the chord matching the platform. e.code rather
    // than e.key so the comparison stays stable when shift is held
    // (browsers report the shifted glyph "{"/"}" for e.key with
    // shift down on a US layout).
    const tabChord = nativeDesktop
      ? (e.metaKey || e.ctrlKey) && e.shiftKey && !e.altKey
      : e.altKey && e.shiftKey && !(e.metaKey || e.ctrlKey);
    if (tabChord && (e.code === "BracketLeft" || e.key === "[")) {
      e.preventDefault();
      selectPrevTabInActivePane();
      return;
    }
    if (tabChord && (e.code === "BracketRight" || e.key === "]")) {
      e.preventDefault();
      selectNextTabInActivePane();
      return;
    }
    // Alt+1..9 jump-to-tab works on both web and native (no
    // browser conflict; Alt+digit isn't a standard chord). We use
    // e.code === "Digit<N>" so the comparison survives modifiers
    // changing e.key to a glyph on non-US layouts.
    if (e.altKey && !e.shiftKey && !(e.metaKey || e.ctrlKey)) {
      const m = e.code.match(/^Digit([1-9])$/);
      if (m) {
        e.preventDefault();
        selectTabAtIndexInActivePane(Number(m[1]) - 1);
        return;
      }
    }
    if (!nativeDesktop) return;
    const meta = e.metaKey || e.ctrlKey;
    if (meta && !e.shiftKey && !e.altKey && /^[1-9]$/.test(e.key)) {
      e.preventDefault();
      selectTabAtIndexInActivePane(Number(e.key) - 1);
    }
  }
  onMount(() => document.addEventListener("keydown", onWindowKey));
  onDestroy(() => document.removeEventListener("keydown", onWindowKey));
</script>

<div class="app" class:mobile>
  <main>
    <Workspace />
    {#if ui.status}
      <div class="status">{ui.status}</div>
    {/if}
  </main>
</div>
{#if mobile}
  <!-- Mobile floating bar: nav buttons in reading mode, formatting
       buttons in editing mode (when the soft keyboard is up). The
       reading-mode nav set mirrors the desktop BottomPill so the
       four primary overlays (files / search / graph / assistant)
       are reachable identically across surfaces. -->
  <MobileFloatBar />
{:else}
  <!-- Window-level navigation pill. Web + native desktop share the
       same floating bottom bar so every overlay (files / search /
       graph / settings / assistant) is reachable from anywhere in
       the workspace. -->
  <BottomPill />
{/if}
<!-- Window-level overlays. Mounted once per window; the same set
     applies to web, native desktop, and mobile so every surface has
     the same set of overlays available. -->
<PromptModal />
<SearchPanel />
<InlineAssist />
<GraphPanel />
<SettingsPanel />
<FileBrowserOverlay />
<CloseGuardModal />
<!-- Disconnect overlay applies in every mode: any window is just
     as broken when the watcher dies, regardless of layout. -->
<DisconnectOverlay />

<style>
  /* Theme palette. Defaults to dark; [data-theme="light"] overrides.
     The neutrals mirror Apple's Notes / system grays so chan reads
     as "the markdown notes app" rather than "GitHub Dark with our
     stuff in it"; functional colors (link blue, accent green, warn
     amber, pane focus) are kept distinct from the brand yellow so
     they don't fight the assistant accent. */
  :global(:root) {
    --bg: #1c1c1e;
    --bg-card: #232325;
    --bg-elev: #2a2a2c;
    --border: #3a3a3c;
    --text: #f5f5f7;
    --text-secondary: #98989d;
    --text-heading: #f5f5f7;
    --link: #58a6ff;
    --code-bg: #232325;
    --btn-bg: #2a2a2c;
    --btn-border: #3a3a3c;
    --btn-hover: #98989d;
    --accent: #3fb950;
    --warn-text: #e3b341;
    /* "Unsaved buffer" color used by the dirty-dot in the file
       tree and tab strip. */
    --info-text: #4ade80;
    --hover-bg: rgba(255, 255, 255, 0.06);
    --selection-bg: rgba(56, 139, 253, 0.4);
    /* Subtle alternating row tint for tree views (file browser).
       Sits well below --hover-bg in opacity so hover/selection still
       wins over zebra striping at the cascade level. */
    --zebra-bg: rgba(255, 255, 255, 0.025);
    /* Right-side inspector panel background. Distinct from --bg-card
       so the inspector visually separates from the editor canvas
       (especially in light mode where --bg-card is close to --bg). */
    --inspector-bg: #232325;
    /* Resize-handle bar color. Brighter than --border so the 4px bar
       between inspector and canvas is findable on mobile (no hover
       cue) and in light mode (where --border can blend with bg). */
    --separator: #4a4a4d;
    --separator-hover: #98989d;
    --tab-active-bg: #1c1c1e;
    --tab-inactive-bg: #232325;
    --smart-bg: rgba(88, 166, 255, 0.18);
    --pane-focus: #388bfd;
    /* Brand accent for the assistant button (Notes-style yellow).
       Single source for the ensō tint across desktop toolbar,
       editor floating bar, and mobile float bar. Same value in
       light/dark; the icon is a stroke and reads on both. */
    --assistant-accent: #ffd60a;
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
    --code-bg: #f5f5f7;
    --btn-bg: #f2f2f4;
    --btn-border: #d1d1d6;
    --btn-hover: #6c6c70;
    --accent: #1a7f37;
    --warn-text: #9a6700;
    --info-text: #16a34a;
    --hover-bg: rgba(0, 0, 0, 0.05);
    --selection-bg: rgba(9, 105, 218, 0.18);
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
    --pane-focus: #b9c8e8;
    --assistant-accent: #d2a700;
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
  /* Mobile: respect iOS safe areas (notch, home indicator) and
     reserve room at the bottom of the editor canvas so the floating
     MobileFloatBar doesn't sit on top of the last lines of content.
     The Wysiwyg / Source editors read --mobile-bar-pad-bottom into
     their padding-bottom; on desktop the var is unset and the
     editor pads only its natural 1rem. */
  .app.mobile {
    padding-top: env(safe-area-inset-top);
    padding-bottom: env(safe-area-inset-bottom);
    box-sizing: border-box;
    --mobile-bar-pad-bottom: 56px;
  }
  main {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    position: relative;
  }
  .status {
    position: absolute;
    bottom: 0; left: 0; right: 0;
    background: var(--bg-card);
    color: var(--warn-text);
    padding: .25rem .5rem;
    font-size: 14px;
    border-top: 1px solid var(--border);
  }
</style>
