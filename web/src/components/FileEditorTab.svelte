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
    openBrowser,
    paneWidths,
    persistPaneWidths,
    revealAndSelect,
  } from "../state/store.svelte";
  import {
    PAGE_WIDTH_MAX_PCT,
    PAGE_WIDTH_MIN_PCT,
    PAGE_WIDTH_STEP_PCT,
    pageWidth,
    setPageWidth,
  } from "../state/pageWidth.svelte";
  import { tabMenu, closeTabMenu } from "../state/tabMenu.svelte";

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
  let showInfo = $state(false);

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
        <span class="page-width-label">page width</span>
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

      <!-- Action rows. Keep separator between page-width and
           file-level actions so the affordance reads as two layers. -->
      <div class="action-list">
        <button class="mbtn" onclick={doDuplicate}>
          <span class="mbtn-icon">⎘</span>
          <span class="mbtn-label">Duplicate File</span>
        </button>
        <button class="mbtn" onclick={doRename}>
          <span class="mbtn-icon">✎</span>
          <span class="mbtn-label">Rename File</span>
        </button>
        <button class="mbtn" onclick={revealInBrowser}>
          <span class="mbtn-icon">📄</span>
          <span class="mbtn-label">Show in File Browser</span>
        </button>
        <button class="mbtn" onclick={doToggleMode}>
          <span class="mbtn-icon">{tab.mode === "wysiwyg" ? "</>" : "¶"}</span>
          <span class="mbtn-label">
            {tab.mode === "wysiwyg" ? "Show Source Code" : "Show Rendered"}
          </span>
        </button>
        <button class="mbtn" onclick={doToggleOutline} class:on={tab.inspectorOpen}>
          <span class="mbtn-icon">◫</span>
          <span class="mbtn-label">
            {tab.inspectorOpen ? "Hide Outline" : "Show Outline"}
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
        >
          <Wysiwyg
            bind:this={wysiwygRef}
            bind:value={tab.content}
            readonly={readOnly}
            onSelectionChange={() => (selVer = selVer + 1)}
            wikiPickerPrefix={tab.repoRoot}
            currentPath={tab.path}
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
        <div class="editor-host">
          <Source bind:this={sourceRef} bind:value={tab.content} />
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
          title="Outline"
          bind:width={paneWidths.inspector}
          onResize={persistPaneWidths}
        >
          <div class="outline-slot">
            <OutlineBody content={tab.content} onSelect={jumpTo} />
          </div>
          <button
            class="info-disclosure"
            onclick={() => (showInfo = !showInfo)}
            aria-expanded={showInfo}
          >
            <span class="caret">{showInfo ? "▾" : "▸"}</span>
            {showInfo ? "hide info" : "show info"}
          </button>
          {#if showInfo}
            <FileInfoBody path={tab.path} />
          {/if}
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
  /* Outline body sits at the top of the inspector and grows; the
     info disclosure pins to the bottom so the file metadata never
     pushes the heading list off-screen. */
  .outline-slot {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
  }
  .info-disclosure {
    background: none;
    border: 0;
    border-top: 1px solid var(--separator);
    color: var(--text-secondary);
    cursor: pointer;
    font: inherit;
    font-size: 13px;
    text-align: left;
    padding: 0.4rem 0.6rem;
    display: flex;
    align-items: center;
    gap: 4px;
    flex-shrink: 0;
  }
  .info-disclosure:hover { color: var(--text); }
  .info-disclosure .caret {
    width: 10px;
    display: inline-block;
    text-align: center;
  }
</style>
