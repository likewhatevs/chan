<script lang="ts">
  // Fullscreen side-by-side diff for a pending assistant edit.
  // Mounts a CM6 MergeView with the file's current content on the
  // left (read-only) and the assistant's proposal on the right
  // (read-only by default; the user reviews, then Applies or
  // Discards at the bottom). Markdown highlighting lives in both
  // panes so headings / code / links read the same way they would
  // in the editor.
  //
  // Opened from the edit card's "Diff" button via openDiffOverlay
  // in the store; closed via Esc, the X button, the backdrop, or
  // after Apply / Discard.

  import { onDestroy } from "svelte";
  import { EditorState } from "@codemirror/state";
  import { EditorView } from "@codemirror/view";
  import { MergeView } from "@codemirror/merge";
  import { markdown } from "@codemirror/lang-markdown";
  import {
    closeDiffOverlay,
    diffOverlay,
    ui,
  } from "../state/store.svelte";

  /// Container the MergeView mounts into. Bound once, populated by
  /// the $effect below whenever the overlay opens with a new edit
  /// (or the user's theme flips between light / dark; the merge
  /// view inherits from `ui.theme` via a data attribute on the
  /// host so we don't have to rebuild on toggle).
  let host: HTMLDivElement | undefined = $state();
  let panelEl: HTMLDivElement | undefined = $state();
  let mv: MergeView | null = null;

  /// Focus the panel as soon as the overlay opens so Esc lands in
  /// this component's onkeydown handler instead of bubbling up
  /// through `document` and hitting InlineAssist.onWindowKey,
  /// which would call close() and auto-dismiss the pending edit
  /// the user is just trying to review.
  $effect(() => {
    if (diffOverlay.open) {
      queueMicrotask(() => panelEl?.focus());
    }
  });

  /// (Re)build the MergeView whenever the underlying edit changes.
  /// Reading `diffOverlay.edit?.content`, `.original`, and `host`
  /// makes Svelte's reactivity re-run this on any of those flips.
  $effect(() => {
    const edit = diffOverlay.edit;
    const original = diffOverlay.original;
    if (!host || !edit) {
      destroyMergeView();
      return;
    }
    if (diffOverlay.loading) return;
    destroyMergeView();
    mv = new MergeView({
      a: {
        doc: original,
        extensions: [
          EditorView.editable.of(false),
          EditorState.readOnly.of(true),
          EditorView.lineWrapping,
          markdown(),
        ],
      },
      b: {
        doc: edit.content,
        extensions: [
          EditorView.editable.of(false),
          EditorState.readOnly.of(true),
          EditorView.lineWrapping,
          markdown(),
        ],
      },
      parent: host,
      // Reverting from the proposal side back to the original
      // doesn't make sense for our flow (proposal is the model's
      // suggestion; user accepts or rejects the whole). Hide the
      // revert gutters.
      revertControls: undefined,
    });
  });

  function destroyMergeView(): void {
    if (mv) {
      mv.destroy();
      mv = null;
    }
  }

  onDestroy(destroyMergeView);

  // Imported lazily to avoid a circular import: applyEdit /
  // dismissEdit live in InlineAssist's local module. We
  // dispatch a `chan:assistant-edit-action` event the
  // InlineAssist instance listens for, so the overlay stays
  // decoupled from that component's internals.
  type DiffAction = "apply" | "dismiss" | "save-as" | "copy";
  function dispatchAction(action: DiffAction): void {
    const edit = diffOverlay.edit;
    if (!edit) return;
    window.dispatchEvent(
      new CustomEvent("chan:assistant-edit-action", {
        detail: { action, toolCallId: edit.toolCallId },
      }),
    );
  }

  function onApply(): void {
    dispatchAction("apply");
    // Close immediately for snappy feedback; the actual write
    // happens via the listener in InlineAssist.
    closeDiffOverlay();
  }

  function onKey(e: KeyboardEvent): void {
    if (e.key === "Escape") {
      // Stop propagation so the document-level keydown in
      // InlineAssist doesn't also fire its own close(), which
      // would auto-dismiss the pending edit we just decided not
      // to accept yet. Closing the diff overlay should always be
      // a no-op on the underlying proposal — the user can come
      // back to the chat and Apply / Discard from there.
      e.preventDefault();
      e.stopPropagation();
      e.stopImmediatePropagation();
      closeDiffOverlay();
    }
  }
</script>

{#if diffOverlay.open}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="diff-overlay" onclick={closeDiffOverlay} onkeydown={onKey} data-theme={ui.theme}>
    <div
      class="diff-panel"
      bind:this={panelEl}
      onclick={(e) => e.stopPropagation()}
      role="dialog"
      aria-modal="true"
      aria-label="proposed edit diff"
      tabindex="-1"
    >
      <header>
        <div class="title">
          <span class="kind">DIFF</span>
          <span class="path mono">{diffOverlay.path}</span>
          {#if diffOverlay.edit}
            <span class="size">{diffOverlay.edit.content.length} chars</span>
          {/if}
        </div>
        <div class="legend">
          <span class="legend-l">current</span>
          <span class="legend-sep">→</span>
          <span class="legend-r">proposal</span>
        </div>
        <button class="x" onclick={closeDiffOverlay} title="close (Esc)" aria-label="close">×</button>
      </header>
      {#if diffOverlay.error}
        <div class="err">couldn't read current content: {diffOverlay.error}</div>
      {/if}
      {#if diffOverlay.loading}
        <div class="loading">loading current content…</div>
      {/if}
      <div class="merge-host" bind:this={host}></div>
      {#if diffOverlay.edit && diffOverlay.edit.status === "pending"}
        <footer class="actions">
          <!-- Diff review is read-only: just Accept (apply now) or
               Close (back to the chat). All the other actions —
               Copy / Save as… / Discard — live on the chat bubble
               so the user can pick them after a quick review,
               without the diff overlay competing with PathPrompt
               modal z-indexes or surfacing redundant chrome here. -->
          <button type="button" class="primary" onclick={onApply}>Accept</button>
          <button type="button" onclick={closeDiffOverlay}>Close</button>
        </footer>
      {:else if diffOverlay.edit}
        <footer class="actions status-only">
          <span class="status-tag" class:ok={diffOverlay.edit.status === "applied"}>
            {diffOverlay.edit.status === "applied" ? "accepted" : diffOverlay.edit.status}
          </span>
        </footer>
      {/if}
    </div>
  </div>
{/if}

<style>
  /* Fullscreen scrim above every other overlay (the assistant
     OverlayShell sits at ~25002; this needs to win). */
  .diff-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.55);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 27000;
  }
  .diff-panel {
    background: var(--bg);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: 8px;
    box-shadow: 0 12px 36px rgba(0, 0, 0, 0.45);
    width: 95vw;
    height: 92vh;
    max-width: 1800px;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  header {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 14px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-card);
  }
  header .title {
    display: flex;
    align-items: baseline;
    gap: 8px;
    flex: 1;
    min-width: 0;
  }
  header .kind {
    background: var(--link);
    color: #fff;
    padding: 1px 6px;
    border-radius: 3px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    font-weight: 600;
    font-size: 12px;
  }
  header .path {
    font-family: ui-monospace, monospace;
    color: var(--text);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  header .size {
    color: var(--text-secondary);
    font-variant-numeric: tabular-nums;
  }
  header .legend {
    display: flex;
    gap: 6px;
    color: var(--text-secondary);
    font-size: 13px;
  }
  header .legend-l { color: var(--text-secondary); }
  header .legend-r { color: var(--text); font-weight: 600; }
  header .x {
    background: transparent;
    border: 1px solid var(--btn-border);
    color: var(--text);
    border-radius: 3px;
    width: 28px;
    height: 24px;
    font-size: 18px;
    line-height: 1;
    cursor: pointer;
  }
  header .x:hover { border-color: var(--btn-hover); }
  .err {
    padding: 6px 14px;
    color: #d33;
    font-size: 13px;
    border-bottom: 1px solid var(--border);
  }
  .loading {
    padding: 6px 14px;
    color: var(--text-secondary);
    font-size: 13px;
    border-bottom: 1px solid var(--border);
  }
  .merge-host {
    flex: 1;
    min-height: 0;
    overflow: auto;
    background: var(--bg);
  }
  /* CM6 merge view gets a thin neutral chrome; the heavy work is
     in the gutters which the library paints itself. We tighten
     the line-height a touch so a long file fits more on screen. */
  .merge-host :global(.cm-mergeView) {
    height: 100%;
  }
  .merge-host :global(.cm-editor) {
    height: 100%;
    font-size: 13px;
    line-height: 1.55;
  }
  .merge-host :global(.cm-scroller) {
    font-family: ui-monospace, monospace;
  }
  footer.actions {
    display: flex;
    gap: 6px;
    padding: 8px 14px;
    border-top: 1px solid var(--border);
    background: var(--bg-card);
    justify-content: flex-end;
  }
  footer.actions button {
    background: var(--btn-bg);
    color: var(--text);
    border: 1px solid var(--btn-border);
    border-radius: 4px;
    padding: 5px 14px;
    cursor: pointer;
    font: inherit;
    font-size: 14px;
  }
  footer.actions button:hover { border-color: var(--btn-hover); }
  footer.actions button.primary {
    background: var(--link);
    color: #fff;
    border-color: var(--link);
  }
  footer.actions.status-only { justify-content: flex-start; }
  footer .status-tag {
    text-transform: uppercase;
    letter-spacing: 0.05em;
    font-size: 13px;
    color: var(--text-secondary);
  }
  footer .status-tag.ok { color: var(--accent, #2ea043); }
</style>
