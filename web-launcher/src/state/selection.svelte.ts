// Gmail-style multi-select for the registry lists: a selected set spanning two
// kinds (local workspaces + devservers) with per-kind bulk actions (turn on /
// turn off / remove). Each bulk action loops the singular library op over the
// selected items of that kind (the ops are independent + idempotent), so no new
// bulk endpoints are needed; partial failures are counted and surfaced. The
// per-row quick actions stay the single-item path; remove is bulk-only (behind
// selection + a confirm). Workspace "turn on/off" toggles the local tenant;
// devserver "turn on/off" is connect/disconnect.

import {
  connectDevserver,
  disconnectDevserver,
  removeDevserver,
  removeWorkspace,
  toggleWorkspace,
} from "./library.svelte";

/** The two selectable registry kinds. Each list shows a bar scoped to its kind. */
export type SelKind = "workspace" | "devserver";

/** A selected row, keyed by (kind, id) so a workspace id and a devserver id
 * never collide and each kind's bar filters to its own items. */
interface SelItem {
  kind: SelKind;
  id: string;
}

interface SelectionState {
  /** Selected rows across both kinds. A plain array — deeply reactive under
   * $state (a bare Set would need svelte/reactivity's SvelteSet to track). */
  selected: SelItem[];
  /** A bulk action is running (disables both bars briefly). */
  busy: boolean;
  /** Which kind's bar is awaiting its delete-confirm (null = none). */
  confirmingDelete: SelKind | null;
  /** Last bulk-action outcome, surfaced briefly (e.g. partial failures). */
  note: string | null;
}

export const selection = $state<SelectionState>({
  selected: [],
  busy: false,
  confirmingDelete: null,
  note: null,
});

function indexOf(kind: SelKind, id: string): number {
  return selection.selected.findIndex((s) => s.kind === kind && s.id === id);
}

export function isSelected(kind: SelKind, id: string): boolean {
  return indexOf(kind, id) >= 0;
}

/** How many rows of one kind are selected (each bar's count). */
export function selectedCount(kind: SelKind): number {
  return selection.selected.filter((s) => s.kind === kind).length;
}

export function toggleSelected(kind: SelKind, id: string): void {
  const i = indexOf(kind, id);
  if (i >= 0) selection.selected.splice(i, 1);
  else selection.selected.push({ kind, id });
  // Cancel a pending delete-confirm if the selection changed under it.
  selection.confirmingDelete = null;
  selection.note = null;
}

/** Clear one kind's selection (the bar's Clear), or every selection (no arg). */
export function clearSelection(kind?: SelKind): void {
  selection.selected = kind ? selection.selected.filter((s) => s.kind !== kind) : [];
  selection.confirmingDelete = null;
  selection.note = null;
}

/** Run a singular op over every selected id of one kind; count + report partial
 * failures. The ops throw on failure (uniform across the library actions), so a
 * per-item catch keeps the loop going and tallies the misses. */
async function runBulk(
  kind: SelKind,
  op: (id: string) => Promise<void>,
  verb: string,
): Promise<void> {
  if (selection.busy) return;
  const ids = selection.selected.filter((s) => s.kind === kind).map((s) => s.id);
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

/** Bulk turn on/off: a workspace toggles its local tenant; a devserver
 * connects/disconnects. */
export async function bulkSetOn(kind: SelKind, on: boolean): Promise<void> {
  const verb = on ? "turn on" : "turn off";
  if (kind === "workspace") {
    await runBulk(kind, (id) => toggleWorkspace(id, on), verb);
  } else {
    await runBulk(kind, (id) => (on ? connectDevserver(id) : disconnectDevserver(id)), verb);
  }
}

export function requestBulkDelete(kind: SelKind): void {
  if (selectedCount(kind) > 0) selection.confirmingDelete = kind;
}

export function cancelBulkDelete(): void {
  selection.confirmingDelete = null;
}

/** Bulk remove: a workspace is unregistered; a devserver is removed (the DELETE
 * already disconnects + forgets). Removed rows drop from the selection; a failed
 * remove keeps its row + selection so the count reflects what is left. */
export async function confirmBulkDelete(kind: SelKind): Promise<void> {
  if (kind === "workspace") {
    await runBulk(kind, (id) => removeWorkspace(id), "remove");
  } else {
    await runBulk(kind, (id) => removeDevserver(id), "remove");
  }
  selection.confirmingDelete = null;
  if (!selection.note) clearSelection(kind);
}
