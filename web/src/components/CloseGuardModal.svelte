<script lang="ts">
  // Confirmation rendered when the desktop app window is about to
  // close with unsaved tab buffers. State lives in
  // closeGuard.svelte.ts; this component just renders + dispatches
  // the user's choice. Mounts once at App.svelte.
  //
  // Three actions: Cancel (stay), Discard and close (lose unsaved
  // edits), Save and quit (flush every dirty buffer; on failure
  // we show the error inline and re-open this modal so the user
  // can retry or pick discard to escape).

  import {
    closeGuardState,
    resolveCloseGuard,
    type CloseGuardChoice,
  } from "../state/closeGuard.svelte";

  function pick(choice: CloseGuardChoice): void {
    resolveCloseGuard(choice);
  }

  function onKey(e: KeyboardEvent): void {
    if (e.key === "Escape") {
      e.preventDefault();
      pick("cancel");
    } else if (e.key === "Enter") {
      e.preventDefault();
      pick("save");
    }
  }
</script>

<svelte:window onkeydown={(e) => closeGuardState.open && onKey(e)} />

{#if closeGuardState.open}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="overlay" onclick={() => pick("cancel")}>
    <div class="modal" onclick={(e) => e.stopPropagation()} role="dialog" tabindex="-1">
      <div class="title">{closeGuardState.dirtyCount} unsaved tab(s)</div>
      <div class="body">
        Save the unsaved buffers before quitting? Discarding loses
        your in-editor edits; the files on disk stay whatever they
        were before this session's edits.
      </div>
      {#if closeGuardState.error}
        <div class="error">
          <strong>Save failed:</strong> {closeGuardState.error}
        </div>
      {/if}
      <div class="actions">
        <button class="cancel" onclick={() => pick("cancel")}>Cancel</button>
        <span class="spacer"></span>
        <button class="discard" onclick={() => pick("discard")}>
          Discard and close
        </button>
        <button class="primary" onclick={() => pick("save")}>
          {closeGuardState.error ? "Retry save" : "Save and quit"}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    /* Above InlineAssist (25000) and the wiki/calendar popovers
       (30000), since this is a modal the user must address before
       anything else can happen. */
    z-index: 40000;
  }
  .modal {
    background: var(--bg-elev);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: 6px;
    box-shadow: 0 14px 40px rgba(0, 0, 0, 0.5);
    padding: 1rem 1.1rem;
    min-width: 380px;
    max-width: 90vw;
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }
  .title {
    font-size: 14px;
    font-weight: 600;
    color: var(--text);
  }
  .body {
    font-size: 12.5px;
    color: var(--text-secondary);
    line-height: 1.45;
  }
  .actions {
    display: flex;
    gap: 0.4rem;
    align-items: center;
    margin-top: 0.4rem;
  }
  .actions .spacer { flex: 1; }
  .actions button {
    padding: 0.35rem 0.85rem;
    border-radius: 4px;
    border: 1px solid var(--btn-border);
    background: var(--btn-bg);
    color: var(--text);
    cursor: pointer;
    font: inherit;
    font-size: 12px;
  }
  .actions button:hover { border-color: var(--btn-hover); }
  .actions .primary {
    background: var(--link);
    border-color: var(--link);
    color: #fff;
  }
  .actions .discard {
    color: var(--warn-text);
    border-color: var(--warn-text);
  }
  .error {
    background: rgba(220, 53, 53, 0.12);
    border: 1px solid #d33;
    border-radius: 4px;
    padding: 0.5rem 0.6rem;
    font-size: 12px;
    color: #d33;
    line-height: 1.4;
    word-break: break-word;
  }
  .error strong { color: #d33; }
</style>
