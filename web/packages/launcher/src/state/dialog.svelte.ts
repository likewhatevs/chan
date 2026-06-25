// The New-workspace dialog's open/choice/edit state. The dialog carries two
// choices (Local directory, Devserver); the devserver body doubles as the
// edit form, prefilled from `editing` (null = add a new one).

import type { DevserverEntry } from "../api/library";

export type DialogChoice = "local" | "devserver";

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

export function selectChoice(choice: DialogChoice): void {
  dialog.choice = choice;
}

export function closeDialog(): void {
  dialog.open = false;
  dialog.editing = null;
}
