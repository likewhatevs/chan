// Shared handle to the currently-mounted Wysiwyg editor.
//
// Mobile renders a single-pane shell, so a single "current editor"
// model is enough: whichever FileEditorTab body is mounted writes
// its Wysiwyg ref + selection version into this state, and the
// floating mobile bar reads them to drive the formatting buttons.
//
// Desktop's per-pane FileEditorTab bars don't read this; each owns
// its own ref locally to keep splits independent.

import type Wysiwyg from "../editor/Wysiwyg.svelte";

export const activeEditor = $state<{
  /// Live Wysiwyg component reference, or null when no file
  /// editor tab is mounted.
  wysiwyg: Wysiwyg | null;
  /// Selection-version counter; bumped by the editor on every
  /// selection or doc change so derived isActive() readers
  /// recompute.
  selVer: number;
}>({
  wysiwyg: null,
  selVer: 0,
});
