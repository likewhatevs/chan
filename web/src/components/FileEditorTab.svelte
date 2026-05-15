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
  import JsonPretty from "../editor/JsonPretty.svelte";
  import CsvTable from "../editor/CsvTable.svelte";
  import {
    ArrowLeft,
    ArrowRight,
    Braces,
    Code2,
    Copy,
    FilePlus,
    Folder,
    Highlighter,
    Network,
    Pencil,
    Pilcrow,
    RotateCw,
    Search as SearchIcon,
    Settings as SettingsIcon,
    SquareSplitHorizontal,
    SquareSplitVertical,
    Table2,
    Type,
  } from "lucide-svelte";
  import EnsoIcon from "./EnsoIcon.svelte";
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
    setMode,
    setTabCaret,
    setTabInspectorOpen,
    setTabOutlineOpen,
    setTabStyleToolbarOpen,
    setTabSyntaxHighlight,
    type FileTab,
  } from "../state/tabs.svelte";
  import WikiStatusBar from "./WikiStatusBar.svelte";

  import {
    fileOps,
    openAssistant,
    openGraphForFile,
    openGraphForTag,
    openSettings,
    searchPanel,
    openBrowser,
    openGraphAtNode,
    paneWidths,
    persistPaneWidths,
    revealAndSelect,
  } from "../state/store.svelte";
  import { canSplit, openInActivePane, splitActive } from "../state/tabs.svelte";
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

  let { tab }: { tab: FileTab } = $props();

  // Editor refs so the outline body can call scrollToHeading /
  // scrollToLine on whichever editor variant is showing, and so
  // the toolbar can call into the Wysiwyg formatting API.
  let wysiwygRef: Wysiwyg | undefined = $state();
  let sourceRef: Source | undefined = $state();

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
    setMode(tab, tab.mode === "source" ? rendered : "source");
    closeTabMenu();
  }

  function doToggleSyntaxHighlight(): void {
    setTabSyntaxHighlight(tab, !tab.syntaxHighlight);
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

  /// Right-click menu split actions. Mirror the per-pane hamburger
  /// menu but live here too so the tab/editor context menu can fan
  /// out the current file into a side-by-side view without making
  /// the user reach for the pane chrome. `splitsAllowed` re-derives
  /// on every layout mutation so the row greys out the moment the
  /// platform's split cap is hit (iPad after one split).
  const splitsAllowed = $derived.by(() => {
    void layout.rootId;
    void Object.keys(layout.nodes).length;
    return canSplit();
  });
  function doSplitRight(): void {
    closeTabMenu();
    splitActive("row");
  }
  function doSplitDown(): void {
    closeTabMenu();
    splitActive("column");
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
        <!-- Rendered / source toggle. Hidden for plain text tabs
             (.py / .toml / ...) which have no structured renderer.
             For markdown the pair is wysiwyg <-> source; for JSON
             it is pretty <-> source. CSV will plug into the same
             toggle once its renderer lands. -->
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
            <span class="mbtn-chord"></span>
          </button>
        {/if}
        <!-- Per-tab syntax-highlight toggle. Only meaningful in
             source mode (wysiwyg paints its own decoration set); we
             show it whenever the tab is source-side so the user can
             flip plain-text mode on or off for the file in front of
             them. Hidden in wysiwyg to keep the menu lean. -->
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
        <div class="msep" role="separator"></div>
        <button class="mbtn" onclick={doNewFile}>
          <span class="mbtn-icon">
            <FilePlus size={16} strokeWidth={1.75} aria-hidden="true" />
          </span>
          <span class="mbtn-label">New File</span>
          <span class="mbtn-chord">{chordLabel("app.file.new")}</span>
        </button>
        <button class="mbtn" onclick={doDuplicate}>
          <span class="mbtn-icon">
            <Copy size={16} strokeWidth={1.75} aria-hidden="true" />
          </span>
          <span class="mbtn-label">Duplicate File</span>
          <span class="mbtn-chord"></span>
        </button>
        <button class="mbtn" onclick={doRename}>
          <span class="mbtn-icon">
            <Pencil size={16} strokeWidth={1.75} aria-hidden="true" />
          </span>
          <span class="mbtn-label">Rename File</span>
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
        <button class="mbtn" onclick={revealInBrowser}>
          <span class="mbtn-icon">
            <Folder size={16} strokeWidth={1.75} aria-hidden="true" />
          </span>
          <span class="mbtn-label">Files</span>
          <span class="mbtn-chord">{chordLabel("app.files.toggle")}</span>
        </button>
        <button class="mbtn" onclick={doOpenSearch}>
          <span class="mbtn-icon">
            <SearchIcon size={16} strokeWidth={1.75} aria-hidden="true" />
          </span>
          <span class="mbtn-label">Search</span>
          <span class="mbtn-chord">{chordLabel("app.search.toggle")}</span>
        </button>
        <button class="mbtn" onclick={doOpenGraph}>
          <span class="mbtn-icon">
            <Network size={16} strokeWidth={1.75} aria-hidden="true" />
          </span>
          <span class="mbtn-label">Graph</span>
          <span class="mbtn-chord">{chordLabel("app.graph.toggle")}</span>
        </button>
        <button class="mbtn" onclick={doOpenAssistant}>
          <span class="mbtn-icon">
            <EnsoIcon size={16} />
          </span>
          <span class="mbtn-label">Call Assistant</span>
          <span class="mbtn-chord">{chordLabel("app.assistant.toggle")}</span>
        </button>
        <div class="msep" role="separator"></div>
        {#if splitsAllowed}
          <button class="mbtn" onclick={doSplitRight}>
            <span class="mbtn-icon">
              <SquareSplitHorizontal size={16} strokeWidth={1.75} aria-hidden="true" />
            </span>
            <span class="mbtn-label">Split right</span>
            <span class="mbtn-chord"></span>
          </button>
          <button class="mbtn" onclick={doSplitDown}>
            <span class="mbtn-icon">
              <SquareSplitVertical size={16} strokeWidth={1.75} aria-hidden="true" />
            </span>
            <span class="mbtn-label">Split down</span>
            <span class="mbtn-chord"></span>
          </button>
          <div class="msep" role="separator"></div>
        {/if}
        <button class="mbtn" onclick={doOpenSettings}>
          <span class="mbtn-icon">
            <SettingsIcon size={16} strokeWidth={1.75} aria-hidden="true" />
          </span>
          <span class="mbtn-label">Settings</span>
          <span class="mbtn-chord">{chordLabel("app.settings.toggle")}</span>
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
      {#if tab.outlineOpen}
        <Inspector
          title="Outline"
          side="left"
          bind:width={paneWidths.outline}
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
            initialCaret={tab.caret ?? null}
            onCaretChange={(from, to) => setTabCaret(tab, from, to)}
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
          onClose={() => setTabInspectorOpen(tab, false)}
        >
          <FileInfoBody
            path={tab.path}
            showRefs
            onNavigate={(p) => void openInActivePane(p)}
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
</style>
