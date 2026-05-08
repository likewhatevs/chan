<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import BottomPill from "./components/BottomPill.svelte";
  import ConflictModal from "./components/ConflictModal.svelte";
  import DisconnectOverlay from "./components/DisconnectOverlay.svelte";
  import FileBrowserOverlay from "./components/FileBrowserOverlay.svelte";
  import GraphPanel from "./components/GraphPanel.svelte";
  import InlineAssist from "./components/InlineAssist.svelte";
  import PromptModal from "./components/PromptModal.svelte";
  import SearchPanel from "./components/SearchPanel.svelte";
  import SettingsPanel from "./components/SettingsPanel.svelte";
  import Workspace from "./components/Workspace.svelte";
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
    isWindowFullyReadOnly,
    layout,
    openInActivePane,
    scheduleAutosave,
    selectNextTabInActivePane,
    selectPrevTabInActivePane,
    selectTabAtIndexInActivePane,
  } from "./state/tabs.svelte";
  import { applyFontPrefs, DEFAULT_FONT_PREFS } from "./state/fontPrefs";
  import {
    applyInitialPageWidth,
    watchPageWidth,
  } from "./state/pageWidth.svelte";
  import { installIdleTracker, setReadMode } from "./state/idle.svelte";
  import { loadShared } from "./api/wasm";

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

  // Mirror overlay open-state + per-overlay knobs into the session
  // payload so close-and-quit restores everything on the next
  // launch. Same debounce mechanism as the layout effect; the
  // helpers in store.svelte.ts already coalesce.
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

  onMount(async () => {
    // Apply persisted theme + default fonts to the document root
    // immediately, before any component renders, to avoid a flash.
    applyInitialTheme();
    applyFontPrefs(DEFAULT_FONT_PREFS);
    applyInitialPageWidth();
    // While in "system" mode, follow OS-level theme changes live.
    // The listener stays alive for the whole app's lifetime.
    watchSystemTheme();
    // Cross-window sync of the page-width setting via the storage event.
    watchPageWidth();
    // Idle tracker: after 2.5s without scroll/click/keypress, the
    // floating pills fade. Any input flips them back on.
    installIdleTracker();
    await Promise.all([loadShared(), bootstrap()]);
    // First-launch experience: if the URL hash didn't restore any
    // tabs, the pane stays empty and the file browser overlay
    // auto-opens so the user has somewhere to start.
    const hasTabs = Object.values(layout.nodes).some(
      (n) => n.kind === "leaf" && n.tabs.length > 0,
    );
    if (!hasTabs) openBrowser();
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

  /// App-level keyboard shortcuts:
  ///
  ///   Cmd/Ctrl+,           -> Settings (open)
  ///   Cmd/Ctrl+Shift+O     -> Files (toggle)
  ///   Cmd/Ctrl+P           -> Assistant (toggle)
  ///   Cmd/Ctrl+K           -> Search (toggle)
  ///   Cmd/Ctrl+Shift+G     -> Graph (toggle)
  ///   Alt+Shift+[ / ]      -> previous / next tab
  ///   Ctrl+Alt+1..9        -> jump to tab N
  ///
  /// Mac note: bare-Alt chords are off-limits for letters / digits
  /// because Option is a dead-key for special characters there
  /// (Alt+G prints `©`, Alt+L prints `¬`, Alt+1 prints `¡`, etc.).
  /// All letter / digit chords therefore use Cmd/Ctrl-based combos
  /// or Ctrl+Alt; Alt+Shift+[/] is kept only because we match by
  /// `e.code` (which is layout-independent) and preventDefault
  /// suppresses the typed `«` / `»` before they reach the editor.
  ///
  /// Chord choices avoid browser-reserved combinations where
  /// preventDefault can't reliably win:
  ///   - Cmd+O (system file picker) -> Cmd+Shift+O instead.
  ///   - Cmd+P (browser print) -> intercepted; tolerable because
  ///     the assistant is a primary-use surface.
  ///   - Cmd+Shift+P would clash with Firefox's private window
  ///     (OS-level), so search uses Cmd+K (palette convention).
  ///   - Cmd+G is browser find-next; Cmd+Shift+G is browser find-
  ///     previous on Chrome / Safari. We override the latter for
  ///     the graph because find-prev is the lower-traffic action
  ///     and the chord is mnemonic.
  ///   - Cmd+Shift+[ / ] is the browser's own tab nav, so we use
  ///     Alt+Shift+[ / ] for in-app tab nav. e.code rather than
  ///     e.key so the comparison stays stable when shift is held
  ///     (browsers report "{"/"}" for e.key on US layout AND `«` /
  ///     `»` on Mac when Option is held).
  ///   - Cmd+1..9 is browser tab switching (claimed; preventDefault
  ///     is unreliable for OS-level tab switching). We use
  ///     Ctrl+Alt+1..9 instead — distinct from Cmd-based, and
  ///     unclaimed by mainstream browsers.
  function onWindowKey(e: KeyboardEvent): void {
    const meta = e.metaKey || e.ctrlKey;
    if (meta && !e.shiftKey && !e.altKey && e.key === ",") {
      e.preventDefault();
      openSettings();
      return;
    }
    if (meta && e.shiftKey && !e.altKey && e.code === "KeyO") {
      e.preventDefault();
      browserOverlay.open = !browserOverlay.open;
      if (browserOverlay.open) openBrowser();
      return;
    }
    if (meta && !e.shiftKey && !e.altKey && e.code === "KeyP") {
      // Assistant master switch: when disabled in Settings, the
      // chord falls through and the user gets no visible response,
      // matching the hidden bottom-pill button.
      if (!(drive.info?.preferences.assistant.enabled ?? true)) return;
      e.preventDefault();
      if (assistantOverlay.open) {
        assistantOverlay.open = false;
      } else {
        openAssistant();
      }
      return;
    }
    if (meta && !e.shiftKey && !e.altKey && e.code === "KeyK") {
      e.preventDefault();
      searchPanel.open = !searchPanel.open;
      return;
    }
    if (meta && e.shiftKey && !e.altKey && e.code === "KeyG") {
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
      }
    }
  }
  onMount(() => document.addEventListener("keydown", onWindowKey));
  onDestroy(() => document.removeEventListener("keydown", onWindowKey));
</script>

<div class="app">
  <main>
    <Workspace />
    {#if ui.status}
      <div class="status">{ui.status}</div>
    {/if}
  </main>
</div>
<!-- Floating navigation pill: every overlay (files / search /
     graph / settings / assistant) is reachable from anywhere in
     the workspace. -->
<BottomPill />
<!-- Window-level overlays. Mounted once. -->
<PromptModal />
<SearchPanel />
<InlineAssist />
<GraphPanel />
<SettingsPanel />
<FileBrowserOverlay />
<!-- CAS conflict prompt: surfaces when a save returns 409. Mounted
     once per window so any pane can trigger it; the dialog itself
     keys off `conflictDialog.tabId`. -->
<ConflictModal />
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
    /* Brand accent for the assistant button (Notes-style yellow).
       Single source for the ensō tint. Same value in light/dark;
       the icon is a stroke and reads on both. */
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
    /* Light-mode bubble tints: assistant on a warmer gray so it
       sits clearly above the white panel; user on Apple-Messages
       blue (#dbeafe-ish, slightly more saturated) so the two
       sides are distinguishable at a glance. */
    --assistant-bubble-bg: #ececef;
    --assistant-user-bubble-bg: #cfe1fb;
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
