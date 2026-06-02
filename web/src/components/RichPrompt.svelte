<script lang="ts">
  // Rich Prompt: a floating, inset, rounded, button-less markdown bubble over
  // the BOTTOM of the active terminal. The only chrome is a "submit with
  // cmd+enter" label. ENTER inserts a newline (keep editing); CMD+ENTER (Mod-
  // Enter) submits to the active terminal's write queue via the WS `prompt`
  // frame (NOT the raw keystroke path).
  //
  // Drafts-backed (@@Host): the bubble edits a real per-terminal chan-workspace
  // DRAFT (`tab.richPromptDraftPath` -> `Drafts/<name>/draft.md`). That file IS
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
  import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
  import {
    deleteMarkupBackward,
    insertNewlineContinueMarkup,
    markdown,
  } from "@codemirror/lang-markdown";
  import { imageDropHandlers } from "../editor/bubbles/image_drop";
  import { makeThemeCompartment } from "../editor/base";
  import { effectiveHybridSurfaceTheme } from "../state/store.svelte";
  import { currentOS } from "../state/shortcuts";
  import { hideRichPrompt } from "../state/richPrompt.svelte";
  import {
    sendPromptToActiveTerminal,
    type TerminalTab,
  } from "../state/tabs.svelte";
  import { api } from "../api/client";

  // The terminal this bubble belongs to (TerminalTab mounts one per active
  // terminal). Its `richPromptDraftPath` is the per-terminal draft backing.
  let { tab }: { tab: TerminalTab } = $props();

  let host = $state<HTMLDivElement>();
  let view: EditorView | undefined;
  let destroyed = false;
  let draftPath = "";
  let writeTimer: ReturnType<typeof setTimeout> | null = null;
  const theme = makeThemeCompartment(effectiveHybridSurfaceTheme("terminal"));

  const submitLabel =
    currentOS() === "mac" ? "submit with cmd+enter" : "submit with ctrl+enter";

  // Directory holding the draft (images upload here). "Drafts/x/draft.md" -> "Drafts/x".
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

  // Cmd+Enter: submit the draft text through the queue, then RESET = clear
  // draft.md TEXT but KEEP the folder + any pasted media (the agent reads the
  // media AFTER submit; the folder is cleaned on terminal close). Always
  // returns true so the chord never inserts a newline; empty/whitespace is
  // swallowed; a failed send keeps the text.
  function submit(): boolean {
    if (!view) return true;
    const text = view.state.doc.toString();
    if (!text.trim()) return true;
    if (!sendPromptToActiveTerminal(text)) return true;
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
          // Backspace dedents markup. markdown({addKeymap:false}) keeps syntax
          // only so these bindings own Enter/Backspace unambiguously.
          Prec.high(
            keymap.of([
              { key: "Mod-Enter", run: submit },
              { key: "Enter", run: insertNewlineContinueMarkup },
              { key: "Backspace", run: deleteMarkupBackward },
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
