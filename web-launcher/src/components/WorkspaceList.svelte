<script lang="ts">
  // The local workspace registry. Each row has a select checkbox + an on/off
  // quick-toggle pill. Selecting one or more rows reveals a bulk-action bar
  // (Turn On / Turn Off / Delete) that loops the singular library ops; delete
  // is bulk-only, behind a confirm. (The per-row Remove button is gone.)
  import { library, toggleWorkspace } from "../state/library.svelte";
  import {
    selection,
    isSelected,
    toggleSelected,
    clearSelection,
    bulkSetOn,
    requestBulkDelete,
    cancelBulkDelete,
    confirmBulkDelete,
  } from "../state/selection.svelte";
  import { basename } from "../lib/windowLabel";
  import type { WorkspaceEntry } from "../api/library";

  function displayName(ws: WorkspaceEntry): string {
    return ws.label || basename(ws.path) || ws.path;
  }

  const selectedCount = $derived(selection.selected.size);
</script>

{#if library.workspaces.length}
  <section class="group">
    <h2 class="group-title">🏠 Local</h2>

    {#if selectedCount > 0}
      <div class="bulk-bar" role="toolbar" aria-label="Bulk actions">
        <span class="bulk-count">{selectedCount} selected</span>
        {#if selection.confirmingDelete}
          <span class="bulk-confirm">Delete {selectedCount}?</span>
          <button
            class="btn-ghost danger"
            type="button"
            disabled={selection.busy}
            onclick={confirmBulkDelete}>Confirm delete</button>
          <button class="btn-ghost" type="button" onclick={cancelBulkDelete}>Cancel</button>
        {:else}
          <button
            class="btn-ghost"
            type="button"
            disabled={selection.busy}
            onclick={() => bulkSetOn(true)}>Turn On</button>
          <button
            class="btn-ghost"
            type="button"
            disabled={selection.busy}
            onclick={() => bulkSetOn(false)}>Turn Off</button>
          <button
            class="btn-ghost danger"
            type="button"
            disabled={selection.busy}
            onclick={requestBulkDelete}>Delete</button>
          <button class="btn-ghost" type="button" onclick={clearSelection}>Clear</button>
        {/if}
        {#if selection.note}<span class="bulk-note">{selection.note}</span>{/if}
      </div>
    {/if}

    <ul class="rows">
      {#each library.workspaces as ws (ws.workspace_id)}
        <li class="row" class:selected={isSelected(ws.workspace_id)}>
          <input
            class="row-check"
            type="checkbox"
            checked={isSelected(ws.workspace_id)}
            aria-label={`Select ${displayName(ws)}`}
            onchange={() => toggleSelected(ws.workspace_id)} />
          <div class="row-main">
            <span class="row-name">{displayName(ws)}</span>
            <span class="row-sub" title={ws.path}>{ws.path}</span>
          </div>
          <div class="row-actions">
            <button
              class="pill"
              class:on={ws.on}
              type="button"
              aria-pressed={ws.on}
              onclick={() => toggleWorkspace(ws.workspace_id, !ws.on)}>{ws.on ? "On" : "Off"}</button>
          </div>
        </li>
      {/each}
    </ul>
  </section>
{/if}

<style>
  .bulk-bar {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 0.6rem;
    padding: 0.4rem 0.6rem;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg-card);
    font-size: 0.85rem;
  }
  .bulk-count,
  .bulk-confirm {
    font-weight: 600;
    color: var(--text);
  }
  .bulk-note {
    color: var(--danger);
  }
  .btn-ghost.danger:hover:not(:disabled) {
    color: var(--danger);
    border-color: var(--danger);
  }
  .row-check {
    margin-right: 0.6rem;
    cursor: pointer;
    flex-shrink: 0;
  }
  .row-main {
    flex: 1;
  }
  .row.selected {
    background: color-mix(in srgb, var(--brand) 10%, var(--bg-card));
  }
</style>
