<script lang="ts">
  // Rich Prompt: a floating, inset, rounded, button-less markdown bubble over
  // the BOTTOM of the active terminal. The only chrome is a "submit with
  // cmd+enter" label. ENTER inserts a newline (keep editing); CMD+ENTER (Mod-
  // Enter) submits to the active terminal's write queue via the WS `prompt`
  // frame (NOT the raw keystroke path).
  //
  // Drafts-backed: the bubble edits a real per-terminal chan-workspace
  // DRAFT (`tab.richPromptDraftPath` -> `<draftsDir>/<name>/draft.md`). That file IS
  // the prompt text, and pasted images land in the SAME draft folder via the
  // editor's image-paste machinery (imageDropHandlers) + insert `![](path)`.
  // So an agent reads the media as FILES (chan MCP read_media / disk under
  // ~/.chan) - no base64, seamless across claude/codex/gemini. The draft is
  // created lazily on first open, persists across hide/show + reload, is
  // cleared (text only) on submit, and the whole folder is discarded when the
  // terminal closes (TerminalTab's close sink).

  import { onDestroy, onMount } from "svelte";
  import { EditorState, Prec } from "@codemirror/state";
  import { EditorView, keymap } from "@codemirror/view";
  import {
    defaultKeymap,
    history,
    historyKeymap,
    indentLess,
    indentMore,
  } from "@codemirror/commands";
  import {
    deleteMarkupBackward,
    insertNewlineContinueMarkup,
    markdown,
  } from "@codemirror/lang-markdown";
  // Reuse the editor's list commands (import is shared; no change to
  // list.ts) so the rich prompt indents/outdents lists exactly like the main
  // editor instead of letting Tab escape to the browser.
  import {
    indentListItem,
    outdentListItem,
  } from "../editor/commands/list";
  import { imageDropHandlers } from "../editor/bubbles/image_drop";
  import { makeThemeCompartment } from "../editor/base";
  import { effectiveHybridSurfaceTheme } from "../state/store.svelte";
  import { currentOS } from "../state/shortcuts";
  import { hideRichPromptForTab } from "../state/richPrompt.svelte";
  import {
    sendPromptToTerminal,
    type TerminalTab,
  } from "../state/tabs.svelte";
  import { api } from "../api/client";

  // The terminal this bubble belongs to (TerminalTab mounts one per active
  // terminal). Its `richPromptDraftPath` is the per-terminal draft backing.
  let { tab }: { tab: TerminalTab } = $props();

  let host = $state<HTMLDivElement>();
  let rootEl = $state<HTMLDivElement>();
  // Drag-to-resize the bubble's TOP edge upward. null = content-driven default
  // height (min..32vh); a number pins the bubble height in px, capped so the
  // top never passes the terminal top minus the same 12px inset as the bottom.
  let customHeight = $state<number | null>(null);
  const MIN_PROMPT_HEIGHT = 56;
  let resizing = false;
  let resizeStartY = 0;
  let resizeStartHeight = 0;
  let view: EditorView | undefined;
  let destroyed = false;
  let draftPath = "";
  let writeTimer: ReturnType<typeof setTimeout> | null = null;
  const theme = makeThemeCompartment(effectiveHybridSurfaceTheme("terminal"));

  const submitLabel =
    currentOS() === "mac" ? "submit with cmd+enter" : "submit with ctrl+enter";

  // Directory holding the draft (images upload here).
  // ".Drafts/x/draft.md" -> ".Drafts/x".
  function draftDir(): string {
    const i = draftPath.lastIndexOf("/");
    return i === -1 ? "" : draftPath.slice(0, i);
  }

  function scheduleWrite(): void {
    if (writeTimer !== null) clearTimeout(writeTimer);
    writeTimer = setTimeout(() => void flushWrite(), 400);
  }

  // Persist the current doc to draft.md. Best-effort (a failed write keeps the
  // in-memory text; the next change retries). Captures the text synchronously
  // so an onDestroy flush reads it before the view is torn down.
  async function flushWrite(): Promise<void> {
    if (writeTimer !== null) {
      clearTimeout(writeTimer);
      writeTimer = null;
    }
    if (!view || !draftPath) return;
    const text = view.state.doc.toString();
    try {
      await api.write(draftPath, text);
    } catch {
      // best-effort; leave the in-memory draft intact.
    }
  }

  // The submit chord must match what THIS terminal actually reads, derived from
  // its negotiated keyboard protocol (TerminalTab.keyboardProtocol): claude
  // announces xterm modifyOtherKeys, codex announces the kitty protocol. A
  // plain SHELL (or gemini) announces NEITHER and submits on a bare CR - which
  // is the gemini chord. The old path sent NO agent, so the server defaulted to
  // the claude modifyOtherKeys CSI, which a shell can't read: it left the
  // literal `...7;9;13~` at the prompt and never ran the command.
  // Reuses the shared AGENT_SUBMIT_CHORDS map server-side via the agent name on
  // the prompt frame; no new chord map.
  function submitAgent(): string {
    const kp = tab.keyboardProtocol;
    if (kp) {
      if (kp.xtermModifyOtherKeys > 0) return "claude";
      const kittyFlags =
        kp.kitty.screen === "alternate"
          ? kp.kitty.alternateFlags
          : kp.kitty.mainFlags;
      if (kittyFlags > 0) return "codex";
    }
    return "gemini";
  }

  // Cmd+Enter: submit the draft text through the queue, then RESET = clear
  // draft.md TEXT but KEEP the folder + any pasted media (the agent reads the
  // media AFTER submit; the folder is cleaned on terminal close). Always
  // returns true so the chord never inserts a newline; empty/whitespace is
  // swallowed.
  //
  // Routes to THIS bubble's OWN terminal (`tab`), NOT the focused pane's active
  // terminal: the bubble belongs to `tab`, so its text must land there. And we
  // do NOT reap the composer unless the `prompt` frame actually went out to
  // this terminal's OPEN socket (sendPromptToTerminal returns false on a
  // closed / not-yet-connected socket). That is a real data-loss
  // guard: the old path cleared the text on a local-sink-true that could route
  // to the wrong terminal or nowhere visible. Keeping the text on a failed
  // send lets the user retry instead of losing it.
  function submit(): boolean {
    if (!view) return true;
    const text = view.state.doc.toString();
    if (!text.trim()) return true;
    if (!sendPromptToTerminal(tab.id, text, submitAgent())) return true;
    view.dispatch({
      changes: { from: 0, to: view.state.doc.length, insert: "" },
    });
    void flushWrite();
    view.focus();
    return true;
  }

  // Resolve this terminal's draft, creating it lazily on first open. createDraft
  // seeds "# draft\n"; clear it so the composer starts empty.
  async function ensureDraft(): Promise<string> {
    if (tab.richPromptDraftPath) return tab.richPromptDraftPath;
    const { path } = await api.createDraft();
    tab.richPromptDraftPath = path;
    try {
      await api.write(path, "");
    } catch {
      // best-effort; an unclear seed just shows once.
    }
    return path;
  }

  async function loadContent(path: string): Promise<string> {
    try {
      return (await api.read(path)).content ?? "";
    } catch {
      return "";
    }
  }

  onMount(() => {
    void (async () => {
      if (!host) return;
      draftPath = await ensureDraft();
      if (destroyed || !host) return;
      const content = await loadContent(draftPath);
      if (destroyed || !host) return;
      const state = EditorState.create({
        doc: content,
        extensions: [
          history(),
          keymap.of([...defaultKeymap, ...historyKeymap]),
          // High-prec: Mod-Enter submits; Enter continues markdown markup
          // (lists/quotes), falling through to a plain newline off-markup;
          // Backspace dedents markup. Tab indents the list item (Shift+Tab
          // outdents), falling back to plain indent so Tab NEVER escapes to the
          // browser's focus nav. markdown({addKeymap:false}) keeps
          // syntax only so these bindings own Enter/Tab/Backspace unambiguously.
          Prec.high(
            keymap.of([
              { key: "Mod-Enter", run: submit },
              { key: "Enter", run: insertNewlineContinueMarkup },
              { key: "Backspace", run: deleteMarkupBackward },
              {
                key: "Tab",
                run: (v) => indentListItem(v) || indentMore(v),
                shift: (v) => outdentListItem(v) || indentLess(v),
              },
            ]),
          ),
          markdown({ addKeymap: false }),
          // Real editor image paste/drop: pasted images upload into the draft
          // folder and insert ![](path); the draft text (with the refs) rides
          // the prompt frame on submit, and the agent reads the files.
          imageDropHandlers({
            getUploadDir: () => draftDir(),
            getCurrentPath: () => draftPath,
          }),
          EditorView.lineWrapping,
          theme.extension,
          EditorView.updateListener.of((u) => {
            if (u.docChanged) scheduleWrite();
          }),
        ],
      });
      view = new EditorView({ state, parent: host });
      view.focus();
    })();
  });

  onDestroy(() => {
    destroyed = true;
    // Persist the latest text on hide/unmount (reads the doc before destroy).
    void flushWrite();
    view?.destroy();
  });

  // Track surface-theme flips so the bubble's markdown highlight follows.
  $effect(() => {
    const t = effectiveHybridSurfaceTheme("terminal");
    if (view) theme.reconfigure(view, t);
  });

  // The tallest the bubble may grow: the positioning context (.terminal-tab)
  // height minus the top + bottom 12px insets, so the top stops at the
  // terminal top with the same margin as the bottom.
  function maxPromptHeight(): number {
    const parent = rootEl?.offsetParent as HTMLElement | null;
    const ph = parent?.clientHeight ?? window.innerHeight;
    return Math.max(MIN_PROMPT_HEIGHT, ph - 24);
  }

  // Top-edge resize: dragging UP (smaller clientY) grows the bubble, which is
  // anchored at the bottom, so it extends toward the terminal top. Pointer
  // capture keeps the drag alive outside the thin handle.
  function onResizeStart(e: PointerEvent): void {
    if (!rootEl) return;
    resizing = true;
    resizeStartY = e.clientY;
    resizeStartHeight = rootEl.offsetHeight;
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    e.preventDefault();
  }
  function onResizeMove(e: PointerEvent): void {
    if (!resizing) return;
    const next = resizeStartHeight + (resizeStartY - e.clientY);
    customHeight = Math.min(maxPromptHeight(), Math.max(MIN_PROMPT_HEIGHT, next));
  }
  function onResizeEnd(e: PointerEvent): void {
    if (!resizing) return;
    resizing = false;
    try {
      (e.currentTarget as HTMLElement).releasePointerCapture(e.pointerId);
    } catch {
      // capture may already be released; ignore.
    }
  }

  function onKeydown(e: KeyboardEvent): void {
    // Escape hides the bubble (returns the user to the terminal). Stop it from
    // reaching App.svelte's global Escape handling.
    if (e.key === "Escape") {
      e.stopPropagation();
      e.preventDefault();
      hideRichPromptForTab(tab.id);
    }
  }
</script>

<!-- The container-level keydown is an Escape trap for the bubble, not an
     interactive control; role="group" labels the region for AT. -->
<!-- svelte-ignore a11y_no_static_element_interactions, a11y_no_noninteractive_element_interactions -->
<div
  class="rich-prompt"
  class:resized={customHeight !== null}
  role="group"
  aria-label="Rich Prompt"
  bind:this={rootEl}
  style:height={customHeight !== null ? `${customHeight}px` : null}
  onkeydown={onKeydown}
>
  <!-- Top-edge grab handle: drag up to grow the bubble toward the terminal
       top (mirrors the bottom inset). -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="rp-resize"
    role="separator"
    aria-orientation="horizontal"
    aria-label="Resize Rich Prompt"
    onpointerdown={onResizeStart}
    onpointermove={onResizeMove}
    onpointerup={onResizeEnd}
    onpointercancel={onResizeEnd}
  ></div>
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
    /* Safety cap so a stale custom height (e.g. after the terminal shrank)
       still leaves the same 12px inset at the top as the bottom. */
    max-height: calc(100% - 24px);
    z-index: 20;
    display: flex;
    flex-direction: column;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: 10px;
    box-shadow: 0 6px 24px rgba(0, 0, 0, 0.28);
    overflow: hidden;
  }
  /* The thin top-edge grab strip; the whole strip is the ns-resize target. */
  .rp-resize {
    flex: 0 0 auto;
    height: 8px;
    cursor: ns-resize;
    touch-action: none;
  }
  .rp-resize::before {
    content: "";
    display: block;
    width: 28px;
    height: 3px;
    margin: 2px auto 0;
    border-radius: 2px;
    background: var(--border);
  }
  .rp-editor {
    min-height: 2.4em;
    max-height: 32vh;
    overflow-y: auto;
    padding: 8px 10px;
  }
  /* When the user has pinned a height, the editor fills the bubble and scrolls
     within it (the content-driven max-height no longer applies). */
  .rich-prompt.resized .rp-editor {
    flex: 1 1 auto;
    min-height: 0;
    max-height: none;
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
