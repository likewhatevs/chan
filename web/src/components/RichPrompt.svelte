<script lang="ts">
  // Rich Prompt: a floating, inset, rounded, button-less markdown bubble over
  // the BOTTOM of the active terminal. The only chrome is a "submit with
  // cmd+enter" label. ENTER inserts a newline (keep editing); CMD+ENTER (Mod-
  // Enter) submits the markdown to the active terminal's write queue via the WS
  // `prompt` frame (sendPromptToActiveTerminal -> the per-session FIFO, NOT the
  // raw keystroke path) and clears, leaving the bubble open + focused.
  //
  // Lightweight, hand-assembled CM6: markdown syntax + history + a minimal
  // keymap. No Wysiwyg widgets/decorations/bubbles, no wiki picker, no date
  // macros (v1 is deliberately minimal). Mounted by TerminalTab when its tab is
  // active and `richPrompt.visible`; toggled by Cmd+Shift+P / the terminal
  // right-click menu.

  import { onDestroy, onMount } from "svelte";
  import { EditorState, Prec } from "@codemirror/state";
  import { EditorView, keymap } from "@codemirror/view";
  import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
  import { markdown } from "@codemirror/lang-markdown";
  import { makeThemeCompartment } from "../editor/base";
  import { effectiveHybridSurfaceTheme } from "../state/store.svelte";
  import { currentOS } from "../state/shortcuts";
  import { hideRichPrompt, richPrompt } from "../state/richPrompt.svelte";
  import { sendPromptToActiveTerminal } from "../state/tabs.svelte";

  let host = $state<HTMLDivElement>();
  let view: EditorView | undefined;
  // Reuse the editor's theme/highlight so markdown reads the same as the
  // Source editor; reconfigured below when the surface theme flips.
  const theme = makeThemeCompartment(effectiveHybridSurfaceTheme("terminal"));

  const submitLabel =
    currentOS() === "mac" ? "submit with cmd+enter" : "submit with ctrl+enter";

  /// Submit the current markdown to the active terminal's queue and clear.
  /// Always returns true so Cmd+Enter is consumed (never inserts a newline);
  /// an empty/whitespace draft is swallowed without sending. On a failed send
  /// (no active terminal / WS not open) the draft is kept so nothing is lost.
  function submit(): boolean {
    if (!view) return true;
    const text = view.state.doc.toString();
    if (!text.trim()) return true;
    if (!sendPromptToActiveTerminal(text)) return true;
    view.dispatch({
      changes: { from: 0, to: view.state.doc.length, insert: "" },
    });
    richPrompt.draft = "";
    view.focus();
    return true;
  }

  onMount(() => {
    if (!host) return;
    const state = EditorState.create({
      doc: richPrompt.draft,
      extensions: [
        history(),
        // Plain Enter stays a newline (defaultKeymap's insertNewline); full
        // undo/redo via history.
        keymap.of([...defaultKeymap, ...historyKeymap]),
        // High-prec submit chord so Cmd/Ctrl+Enter beats any newline binding.
        Prec.high(keymap.of([{ key: "Mod-Enter", run: submit }])),
        markdown({ addKeymap: false }),
        EditorView.lineWrapping,
        theme.extension,
        // Persist the draft so a toggle / active-terminal switch keeps the text.
        EditorView.updateListener.of((u) => {
          if (u.docChanged) richPrompt.draft = u.state.doc.toString();
        }),
      ],
    });
    view = new EditorView({ state, parent: host });
    view.focus();
  });

  onDestroy(() => view?.destroy());

  // Track surface-theme flips so the bubble's markdown highlight follows.
  $effect(() => {
    const t = effectiveHybridSurfaceTheme("terminal");
    if (view) theme.reconfigure(view, t);
  });

  function onKeydown(e: KeyboardEvent): void {
    // Escape hides the bubble (returns the user to the terminal). Stop it from
    // reaching App.svelte's global Escape handling.
    if (e.key === "Escape") {
      e.stopPropagation();
      e.preventDefault();
      hideRichPrompt();
    }
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="rich-prompt" role="group" aria-label="Rich Prompt" onkeydown={onKeydown}>
  <div class="rp-editor" bind:this={host}></div>
  <div class="rp-label" aria-hidden="true">{submitLabel}</div>
</div>

<style>
  /* Floats inset from the terminal edges (does not touch them), rounded, no
     buttons. The terminal body is the positioning context (position:relative).
     z above the xterm canvas but well below the terminal menu bubble. */
  .rich-prompt {
    position: absolute;
    left: 12px;
    right: 12px;
    bottom: 12px;
    z-index: 20;
    display: flex;
    flex-direction: column;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: 10px;
    box-shadow: 0 6px 24px rgba(0, 0, 0, 0.28);
    overflow: hidden;
  }
  .rp-editor {
    min-height: 2.4em;
    max-height: 32vh;
    overflow-y: auto;
    padding: 8px 10px;
  }
  .rp-editor :global(.cm-editor) {
    background: transparent;
  }
  .rp-editor :global(.cm-editor.cm-focused) {
    outline: none;
  }
  .rp-editor :global(.cm-content) {
    padding: 0;
  }
  .rp-editor :global(.cm-line) {
    padding: 0;
  }
  .rp-label {
    padding: 4px 10px 6px;
    font-size: 11px;
    color: var(--text-secondary);
    text-align: right;
    border-top: 1px solid var(--border);
    user-select: none;
  }
</style>
