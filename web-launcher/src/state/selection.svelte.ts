// Gmail-style multi-select for the workspace list: a selected set + bulk
// actions (turn on / turn off / delete). Each bulk action loops the singular
// library op over the selected set (the ops are independent + idempotent), so
// no new HTTP endpoints are needed; partial failures are counted and surfaced.
// The per-row On/Off pill stays the quick single-toggle path; delete is
// bulk-only (behind selection + a confirm).

import { removeWorkspace, toggleWorkspace } from "./library.svelte";

interface SelectionState {
  /** Selected workspace ids. A plain array — deeply reactive under $state (a
   * bare Set would need svelte/reactivity's SvelteSet to track). */
  selected: string[];
  /** A bulk action is running. */
  busy: boolean;
  /** The Delete action is awaiting its confirm. */
  confirmingDelete: boolean;
  /** Last bulk-action outcome, surfaced briefly (e.g. partial failures). */
  note: string | null;
}

export const selection = $state<SelectionState>({
  selected: [],
  busy: false,
  confirmingDelete: false,
  note: null,
});

export function isSelected(id: string): boolean {
  return selection.selected.includes(id);
}

export function toggleSelected(id: string): void {
  const i = selection.selected.indexOf(id);
  if (i >= 0) selection.selected.splice(i, 1);
  else selection.selected.push(id);
  // Cancel a pending delete-confirm if the selection changed under it.
  selection.confirmingDelete = false;
  selection.note = null;
}

export function clearSelection(): void {
  selection.selected = [];
  selection.confirmingDelete = false;
  selection.note = null;
}

/** Run a singular op over every selected id; count + report partial failures. */
async function runBulk(op: (id: string) => Promise<void>, verb: string): Promise<void> {
  if (selection.busy) return;
  const ids = [...selection.selected];
  if (ids.length === 0) return;
  selection.busy = true;
  selection.note = null;
  let failed = 0;
  for (const id of ids) {
    try {
      await op(id);
    } catch {
      failed += 1;
    }
  }
  selection.busy = false;
  selection.note = failed > 0 ? `${failed} of ${ids.length} failed to ${verb}` : null;
}

export async function bulkSetOn(on: boolean): Promise<void> {
  await runBulk((id) => toggleWorkspace(id, on), on ? "turn on" : "turn off");
}

export function requestBulkDelete(): void {
  if (selection.selected.length > 0) selection.confirmingDelete = true;
}

export function cancelBulkDelete(): void {
  selection.confirmingDelete = false;
}

export async function confirmBulkDelete(): Promise<void> {
  await runBulk((id) => removeWorkspace(id), "delete");
  selection.confirmingDelete = false;
  // Drop ids that were removed (a failed delete keeps its row + selection).
  if (!selection.note) selection.selected = [];
}
