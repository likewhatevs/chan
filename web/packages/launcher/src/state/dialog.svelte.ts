// The New-workspace / devserver / gateway dialog's open/choice/edit state.
// Context-anchored entry points open it pre-set to a choice: the LOCAL
// [new workspace] button opens "local", the bottom "Add devserver" button
// opens "devserver", the Gateways screen's "Add gateway" button opens
// "gateway". There is no in-dialog chooser. The devserver body doubles as the
// edit form, prefilled from `editing` (null = add a new one); local and
// gateway are add-only.

import type { DevserverEntry } from "../api/library";

export type DialogChoice = "local" | "devserver" | "gateway";

interface DialogState {
  open: boolean;
  choice: DialogChoice;
  editing: DevserverEntry | null;
}

export const dialog = $state<DialogState>({
  open: false,
  choice: "local",
  editing: null,
});

export function openNewDialog(choice: DialogChoice = "local"): void {
  dialog.choice = choice;
  dialog.editing = null;
  dialog.open = true;
}

export function openEditDevserver(ds: DevserverEntry): void {
  dialog.choice = "devserver";
  dialog.editing = ds;
  dialog.open = true;
}

export function closeDialog(): void {
  dialog.open = false;
  dialog.editing = null;
}
