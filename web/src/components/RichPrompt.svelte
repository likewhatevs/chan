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
  import { Compartment, EditorState, Prec } from "@codemirror/state";
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
    beginPendingPrompt,
    failPendingPrompt,
    sendCancelToTerminal,
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

  // ---- Pending-message state machine (queue visibility) -----------------
  // The in-flight message itself lives on the TAB (tab.pendingPrompt), so it
  // survives hide/show of the bubble; this component owns only presentation
  // state (chip grace, transient notes) and the editor lock.
  //
  // Fast-path grace: an idle agent consumes a submit within ~1 drainer tick,
  // so the "queued" chip only appears after PENDING_CHIP_GRACE_MS — no label
  // flash on routine submits. The lock applies immediately (the bytes are
  // queued; edits would desync what the agent will read).
  const PENDING_CHIP_GRACE_MS = 300;
  // No prompt-ack within this window means the socket is effectively dead
  // even if not yet closed: fail the pending (unlock, keep text).
  const PROMPT_ACK_TIMEOUT_MS = 5000;
  // How long the transient rejected/failed note replaces the idle label.
  const TRANSIENT_NOTE_MS = 5000;

  let pendingChipVisible = $state(false);
  let transientNote = $state<string | null>(null);
  let graceTimer: ReturnType<typeof setTimeout> | null = null;
  let ackTimer: ReturnType<typeof setTimeout> | null = null;
  let noteTimer: ReturnType<typeof setTimeout> | null = null;
  // A `cancel-prompt` recall is in flight (ArrowUp pressed; awaiting the
  // `prompt-cancelled` ack). Guards against double-sending while the editor
  // stays locked through the round-trip. Cleared when a terminal phase
  // resolves (consumeTerminalPhase).
  let recallInFlight = false;

  // Read-only while a message is in flight ("sent"/"queued"); terminal
  // phases are consumed (cleared) by the phase effect below, so they never
  // hold the lock.
  const isPending = $derived.by(() => {
    const phase = tab.pendingPrompt?.phase;
    return phase === "sent" || phase === "queued";
  });

  // CodeMirror lock seam. While a message is in flight ("sent"/"queued") the
  // editor is readOnly (transactions that change the doc are dropped), but it
  // stays EDITABLE (contenteditable) so the caret/keymap remain live — that is
  // what lets ArrowUp-at-doc-start trigger recall while queued (GAP 1) and
  // keeps focus through the pending round-trip. readOnly alone blocks typing,
  // so the previous editable:false was redundant insurance; dropping it is what
  // makes recall reliable (an editable:false editor doesn't receive the key).
  const lockCompartment = new Compartment();
  function lockExtensions(locked: boolean) {
    return [EditorState.readOnly.of(locked), EditorView.editable.of(true)];
  }
  $effect(() => {
    view?.dispatch({ effects: lockCompartment.reconfigure(lockExtensions(isPending)) });
  });

  const labelText = $derived.by(() => {
    const pending = tab.pendingPrompt;
    if (pending && isPending && pendingChipVisible) {
      const position =
        pending.phase === "queued" && (pending.depth ?? 0) > 1 ? ` (#${pending.depth})` : "";
      return `queued — waiting for agent${position}`;
    }
    if (transientNote) return transientNote;
    // Teammate pokes (`cs terminal write`) share the queue: surface the
    // depth in the idle label so the user sees them from the prompt itself.
    if ((tab.queueDepth ?? 0) > 0) return `${tab.queueDepth} queued · ${submitLabel}`;
    return submitLabel;
  });

  function clearPendingTimers(): void {
    if (graceTimer !== null) clearTimeout(graceTimer);
    if (ackTimer !== null) clearTimeout(ackTimer);
    graceTimer = null;
    ackTimer = null;
  }

  function showTransientNote(text: string): void {
    transientNote = text;
    if (noteTimer !== null) clearTimeout(noteTimer);
    noteTimer = setTimeout(() => {
      noteTimer = null;
      transientNote = null;
    }, TRANSIENT_NOTE_MS);
  }

  // Consume a terminal phase: unlock + clear tab.pendingPrompt. Runs from
  // the phase effect (live transitions) and once post-mount (a phase that
  // resolved while the bubble was hidden — the effect's first run sees it
  // before the editor view exists, so it defers via the !view guard).
  function consumeTerminalPhase(
    phase: "delivered" | "rejected" | "failed" | "recalled" | "drained",
  ): void {
    if (!view) return;
    clearPendingTimers();
    pendingChipVisible = false;
    recallInFlight = false;
    if (phase === "delivered") {
      // The agent consumed the message: NOW clear the composer + the draft
      // (the draft held the text the whole time it was queued).
      view.dispatch({
        changes: { from: 0, to: view.state.doc.length, insert: "" },
      });
      void flushWrite();
      view.focus();
    } else if (phase === "recalled") {
      // Recall succeeded: the still-queued message was pulled before the PTY.
      // KEEP the draft text and just unlock (clearing pendingPrompt below drops
      // the lock) so the user edits and resubmits with a fresh id — no
      // double-delivery, since the original is gone from the queue.
      view.focus();
    } else if (phase === "drained") {
      // Recall raced a drain: the message already hit the PTY. Surface it and
      // clear the composer like a normal delivery — do NOT silently re-edit a
      // message that was already sent.
      showTransientNote("already sent — too late to recall");
      view.dispatch({
        changes: { from: 0, to: view.state.doc.length, insert: "" },
      });
      void flushWrite();
      view.focus();
    } else if (phase === "rejected") {
      // Queue full, nothing enqueued: keep the text for a retry.
      showTransientNote("queue full — try again");
    } else {
      // WS close / ack timeout / session end: the message may still be
      // queued server-side, but this client can't observe delivery anymore.
      // Keep the text; a resubmit is a visible, recoverable duplicate.
      showTransientNote("connection lost — message may still be queued");
    }
    tab.pendingPrompt = undefined;
  }

  $effect(() => {
    const phase = tab.pendingPrompt?.phase;
    if (phase === "queued") {
      // Acked: the ack timeout is done; the grace timer keeps gating the chip.
      if (ackTimer !== null) clearTimeout(ackTimer);
      ackTimer = null;
    } else if (
      phase === "delivered" ||
      phase === "rejected" ||
      phase === "failed" ||
      phase === "recalled" ||
      phase === "drained"
    ) {
      consumeTerminalPhase(phase);
    }
  });

  // Recall a still-queued message to edit it (GAP 1 "press up to edit"). Only
  // while QUEUED (acked but not yet drained); "sent" (pre-ack) can't be safely
  // cancelled by id yet, and a terminal phase has nothing to recall. Sends
  // `cancel-prompt`; the editor stays locked until the `prompt-cancelled` ack
  // resolves to "recalled" (unlock + keep text) or "drained" (already sent).
  // Returns true to consume the ArrowUp; false lets it move the caret.
  function recall(view: EditorView): boolean {
    const pending = tab.pendingPrompt;
    if (!pending || pending.phase !== "queued") return false;
    // Only at the very start of the doc (empty selection at offset 0), so
    // ArrowUp still navigates within a multi-line draft.
    const sel = view.state.selection.main;
    if (!sel.empty || sel.from !== 0) return false;
    if (recallInFlight) return true;
    if (!sendCancelToTerminal(tab.id, pending.id)) {
      // Socket down: can't recall. Surface it; the message may still be queued.
      showTransientNote("connection lost — can't recall");
      return true;
    }
    recallInFlight = true;
    return true;
  }

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

  // Cmd+Enter: submit the draft text through the queue and KEEP it visible
  // (read-only) until the server reports the message's last write reached
  // the PTY — the prompt-delivered resolution clears the composer + the
  // draft text (folder + pasted media stay; the agent reads the media AFTER
  // delivery; the folder is cleaned on terminal close). Always returns true
  // so the chord never inserts a newline; empty/whitespace is swallowed, and
  // a second Cmd+Enter while a message is in flight is a no-op (replace
  // would need cancel-by-id; deferred together, v2).
  //
  // Routes to THIS bubble's OWN terminal (`tab`), NOT the focused pane's active
  // terminal: the bubble belongs to `tab`, so its text must land there. And we
  // do NOT begin a pending unless the `prompt` frame actually went out to
  // this terminal's OPEN socket (sendPromptToTerminal returns false on a
  // closed / not-yet-connected socket). That is a real data-loss
  // guard: keeping the text on a failed send lets the user retry instead of
  // losing it.
  function submit(): boolean {
    if (!view) return true;
    if (tab.pendingPrompt) return true;
    const text = view.state.doc.toString();
    if (!text.trim()) return true;
    const id = crypto.randomUUID();
    // Persist exactly what is being submitted: the draft holds the text
    // while it is queued, so a reload mid-pending restores it.
    void flushWrite();
    if (!sendPromptToTerminal(tab.id, text, submitAgent(), id)) return true;
    beginPendingPrompt(tab, id);
    pendingChipVisible = false;
    graceTimer = setTimeout(() => {
      graceTimer = null;
      pendingChipVisible = true;
    }, PENDING_CHIP_GRACE_MS);
    ackTimer = setTimeout(() => {
      ackTimer = null;
      failPendingPrompt(tab);
    }, PROMPT_ACK_TIMEOUT_MS);
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
          // Locked from creation when a message is already in flight (the
          // bubble was hidden mid-pending, or a reload restored a queued
          // message; the pending lives on the tab).
          lockCompartment.of(lockExtensions(isPending)),
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
              // ArrowUp at doc-start while queued recalls the message to edit
              // (GAP 1). Off doc-start / not queued it returns false and falls
              // through to default caret movement.
              { key: "ArrowUp", run: recall },
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
      // Catch up with the tab's pending state machine. A terminal phase
      // that resolved while the bubble was hidden (no component to consume
      // it) is applied now that the view + draft exist — e.g. "delivered"
      // clears the already-consumed text instead of re-showing it. The
      // phase $effect's first run saw it before the view existed and
      // deferred (the !view guard).
      const phase = tab.pendingPrompt?.phase;
      if (phase === "delivered" || phase === "rejected" || phase === "failed") {
        consumeTerminalPhase(phase);
      } else if (phase === "sent" || phase === "queued") {
        // Still in flight: show the chip immediately (it has been pending
        // at least one hide/show round-trip; the grace window is for the
        // routine submit-while-open fast path) and re-arm the ack guard
        // for an unacked send.
        pendingChipVisible = true;
        if (phase === "sent" && ackTimer === null) {
          ackTimer = setTimeout(() => {
            ackTimer = null;
            failPendingPrompt(tab);
          }, PROMPT_ACK_TIMEOUT_MS);
        }
      }
    })();
  });

  onDestroy(() => {
    destroyed = true;
    // Presentation timers die with the component; the pending itself lives
    // on the tab (TerminalTab's WS handler keeps resolving it while the
    // bubble is hidden, and the next mount catches up).
    clearPendingTimers();
    if (noteTimer !== null) clearTimeout(noteTimer);
    noteTimer = null;
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
  class:pending={isPending}
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
  <div class="rp-editor" data-file-drop-zone bind:this={host}></div>
  <div class="rp-label" class:queued={isPending && pendingChipVisible} aria-hidden="true">
    {labelText}
  </div>
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
  /* In-flight message: the text stays visible but clearly inert (read-only
     is enforced in the editor state; the dim is the affordance). */
  .rich-prompt.pending .rp-editor {
    opacity: 0.55;
  }
  .rich-prompt.pending .rp-label.queued {
    color: var(--text-primary);
  }
</style>
