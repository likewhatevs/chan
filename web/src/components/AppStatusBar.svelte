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
    ui,
  } from "../state/store.svelte";
  import { paneMode } from "../state/tabs.svelte";

  let collapsed = $state(false);

  // `fullstack-a-2`: status-bar sections are ambient state, not
  // navigation. Per the phase-8 bug list, clicking the index /
  // import / status pills should NOT open Settings, Files, or
  // dismiss the message — clicks were too easy to land by
  // accident and bled into the Settings overlay. The only click
  // surface left is the collapse handle that toggles the pill's
  // visibility.

  /// Indexer section: hidden when idle (steady state should be
  /// quiet) and when the poller hasn't replied yet (`null`).
  const indexVisible = $derived(
    indexStatus.value !== null && indexStatus.value.state !== "idle",
  );
  const importVisible = $derived(importStatus.value !== null);
  const statusVisible = $derived(!!ui.status);
  const paneModeVisible = $derived(paneMode.active);
  const anyVisible = $derived(
    indexVisible || importVisible || statusVisible || paneModeVisible,
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
          <span
            class="section"
            class:err={s.state === "error"}
            aria-label="index status"
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
          </span>
        {/if}
        {#if indexVisible && importVisible}
          <span class="sep">·</span>
        {/if}
        {#if importVisible}
          <span class="section" aria-label="import status">
            <span class="dot working"></span>
            {importStatus.value!.label}
          </span>
        {/if}
        {#if (indexVisible || importVisible) && statusVisible}
          <span class="sep">·</span>
        {/if}
        {#if statusVisible}
          <span class="section status-msg" aria-label="status message">{ui.status}</span>
        {/if}
        {#if (indexVisible || importVisible || statusVisible) && paneModeVisible}
          <span class="sep">·</span>
        {/if}
        {#if paneModeVisible}
          <!-- `fullstack-a-3`: status-bar label for Hybrid Nav
               (Cmd+K). Per the phase-8 bug list the wording is
               `Hybrid ☯ Enter commit, Esc discard, H help`; the
               spawn-intent chip stays because 1/2/3 still stage
               a tab spawn before commit. -->
          <span class="section pane-mode-pill" aria-label="Hybrid Nav active">
            <span class="dot working"></span>
            Hybrid ☯ Enter commit, Esc discard, H help
            {#if paneMode.spawnIntent}
              <span class="muted">→ stage {paneMode.spawnIntent.kind}</span>
            {/if}
          </span>
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
  /* `fullstack-a-2`: status-bar sections are ambient labels now;
     the click affordances + hover/focus chrome that used to wrap
     each pill in a `<button>` are gone. The collapse handle
     stays interactive. */
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
