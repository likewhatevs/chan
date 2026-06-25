<script lang="ts">
  import type { WorkspaceWarning } from "../api/types";
  import {
    canDiscardWorkspaceWarning,
    closeWorkspaceWarningsDialog,
    copyWorkspaceWarningPath,
    discardWorkspaceWarning,
    dismissWorkspaceWarning,
    workspaceWarningLabel,
    workspaceWarningsDialog,
  } from "../state/store.svelte";

  let dialogEl: HTMLElement | undefined = $state();

  const warnings = $derived(workspaceWarningsDialog.warnings);

  $effect(() => {
    if (workspaceWarningsDialog.open) {
      queueMicrotask(() => dialogEl?.focus());
    }
  });

  function keyFor(warning: WorkspaceWarning): string {
    return `${warning.kind}\u0000${warning.path}\u0000${warning.message}`;
  }

  function onKey(e: KeyboardEvent): void {
    if (e.key === "Escape") {
      e.preventDefault();
      closeWorkspaceWarningsDialog();
    }
  }
</script>

{#if workspaceWarningsDialog.open}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="workspace-warnings-backdrop" onclick={closeWorkspaceWarningsDialog}>
    <div
      bind:this={dialogEl}
      class="workspace-warnings-modal"
      role="dialog"
      aria-modal="true"
      aria-labelledby="workspace-warnings-title"
      tabindex="-1"
      onkeydown={onKey}
      onclick={(e) => e.stopPropagation()}
    >
      <header class="modal-header">
        <h2 id="workspace-warnings-title">Workspace warnings</h2>
        <button
          type="button"
          class="icon-button"
          aria-label="Close workspace warnings"
          title="Close"
          onclick={closeWorkspaceWarningsDialog}
          disabled={workspaceWarningsDialog.busyKey !== null}
        >x</button>
      </header>

      <div class="modal-body">
        {#if warnings.length === 0}
          <p class="empty">No current workspace warnings.</p>
        {:else}
          <ul class="warning-list">
            {#each warnings as warning (keyFor(warning))}
              {@const busy = workspaceWarningsDialog.busyKey === keyFor(warning)}
              <li class="warning-item">
                <div class="warning-main">
                  <div class="warning-title">{workspaceWarningLabel(warning)}</div>
                  <div class="warning-meta">
                    <code>{warning.path}</code>
                    <span>{warning.kind}</span>
                  </div>
                </div>
                <div class="warning-actions">
                  <button
                    type="button"
                    onclick={() => void copyWorkspaceWarningPath(warning)}
                    disabled={workspaceWarningsDialog.busyKey !== null}
                  >Copy path</button>
                  <button
                    type="button"
                    onclick={() => dismissWorkspaceWarning(warning)}
                    disabled={workspaceWarningsDialog.busyKey !== null}
                  >Dismiss</button>
                  {#if canDiscardWorkspaceWarning(warning)}
                    <button
                      type="button"
                      class="danger"
                      onclick={() => void discardWorkspaceWarning(warning)}
                      disabled={workspaceWarningsDialog.busyKey !== null}
                    >{busy ? "Discarding..." : "Discard metadata"}</button>
                  {/if}
                </div>
              </li>
            {/each}
          </ul>
        {/if}

        {#if workspaceWarningsDialog.error}
          <p class="dialog-error" role="alert">{workspaceWarningsDialog.error}</p>
        {:else if workspaceWarningsDialog.notice}
          <p class="dialog-notice" role="status">{workspaceWarningsDialog.notice}</p>
        {/if}
      </div>

      <footer class="modal-footer">
        <button
          type="button"
          onclick={closeWorkspaceWarningsDialog}
          disabled={workspaceWarningsDialog.busyKey !== null}
        >OK</button>
      </footer>
    </div>
  </div>
{/if}

<style>
  .workspace-warnings-backdrop {
    position: fixed;
    inset: 0;
    z-index: 25500;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
    background: rgba(0, 0, 0, 0.42);
  }
  .workspace-warnings-modal {
    width: min(720px, 92vw);
    max-height: min(640px, 86vh);
    display: flex;
    flex-direction: column;
    background: var(--bg-elev);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: 8px;
    box-shadow: 0 18px 48px rgba(0, 0, 0, 0.38);
    outline: none;
  }
  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 14px 16px;
    border-bottom: 1px solid var(--border);
  }
  .modal-header h2 {
    margin: 0;
    font-size: 16px;
    font-weight: 650;
  }
  .icon-button {
    width: 28px;
    height: 28px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: 1px solid transparent;
    border-radius: 6px;
    background: transparent;
    color: var(--muted);
    cursor: pointer;
    font: inherit;
    font-size: 20px;
    line-height: 1;
  }
  .icon-button:hover:not(:disabled) {
    border-color: var(--border);
    color: var(--text);
  }
  .modal-body {
    min-height: 0;
    overflow: auto;
    padding: 14px 16px;
  }
  .warning-list {
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 10px;
    margin: 0;
    padding: 0;
  }
  .warning-item {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 12px;
    align-items: center;
    padding: 12px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg-card);
  }
  .warning-main {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .warning-title {
    color: var(--warn-text);
    font-size: 14px;
    line-height: 1.35;
    overflow-wrap: anywhere;
  }
  .warning-meta {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--muted);
    font-size: 12px;
    min-width: 0;
  }
  .warning-meta code {
    max-width: 42ch;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font: inherit;
    font-family: var(--font-mono, ui-monospace, SFMono-Regular, Menlo, monospace);
    color: var(--text-secondary);
  }
  .warning-actions {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 8px;
  }
  .warning-actions button,
  .modal-footer button {
    border: 1px solid var(--btn-border);
    border-radius: 6px;
    background: var(--btn-bg);
    color: var(--text);
    cursor: pointer;
    font: inherit;
    font-size: 13px;
    padding: 6px 10px;
  }
  .warning-actions button:hover:not(:disabled),
  .modal-footer button:hover:not(:disabled) {
    border-color: var(--btn-hover);
  }
  .warning-actions button:disabled,
  .modal-footer button:disabled,
  .icon-button:disabled {
    opacity: 0.55;
    cursor: default;
  }
  .warning-actions .danger {
    border-color: color-mix(in srgb, var(--danger, #d33) 70%, var(--border));
    color: var(--danger, #d33);
  }
  .dialog-error,
  .dialog-notice,
  .empty {
    margin: 12px 0 0;
    font-size: 13px;
    line-height: 1.4;
  }
  .dialog-error {
    color: var(--warn-text);
  }
  .dialog-notice,
  .empty {
    color: var(--muted);
  }
  .modal-footer {
    display: flex;
    justify-content: flex-end;
    padding: 12px 16px 14px;
    border-top: 1px solid var(--border);
  }
  @media (max-width: 640px) {
    .workspace-warnings-backdrop {
      padding: 12px;
      align-items: stretch;
    }
    .workspace-warnings-modal {
      width: 100%;
      max-height: none;
    }
    .warning-item {
      grid-template-columns: 1fr;
    }
    .warning-actions {
      justify-content: flex-start;
    }
  }
</style>
