// Task-list checkbox widget. Replaces the literal `[ ]` / `[x]` / `[X]`
// source text with a real `<input type="checkbox">` that toggles the
// underlying source on click.
//
// Live position: the click handler does NOT trust the from/to captured
// at widget construction time (those may have shifted by the time the
// user clicks). Instead it resolves the current position via
// `view.posAtDOM(el)` and re-reads the 3 chars at that position to
// decide what to swap. Safe across intervening edits and atomic
// undo/redo.
//
// `eq()` compares only the `checked` state — same value === reuse the
// same DOM, no remount on unrelated transactions.

import { WidgetType, type EditorView } from "@codemirror/view";

export class CheckboxWidget extends WidgetType {
  constructor(readonly checked: boolean) {
    super();
  }

  eq(other: CheckboxWidget): boolean {
    return this.checked === other.checked;
  }

  toDOM(view: EditorView): HTMLElement {
    const el = document.createElement("input");
    el.type = "checkbox";
    el.className = "cm-md-task-checkbox";
    el.checked = this.checked;
    el.addEventListener("mousedown", (e) => {
      // Stop CM6 from moving the caret on the click; the change handler
      // owns the toggle. Without this the click first places the caret
      // (which may collapse a selection the user wanted to keep) and
      // then fires the change.
      e.preventDefault();
      e.stopPropagation();
      // Manually dispatch the toggle here (mousedown + checkbox + the
      // preventDefault means the native `change` event won't fire).
      togglePosition(view, el);
    });
    return el;
  }

  ignoreEvent(): boolean {
    // Returning false would let CM6 process events on the widget DOM
    // as part of normal editor input. Returning true keeps the widget
    // self-contained — our mousedown handler owns the toggle.
    return true;
  }
}

function togglePosition(view: EditorView, el: HTMLElement): void {
  const pos = view.posAtDOM(el);
  if (pos < 0 || pos > view.state.doc.length - 3) return;
  const text = view.state.doc.sliceString(pos, pos + 3);
  let next: string | null = null;
  if (text === "[ ]") next = "[x]";
  else if (text === "[x]" || text === "[X]") next = "[ ]";
  if (next === null) return;
  view.dispatch({
    changes: { from: pos, to: pos + 3, insert: next },
  });
}
