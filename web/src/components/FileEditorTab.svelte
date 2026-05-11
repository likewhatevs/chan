<script lang="ts">
  // Body of a file tab. The previous top "tab-bar" (Aa, page-width,
  // formatting group, reveal-in-browser, mode toggle, outline toggle)
  // has been collapsed into a single popover anchored to the tab
  // strip's ⋯ button. This file now hosts the bubble and exposes the
  // formatting / state hooks it needs from Wysiwyg.

  import Wysiwyg, { type BlockKind } from "../editor/Wysiwyg.svelte";
  import Source from "../editor/Source.svelte";
  import Inspector from "./Inspector.svelte";
  import OutlineBody, { type Heading } from "./OutlineBody.svelte";
  import FileInfoBody from "./FileInfoBody.svelte";
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
    PAGE_WIDTH_MAX,
    PAGE_WIDTH_MIN,
    PAGE_WIDTH_STEP,
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

  // Bumped on every selection / doc change in the WYSIWYG editor
  // so the active-mark / current-block derivations re-run. The
  // value itself doesn't matter; the dependency does.
  let selVer = $state(0);

  function jumpTo(h: Heading): void {
    if (tab.mode === "wysiwyg") wysiwygRef?.scrollToHeading(h.index);
    else sourceRef?.scrollToLine(h.line);
  }

  // Reactive accessors; reading `selVer` ties them to the editor's
  // selection updates so the toolbar buttons reflect cursor moves.
  // The void cast on `selVer` makes the dependency explicit without
  // tripping the unused-expression lint.
  const isBold = $derived.by(() => {
    void selVer;
    return wysiwygRef?.isActive("bold") ?? false;
  });
  const isItalic = $derived.by(() => {
    void selVer;
    return wysiwygRef?.isActive("italic") ?? false;
  });
  const isStrike = $derived.by(() => {
    void selVer;
    return wysiwygRef?.isActive("strike") ?? false;
  });
  const isInlineCode = $derived.by(() => {
    void selVer;
    return wysiwygRef?.isActive("code") ?? false;
  });
  const isBulletList = $derived.by(() => {
    void selVer;
    return wysiwygRef?.isActive("bulletList") ?? false;
  });
  const isOrderedList = $derived.by(() => {
    void selVer;
    return wysiwygRef?.isActive("orderedList") ?? false;
  });
  const isTaskList = $derived.by(() => {
    void selVer;
    return wysiwygRef?.isActive("taskList") ?? false;
  });
  const isLink = $derived.by(() => {
    void selVer;
    return wysiwygRef?.isActive("link") ?? false;
  });
  const blockKind = $derived.by<BlockKind>(() => {
    void selVer;
    return wysiwygRef?.currentBlockKind() ?? "normal";
  });

  function onBlockKindChange(e: Event): void {
    const v = (e.currentTarget as HTMLSelectElement).value as BlockKind;
    wysiwygRef?.setBlockKind(v);
  }

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

  /// Dismiss when the click lands outside the bubble AND outside the
  /// trigger button (the trigger's own click handler already toggles
  /// the state, so we ignore clicks that bubble up from it).
  function onDocPointerDown(e: PointerEvent): void {
    if (!menuOpen) return;
    const t = e.target as Node | null;
    if (!t) return;
    const bubble = document.querySelector(".tab-menu-bubble");
    if (bubble && bubble.contains(t)) return;
    // Any click on a tab's ⋯ trigger is handled by Pane.svelte.
    const trigger = (t as Element).closest?.(".tb-toggle");
    if (trigger) return;
    closeTabMenu();
  }

  function onPageWidthSlider(e: Event): void {
    const v = Number((e.currentTarget as HTMLInputElement).value);
    if (v >= PAGE_WIDTH_MAX) setPageWidth(null);
    else setPageWidth(v);
  }

  function doDuplicate(): void {
    closeTabMenu();
    void fileOps.duplicateFile(tab.path);
  }

  function doToggleMode(): void {
    setMode(tab, tab.mode === "wysiwyg" ? "source" : "wysiwyg");
    closeTabMenu();
  }

  function doToggleOutline(): void {
    tab.inspectorOpen = !tab.inspectorOpen;
    closeTabMenu();
  }
</script>

<svelte:window onkeydown={onMenuKeydown} onpointerdown={onDocPointerDown} />

<div class="editor-tab">
  {#if menuOpen}
    <!-- Tab menu bubble. Anchored to the ⋯ button in the pane's
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
      <!-- Formatting toolbar row. Only enabled in WYSIWYG mode and
           when the tab is writable; in source mode or read-only the
           row dims to hint that the commands wouldn't apply. The
           onmousedown=preventDefault dance keeps the editor focused
           while clicking. -->
      <div
        class="fmt-row"
        role="toolbar"
        aria-label="Formatting"
        class:disabled={tab.mode !== "wysiwyg" || readOnly}
      >
        <select
          class="block-kind"
          value={blockKind}
          onchange={onBlockKindChange}
          onmousedown={(e) => e.stopPropagation()}
          disabled={tab.mode !== "wysiwyg" || readOnly}
          title="block style"
        >
          <option value="h1">h1</option>
          <option value="h2">h2</option>
          <option value="h3">h3</option>
          <option value="normal">text</option>
          <option value="code">code</option>
          <option value="quote">quote</option>
        </select>
        <button
          class="fbtn"
          class:on={isBold}
          title="bold (Cmd/Ctrl+B)"
          disabled={tab.mode !== "wysiwyg" || readOnly}
          onmousedown={(e) => e.preventDefault()}
          onclick={() => wysiwygRef?.toggleBold()}
        ><b>B</b></button>
        <button
          class="fbtn"
          class:on={isItalic}
          title="italic (Cmd/Ctrl+I)"
          disabled={tab.mode !== "wysiwyg" || readOnly}
          onmousedown={(e) => e.preventDefault()}
          onclick={() => wysiwygRef?.toggleItalic()}
        ><i>I</i></button>
        <button
          class="fbtn"
          class:on={isStrike}
          title="strikethrough (Cmd/Ctrl+Shift+S)"
          disabled={tab.mode !== "wysiwyg" || readOnly}
          onmousedown={(e) => e.preventDefault()}
          onclick={() => wysiwygRef?.toggleStrike()}
        ><s>S</s></button>
        <button
          class="fbtn"
          class:on={isInlineCode}
          title="inline code (Cmd/Ctrl+E)"
          disabled={tab.mode !== "wysiwyg" || readOnly}
          onmousedown={(e) => e.preventDefault()}
          onclick={() => wysiwygRef?.toggleInlineCode()}
        ><code>{`<>`}</code></button>
        <button
          class="fbtn"
          class:on={isLink}
          title="link"
          aria-label="toggle link"
          disabled={tab.mode !== "wysiwyg" || readOnly}
          onmousedown={(e) => e.preventDefault()}
          onclick={() => wysiwygRef?.toggleLink()}
        >🔗</button>
        <button
          class="fbtn"
          class:on={isBulletList}
          title="bullet list"
          aria-label="bullet list"
          disabled={tab.mode !== "wysiwyg" || readOnly}
          onmousedown={(e) => e.preventDefault()}
          onclick={() => wysiwygRef?.toggleBulletList()}
        >•</button>
        <button
          class="fbtn"
          class:on={isOrderedList}
          title="ordered list"
          aria-label="ordered list"
          disabled={tab.mode !== "wysiwyg" || readOnly}
          onmousedown={(e) => e.preventDefault()}
          onclick={() => wysiwygRef?.toggleOrderedList()}
        >1.</button>
        <button
          class="fbtn"
          class:on={isTaskList}
          title="task list"
          aria-label="task list"
          disabled={tab.mode !== "wysiwyg" || readOnly}
          onmousedown={(e) => e.preventDefault()}
          onclick={() => wysiwygRef?.toggleTaskList()}
        >☐</button>
        <button
          class="fbtn"
          title="horizontal rule (insert ---)"
          aria-label="insert horizontal rule"
          disabled={tab.mode !== "wysiwyg" || readOnly}
          onmousedown={(e) => e.preventDefault()}
          onclick={() => wysiwygRef?.insertHorizontalRule()}
        >―</button>
        <button
          class="fbtn"
          title="insert image"
          aria-label="insert image"
          disabled={tab.mode !== "wysiwyg" || readOnly}
          onmousedown={(e) => e.preventDefault()}
          onclick={() => wysiwygRef?.insertImage()}
        >🖼</button>
      </div>

      <!-- Zoom row: page-width as a real slider. The range hits
           PAGE_WIDTH_MAX + 1 step at the top so users can land in
           the "100% / unbounded" sentinel (stored as null) by
           dragging all the way right. -->
      <div class="zoom-row">
        <span class="zoom-label">zoom</span>
        <input
          class="zoom-slider"
          type="range"
          min={PAGE_WIDTH_MIN}
          max={PAGE_WIDTH_MAX}
          step={PAGE_WIDTH_STEP}
          value={pageWidth.value ?? PAGE_WIDTH_MAX}
          oninput={onPageWidthSlider}
          onmousedown={(e) => e.stopPropagation()}
          aria-label="page width"
        />
        <span class="zoom-value">{pageWidth.value == null
          ? "100%"
          : `${Math.round((pageWidth.value / PAGE_WIDTH_MAX) * 100)}%`}</span>
      </div>

      <!-- Action rows. Keep separator between formatting/zoom and
           file-level actions so the affordance reads as two layers. -->
      <div class="action-list">
        <button class="mbtn" onclick={doDuplicate}>
          <span class="mbtn-icon">⎘</span>
          <span class="mbtn-label">Duplicate File</span>
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
        <Wysiwyg
          bind:this={wysiwygRef}
          bind:value={tab.content}
          readonly={readOnly}
          onSelectionChange={() => (selVer = selVer + 1)}
          wikiPickerPrefix={tab.repoRoot}
          currentPath={tab.path}
        />
      {:else}
        <Source bind:this={sourceRef} bind:value={tab.content} />
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
     icon column so the buttons read as a tidy list. */
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
    /* Anti-overflow on narrow viewports. */
    transform: translateX(0);
  }
  .fmt-row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 2px;
    padding: 2px 4px 6px;
    border-bottom: 1px solid var(--separator);
  }
  .fmt-row.disabled { opacity: 0.55; }
  .block-kind {
    background: transparent;
    color: var(--text);
    border: 1px solid var(--btn-border);
    border-radius: 3px;
    padding: 0 4px;
    margin-right: 2px;
    font: inherit;
    font-size: 12px;
    height: 22px;
  }
  .fbtn {
    min-width: 24px;
    height: 22px;
    text-align: center;
    background: transparent;
    border: 1px solid transparent;
    border-radius: 3px;
    color: var(--text);
    cursor: pointer;
    font: inherit;
    font-size: 13px;
    padding: 0 4px;
    line-height: 20px;
  }
  .fbtn:hover:not(:disabled) {
    background: var(--hover-bg);
    border-color: var(--btn-border);
  }
  .fbtn.on {
    background: var(--hover-bg);
    border-color: var(--btn-hover);
  }
  .fbtn:disabled { cursor: default; opacity: 0.55; }
  .fbtn b, .fbtn i, .fbtn s, .fbtn code { font-size: 13px; }
  .fbtn code { font-family: ui-monospace, monospace; }

  .zoom-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 4px;
    border-bottom: 1px solid var(--separator);
  }
  .zoom-label {
    color: var(--text-secondary);
    font-size: 12px;
    min-width: 36px;
  }
  .zoom-slider {
    flex: 1;
    accent-color: var(--btn-hover);
  }
  .zoom-value {
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
