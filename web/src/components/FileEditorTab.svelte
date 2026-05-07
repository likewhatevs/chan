<script lang="ts">
  // Body of a file tab. Lifted out of Pane.svelte so each tab kind
  // owns its own top bar; the pane action strip now carries only
  // pane-scoped buttons (split + close).
  //
  // Top bar layout:
  //   left  : formatting toolbar (heading dropdown, B / I / S /
  //           inline-code). Shown only in WYSIWYG mode; the
  //           buttons can't act on a textarea source view.
  //   right : wysiwyg/source toggle, inspector toggle, assistant
  //           button (same trigger as Cmd+P).

  import Wysiwyg, { type BlockKind } from "../editor/Wysiwyg.svelte";
  import Source from "../editor/Source.svelte";
  import Inspector from "./Inspector.svelte";
  import OutlineBody, { type Heading } from "./OutlineBody.svelte";
  import FileInfoBody from "./FileInfoBody.svelte";
  import { setMode, type FileTab } from "../state/tabs.svelte";
  import { idle, pinAccessory, setReadMode } from "../state/idle.svelte";
  import WikiStatusBar from "./WikiStatusBar.svelte";

  // Hover pin: while the cursor is over the floating fmt-bar, the
  // idle tracker won't fade it out. Refcounted so a second hover
  // doesn't leak a release fn.
  let fmtBarRelease: (() => void) | null = null;
  function fmtBarEnter(): void {
    fmtBarRelease?.();
    fmtBarRelease = pinAccessory();
  }
  function fmtBarLeave(): void {
    fmtBarRelease?.();
    fmtBarRelease = null;
  }
  import {
    assistantOverlay,
    browserOverlay,
    graphOverlay,
    paneWidths,
    persistPaneWidths,
    searchPanel,
    settingsOverlay,
  } from "../state/store.svelte";
  import {
    PAGE_WIDTH_MAX,
    PAGE_WIDTH_MIN,
    PAGE_WIDTH_STEP,
    pageWidth,
    setPageWidth,
  } from "../state/pageWidth.svelte";

  let { tab }: { tab: FileTab } = $props();

  /// Overlay is up: hide the floating formatting bar so it doesn't
  /// peek out from behind the modal. Cheaper than a stacking-context
  /// dance.
  const overlayOpen = $derived(
    settingsOverlay.open ||
      searchPanel.open ||
      assistantOverlay.open ||
      browserOverlay.open ||
      graphOverlay.open,
  );

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
  /// drives this; Wysiwyg becomes non-editable + caret-hidden, and
  /// the floating fmt-bar is removed from the DOM. Mirrors the
  /// global readMode flag while this tab is the focused one so
  /// the bottom-pill auto-hide accelerates accordingly.
  let readMode = $state(false);
  $effect(() => {
    setReadMode(readMode);
    return () => setReadMode(false);
  });

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
  const blockKind = $derived.by<BlockKind>(() => {
    void selVer;
    return wysiwygRef?.currentBlockKind() ?? "normal";
  });

  function onBlockKindChange(e: Event): void {
    const v = (e.currentTarget as HTMLSelectElement).value as BlockKind;
    wysiwygRef?.setBlockKind(v);
  }

  // In-tab find was removed; the browser's native ⌘F applies. The
  // editor's selectable text (WYSIWYG and source) is plain DOM, so
  // browser find lights up matches the way users already expect.
</script>

<div class="editor-tab">
  <div class="tab-bar">
    <span class="left">
      <!-- Page-width adjuster. Hover + wheel, or focus + Up/Down,
           or focus + Shift+Up/Down for a 5x step. Above the cap
           range falls off into `null` (unbounded / full width).
           No click-drag: a wheel-/key-only control plays nicer with
           trackpads and keyboard navigation than a draggable knob. -->
      <button
        type="button"
        class="hbtn page-width-btn"
        title="page width — scroll or arrow keys; Shift = bigger step"
        aria-label="page width"
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
    </span>
    <span class="actions">
      <!-- Assistant button moved to the global toolbar (top-right,
           ensō glyph). Cmd/Ctrl+H from anywhere on this tab still
           opens it pre-scoped to this file. Formatting buttons
           moved to the floating .fmt-bar below; the tab-bar now
           carries only mode + inspector toggles. -->
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
      >≡</button>
    </span>
  </div>

  {#if !tab.loading && !overlayOpen && tab.mode === "wysiwyg" && !readMode}
    <!-- svelte-ignore a11y_interactive_supports_focus -->
    <!-- Wrapper isn't tabbable; the formatting buttons inside are. -->
    <div
      class="fmt-bar"
      class:idle={idle.active}
      role="toolbar"
      aria-label="Formatting"
      onmouseenter={fmtBarEnter}
      onmouseleave={fmtBarLeave}
    >
      <select
        class="block-kind"
        value={blockKind}
        onchange={onBlockKindChange}
        title="block style"
      >
        <option value="h1">h1</option>
        <option value="h2">h2</option>
        <option value="h3">h3</option>
        <option value="normal">text</option>
        <option value="code">code</option>
        <option value="quote">quote</option>
      </select>
      <!-- onmousedown preventDefault keeps the editor focused when
           the button is clicked, so the chain commands below don't
           re-focus and scroll the selection into view. Without it,
           clicking inline-code (or any other toolbar button) on a
           multi-line selection scrolls the editor to wherever the
           focus lands. -->
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
          readonly={readMode}
          onSelectionChange={() => (selVer = selVer + 1)}
          wikiPickerPrefix={tab.repoRoot}
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
        bind:readMode
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
    /* Anchor for the absolutely-positioned floating format bar. */
    position: relative;
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
  .tab-bar .left { flex: 1; display: flex; align-items: center; }
  .tab-bar .actions { display: flex; gap: 2px; }
  /* Page-width button: same visual as .hbtn, just wide enough for
     a 4-digit number ("1200") or "full". Tabular-nums so the width
     doesn't jiggle as digits change. */
  .page-width-btn {
    min-width: 40px;
    font-variant-numeric: tabular-nums;
    font-size: 12px;
  }
  /* Floating formatting pill anchored near the top of the editor
     area, centered, hovering over the canvas like Apple Notes. */
  .fmt-bar {
    position: absolute;
    top: 12px;
    left: 50%;
    transform: translateX(-50%);
    z-index: 20;
    display: flex;
    gap: 4px;
    align-items: center;
    padding: 8px 12px;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: 999px;
    box-shadow: 0 6px 18px rgba(0, 0, 0, 0.18);
    font-size: 16px;
    color: var(--text);
    /* Pointer-target spacing only; the editor canvas underneath
       remains clickable around the pill. */
    transition: opacity 200ms ease;
  }
  /* Idle: fade out + drop pointer events. Same recipe as
     BottomPill so both pills idle together. */
  .fmt-bar.idle {
    opacity: 0;
    pointer-events: none;
  }
  .block-kind {
    background: var(--bg-card);
    color: var(--text);
    border: 1px solid var(--btn-border);
    border-radius: 14px;
    padding: 1px 8px;
    font: inherit;
    font-size: 15px;
    height: 28px;
  }
  .fbtn {
    min-width: 34px;
    height: 28px;
    text-align: center;
    background: transparent;
    border: 1px solid transparent;
    border-radius: 14px;
    color: var(--text);
    cursor: pointer;
    font: inherit;
    padding: 0 8px;
    line-height: 26px;
  }
  .fbtn:hover { background: var(--hover-bg); }
  .fbtn.on {
    background: var(--hover-bg);
    border-color: var(--btn-hover);
  }
  .fbtn b, .fbtn i, .fbtn s, .fbtn code { font-size: 16px; }
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
