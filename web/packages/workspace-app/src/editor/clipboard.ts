// Menu-triggered clipboard actions for the CodeMirror editors (the
// body-context menu). The native Cmd+C/X/V keymap still owns keyboard
// clipboard (including the rich HTML paste handler in paste_html.ts);
// these are the menu entries, which operate on plain text so a
// right-click Cut/Copy/Paste is predictable. Each takes the live
// EditorView so the same helper serves the WYSIWYG and Source editors.

import { EditorSelection } from "@codemirror/state";
import { EditorView } from "@codemirror/view";

import { notify } from "../state/notify.svelte";
import {
  findWorkspaceImageRefs,
  writeDocSelectionToClipboard,
  type ChanClipboardContext,
} from "./copy_html";

/// The current primary-selection text ("" when the selection is empty).
/// Callers gate Cut/Copy on this being non-empty.
export function selectionText(view: EditorView): string {
  const { from, to } = view.state.selection.main;
  return view.state.sliceDoc(from, to);
}

/// Copy the primary selection to the system clipboard. No-op (and no
/// clipboard write) when the selection is empty. With a rich context (the
/// WYSIWYG body menu), a selection carrying workspace image refs writes the
/// rich HTML + plain flavors; the plain writeText is the fallback (and the
/// only path for the Source editor, which passes no context).
export async function copySelection(
  view: EditorView,
  ctx?: ChanClipboardContext,
): Promise<void> {
  const text = selectionText(view);
  if (!text) return;
  if (ctx && findWorkspaceImageRefs(text).length > 0) {
    try {
      await writeDocSelectionToClipboard(text, ctx);
      return;
    } catch (err) {
      console.warn("editor rich copy failed, falling back to text", err);
    }
  }
  try {
    await navigator.clipboard.writeText(text);
  } catch (err) {
    console.warn("editor copy failed", err);
    notify("Couldn't copy to clipboard");
  }
}

/// Copy then delete the primary selection, leaving the caret where the
/// selection started. No-op when the selection is empty. Rich context as in
/// `copySelection`.
export async function cutSelection(
  view: EditorView,
  ctx?: ChanClipboardContext,
): Promise<void> {
  const { from, to } = view.state.selection.main;
  if (from === to) return;
  const text = view.state.sliceDoc(from, to);
  let wrote = false;
  if (ctx && findWorkspaceImageRefs(text).length > 0) {
    try {
      await writeDocSelectionToClipboard(text, ctx);
      wrote = true;
    } catch (err) {
      console.warn("editor rich cut failed, falling back to text", err);
    }
  }
  if (!wrote) {
    try {
      await navigator.clipboard.writeText(text);
    } catch (err) {
      console.warn("editor cut failed", err);
      notify("Couldn't copy to clipboard");
      return;
    }
  }
  view.dispatch({
    changes: { from, to, insert: "" },
    selection: EditorSelection.cursor(from),
  });
  view.focus();
}

/// Insert the clipboard text (plain) at the caret, replacing any
/// selection. Rich paste stays on Cmd+V (paste_html.ts); this is the
/// menu entry, so it is plain text by design.
export async function pasteClipboard(view: EditorView): Promise<void> {
  let text = "";
  try {
    // Guard against a clipboard read that never settles: a webview that
    // silently gates clipboard-read, or a browser permission prompt the
    // user neither grants nor denies, leaves readText() pending forever.
    // Give up after a few seconds so the menu action fails loud instead
    // of hanging.
    text = await Promise.race([
      navigator.clipboard.readText(),
      new Promise<string>((_, reject) =>
        setTimeout(() => reject(new Error("clipboard read timed out")), 4000),
      ),
    ]);
  } catch (err) {
    console.warn("editor paste failed", err);
    notify("Couldn't read clipboard");
    return;
  }
  if (!text) return;
  const { from, to } = view.state.selection.main;
  view.dispatch({
    changes: { from, to, insert: text },
    selection: EditorSelection.cursor(from + text.length),
  });
  view.focus();
}
