<script lang="ts">
  // One pane: a horizontal tab strip on top, an editor below.

  import {
    canSplit,
    closePane,
    closeTab,
    isDirty,
    layout,
    moveTab,
    openInPane,
    reorderTab,
    saveTab,
    setActivePane,
    splitActive,
    type LeafNode,
  } from "../state/tabs.svelte";

  import { Bell, FileText, User } from "lucide-svelte";
  import FileEditorTab from "./FileEditorTab.svelte";
  import HamburgerMenu from "./HamburgerMenu.svelte";
  import {
    assistantHasUnreadForPath,
    assistantStream,
    pathInAssistantScope,
    tree,
  } from "../state/store.svelte";
  import { tabLabel, tabTooltip } from "../state/tabs.svelte";
  import {
    SHORTCUTS,
    currentOS,
    currentPlatform,
    formatChord,
    renderTable,
  } from "../state/shortcuts";
  import { tabMenu, toggleTabMenu } from "../state/tabMenu.svelte";
  import { onDestroy } from "svelte";
  import { applyPageWidthToElement, pageWidth } from "../state/pageWidth.svelte";

  let { pane }: { pane: LeafNode } = $props();

  const active = $derived(pane.tabs.find((t) => t.id === pane.activeTabId) ?? null);

  /// Per-path "is this file a contact?" lookup. Drives the tab-strip
  /// icon (User glyph for contacts, FileText otherwise) so a row of
  /// tabs reads as "person, file, file, person" rather than a wall
  /// of equally-weighted text. The kind comes from the tree listing's
  /// `chan.kind: contact` discriminator; re-derives whenever the
  /// tree refreshes.
  const contactPaths = $derived<Set<string>>(
    new Set(
      tree.entries
        .filter((e) => !e.is_dir && e.kind === "contact")
        .map((e) => e.path),
    ),
  );

  /// ASCII shortcut table painted on the empty-pane background.
  /// Picks the chord set (web vs native) and Mod label (Cmd vs Ctrl)
  /// once at module init — these don't change at runtime.
  const platform = currentPlatform();
  const os = currentOS();
  const shortcutTable = renderTable(platform, os);

  /// Subset of the registry shown in the empty-pane right-click
  /// menu. Tabs + Esc are excluded: there are no tabs in this pane
  /// (it's empty) and Esc only matters when an overlay is up.
  const emptyPaneMenuItems = SHORTCUTS.filter(
    (s) => s.group !== "Tabs" && s.id !== "ui.overlay.dismiss" && s[platform],
  );

  /// Right-click menu state. The HamburgerMenu component owns the
  /// bubble chrome and outside-click dismiss; we just hold the
  /// handle so the contextmenu handler can open it at the cursor.
  let emptyPaneMenu: HamburgerMenu | undefined = $state();
  let emptyPaneMenuOpen = $state(false);

  function onEmptyPaneContextMenu(e: MouseEvent): void {
    e.preventDefault();
    emptyPaneMenu?.openAtCursor(e.clientX, e.clientY);
  }

  /// Pane chrome menu: the ⋮ in the tab strip that replaces the
  /// per-button split / close controls.
  let paneMenu: HamburgerMenu | undefined = $state();
  let paneMenuOpen = $state(false);

  /// Per-pane page-width cap. The ratio (state/pageWidth) is
  /// global, but the px cap is pane-relative so splitting one
  /// pane into two halves correctly halves the cap. A
  /// ResizeObserver on .editor-wrap fires whenever the pane
  /// resizes (split, close, window resize, browser zoom) and
  /// pushes a fresh `--chan-page-max-width` onto the wrapper as
  /// inline style. CSS cascade beats the document-root fallback.
  let editorWrapEl: HTMLDivElement | undefined = $state();
  let editorWrapWidth = $state(0);
  let resizeObs: ResizeObserver | null = null;
  $effect(() => {
    if (!editorWrapEl) return;
    resizeObs?.disconnect();
    const target = editorWrapEl;
    resizeObs = new ResizeObserver((entries) => {
      for (const entry of entries) {
        editorWrapWidth = Math.round(entry.contentRect.width);
      }
    });
    resizeObs.observe(target);
    // Prime synchronously so the first paint already has the cap.
    editorWrapWidth = Math.round(target.getBoundingClientRect().width);
    return () => {
      resizeObs?.disconnect();
      resizeObs = null;
    };
  });
  $effect(() => {
    if (!editorWrapEl) return;
    applyPageWidthToElement(editorWrapEl, editorWrapWidth, pageWidth.ratio);
  });
  onDestroy(() => resizeObs?.disconnect());

  function onSplitRight(): void {
    paneMenu?.close();
    splitActive("row");
  }
  function onSplitDown(): void {
    paneMenu?.close();
    splitActive("column");
  }
  function onClosePane(): void {
    paneMenu?.close();
    closePane(pane.id);
  }

  /// Fire the same `chan:command` event the keymap layer uses so
  /// every shortcut row routes through the existing dispatcher in
  /// App.svelte. Avoids re-implementing the actions here.
  function dispatchCommand(id: string): void {
    emptyPaneMenu?.close();
    window.dispatchEvent(
      new CustomEvent("chan:command", { detail: { name: id } }),
    );
  }
  // Single-pane layouts hide the focus highlight: it's only useful
  // when there's more than one pane to disambiguate. Re-derives on
  // every layout mutation so the highlight reappears the moment a
  // split is added and disappears the moment the second pane closes.
  const multiPane = $derived.by(() => {
    void layout.rootId;
    return Object.values(layout.nodes).some((n) => n.kind === "split");
  });
  const isFocused = $derived(multiPane && layout.activePaneId === pane.id);
  // Re-derive on every layout mutation so the split buttons grey
  // out the instant a tablet user adds their one allowed split.
  const splitsAllowed = $derived.by(() => {
    void layout.rootId;
    void Object.keys(layout.nodes).length;
    return canSplit();
  });
  // The button is always live: on a non-root pane it collapses the pane;
  // on the only/root pane it clears all tabs (since we can't have zero
  // panes on screen).
  const closeLabel = $derived(
    layout.rootId === pane.id ? "close all tabs" : "close pane",
  );

  // Drag state: highlight the tab strip while another pane's tab is being
  // dragged over it. Keyed by pane id so we don't bleed state between
  // panes that share this Svelte 5 component instance.
  let dropActive = $state(false);
  // Index where a per-tab drop indicator (a thin vertical bar) is
  // currently shown. -1 means no indicator. We render a visual bar
  // before tab[idx]; idx === pane.tabs.length means "after the last
  // tab".
  let dropIndicator = $state<number>(-1);
  /// Snapshot of `pane.activeTabId` taken in a tab's onmousedown,
  /// read in the same tab's onclick. Lets the click handler tell a
  /// tab-switch (was a different tab) apart from a re-click on the
  /// already-active tab (only the latter pops the menu).
  let tabMouseDownPrevActive: string | null = null;

  async function onSave(): Promise<void> {
    if (!active) return;
    try {
      await saveTab(active);
    } catch (e) {
      active.error = (e as Error).message;
    }
  }

  function onKeyDown(e: KeyboardEvent): void {
    const meta = e.metaKey || e.ctrlKey;
    // Plain Cmd/Ctrl+S only. Cmd/Ctrl+Shift+S is the editor's
    // strikethrough toggle; without the shift gate this handler
    // would swallow strike.
    if (meta && !e.shiftKey && !e.altKey && e.key === "s") {
      e.preventDefault();
      void onSave();
    }
  }

  // ----- drag & drop ------------------------------------------------------

  // Custom mimes for the kinds of drops a pane accepts:
  //   - intra-window tab move (drop from another pane's tab strip in
  //     the same window): TAB_DRAG_MIME carries `{fromPaneId, tabId}`,
  //     resolved against the local layout.
  //   - cross-window tab move (drop from another Tauri window on the
  //     same chan-app instance): CROSS_TAB_MIME carries the full
  //     payload needed to reconstruct the tab in the target window.
  //     File tabs only; special tabs are window-bound on native.
  //   - file open (from the file tree): FILE_DRAG_MIME carries
  //     `{path}`.
  //
  // dragstart sets both TAB_DRAG_MIME and CROSS_TAB_MIME for file
  // tabs so the same drag works whether the user drops in the same
  // window (intra) or another (cross). The receiver tries intra
  // first; if `fromPaneId` is not in its layout, it falls through to
  // the cross path. The source's dragend handler removes the tab if
  // the drop happened cross-window (intra moves already mutated the
  // local layout).
  const TAB_DRAG_MIME = "application/x-md-tab";
  const CROSS_TAB_MIME = "application/x-chan-tab+json";
  const FILE_DRAG_MIME = "application/x-md-file";

  function onDragStart(e: DragEvent, tabId: string): void {
    if (!e.dataTransfer) return;
    e.dataTransfer.effectAllowed = "move";
    e.dataTransfer.setData(
      TAB_DRAG_MIME,
      JSON.stringify({ fromPaneId: pane.id, tabId }),
    );
    const t = pane.tabs.find((tab) => tab.id === tabId);
    if (t) {
      e.dataTransfer.setData(
        CROSS_TAB_MIME,
        JSON.stringify({
          path: t.path,
          mode: t.mode,
          inspectorOpen: t.inspectorOpen,
        }),
      );
    }
  }

  /// Fired on the SOURCE element after the drop completes (anywhere).
  /// `dropEffect === "move"` means a target accepted; for an
  /// intra-window move, the layout was already mutated by
  /// moveTab/reorderTab and the tab is no longer here. If the tab is
  /// still in this pane after a successful move, the drop must have
  /// landed in another window — close it locally so the visual
  /// matches the cross-window result.
  function onDragEnd(e: DragEvent, tabId: string): void {
    if (e.dataTransfer?.dropEffect !== "move") return;
    const stillHere = pane.tabs.some((t) => t.id === tabId);
    if (stillHere) {
      closeTab(pane.id, tabId);
    }
  }

  /// Open a cross-window file tab payload in this pane. Used by the
  /// drop handlers when the intra-window check fails (the source
  /// pane belongs to a different window). The tab gets appended via
  /// the same `openInPane` helper used by every other "open file
  /// here" entry point, so the load + dedupe-existing semantics
  /// stay uniform. Drop position is not honoured for cross-window
  /// drops; the user can reorder within the strip afterwards.
  function acceptCrossWindowTab(payload: string): boolean {
    let parsed: { path?: string };
    try {
      parsed = JSON.parse(payload);
    } catch {
      return false;
    }
    if (!parsed.path) return false;
    void openInPane(pane.id, parsed.path);
    return true;
  }

  function isAcceptedDrag(e: DragEvent): boolean {
    const types = e.dataTransfer?.types;
    if (!types) return false;
    return (
      types.includes(TAB_DRAG_MIME) ||
      types.includes(CROSS_TAB_MIME) ||
      types.includes(FILE_DRAG_MIME)
    );
  }

  function onDragOver(e: DragEvent): void {
    if (!isAcceptedDrag(e)) return;
    e.preventDefault();
    if (e.dataTransfer) {
      const isTabMove =
        e.dataTransfer.types.includes(TAB_DRAG_MIME) ||
        e.dataTransfer.types.includes(CROSS_TAB_MIME);
      e.dataTransfer.dropEffect = isTabMove ? "move" : "copy";
    }
    dropActive = true;
  }

  function onDragLeave(e: DragEvent): void {
    // Only clear the highlight when the cursor truly leaves the tab strip,
    // not when it crosses into a child element.
    const related = e.relatedTarget as Node | null;
    if (!related || !(e.currentTarget as Node).contains(related)) {
      dropActive = false;
      dropIndicator = -1;
    }
  }

  /// Compute the insertion index for a drop with cursor position `clientX`
  /// over a tab element. We split the tab in half: drop on the left half
  /// → insert before; right half → insert after.
  function indicatorIndexFor(tabIdx: number, e: DragEvent): number {
    const el = e.currentTarget as HTMLElement;
    const r = el.getBoundingClientRect();
    return e.clientX < r.left + r.width / 2 ? tabIdx : tabIdx + 1;
  }

  /// Is the current drag from a tab in THIS pane? Affects both the
  /// visual indicator and the drop semantics: same-pane drops use a
  /// "drop on tab T = land at slot T" rule (a swap-like reorder, no
  /// half-tab logic), because users naturally expect dropping on a tab
  /// to displace it. Cross-pane drops keep the precise "insert before
  /// or after this tab" semantic since insertion between tabs is the
  /// useful action there.
  function isSamePaneDrag(e: DragEvent): boolean {
    const raw = e.dataTransfer?.getData(TAB_DRAG_MIME);
    if (!raw) return false;
    try {
      return (JSON.parse(raw) as { fromPaneId: string }).fromPaneId === pane.id;
    } catch {
      return false;
    }
  }

  function onTabDragOver(e: DragEvent, tabIdx: number): void {
    if (!isAcceptedDrag(e)) return;
    e.preventDefault();
    e.stopPropagation(); // don't let the strip-level handler also fire
    if (e.dataTransfer) {
      const isTabMove =
        e.dataTransfer.types.includes(TAB_DRAG_MIME) ||
        e.dataTransfer.types.includes(CROSS_TAB_MIME);
      e.dataTransfer.dropEffect = isTabMove ? "move" : "copy";
    }
    dropActive = true;
    // For same-pane drags we show the indicator at the target tab's
    // slot (i.e., "this is where your tab will land"). For cross-pane
    // drags the half-tab heuristic gives precise insertion control.
    dropIndicator = isSamePaneDrag(e) ? tabIdx : indicatorIndexFor(tabIdx, e);
  }

  /// Whether a `fromPaneId` belongs to this window's layout. Cross-
  /// window drops carry a `fromPaneId` that doesn't exist in the
  /// receiving window; we use that to distinguish.
  function paneInThisWindow(paneId: string): boolean {
    const n = layout.nodes[paneId];
    return !!n && n.kind === "leaf";
  }

  function onTabDrop(e: DragEvent, tabIdx: number): void {
    const dt = e.dataTransfer;
    if (!dt) return;
    dropActive = false;
    dropIndicator = -1;
    const tabRaw = dt.getData(TAB_DRAG_MIME);
    if (tabRaw) {
      try {
        const { fromPaneId, tabId } = JSON.parse(tabRaw) as {
          fromPaneId: string;
          tabId: string;
        };
        if (paneInThisWindow(fromPaneId)) {
          e.preventDefault();
          e.stopPropagation();
          if (fromPaneId === pane.id) {
            // Same-pane reorder: drop on tab T means source lands at
            // position T in the final array. No half-tab logic; drops
            // on either half of the target produce the same swap.
            reorderTab(pane.id, tabId, tabIdx);
          } else {
            moveTab(fromPaneId, tabId, pane.id, indicatorIndexFor(tabIdx, e));
          }
          return;
        }
        // Fall through to cross-window path: fromPaneId is a stranger.
      } catch {
        // malformed payload; fall through.
      }
    }
    const crossRaw = dt.getData(CROSS_TAB_MIME);
    if (crossRaw) {
      e.preventDefault();
      e.stopPropagation();
      acceptCrossWindowTab(crossRaw);
      return;
    }
    const fileRaw = dt.getData(FILE_DRAG_MIME);
    if (fileRaw) {
      e.preventDefault();
      e.stopPropagation();
      try {
        const { path } = JSON.parse(fileRaw) as { path: string };
        if (path) void openInPane(pane.id, path);
      } catch {
        // ignore
      }
    }
  }

  function onDrop(e: DragEvent): void {
    dropActive = false;
    dropIndicator = -1;
    const dt = e.dataTransfer;
    if (!dt) return;
    // Tab move takes precedence over file open if both are present.
    const tabRaw = dt.getData(TAB_DRAG_MIME);
    if (tabRaw) {
      try {
        const { fromPaneId, tabId } = JSON.parse(tabRaw) as {
          fromPaneId: string;
          tabId: string;
        };
        if (paneInThisWindow(fromPaneId)) {
          e.preventDefault();
          if (fromPaneId === pane.id) {
            // Strip-level drop in the same pane (i.e., dropped on the
            // background or actions area, not directly on a tab). Treat
            // it as "move source to the end" so dragging a leftmost tab
            // rightward past the last tab does the obvious thing instead
            // of silently no-op'ing.
            reorderTab(pane.id, tabId, Math.max(0, pane.tabs.length - 1));
          } else {
            moveTab(fromPaneId, tabId, pane.id);
          }
          return;
        }
        // Fall through to cross-window path.
      } catch {
        // malformed payload; fall through.
      }
    }
    const crossRaw = dt.getData(CROSS_TAB_MIME);
    if (crossRaw) {
      e.preventDefault();
      acceptCrossWindowTab(crossRaw);
      return;
    }
    const fileRaw = dt.getData(FILE_DRAG_MIME);
    if (fileRaw) {
      e.preventDefault();
      try {
        const { path } = JSON.parse(fileRaw) as { path: string };
        if (path) void openInPane(pane.id, path);
      } catch {
        // ignore
      }
    }
  }
</script>

<svelte:window onkeydown={onKeyDown} />

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  class="pane"
  class:focused={isFocused}
  onmousedown={() => setActivePane(pane.id)}
  role="region"
  aria-label="editor pane"
>
  <!-- svelte-ignore a11y_interactive_supports_focus -->
  <div
    class="tabs"
    class:drop-active={dropActive}
    role="tablist"
    ondragover={onDragOver}
    ondragleave={onDragLeave}
    ondrop={onDrop}
  >
    {#each pane.tabs as t, i (t.id)}
      {#if dropIndicator === i}
        <div class="drop-bar" aria-hidden="true"></div>
      {/if}
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <div
        class="tab"
        class:active={t.id === pane.activeTabId}
        onmousedown={() => {
          // Stash the pre-switch active tab id so the onclick handler
          // can tell whether this is a tab-switch (do NOT pop the
          // menu) or a re-click on the already-active tab (DO pop the
          // menu). Cleared in onclick. mousedown fires before click,
          // so this captures the previous value before we overwrite
          // it below.
          tabMouseDownPrevActive = pane.activeTabId;
          pane.activeTabId = t.id;
        }}
        onclick={(e) => {
          // Click on the filename (anywhere on the tab body that
          // isn't the × close button) toggles the menu — but only
          // when the tab was already active. A click on an inactive
          // tab is a tab-switch (the onmousedown above set
          // activeTabId); popping the menu there feels like the user
          // committed to an action they didn't take. The next click
          // on the now-active tab pops the menu as expected.
          const wasActive = tabMouseDownPrevActive === t.id;
          tabMouseDownPrevActive = null;
          if (t.kind !== "file" || !wasActive) return;
          const row = e.currentTarget as HTMLElement;
          const r = row.getBoundingClientRect();
          toggleTabMenu(t.id, {
            left: r.left,
            top: r.top,
            right: r.right,
            bottom: r.bottom,
          });
        }}
        role="tab"
        tabindex="0"
        title={tabTooltip(t)}
        draggable="true"
        ondragstart={(e) => onDragStart(e, t.id)}
        ondragend={(e) => onDragEnd(e, t.id)}
        ondragover={(e) => onTabDragOver(e, i)}
        ondrop={(e) => onTabDrop(e, i)}
      >
        <!-- Spinner appears while the file is still loading from
             disk; once loaded the tab leads with a kind icon (User
             for contacts, FileText otherwise) so the row reads
             scannably. -->
        {#if t.kind === "file" && t.loading}
          <span class="marker spinner" aria-hidden="true"></span>
        {:else if t.kind === "file"}
          <!-- Bell takes precedence over the kind icon when the
               assistant left an unread reply for this file (user
               had the overlay closed when it landed). Clearing
               happens when the user opens the assistant on this
               scope. -->
          <span class="tab-icon" class:assist-unread={assistantHasUnreadForPath(t.path)} aria-hidden="true">
            {#if assistantHasUnreadForPath(t.path)}
              <Bell size={14} strokeWidth={1.75} />
            {:else if contactPaths.has(t.path)}
              <User size={14} strokeWidth={1.75} />
            {:else}
              <FileText size={14} strokeWidth={1.75} />
            {/if}
          </span>
        {/if}
        <span
          class="path"
          aria-haspopup={t.kind === "file" ? "menu" : undefined}
          aria-expanded={t.kind === "file" && tabMenu.openForTabId === t.id}
        >{tabLabel(t)}</span>
        {#if t.kind === "file" && assistantStream.sessionId !== null && pathInAssistantScope(t.path)}
          <!-- Flashing amber dot next to the title: the assistant
               has an in-flight turn scoped to this file (file: or
               group: context that includes this path). Disappears
               when the request completes or the user dismisses /
               re-scopes the overlay. -->
          <span class="assist-pulse" title="assistant working" aria-label="assistant working"></span>
        {/if}
        {#if isDirty(t)}
          <span class="dirty unsaved" title="unsaved changes">●</span>
        {/if}
        <button
          class="close"
          onclick={(e) => {
            e.stopPropagation();
            closeTab(pane.id, t.id);
          }}
          title="close"
        >×</button>
      </div>
    {/each}
    {#if dropIndicator === pane.tabs.length}
      <div class="drop-bar" aria-hidden="true"></div>
    {/if}
    <div class="actions">
      <!-- Pane-only controls live inside a single hamburger menu
           to match the file browser / search / graph overlays.
           Split rows hide when the platform doesn't allow any splits
           (iPhone) and grey out when the platform's cap is reached
           (iPad after one split, native desktop / web have no cap). -->
      <HamburgerMenu
        bind:this={paneMenu}
        bind:open={paneMenuOpen}
        width={220}
        height={140}
      >
        {#if splitsAllowed}
          <li>
            <button role="menuitem" onclick={onSplitRight}>
              <span class="glyph" aria-hidden="true">⇢</span>
              <span>Split right</span>
            </button>
          </li>
          <li>
            <button role="menuitem" onclick={onSplitDown}>
              <span class="glyph" aria-hidden="true">⇣</span>
              <span>Split down</span>
            </button>
          </li>
          <li class="sep" role="separator"></li>
        {/if}
        <li>
          <button role="menuitem" onclick={onClosePane}>
            <span class="glyph" aria-hidden="true">⊠</span>
            <span>{closeLabel}</span>
          </button>
        </li>
      </HamburgerMenu>
    </div>
  </div>

  <div class="editor-wrap" bind:this={editorWrapEl}>
    {#if active}
      <FileEditorTab tab={active} />
    {:else}
      <div
        class="placeholder"
        aria-label="no tab open"
        oncontextmenu={onEmptyPaneContextMenu}
        role="presentation"
      >
        <div class="placeholder-stack">
          <div class="placeholder-mark"></div>
          <!-- Hint + shortcut table only on the lone-pane case. In a
               multi-pane layout the extra panes are workspace setup
               (the user is about to drop files in), so the chrome
               just gets in the way; the logo alone is enough. -->
          {#if !multiPane}
            <p class="placeholder-hint">
              Each pane's visible tab is part of the scope<br />
              for Assistant and Graph.
            </p>
            <pre class="placeholder-shortcuts">{shortcutTable}</pre>
          {/if}
        </div>
        <!-- Right-click menu. Triggerless: opens only via the
             contextmenu handler above. Same `chan:command` ids as
             the keymap layer so actions stay unified. -->
        <HamburgerMenu
          bind:this={emptyPaneMenu}
          bind:open={emptyPaneMenuOpen}
          showTrigger={false}
          width={280}
          height={260}
        >
          {#each emptyPaneMenuItems as s (s.id)}
            <li>
              <button role="menuitem" onclick={() => dispatchCommand(s.id)}>
                <span class="empty-pane-menu-label">{s.label}</span>
                <span class="empty-pane-menu-chord">{formatChord(s[platform]!, os)}</span>
              </button>
            </li>
          {/each}
        </HamburgerMenu>
      </div>
    {/if}
  </div>
</div>

<style>
  .pane {
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    flex: 1;
    border: 1px solid transparent;
    background: var(--bg);
    color: var(--text);
  }
  .pane.focused { border-color: var(--pane-focus); }
  /* iTerm-style strip: a dark bar with no per-tab dividers. The
     active tab is a rounded pill sitting on the bar rather than a
     beveled cap that "merges" with the editor below. */
  .tabs {
    display: flex;
    align-items: center;
    gap: 2px;
    background: var(--bg);
    border-bottom: 1px solid var(--border);
    padding: 4px 6px;
    overflow-x: auto;
    flex-shrink: 0;
    transition: box-shadow 0.1s;
  }
  .tabs.drop-active {
    box-shadow: inset 0 0 0 2px var(--pane-focus);
  }
  .tab[draggable="true"] { -webkit-user-drag: element; }
  /* Vertical bar between tabs that shows where a drop will land. */
  .drop-bar {
    width: 2px;
    align-self: stretch;
    background: var(--pane-focus);
    flex-shrink: 0;
  }
  .tab {
    display: flex;
    gap: 6px;
    align-items: center;
    padding: 3px 10px;
    border-radius: 999px;
    cursor: pointer;
    font-size: 13px;
    color: var(--text-secondary);
    background: transparent;
    user-select: none;
    /* `transform-origin: center bottom` anchors the hover wobble to
       the tab strip's baseline so the tab lifts upward rather than
       drifting sideways. Same easeOutBack curve as the tab-menu
       bubble, bottom pill, and style toolbar so the motion language
       reads as one set. */
    transform-origin: center bottom;
    transition:
      background 80ms ease,
      color 80ms ease,
      transform 260ms cubic-bezier(0.34, 1.56, 0.64, 1);
  }
  .tab:hover {
    color: var(--text);
    transform: scale(1.04);
  }
  .tab.active {
    background: var(--bg-elev);
    color: var(--text);
    font-weight: 500;
    box-shadow: 0 0 0 1px var(--border);
  }
  /* CSS-only spinner shown while a tab's content is loading. Inherits
     color from the tab's text-secondary so it sits at the same
     visual weight as the surrounding label. */
  .tab .marker.spinner {
    width: 10px;
    height: 10px;
    border: 1.5px solid currentColor;
    border-top-color: transparent;
    border-radius: 50%;
    animation: tab-spin 0.8s linear infinite;
    flex-shrink: 0;
  }
  @keyframes tab-spin {
    to { transform: rotate(360deg); }
  }
  /* Close button stays invisible until the row is hovered or the
     tab is active; matches iTerm / Chrome's behavior so the strip
     reads cleanly with many tabs open. */
  .tab .close {
    background: none;
    border: 0;
    cursor: pointer;
    font-size: 14px;
    line-height: 1;
    color: var(--text-secondary);
    padding: 0 2px;
    opacity: 0;
    transition: opacity 80ms ease, color 80ms ease;
    flex-shrink: 0;
  }
  .tab:hover .close,
  .tab.active .close { opacity: 1; }
  .tab .close:hover { color: var(--text); }
  .dirty {
    font-size: 10px;
    line-height: 1;
    color: var(--info-text);
  }
  /* Flashing dot rendered next to the file title while the
     assistant has an in-flight turn scoped to this file. Same
     amber palette the bottom-left status bar uses for "working"
     dots so the visual language stays consistent across surfaces;
     the pulse animation makes it readable at a glance without
     adding text chrome to the tab strip. */
  .assist-pulse {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: #d29922;
    box-shadow: 0 0 4px rgba(210, 153, 34, 0.55);
    flex-shrink: 0;
    animation: chan-tab-assist-pulse 1.1s ease-in-out infinite;
  }
  @keyframes chan-tab-assist-pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.35; }
  }
  @media (prefers-reduced-motion: reduce) {
    .assist-pulse { animation: none; opacity: 0.85; }
  }
  .path { white-space: nowrap; }
  /* Per-tab kind icon: User for contact-kind files, FileText
     otherwise. Sized to the tab font and stroked with text-secondary
     so it sits one step below the label. */
  .tab-icon {
    display: inline-flex;
    align-items: center;
    color: var(--text-secondary);
    flex-shrink: 0;
  }
  .tab.active .tab-icon { color: var(--text); }
  /* Bell variant when an assistant reply is waiting on this file:
     amber tint to match the per-tab "working" dot palette, so the
     two assistant-related tab signals (working / unread) share a
     consistent visual language. */
  .tab-icon.assist-unread { color: #d29922; }
  .tab.active .tab-icon.assist-unread { color: #d29922; }
  .actions { margin-left: auto; display: flex; align-items: center; padding-left: 4px; }
  .editor-wrap { flex: 1; display: flex; flex-direction: column; min-height: 0; }
  /* Empty pane: muted chan logo watermark above the keyboard-
     shortcut table, both centered. CSS mask paints the silhouette
     in the current text-secondary color so it adapts to light /
     dark themes; the image itself (web/public/chan-mark.png) is
     alpha-only. The shortcut table comes from
     state/shortcuts.ts so the surface stays in sync with the
     `chan serve --help` text — resync via
     `node web/scripts/shortcuts-table.mjs`. */
  .placeholder {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 2rem 1rem;
    overflow: auto;
  }
  .placeholder-stack {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1.25rem;
    color: var(--text-secondary);
    opacity: 0.6;
  }
  .placeholder-mark {
    width: 160px;
    height: 160px;
    background-color: var(--text-secondary);
    -webkit-mask: url('/chan-mark.png') center / contain no-repeat;
            mask: url('/chan-mark.png') center / contain no-repeat;
    opacity: 0.45;
  }
  .placeholder-shortcuts {
    margin: 0;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 12px;
    line-height: 1.5;
    white-space: pre;
    color: var(--text-secondary);
  }
  .placeholder-hint {
    margin: 0;
    text-align: center;
    color: var(--text-secondary);
    font-size: 13px;
    line-height: 1.4;
    max-width: 360px;
  }
  /* Empty-pane right-click menu rows. Two columns: label flush left
     and chord flush right. Reuses the shared bubble chrome from
     HamburgerMenu (.hamburger-menu via :global) so spacing and
     hover state match the overlay menus. */
  :global(.empty-pane-menu-label) {
    flex: 1;
  }
  :global(.empty-pane-menu-chord) {
    margin-left: 1.5rem;
    color: var(--text-secondary);
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 11.5px;
  }
  @media (prefers-reduced-motion: reduce) {
    .tab,
    .tab:hover {
      transition: background 80ms ease, color 80ms ease;
      transform: none;
    }
  }
</style>
