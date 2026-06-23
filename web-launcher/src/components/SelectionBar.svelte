<script lang="ts">
  // The single global bulk-action bar. Rendered once App-level above the lists
  // (gated to the mutable surface), it shows while one or more rows of ANY kind
  // are selected: it reads the whole selection (a combined count across local
  // workspaces, served workspaces, and devservers) and its buttons drive the
  // global ops — Turn On / Turn Off (per-kind under the hood), an ordered
  // cross-kind Remove (forget served → remove devservers → remove local), and
  // Clear. The single-row quick actions stay the per-item path.
  import {
    selection,
    selectedCount,
    clearSelection,
    bulkSetOnAll,
    requestBulkDelete,
    cancelBulkDelete,
    confirmBulkDelete,
  } from "../state/selection.svelte";

  const count = $derived(selectedCount());
  const confirming = $derived(selection.confirmingDelete);
</script>

{#if count > 0}
  <div class="bulk-bar" role="toolbar" aria-label="Bulk actions">
    <span class="bulk-count">{count} selected</span>
    {#if confirming}
      <span class="bulk-confirm">Remove {count}?</span>
      <button
        class="btn-ghost danger"
        type="button"
        disabled={selection.busy}
        onclick={confirmBulkDelete}>Confirm remove</button>
      <button class="btn-ghost" type="button" onclick={cancelBulkDelete}>Cancel</button>
    {:else}
      <button
        class="btn-ghost"
        type="button"
        disabled={selection.busy}
        onclick={() => bulkSetOnAll(true)}>Turn On</button>
      <button
        class="btn-ghost"
        type="button"
        disabled={selection.busy}
        onclick={() => bulkSetOnAll(false)}>Turn Off</button>
      <button
        class="btn-ghost danger"
        type="button"
        disabled={selection.busy}
        onclick={requestBulkDelete}>Remove</button>
      <button class="btn-ghost" type="button" onclick={() => clearSelection()}>Clear</button>
    {/if}
    {#if selection.note}<span class="bulk-note">{selection.note}</span>{/if}
  </div>
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
</style>
