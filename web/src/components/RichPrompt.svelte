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
  import { rewriteImagePathsForDelivery } from "../editor/deliver_images";
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
  // True while a just-submitted message is still in flight ("sent"/"queued").
  // It presents the GREYED, read-only "queued" card (@@Alex's model): the
  // submitted text stays visible, dimmed, caret hidden, and the editor is
  // read-only so the bytes the agent will read can't be edited out from under
  // it. Back-to-back is reconciled NOT by dropping the lock (that loses the
  // card) but by `beforeinput` "type to move on": the first character clears the
  // card to a fresh editable composer (the queued message stays in the server
  // FIFO). So the lock never STICKS — it exits the instant you type, recall, or
  // the message drains.
  const isPending = $derived.by(() => {
    const phase = tab.pendingPrompt?.phase;
    return phase === "sent" || phase === "queued";
  });

  // The last message submitted into the queue, kept so ArrowUp can recall it
  // (pull it back out of the queue to edit) and Esc can drop it.
  let lastQueued: { id: string; text: string } | null = null;

  // CodeMirror read-only lock seam. While a message is in flight the editor is
  // readOnly (programmatic editing commands — Enter-continue-markup, Backspace,
  // Tab-indent — respect it and no-op) but stays EDITABLE (contenteditable) so
  // the caret/keymap remain live: ArrowUp-recall + the `beforeinput` move-on
  // still fire. `EditorState.readOnly` does not block programmatic
  // `view.dispatch`, so move-on/recall/delivered can still clear or seed the
  // doc; only user editing of the locked card is blocked.
  const lockCompartment = new Compartment();
  function lockExtensions(locked: boolean) {
    return [EditorState.readOnly.of(locked), EditorView.editable.of(true)];
  }
  $effect(() => {
    view?.dispatch({ effects: lockCompartment.reconfigure(lockExtensions(isPending)) });
  });

  // Queued count to surface in the label: the server queue depth (which a
  // teammate `cs terminal write` shares too) plus the just-submitted local
  // message, the latter only after the grace window so a fast-drained submit
  // does not flash the count.
  const queuedCount = $derived(
    Math.max(tab.queueDepth ?? 0, isPending && pendingChipVisible ? 1 : 0),
  );
  const labelText = $derived.by(() => {
    if (transientNote) return transientNote;
    // Greyed card up: the affordances ARE the chrome (the card has no buttons).
    // ↑ edits the queued message, Esc drops it. Otherwise, when messages sit in
    // the queue but the card is gone (you moved on / a teammate poked), show the
    // depth + the recall hint; idle shows the submit hint.
    if (isPending) return `${queuedCount} queued · ↑ edit · esc cancel`;
    if (queuedCount > 0) return `${queuedCount} queued · ↑ recall · ${submitLabel}`;
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
  function consumeTerminalPhase(phase: "delivered" | "rejected" | "failed"): void {
    if (!view) return;
    clearPendingTimers();
    pendingChipVisible = false;
    if (phase === "delivered") {
      // The agent consumed the message: clear the greyed card (text + draft) AND
      // re-enable editing in the SAME transaction, instead of leaning on the
      // out-of-band lock $effect to unlock after the fact. Then defer view.focus()
      // to a microtask so it lands AFTER that $effect's trailing reconfigure: a
      // programmatic (no user gesture) focus during the readOnly->editable flip is
      // otherwise dropped by the following transaction, leaving the just-cleared
      // composer un-typeable in WKWebView until a hide/show remount. (Mirrors the
      // beforeinput move-on path's folded unlock + TerminalTab's deferred focus.)
      // The media folder stays; the agent reads it after delivery, cleaned on
      // terminal close.
      view.dispatch({
        changes: { from: 0, to: view.state.doc.length, insert: "" },
        effects: lockCompartment.reconfigure(lockExtensions(false)),
      });
      void flushWrite();
      lastQueued = null;
      queueMicrotask(() => view?.focus());
    } else {
      // Rejected (queue full) / failed (socket dead, delivery unobservable):
      // the text is still in the card. Clearing the pending below un-greys it,
      // so the text is editable again for a retry; warn honestly.
      showTransientNote(
        phase === "rejected"
          ? "queue full — try again"
          : "connection lost — message may still be queued",
      );
    }
    tab.pendingPrompt = undefined;
  }

  $effect(() => {
    const phase = tab.pendingPrompt?.phase;
    if (phase === "queued") {
      // Acked: the ack timeout is done; the grace timer keeps gating the chip.
      if (ackTimer !== null) clearTimeout(ackTimer);
      ackTimer = null;
    } else if (phase === "delivered" || phase === "rejected" || phase === "failed") {
      consumeTerminalPhase(phase);
    }
  });

  // ↑ recalls the LAST queued message to edit + resubmit — pulls it back out of
  // the server queue. Two cases:
  //  - the GREYED card is up: its text is already in the composer, so just
  //    un-grey (un-lock) it and best-effort cancel the server-side message.
  //  - the card is gone (moved on / reload) but a message is still recallable:
  //    only from an EMPTY composer (never clobber an in-progress next draft),
  //    restore the buffered text + cancel.
  // The server cancel is best-effort (a drained message just resubmits as a
  // visible, recoverable duplicate). Returns false (lets ArrowUp move the caret)
  // when the composer is non-empty and not greyed, or nothing is recallable.
  function recall(): boolean {
    if (!view) return false;
    if (isPending) {
      if (lastQueued) sendCancelToTerminal(tab.id, lastQueued.id);
      lastQueued = null;
      enterLocalEdit();
      // Un-grey the card in the SAME transaction — the text is already shown, so
      // keep it and drop the caret at the end — instead of leaning on the
      // out-of-band lock $effect to unlock after the fact. That flip lands a
      // synchronous view.focus() while the editor is still readOnly, which
      // WKWebView drops, leaving the card un-typeable until a remount (the same
      // failure consumeTerminalPhase's delivered path folds away). Defer focus to
      // a microtask so it lands AFTER the $effect's trailing reconfigure.
      view.dispatch({
        selection: { anchor: view.state.doc.length },
        effects: lockCompartment.reconfigure(lockExtensions(false)),
      });
      queueMicrotask(() => view?.focus());
      return true;
    }
    if (view.state.doc.length > 0 || !lastQueued) return false;
    const { id, text } = lastQueued;
    lastQueued = null;
    sendCancelToTerminal(tab.id, id);
    view.dispatch({ changes: { from: 0, to: 0, insert: text } });
    void flushWrite();
    view.focus();
    return true;
  }

  // Clear the in-flight pending + reset the queued-state presentation (the
  // composer text is untouched). Shared by ↑ (recall) and the Escape abandon.
  function enterLocalEdit(): void {
    clearPendingTimers();
    if (noteTimer !== null) {
      clearTimeout(noteTimer);
      noteTimer = null;
    }
    pendingChipVisible = false;
    transientNote = null;
    tab.pendingPrompt = undefined;
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

  // Cmd+Enter: submit the draft text through the queue and KEEP it visible as a
  // GREYED, read-only card — the submitted bytes stay on screen, dimmed, so the
  // user sees exactly what is queued. `prompt-delivered` clears the card (the
  // composer returns to empty + editable); the media folder stays (the agent
  // reads it after delivery; cleaned on terminal close). Always returns true so
  // the chord never inserts a newline; empty/whitespace is swallowed; and a
  // second Cmd+Enter WHILE the card is up is a no-op (the shown text is already
  // queued — to queue another, TYPE to move on, then submit). Back-to-back
  // type->submit->type->submit therefore works via the `beforeinput` move-on,
  // without the old stuck-read-only bug.
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
    // Card up: the shown text is already queued. Don't re-submit it (that would
    // double-deliver); the user types to move on or ArrowUp to edit.
    if (isPending) return true;
    const text = view.state.doc.toString();
    if (!text.trim()) return true;
    const id = crypto.randomUUID();
    // The composer's pasted-image refs are relative to the DRAFT file (so the
    // in-compose preview resolves them); the receiving agent runs at $CWD =
    // workspace root, so deliver them workspace-rooted or it 404s the image.
    // The draft/card text stays as-is — only the wire payload is rewritten.
    const delivered = rewriteImagePathsForDelivery(text, draftPath);
    if (!sendPromptToTerminal(tab.id, delivered, submitAgent(), id)) return true;
    // Queued. KEEP the text in the composer as the greyed read-only card (the
    // lock $effect greys it the moment `isPending` flips). Persist it so a
    // reload restores the card, and remember it for ArrowUp recall / Esc drop.
    lastQueued = { id, text };
    beginPendingPrompt(tab, id);
    void flushWrite();
    view.focus();
    pendingChipVisible = false;
    if (graceTimer !== null) clearTimeout(graceTimer);
    graceTimer = setTimeout(() => {
      graceTimer = null;
      pendingChipVisible = true;
    }, PENDING_CHIP_GRACE_MS);
    if (ackTimer !== null) clearTimeout(ackTimer);
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
          // message; the pending lives on the tab). The $effect keeps it in
          // sync after mount.
          lockCompartment.of(lockExtensions(isPending)),
          // Type to move on: while the greyed card is up, the editor is
          // read-only, so a user TEXT input means "I'm done looking at the
          // queued message, start the next one". Intercept it BEFORE
          // CodeMirror drops it: un-grey (un-lock) the composer, clear the card,
          // and seed it with what was typed/pasted — the queued message stays in
          // the server FIFO. Non-text input (delete, etc.) is swallowed (the
          // card is read-only; use ArrowUp to edit it, Esc to drop it).
          EditorView.domEventHandlers({
            beforeinput: (event, v) => {
              if (!isPending) return false;
              event.preventDefault();
              const seeds = [
                "insertText",
                "insertReplacementText",
                "insertFromPaste",
                "insertFromDrop",
                "insertCompositionText",
              ];
              if (seeds.includes(event.inputType) && event.data) {
                const seed = event.data;
                enterLocalEdit();
                v.dispatch({
                  changes: { from: 0, to: v.state.doc.length, insert: seed },
                  selection: { anchor: seed.length },
                  effects: lockCompartment.reconfigure(lockExtensions(false)),
                });
                void flushWrite();
                v.focus();
              }
              return true;
            },
          }),
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
              // ArrowUp recalls the queued message to edit: from the greyed card
              // (un-grey it), or from an empty composer (restore the buffer).
              // Otherwise it returns false and falls through to caret movement.
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
        // routine submit-while-open fast path). `isPending` is already true, so
        // the lock $effect has greyed the card; the restored draft content IS
        // the card text. Reload re-hydration: remember it so ArrowUp can edit
        // and Esc can drop the reproved-queued message after a reload (the
        // server `session` frame's queued_prompt_ids reproved it via
        // reproveRestoredPrompt). Re-arm the ack guard for an unacked send.
        pendingChipVisible = true;
        if (content.trim()) {
          lastQueued = { id: tab.pendingPrompt!.id, text: content };
        }
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
    if (e.key !== "Escape") return;
    // Stop it from reaching App.svelte's global Escape handling.
    e.stopPropagation();
    e.preventDefault();
    // Dequeue-if-enqueued else abandon: the greyed card up (or an empty composer
    // with a still-queued message) → drop that queued message (cancel + clear,
    // the counterpart to ↑ which edits it), keeping the bubble open for the next
    // message. Otherwise (an editable draft in the composer, nothing queued) →
    // abandon the current draft + hide.
    if (view && lastQueued && (isPending || view.state.doc.length === 0)) {
      sendCancelToTerminal(tab.id, lastQueued.id);
      lastQueued = null;
      enterLocalEdit();
      view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: "" } });
      void flushWrite();
      return;
    }
    abandonDraft();
  }

  // Clear the composer + discard the persisted draft, then hide the bubble.
  function abandonDraft(): void {
    if (view) {
      view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: "" } });
    }
    void flushWrite();
    hideRichPromptForTab(tab.id);
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
  <div class="rp-label" class:queued={queuedCount > 0} aria-hidden="true">
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
  /* Queued indicator: highlight the label when messages sit in the queue. */
  .rp-label.queued {
    color: var(--text-primary);
  }
  /* Greyed read-only "queued" card (@@Alex's model): the submitted text stays
     visible but dimmed, with the caret hidden, so it reads as queued-not-
     editable. The editor is `EditorState.readOnly` underneath; typing exits via
     `beforeinput` move-on. The opacity + the hidden caret are the only visual
     affordance (the keymap stays live so ArrowUp-edit / Esc-drop work). */
  .rich-prompt.pending .rp-editor {
    opacity: 0.55;
  }
  .rich-prompt.pending .rp-editor :global(.cm-content) {
    caret-color: transparent;
  }
</style>
