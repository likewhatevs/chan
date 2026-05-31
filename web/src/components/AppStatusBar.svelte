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
  //   - transfer : File Browser uploads with a cancel affordance.
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
    fileTransferStatus,
    indexStatus,
    importStatus,
    openWorkspaceWarningsDialog,
    ui,
  } from "../state/store.svelte";
  import { paneMode } from "../state/tabs.svelte";

  let collapsed = $state(false);

  // Status-bar sections are ambient state. Only statuses that carry
  // an explicit typed action become buttons; generic status text
  // stays passive.

  /// Indexer section: hidden when idle (steady state should be
  /// quiet) and when the poller hasn't replied yet (`null`). The one
  /// idle case that still shows is background embedding (idle + a set
  /// `embedding`): the index is BM25-ready, so we surface a passive
  /// progress chip rather than an active pill.
  const indexVisible = $derived(
    indexStatus.value !== null &&
      (indexStatus.value.state !== "idle" ||
        indexStatus.value.embedding != null),
  );
  const importVisible = $derived(importStatus.value !== null);
  const transferVisible = $derived(fileTransferStatus.value !== null);
  const statusVisible = $derived(!!ui.status);
  const statusActionVisible = $derived(
    statusVisible &&
      ui.statusAction?.kind === "workspace-warnings" &&
      ui.statusAction.label === ui.status,
  );
  const paneModeVisible = $derived(paneMode.active);
  const anyVisible = $derived(
    indexVisible || importVisible || transferVisible || statusVisible || paneModeVisible,
  );

  function toggleCollapse(): void {
    collapsed = !collapsed;
  }

  function activateStatus(): void {
    if (statusActionVisible) openWorkspaceWarningsDialog();
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
              class:working={s.state !== "error" && s.state !== "idle"}
              class:err={s.state === "error"}
            ></span>
            {#if s.state === "building"}
              <!-- The `IndexFile` / `GraphRebuild`
                   stages set `s.current / s.total` to "files
                   indexed / total files" - readable. The
                   `EmbedBatch` stage (sentinel `s.file === "embedding"`)
                   sets them to "chunks pending flush / batch
                   budget" instead, and once more chunks accumulate
                   than the budget the counter reads as nonsense
                   ("indexing 4143/4096 (embedding)"). Hide the
                   count during the embedding phase so the pill
                   just signals "embedding in progress" without the
                   misleading numbers. -->
              indexing
              {#if s.file !== "embedding"}
                <span class="num">{s.current}/{s.total}</span>
              {/if}
              {#if s.file}<span class="muted">({s.file})</span>{/if}
            {:else if s.state === "reindexing"}
              reindexing <span class="muted">{s.file}</span>
            {:else if s.state === "error"}
              index error: <span class="muted">{s.message}</span>
            {:else if s.state === "idle" && s.embedding}
              <!-- BM25-ready, embeddings still generating in the
                   background (preflight already unlocked). Passive chip:
                   the dot is static (no `working` pulse) so this reads as
                   quiet progress, not the active reindexing pill. The
                   done/total here are real chunk counts, not the building
                   phase's misleading EmbedBatch sentinel. -->
              embedding
              <span class="num">{s.embedding.done}/{s.embedding.total}</span>
            {/if}
          </span>
        {/if}
        {#if indexVisible && importVisible}
          <span class="sep"> - </span>
        {/if}
        {#if importVisible}
          <span class="section" aria-label="import status">
            <span class="dot working"></span>
            {importStatus.value!.label}
          </span>
        {/if}
        {#if (indexVisible || importVisible) && transferVisible}
          <span class="sep"> - </span>
        {/if}
        {#if transferVisible}
          {@const transfer = fileTransferStatus.value!}
          <span class="section" aria-label="file transfer status">
            <span class="dot working"></span>
            {transfer.label}
            {#if transfer.cancel}
              <button
                type="button"
                class="transfer-cancel"
                onclick={transfer.cancel}
                title="cancel upload"
                aria-label="cancel upload"
              >×</button>
            {/if}
          </span>
        {/if}
        {#if (indexVisible || importVisible || transferVisible) && statusVisible}
          <span class="sep"> - </span>
        {/if}
        {#if statusVisible}
          {#if statusActionVisible}
            <button
              type="button"
              class="section status-msg status-action"
              aria-label="open workspace warnings"
              onclick={activateStatus}
            >{ui.status}</button>
          {:else}
            <span class="section status-msg" aria-label="status message">{ui.status}</span>
          {/if}
        {/if}
        {#if (indexVisible || importVisible || transferVisible || statusVisible) && paneModeVisible}
          <span class="sep"> - </span>
        {/if}
        {#if paneModeVisible}
          <!-- Status-bar label for Hybrid Nav (Cmd+K). The
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
  /* Most status-bar sections are ambient labels. Typed actions get
     their own button styling below. */
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
  .status-action {
    border: 0;
    background: transparent;
    padding: 0;
    cursor: pointer;
    font: inherit;
  }
  .status-action:hover {
    color: var(--text);
    text-decoration: underline;
    text-underline-offset: 2px;
  }
  .status-action:focus-visible {
    outline: 2px solid var(--link);
    outline-offset: 3px;
    border-radius: 4px;
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
  .transfer-cancel {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 16px;
    height: 16px;
    border: 1px solid var(--border);
    border-radius: 50%;
    background: var(--bg);
    color: var(--text);
    cursor: pointer;
    font: inherit;
    line-height: 1;
    padding: 0;
  }
  .transfer-cancel:hover {
    border-color: var(--warn-text);
    color: var(--warn-text);
  }
  @media (prefers-reduced-motion: reduce) {
    .app-statusbar,
    .app-statusbar:hover {
      transition: opacity 120ms linear;
      transform: none;
    }
  }
</style>
