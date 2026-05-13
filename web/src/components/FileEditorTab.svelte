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
  import FindBar from "./FindBar.svelte";
  import Inspector from "./Inspector.svelte";
  import OutlineBody, { type Heading } from "./OutlineBody.svelte";
  import FileInfoBody from "./FileInfoBody.svelte";
  import StyleToolbar from "./StyleToolbar.svelte";
  import { setMode, type FileTab } from "../state/tabs.svelte";
  import WikiStatusBar from "./WikiStatusBar.svelte";

  import {
    fileOps,
    openAssistant,
    openGraphForFile,
    openSettings,
    searchPanel,
    openBrowser,
    openGraphAtNode,
    paneWidths,
    persistPaneWidths,
    revealAndSelect,
  } from "../state/store.svelte";
  import { openInActivePane } from "../state/tabs.svelte";
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

  let { tab }: { tab: FileTab } = $props();

  // Editor refs so the outline body can call scrollToHeading /
  // scrollToLine on whichever editor variant is showing, and so
  // the toolbar can call into the Wysiwyg formatting API.
  let wysiwygRef: Wysiwyg | undefined = $state();
  let sourceRef: Source | undefined = $state();

  /// "show info" disclosure inside the inspector. Per-tab session
  /// state; intentionally not persisted (would grow the tab schema
  /// for a small UI affordance and the disclosure starts collapsed
  /// every tab restore is fine).
  /// Active inspector tab. "outline" lists the file's headings (with
  /// click-to-scroll); "info" shows the same FileInfoBody surface the
  /// file browser inspector uses (tags, backlinks, refs). Per-tab
  /// state lives on the FileTab so each open file remembers its own
  /// choice across pane / tab focus changes.
  type InspectorTab = "outline" | "info";
  let inspectorTab = $state<InspectorTab>("outline");

  /// Read-only mode for this tab. The status bar's lamp toggle
  /// drives `tab.readMode` directly; an OS-level read-only file
  /// (no user-write bit) is reflected through `tab.fsWritable`
  /// and overrides the lamp so the user can't try to write.
  /// Per-tab so multi-pane layouts can mix read/write without
  /// a global signal fighting between panes.
  const readOnly = $derived(tab.readMode || !tab.fsWritable);

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

  /// Reveal the open file in the File Browser overlay. Expand every
  /// ancestor folder so the row is visible, set the browser
  /// selection to this file, then open the overlay. Mirrors the
  /// post-create/move "land next to the thing you just produced"
  /// flow in `revealAndSelect`.
  function revealInBrowser(): void {
    revealAndSelect(tab.path);
    openBrowser();
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

  /// Bubble positioning. We pin the bubble's top-left to the anchor's
  /// bottom-left and let the CSS clamp it to the viewport so a tab
  /// docked near the right edge still renders the menu on-screen.
  const menuStyle = $derived.by(() => {
    const a = tabMenu.anchor;
    if (!a) return "";
    return `top: ${Math.round(a.bottom + 4)}px; left: ${Math.round(a.left)}px;`;
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
    void fileOps.createFile("");
  }

  function doDuplicate(): void {
    closeTabMenu();
    void fileOps.duplicateFile(tab.path);
  }

  function doRename(): void {
    closeTabMenu();
    void fileOps.rename(tab.path, false);
  }

  function doToggleMode(): void {
    setMode(tab, tab.mode === "wysiwyg" ? "source" : "wysiwyg");
    closeTabMenu();
  }

  // ---- right-click context menu --------------------------------------
  // Re-uses the existing tab menu bubble (the same one that opens
  // from the tab dot). The bubble carries Duplicate / Rename /
  // mode-toggle / outline / style-toolbar plus our three new
  // actions (Reload / Call assistant / Show in graph). Anchored at
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

  function doOpenAssistant(): void {
    closeTabMenu();
    openAssistant();
  }

  function doOpenSettings(): void {
    closeTabMenu();
    openSettings();
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

  function doOpenGraph(): void {
    closeTabMenu();
    // "Show in Graph" from a file's menu scopes the graph to that
    // file (file:<path>), not the whole drive. Hashtags etc. still
    // route through openGraphAtNode at drive scope.
    openGraphForFile(tab.path);
  }

  function doToggleOutline(): void {
    tab.inspectorOpen = !tab.inspectorOpen;
    closeTabMenu();
  }

  function doToggleStyleToolbar(): void {
    tab.styleToolbarOpen = !tab.styleToolbarOpen;
    closeTabMenu();
  }
</script>

<svelte:window onkeydown={onMenuKeydown} onpointerdown={onDocPointerDown} />

<div class="editor-tab">
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
      style={menuStyle}
      onmousedown={(e) => e.stopPropagation()}
    >
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

      <!-- Action rows. Grouping mirrors the overlay menus: view
           toggles first, then content (reload), then file ops,
           then navigation. Page-width slider above stays in its
           own visual layer (already separated by the action-list
           top border). -->
      <div class="action-list">
        <button class="mbtn" onclick={doToggleMode}>
          <span class="mbtn-icon">{tab.mode === "wysiwyg" ? "</>" : "¶"}</span>
          <span class="mbtn-label">
            {tab.mode === "wysiwyg" ? "Show Source Code" : "Show Rendered"}
          </span>
        </button>
        <button class="mbtn" onclick={doToggleOutline} class:on={tab.inspectorOpen}>
          <span class="mbtn-icon">◫</span>
          <span class="mbtn-label">
            {tab.inspectorOpen ? "Hide Details" : "Show Details"}
          </span>
        </button>
        <button
          class="mbtn"
          onclick={doToggleStyleToolbar}
          class:on={tab.styleToolbarOpen}
        >
          <span class="mbtn-icon">Aa</span>
          <span class="mbtn-label">
            {tab.styleToolbarOpen ? "Hide Style Toolbar" : "Show Style Toolbar"}
          </span>
        </button>
        <div class="msep" role="separator"></div>
        <button class="mbtn" onclick={doNewFile}>
          <span class="mbtn-icon">＋</span>
          <span class="mbtn-label">New File</span>
        </button>
        <button class="mbtn" onclick={doDuplicate}>
          <span class="mbtn-icon">⎘</span>
          <span class="mbtn-label">Duplicate File</span>
        </button>
        <button class="mbtn" onclick={doRename}>
          <span class="mbtn-icon">✎</span>
          <span class="mbtn-label">Rename File</span>
        </button>
        <button class="mbtn" onclick={doReload}>
          <span class="mbtn-icon">↻</span>
          <span class="mbtn-label">Reload from Disk</span>
        </button>
        <div class="msep" role="separator"></div>
        <button class="mbtn" onclick={revealInBrowser}>
          <span class="mbtn-icon">📄</span>
          <span class="mbtn-label">Show in File Browser</span>
        </button>
        <button class="mbtn" onclick={doOpenSearch}>
          <span class="mbtn-icon">⌕</span>
          <span class="mbtn-label">Search</span>
        </button>
        <button class="mbtn" onclick={doOpenGraph}>
          <span class="mbtn-icon">⛬</span>
          <span class="mbtn-label">Show in Graph</span>
        </button>
        <button class="mbtn" onclick={doOpenAssistant}>
          <span class="mbtn-icon">✦</span>
          <span class="mbtn-label">Call Assistant</span>
        </button>
        <div class="msep" role="separator"></div>
        <button class="mbtn" onclick={doOpenSettings}>
          <span class="mbtn-icon">⚙</span>
          <span class="mbtn-label">Settings</span>
        </button>
      </div>
    </div>
  {/if}

  {#if tab.error}
    <div class="editor-toolbar">
      <span class="error">{tab.error}</span>
    </div>
  {/if}
  {#if tab.loading}
    <div class="placeholder">loading…</div>
  {:else}
    <div class="editor-inspector-row">
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
            initialCaret={tab.caret ?? null}
            onCaretChange={(from, to) => (tab.caret = { from, to })}
            onSelectionChange={() => (selVer = selVer + 1)}
            wikiPickerPrefix={tab.repoRoot}
            currentPath={tab.path}
            onWikiClick={(args) => {
              // Navigation: click on a wikilink pill opens the
              // target in the active pane (or a new pane on Cmd /
              // Ctrl click).
              void openInActivePane(args.target);
            }}
            onTagClick={(name) => openGraphAtNode(`#${name}`)}
          />
          {#if tab.styleToolbarOpen}
            <StyleToolbar
              wysiwyg={wysiwygRef}
              selVer={selVer}
              disabled={readOnly}
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
            initialCaret={tab.caret ?? null}
            onCaretChange={(from, to) => (tab.caret = { from, to })}
          />
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
          bind:width={paneWidths.inspector}
          onResize={persistPaneWidths}
          onClose={() => (tab.inspectorOpen = false)}
        >
          <!-- Single toggle button instead of a tab strip. Reads as
               "you're on Outline; click to swap to File info" (and
               vice-versa). The swap glyph hints at the toggle action;
               the label always names the *current* view so the user
               sees what they're looking at, not what's behind it. -->
          <button
            class="inspector-toggle"
            type="button"
            aria-label={inspectorTab === "outline"
              ? "Switch to file info"
              : "Switch to outline"}
            onclick={() =>
              (inspectorTab = inspectorTab === "outline" ? "info" : "outline")}
          >
            <span class="inspector-toggle-label">
              {inspectorTab === "outline" ? "Outline" : "File info"}
            </span>
            <span class="inspector-toggle-glyph" aria-hidden="true">⇄</span>
          </button>
          <div class="inspector-body">
            {#if inspectorTab === "outline"}
              <OutlineBody content={tab.content} onSelect={jumpTo} />
            {:else}
              <FileInfoBody
                path={tab.path}
                showRefs
                onNavigate={(p) => void openInActivePane(p)}
              />
            {/if}
          </div>
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
    color: var(--text-secondary);
    flex-shrink: 0;
  }
  .mbtn-label { flex: 1; }
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
  .placeholder {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-secondary);
    font-style: italic;
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
  /* Single-button toggle across the top of the inspector body.
     Names the *current* view ("Outline" or "File info") so the
     user sees what they're looking at; the swap glyph hints that
     clicking flips to the other view. */
  .inspector-toggle {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    width: 100%;
    flex-shrink: 0;
    background: none;
    border: 0;
    border-bottom: 1px solid var(--separator);
    color: var(--text);
    cursor: pointer;
    font: inherit;
    font-size: 13px;
    padding: 0.45rem 0.6rem;
    text-align: left;
  }
  .inspector-toggle:hover {
    background: var(--hover-bg);
  }
  .inspector-toggle-label {
    font-weight: 600;
  }
  .inspector-toggle-glyph {
    color: var(--text-secondary);
    font-size: 14px;
  }
  .inspector-body {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
  }
</style>
