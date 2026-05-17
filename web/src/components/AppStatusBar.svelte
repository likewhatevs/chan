<script lang="ts">
  // Bottom-left counterpart of WikiStatusBar: a window-level pill
  // that surfaces app-wide ambient state. Sections render only when
  // they have content; the whole bar disappears when none do.
  //
  // Sections (left -> right):
  //   - index    : indexer state (building / reindexing / error).
  //                Idle is completely hidden so the bar stays quiet
  //                when nothing's happening.
  //   - import   : long-running import progress (contacts today;
  //                others slot in via the same `importStatus` store).
  //   - status   : transient `ui.status` messages (move/rename/delete
  //                failures, etc).
  //
  // Note: assistant activity intentionally does NOT live here.
  // The per-tab flashing dot on file tabs covers in-scope files;
  // tool-loop narration streams inline in the chat. Surfacing the
  // same signal in a third place added noise without information.
  //
  // Hide model: bar disappears entirely when no section has
  // content, and collapses to a pill on click. No idle fade and no
  // time-based auto-dismiss; a section clears only when its source
  // clears. Indexing progress and error messages are important
  // enough that the user shouldn't lose them just by holding still.
  //
  // Position: fixed bottom-left so it's independent of the workspace
  // layout, matching how BottomPill is anchored.
  import {
    indexStatus,
    importStatus,
    openBrowser,
    openSettings,
    ui,
  } from "../state/store.svelte";

  let collapsed = $state(false);

  /// Click handlers per status section, per request.md "Clicking on
  /// the status bar when an event is shown should pop up the overlay
  /// related to that event." Backend-1 confirms the wire already
  /// carries enough info to route — the rule lives entirely on the
  /// frontend.
  ///
  /// - index → Settings overlay (its About / search-index area is
  ///   the closest "index status page" we have today; no dedicated
  ///   overlay exists, and adding one is out of scope for phase 3
  ///   per the journal "narrow rather than new surfaces" note).
  /// - import → File Browser overlay (where the contacts importer
  ///   is launched / lives; the modal that fires the long-running
  ///   import is reachable from the file browser hamburger).
  /// - transient ui.status → clear the message. These are error
  ///   crumbs (move/rename/delete failures); clicking to dismiss
  ///   matches the user's expectation of "I saw it, move on".
  ///
  /// Agent activity is intentionally NOT surfaced here (see the
  /// block comment at the top of this file) so there is no
  /// agent-routing case.
  function onClickIndex(): void {
    openSettings();
  }
  function onClickImport(): void {
    openBrowser();
  }
  function onClickStatus(): void {
    ui.status = null;
  }

  /// Indexer section: hidden when idle (steady state should be
  /// quiet) and when the poller hasn't replied yet (`null`).
  const indexVisible = $derived(
    indexStatus.value !== null && indexStatus.value.state !== "idle",
  );
  const importVisible = $derived(importStatus.value !== null);
  const statusVisible = $derived(!!ui.status);
  const anyVisible = $derived(
    indexVisible || importVisible || statusVisible,
  );

  function toggleCollapse(): void {
    collapsed = !collapsed;
  }
</script>

{#if anyVisible}
  <div class="app-statusbar" class:collapsed>
    <button
      class="collapse"
      title={collapsed ? "show status" : "hide status"}
      onclick={toggleCollapse}
      onmousedown={(e) => e.preventDefault()}
    >{collapsed ? "›" : "‹"}</button>
    {#if !collapsed}
      <div class="row">
        {#if indexVisible}
          {@const s = indexStatus.value!}
          <button
            type="button"
            class="section btn"
            class:err={s.state === "error"}
            onclick={onClickIndex}
            title="open index status"
            aria-label="open index status"
          >
            <span
              class="dot"
              class:working={s.state !== "error"}
              class:err={s.state === "error"}
            ></span>
            {#if s.state === "building"}
              indexing
              <span class="num">{s.current}/{s.total}</span>
              {#if s.file}<span class="muted">({s.file})</span>{/if}
            {:else if s.state === "reindexing"}
              reindexing <span class="muted">{s.file}</span>
            {:else if s.state === "error"}
              index error: <span class="muted">{s.message}</span>
            {/if}
          </button>
        {/if}
        {#if indexVisible && importVisible}
          <span class="sep">·</span>
        {/if}
        {#if importVisible}
          <button
            type="button"
            class="section btn"
            onclick={onClickImport}
            title="open import surface"
            aria-label="open import surface"
          >
            <span class="dot working"></span>
            {importStatus.value!.label}
          </button>
        {/if}
        {#if (indexVisible || importVisible) && statusVisible}
          <span class="sep">·</span>
        {/if}
        {#if statusVisible}
          <button
            type="button"
            class="section status-msg btn"
            onclick={onClickStatus}
            title="dismiss"
            aria-label="dismiss status message"
          >{ui.status}</button>
        {/if}
      </div>
    {/if}
  </div>
{/if}

<style>
  /* Mirror of WikiStatusBar, anchored bottom-left. position:fixed
     so it floats above whatever layout the workspace settled on.
     z-index sits above every stacked overlay (modals at 26000,
     OverlayShell stack starts at 25002) but below the disconnect
     overlay (30000), which preempts everything when the watcher
     dies. The bar is small, anchored to a corner, and surfaces
     state the user needs visibility of even while a settings panel
     or file browser is open. */
  .app-statusbar {
    position: fixed;
    left: 12px;
    bottom: 8px;
    z-index: 28000;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 6px 10px;
    background: color-mix(in srgb, var(--bg-elev) 88%, transparent);
    border: 1px solid var(--border);
    border-radius: 999px;
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.18);
    font-size: 12px;
    color: var(--muted);
    user-select: none;
    /* Wobble + lift on hover, matching the editor status bar. */
    transform-origin: left bottom;
    transition:
      opacity 200ms ease,
      transform 260ms cubic-bezier(0.34, 1.56, 0.64, 1),
      box-shadow 160ms ease;
  }
  .app-statusbar:hover {
    transform: scale(1.04);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.24);
  }
  .app-statusbar.collapsed {
    padding: 6px;
    gap: 0;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .section {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--text);
    font-variant-numeric: tabular-nums;
  }
  /* Section rendered as a button (clickable per request.md). Keep
     the visual identical to the original <span> so the bar's
     ambient style does not change; only adds pointer affordance
     + a subtle hover hint. */
  .section.btn {
    background: transparent;
    border: 0;
    padding: 0;
    font: inherit;
    cursor: pointer;
    border-radius: 4px;
    transition: background 120ms ease, color 120ms ease;
  }
  .section.btn:hover {
    background: var(--hover-bg);
    color: var(--text);
  }
  .section.btn:focus-visible {
    outline: 1px solid var(--link);
    outline-offset: 1px;
  }
  .section.err {
    color: var(--warn-text);
  }
  .num {
    font-variant-numeric: tabular-nums;
  }
  .muted {
    color: var(--muted);
    /* Truncate long filenames so a single section can't push the
       bar across the screen. The full path is still in the status
       source if anyone needs it. */
    max-width: 28ch;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .status-msg {
    color: var(--warn-text);
  }
  .sep {
    color: var(--border);
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--muted);
    box-shadow: 0 0 0 1px color-mix(in srgb, var(--text) 12%, transparent);
  }
  /* Amber = something running; matches the conventional "working"
     signal without colliding with the editor lamp's green (which
     specifically means "write mode is active"). */
  .dot.working {
    background: #d29922;
    box-shadow: 0 0 4px rgba(210, 153, 34, 0.55);
  }
  .dot.err {
    background: var(--warn-text);
    box-shadow: 0 0 0 1px color-mix(in srgb, var(--warn-text) 35%, transparent);
  }
  .collapse {
    background: transparent;
    border: 0;
    color: var(--muted);
    cursor: pointer;
    padding: 0 4px;
    font: inherit;
    line-height: 1;
  }
  .collapse:hover {
    color: var(--text);
  }
  @media (prefers-reduced-motion: reduce) {
    .app-statusbar,
    .app-statusbar:hover {
      transition: opacity 120ms linear;
      transform: none;
    }
  }
</style>
