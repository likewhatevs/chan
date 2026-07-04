// A registry of the mounted file editors' imperative command surfaces,
// keyed by tab id. A CodeMirror editor owns state the tab model cannot
// express: which fenced code blocks are folded, and the live selection
// (which lives in EditorState and so survives the view losing focus, for
// example when the launcher takes it). The command catalog reaches the
// active editor's view through here instead of mutating tab state.
//
// Every file tab is kept mounted (Pane.svelte keep-alive), so each
// FileEditorTab registers under its own tab id on mount and clears on
// destroy; the catalog looks up activeFileTab().id to reach the focused
// editor. A plain Map, not $state: the catalog reads it at run() time,
// not reactively.

export type EditorCommands = {
  /// Fold or unfold the fenced code blocks in the editor view.
  toggleCodeBlocks: () => void;
  /// The text currently selected in the editor view, empty when the
  /// selection is collapsed. Read from EditorState, so it is still
  /// present after the launcher takes focus.
  selectionText: () => string;
};

const registry = new Map<string, EditorCommands>();

export function registerEditorCommands(
  tabId: string,
  cmds: EditorCommands,
): void {
  registry.set(tabId, cmds);
}

export function unregisterEditorCommands(tabId: string): void {
  registry.delete(tabId);
}

export function editorCommandsFor(tabId: string): EditorCommands | undefined {
  return registry.get(tabId);
}
