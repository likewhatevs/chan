import { EditorView } from "@codemirror/view";

/// `fullstack-a-65`: suppress CodeMirror's default mousedown
/// handling for right-clicks (button === 2). Without this, a
/// right-click inside the editor surface triggers a selection
/// gesture (whole line or word, depending on prior state) BEFORE
/// the outer `oncontextmenu` handler in `FileEditorTab.svelte`
/// runs — so the user sees an unintended selection alongside the
/// menu opening.
///
/// Returning `true` from the handler tells CodeMirror "I've
/// handled this event, don't run your default mousedown logic".
/// The contextmenu event itself still fires (we don't
/// preventDefault here) — `FileEditorTab.svelte`'s
/// `oncontextmenu={onEditorContext}` handles the menu pop.
export function rightClickNoSelect(): ReturnType<
  typeof EditorView.domEventHandlers
> {
  return EditorView.domEventHandlers({
    mousedown(e) {
      if (e.button === 2) return true;
      return false;
    },
  });
}
