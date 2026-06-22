<script lang="ts">
  // The bulk-action bar a registry list shows while one or more of its rows are
  // selected. One component, two kinds: a workspace bar (Turn On / Turn Off /
  // Remove the local tenant) and a devserver bar (Connect / Disconnect / Remove
  // the registry entry). It is scoped to its `kind` — the count, the actions,
  // and the delete-confirm all read/write only that kind's slice of the shared
  // selection — so the two bars coexist without crossing wires.
  import {
    selection,
    selectedCount,
    clearSelection,
    bulkSetOn,
    requestBulkDelete,
    cancelBulkDelete,
    confirmBulkDelete,
    type SelKind,
  } from "../state/selection.svelte";

  let {
    kind,
    onLabel = "Turn On",
    offLabel = "Turn Off",
  }: { kind: SelKind; onLabel?: string; offLabel?: string } = $props();

  const count = $derived(selectedCount(kind));
  const confirming = $derived(selection.confirmingDelete === kind);
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
        onclick={() => confirmBulkDelete(kind)}>Confirm remove</button>
      <button class="btn-ghost" type="button" onclick={cancelBulkDelete}>Cancel</button>
    {:else}
      <button
        class="btn-ghost"
        type="button"
        disabled={selection.busy}
        onclick={() => bulkSetOn(kind, true)}>{onLabel}</button>
      <button
        class="btn-ghost"
        type="button"
        disabled={selection.busy}
        onclick={() => bulkSetOn(kind, false)}>{offLabel}</button>
      <button
        class="btn-ghost danger"
        type="button"
        disabled={selection.busy}
        onclick={() => requestBulkDelete(kind)}>Remove</button>
      <button class="btn-ghost" type="button" onclick={() => clearSelection(kind)}>Clear</button>
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
