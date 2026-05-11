<script lang="ts">
  // Body of a file tab. Lifted out of Pane.svelte so each tab kind
  // owns its own top bar; the pane action strip now carries only
  // pane-scoped buttons (split + close).
  //
  // Top bar layout:
  //   left  : `Aa` toggle, page-width adjuster, and (when the toggle
  //           is on AND we're in WYSIWYG mode) inline formatting
  //           controls — block-kind dropdown, B / I / S / inline-code,
  //           link, lists, task list, horizontal rule.
  //   right : wysiwyg/source toggle, inspector toggle.

  import Wysiwyg, { type BlockKind } from "../editor/Wysiwyg.svelte";
  import Source from "../editor/Source.svelte";
  import Inspector from "./Inspector.svelte";
  import OutlineBody, { type Heading } from "./OutlineBody.svelte";
  import FileInfoBody from "./FileInfoBody.svelte";
  import { setMode, type FileTab } from "../state/tabs.svelte";
  import WikiStatusBar from "./WikiStatusBar.svelte";

  import {
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
  }

  // In-tab find was removed; the browser's native ⌘F applies. The
  // editor's selectable text (WYSIWYG and source) is plain DOM, so
  // browser find lights up matches the way users already expect.

  /// Vertical click-drag on the page-width button. Up = wider, down
  /// = narrower. Coexists with the existing wheel + arrow-key
  /// affordances; the drag is the discoverable one for new users.
  ///
  /// `dragMoved` flips true once the pointer travels enough to count
  /// as a drag, and is consulted by the click handler so a drag
  /// release doesn't also fire an implicit click.
  const PAGE_WIDTH_DRAG_PX_PER_PX = 4;
  const DRAG_THRESHOLD_PX = 3;
  let pwDragStartY = 0;
  let pwDragStart = 0;
  let pwDragging = $state(false);
  let pwDragMoved = false;

  function pwPointerDown(e: PointerEvent): void {
    // Only react to the primary mouse / pen / touch contact. Wheel
    // and the existing keyboard handlers cover the other paths.
    if (e.button !== 0) return;
    const target = e.currentTarget as HTMLButtonElement;
    target.setPointerCapture(e.pointerId);
    pwDragging = true;
    pwDragMoved = false;
    pwDragStartY = e.clientY;
    pwDragStart = pageWidth.value ?? PAGE_WIDTH_MAX;
    document.body.classList.add("page-width-dragging");
  }

  function pwPointerMove(e: PointerEvent): void {
    if (!pwDragging) return;
    const dy = pwDragStartY - e.clientY;
    if (!pwDragMoved && Math.abs(dy) >= DRAG_THRESHOLD_PX) pwDragMoved = true;
    if (!pwDragMoved) return;
    const raw = pwDragStart + dy * PAGE_WIDTH_DRAG_PX_PER_PX;
    const snapped = Math.round(raw / PAGE_WIDTH_STEP) * PAGE_WIDTH_STEP;
    if (snapped >= PAGE_WIDTH_MAX) {
      setPageWidth(null);
    } else if (snapped < PAGE_WIDTH_MIN) {
      setPageWidth(PAGE_WIDTH_MIN);
    } else {
      setPageWidth(snapped);
    }
  }

  function pwPointerUp(e: PointerEvent): void {
    if (!pwDragging) return;
    const target = e.currentTarget as HTMLButtonElement;
    if (target.hasPointerCapture(e.pointerId)) {
      target.releasePointerCapture(e.pointerId);
    }
    pwDragging = false;
    document.body.classList.remove("page-width-dragging");
  }

  function pwClick(e: MouseEvent): void {
    // Suppress the implicit click that follows a drag release. A
    // bare click (no drag motion) falls through and stays a no-op,
    // matching the previous wheel / keys-only design.
    if (pwDragMoved) {
      e.preventDefault();
      e.stopPropagation();
      pwDragMoved = false;
    }
  }
</script>

<div class="editor-tab">
  {#if tab.toolbarOpen}
  <div class="tab-bar">
    <span class="left">
      <!-- `Aa` toggle: reveal the inline formatting controls. Always
           rendered so the affordance is discoverable, but disabled in
           source mode and on read-only tabs (where the formatting
           commands wouldn't apply anyway). The state is per-tab and
           ephemeral; session restore starts every tab collapsed. -->
      <button
        class="hbtn aa-toggle"
        class:on={tab.formattingBarOpen}
        title={tab.formattingBarOpen ? "hide formatting" : "show formatting"}
        aria-label="toggle formatting toolbar"
        aria-pressed={tab.formattingBarOpen}
        disabled={tab.mode !== "wysiwyg" || readOnly}
        onclick={() => (tab.formattingBarOpen = !tab.formattingBarOpen)}
      >Aa</button>
      <!-- Page-width adjuster. Vertical click-drag (up = wider,
           down = narrower) is the discoverable affordance; hover +
           wheel and focus + Up/Down/Shift+Up/Down still work for
           power users. Above the cap range falls off into `null`
           (unbounded / full width). -->
      <button
        type="button"
        class="hbtn page-width-btn"
        class:dragging={pwDragging}
        title="page width — drag, scroll, or arrow keys; Shift = bigger step"
        aria-label="page width"
        onpointerdown={pwPointerDown}
        onpointermove={pwPointerMove}
        onpointerup={pwPointerUp}
        onpointercancel={pwPointerUp}
        onclick={pwClick}
        onwheel={(e) => {
          e.preventDefault();
          // Wheel up (deltaY < 0) widens toward `full`; wheel down narrows.
          const dir = e.deltaY > 0 ? -1 : 1;
          const cur = pageWidth.value ?? PAGE_WIDTH_MAX;
          const next = cur + dir * PAGE_WIDTH_STEP;
          setPageWidth(next >= PAGE_WIDTH_MAX ? null : next);
        }}
        onkeydown={(e) => {
          if (e.key !== "ArrowUp" && e.key !== "ArrowDown") return;
          e.preventDefault();
          const dir = e.key === "ArrowUp" ? 1 : -1;
          const step = e.shiftKey ? PAGE_WIDTH_STEP * 5 : PAGE_WIDTH_STEP;
          const cur = pageWidth.value ?? PAGE_WIDTH_MAX;
          const next = cur + dir * step;
          setPageWidth(next >= PAGE_WIDTH_MAX ? null : next);
        }}
      >{pageWidth.value == null
          ? "100%"
          : `${Math.round((pageWidth.value / PAGE_WIDTH_MAX) * 100)}%`}</button>

      {#if tab.formattingBarOpen && !tab.loading && tab.mode === "wysiwyg" && !readOnly}
        <!-- Inline formatting controls. Same buttons that used to
             live in the floating `.fmt-bar` pill; now collapsed into
             the tab-bar so the editor canvas is uncluttered. The
             onmousedown=preventDefault dance keeps the editor focused
             while clicking, which avoids ProseMirror re-scrolling the
             selection into view on every click. -->
        <span class="fmt-group" role="toolbar" aria-label="Formatting">
          <select
            class="block-kind"
            value={blockKind}
            onchange={onBlockKindChange}
            onmousedown={(e) => e.stopPropagation()}
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
            onmousedown={(e) => e.preventDefault()}
            onclick={() => wysiwygRef?.toggleBold()}
          ><b>B</b></button>
          <button
            class="fbtn"
            class:on={isItalic}
            title="italic (Cmd/Ctrl+I)"
            onmousedown={(e) => e.preventDefault()}
            onclick={() => wysiwygRef?.toggleItalic()}
          ><i>I</i></button>
          <button
            class="fbtn"
            class:on={isStrike}
            title="strikethrough (Cmd/Ctrl+Shift+S)"
            onmousedown={(e) => e.preventDefault()}
            onclick={() => wysiwygRef?.toggleStrike()}
          ><s>S</s></button>
          <button
            class="fbtn"
            class:on={isInlineCode}
            title="inline code (Cmd/Ctrl+E)"
            onmousedown={(e) => e.preventDefault()}
            onclick={() => wysiwygRef?.toggleInlineCode()}
          ><code>{`<>`}</code></button>
          <button
            class="fbtn"
            class:on={isLink}
            title="link"
            aria-label="toggle link"
            onmousedown={(e) => e.preventDefault()}
            onclick={() => wysiwygRef?.toggleLink()}
          >🔗</button>
          <button
            class="fbtn"
            class:on={isBulletList}
            title="bullet list"
            aria-label="bullet list"
            onmousedown={(e) => e.preventDefault()}
            onclick={() => wysiwygRef?.toggleBulletList()}
          >•</button>
          <button
            class="fbtn"
            class:on={isOrderedList}
            title="ordered list"
            aria-label="ordered list"
            onmousedown={(e) => e.preventDefault()}
            onclick={() => wysiwygRef?.toggleOrderedList()}
          >1.</button>
          <button
            class="fbtn"
            class:on={isTaskList}
            title="task list"
            aria-label="task list"
            onmousedown={(e) => e.preventDefault()}
            onclick={() => wysiwygRef?.toggleTaskList()}
          >☐</button>
          <button
            class="fbtn"
            title="horizontal rule (insert ---)"
            aria-label="insert horizontal rule"
            onmousedown={(e) => e.preventDefault()}
            onclick={() => wysiwygRef?.insertHorizontalRule()}
          >―</button>
        </span>
      {/if}
    </span>
    <span class="actions">
      <!-- Assistant button moved to the global toolbar (top-right,
           ensō glyph). Cmd/Ctrl+H from anywhere on this tab still
           opens it pre-scoped to this file. -->
      <button
        class="hbtn"
        title="show in file browser"
        aria-label="show in file browser"
        onclick={revealInBrowser}
      >📄</button>
      <button
        class="hbtn"
        title={tab.mode === "wysiwyg" ? "view source" : "view rendered"}
        onclick={() => setMode(tab, tab.mode === "wysiwyg" ? "source" : "wysiwyg")}
      >{tab.mode === "wysiwyg" ? "</>" : "¶"}</button>
      <button
        class="hbtn"
        class:on={tab.inspectorOpen}
        title={tab.inspectorOpen ? "hide inspector" : "show inspector"}
        onclick={() => (tab.inspectorOpen = !tab.inspectorOpen)}
      >◫</button>
    </span>
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
  /* Same look + dimensions as the other tab kinds' headers
     (FileBrowserTab). Visual consistency
     across tab kinds is the entire point of this refactor. */
  .tab-bar {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.25rem 0.5rem;
    background: var(--bg-card);
    border-bottom: 1px solid var(--border);
    font-size: 14px;
    color: var(--text-secondary);
    flex-shrink: 0;
    min-height: 28px;
  }
  .tab-bar .left {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }
  .tab-bar .actions { display: flex; gap: 2px; }
  /* The Aa toggle uses the same .hbtn frame as the page-width and
     mode buttons; italic + serif feel hint "typography" without
     fighting the bar's flat aesthetic. Disabled state (source mode
     or read-only) drops the cursor and dims further. */
  .aa-toggle {
    font-style: italic;
    font-family: ui-serif, Georgia, serif;
    font-size: 14px;
    min-width: 28px;
  }
  .aa-toggle:disabled {
    cursor: default;
    opacity: 0.45;
  }
  .aa-toggle:disabled:hover {
    color: var(--text-secondary);
    border-color: transparent;
  }
  /* Page-width button: same visual as .hbtn, just wide enough for
     a 4-digit number ("1200") or "full". Tabular-nums so the width
     doesn't jiggle as digits change. */
  .page-width-btn {
    min-width: 40px;
    font-variant-numeric: tabular-nums;
    font-size: 12px;
    cursor: ns-resize;
    touch-action: none;
  }
  .page-width-btn.dragging {
    color: var(--text);
    border-color: var(--btn-hover);
    background: var(--hover-bg);
  }
  /* While dragging, lock the cursor and suppress text selection
     so a long upward sweep over the editor doesn't highlight prose. */
  :global(body.page-width-dragging) {
    cursor: ns-resize !important;
    user-select: none;
  }
  /* Inline formatting controls revealed by the Aa toggle. Sits in
     the tab-bar next to the page-width pill; uses smaller, flatter
     buttons than the old floating pill so it doesn't dominate the
     bar at the top of the editor. */
  .fmt-group {
    display: flex;
    align-items: center;
    gap: 1px;
    margin-left: 6px;
    padding-left: 8px;
    border-left: 1px solid var(--border);
  }
  .block-kind {
    background: transparent;
    color: var(--text);
    border: 1px solid var(--btn-border);
    border-radius: 3px;
    padding: 0 4px;
    margin-right: 2px;
    font: inherit;
    font-size: 12px;
    height: 20px;
  }
  .fbtn {
    min-width: 22px;
    height: 20px;
    text-align: center;
    background: transparent;
    border: 1px solid transparent;
    border-radius: 3px;
    color: var(--text);
    cursor: pointer;
    font: inherit;
    font-size: 13px;
    padding: 0 4px;
    line-height: 18px;
  }
  .fbtn:hover { background: var(--hover-bg); border-color: var(--btn-border); }
  .fbtn.on {
    background: var(--hover-bg);
    border-color: var(--btn-hover);
  }
  .fbtn b, .fbtn i, .fbtn s, .fbtn code { font-size: 13px; }
  .fbtn code { font-family: ui-monospace, monospace; }
  .hbtn {
    background: none;
    border: 1px solid transparent;
    border-radius: 3px;
    cursor: pointer;
    color: var(--text-secondary);
    font: inherit;
    /* Fixed min-width keeps the hit area constant when the glyph
       swaps (e.g. </> -> ¶ for the source/wysiwyg toggle): a single
       narrow character would otherwise collapse the button to a
       hard-to-click sliver. */
    min-width: 28px;
    text-align: center;
    padding: 0 5px;
    line-height: 18px;
    height: 20px;
  }
  .hbtn:hover { color: var(--text); border-color: var(--btn-border); }
  .hbtn.on {
    color: var(--text);
    border-color: var(--btn-hover);
    background: var(--hover-bg);
  }

  /* The toolbar slot is reserved for one-off error surfacing.
     Mode + inspector toggles moved into .tab-bar; save is implicit
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
