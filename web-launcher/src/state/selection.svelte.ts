// Gmail-style multi-select for the registry lists. ONE selection spans three
// kinds — local workspaces, served (devserver-mounted) workspaces, and
// devservers — feeding ONE global bulk bar (rendered App-level above the lists).
// Bulk turn on/off loops the per-kind singular op (the ops are independent +
// idempotent, so no bulk endpoints are needed); bulk remove runs an ORDERED
// cross-kind delete (forget served → remove devservers → remove local) so a
// devserver and its served workspaces tear down in the order @@Alex asked for.
// Served rows carry their owning `devserverId` so the delete is self-sufficient
// and immune to the live window-watch re-fetch dropping a row mid-bulk. Partial
// failures are counted and surfaced; the per-row quick actions stay the
// single-item path; remove is bulk-only (behind selection + a confirm).

import {
  connectDevserver,
  disconnectDevserver,
  forgetDevserverWorkspace,
  removeDevserver,
  removeWorkspace,
  setDevserverWorkspaceOn,
  toggleWorkspace,
} from "./library.svelte";

/** The three selectable registry kinds, all feeding the one global bulk bar. */
export type SelKind = "workspace" | "served" | "devserver";

/** A selected row, keyed by (kind, id, devserverId) so ids never collide across
 * kinds (a local workspace_id, a served prefix, and a devserver id), and two
 * devservers serving the same mount prefix stay distinct. A served row carries
 * `devserverId` = its owning devserver and `id` = the mount prefix, so the
 * ordered delete + bulk on/off route to the remote without re-deriving anything
 * from the (live-re-fetched) list. For the other kinds `devserverId` is unset. */
interface SelItem {
  kind: SelKind;
  id: string;
  devserverId?: string;
}

interface SelectionState {
  /** Selected rows across all kinds. A plain array — deeply reactive under
   * $state (a bare Set would need svelte/reactivity's SvelteSet to track). */
  selected: SelItem[];
  /** A bulk action is running (disables the bar briefly). */
  busy: boolean;
  /** The single global bulk bar is awaiting its delete-confirm. */
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

function indexOf(kind: SelKind, id: string, devserverId?: string): number {
  return selection.selected.findIndex(
    (s) => s.kind === kind && s.id === id && s.devserverId === devserverId,
  );
}

export function isSelected(kind: SelKind, id: string, devserverId?: string): boolean {
  return indexOf(kind, id, devserverId) >= 0;
}

/** How many rows are selected: one kind's slice, or the whole selection (no arg
 * → the global bar's combined count). */
export function selectedCount(kind?: SelKind): number {
  return kind
    ? selection.selected.filter((s) => s.kind === kind).length
    : selection.selected.length;
}

export function toggleSelected(kind: SelKind, id: string, devserverId?: string): void {
  const i = indexOf(kind, id, devserverId);
  if (i >= 0) selection.selected.splice(i, 1);
  else selection.selected.push({ kind, id, devserverId });
  // Cancel a pending delete-confirm if the selection changed under it.
  selection.confirmingDelete = false;
  selection.note = null;
}

/** Clear one kind's selection, or every selection (no arg — the bar's Clear). */
export function clearSelection(kind?: SelKind): void {
  selection.selected = kind ? selection.selected.filter((s) => s.kind !== kind) : [];
  selection.confirmingDelete = false;
  selection.note = null;
}

/** Loop a singular op over a snapshot of selected items, counting per-item
 * failures (the ops throw uniformly across the library actions, so a per-item
 * catch keeps the loop going). Returns the items that FAILED; the caller decides
 * what stays selected. Iterating the snapshot (not the live list) keeps a bulk
 * run immune to the window-watch re-fetch mutating `library.*` mid-loop. */
async function runBulk(
  items: SelItem[],
  op: (item: SelItem) => Promise<void>,
): Promise<SelItem[]> {
  const failures: SelItem[] = [];
  for (const item of items) {
    try {
      await op(item);
    } catch {
      failures.push(item);
    }
  }
  return failures;
}

/** Bulk turn on/off across every selected kind: a local workspace toggles its
 * tenant, a served workspace toggles on its owning devserver, a devserver
 * connects/disconnects. Bulk-off stays a fail-safe — an unforced off that 409s
 * (live terminals) just counts as a failure; never a per-item confirm, never a
 * force-kill (phase-35 F6 deferral). The single-row Off confirm is where that
 * path lives. */
export async function bulkSetOnAll(on: boolean): Promise<void> {
  if (selection.busy) return;
  const items = [...selection.selected];
  if (items.length === 0) return;
  selection.busy = true;
  selection.note = null;
  const failures = await runBulk(items, (item) => {
    if (item.kind === "workspace") return toggleWorkspace(item.id, on);
    if (item.kind === "served") return setDevserverWorkspaceOn(item.devserverId!, item.id, on);
    return on ? connectDevserver(item.id) : disconnectDevserver(item.id);
  });
  selection.busy = false;
  const verb = on ? "turn on" : "turn off";
  selection.note =
    failures.length > 0 ? `${failures.length} of ${items.length} failed to ${verb}` : null;
}

export function requestBulkDelete(): void {
  if (selection.selected.length > 0) selection.confirmingDelete = true;
}

export function cancelBulkDelete(): void {
  selection.confirmingDelete = false;
}

/** Ordered cross-kind bulk remove (the order is deliberate, confirmed with
 * @@Alex):
 *   1. Forget every selected SERVED workspace — a desktop action that REQUIRES
 *      the devserver still connected (it tells the remote to unmount+drop), so
 *      it must run before the devserver removal below disconnects it.
 *   2. Remove selected DEVSERVERS — `reg.remove`'s `on_remove` hook reaps the
 *      live connection + windows, so removal disconnects the devserver itself
 *      (no-op if it was not connected).
 *   3. Remove selected LOCAL workspaces.
 * Succeeded rows drop from the selection; failures stay so the count reflects
 * what is left. */
export async function confirmBulkDelete(): Promise<void> {
  if (selection.busy) return;
  const served = selection.selected.filter((s) => s.kind === "served");
  const devservers = selection.selected.filter((s) => s.kind === "devserver");
  const locals = selection.selected.filter((s) => s.kind === "workspace");
  const total = served.length + devservers.length + locals.length;
  if (total === 0) {
    selection.confirmingDelete = false;
    return;
  }
  selection.busy = true;
  selection.note = null;
  const failures: SelItem[] = [];
  failures.push(...(await runBulk(served, (s) => forgetDevserverWorkspace(s.devserverId!, s.id))));
  failures.push(...(await runBulk(devservers, (s) => removeDevserver(s.id))));
  failures.push(...(await runBulk(locals, (s) => removeWorkspace(s.id))));
  selection.busy = false;
  selection.confirmingDelete = false;
  // Keep only the failures selected (succeeded rows drop); surface the count.
  selection.selected = failures;
  selection.note = failures.length > 0 ? `${failures.length} of ${total} failed to remove` : null;
}
