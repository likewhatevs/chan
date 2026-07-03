<script lang="ts">
  import { openInPane } from "../state/tabs.svelte";
  import {
    dismissSurveyDraftDialog,
    surveyCloseTitle,
    surveyDraftDialogFor,
  } from "../state/survey.svelte";

  let { tabId, paneId }: { tabId: string; paneId: string } = $props();

  const dialog = $derived(surveyDraftDialogFor(tabId));
  const title = $derived(dialog ? surveyCloseTitle(dialog.reason) : "");

  let card = $state<HTMLDivElement | null>(null);
  $effect(() => {
    const id = dialog?.id;
    if (id && card) card.focus();
  });

  function dismiss(): void {
    dismissSurveyDraftDialog(tabId);
  }

  function openDraft(): void {
    const path = dialog?.draftPath;
    if (!path) return;
    dismissSurveyDraftDialog(tabId);
    void openInPane(paneId, path, { landAtTop: true });
  }

  function onKeydown(event: KeyboardEvent): void {
    if (event.key !== "Escape") return;
    event.preventDefault();
    event.stopPropagation();
    dismiss();
  }
</script>

{#if dialog}
  <div class="survey-draft-backdrop" role="presentation">
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="survey-draft-dialog"
      role="dialog"
      aria-modal="true"
      aria-labelledby={`survey-draft-title-${tabId}`}
      tabindex="-1"
      bind:this={card}
      onkeydown={onKeydown}
    >
      <h2 id={`survey-draft-title-${tabId}`}>{title}</h2>
      <p>
        Your Rich Prompt draft was saved at
        <code>{dialog.draftPath}</code>.
      </p>
      <div class="survey-draft-actions">
        <button type="button" class="survey-draft-open" onclick={openDraft}>Open</button>
        <button type="button" class="survey-draft-dismiss" onclick={dismiss}>Dismiss</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .survey-draft-backdrop {
    position: absolute;
    inset: 0;
    z-index: 25000;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 5%;
    box-sizing: border-box;
    background: color-mix(in srgb, var(--bg) 72%, transparent);
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
  }

  .survey-draft-dialog {
    width: min(520px, 100%);
    max-height: 100%;
    overflow-y: auto;
    box-sizing: border-box;
    padding: 1.25rem 1.5rem;
    color: var(--text);
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 8px;
    box-shadow: 0 12px 40px rgba(0, 0, 0, 0.35);
    outline: none;
  }

  .survey-draft-dialog h2 {
    margin: 0 0 0.75rem;
    font-size: 16px;
    font-weight: 600;
  }

  .survey-draft-dialog p {
    margin: 0;
    font-size: 14px;
    line-height: 1.5;
    color: var(--text);
  }

  .survey-draft-dialog code {
    display: block;
    margin-top: 0.5rem;
    padding: 0.45rem 0.55rem;
    overflow-wrap: anywhere;
    color: var(--text);
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: 6px;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 12px;
  }

  .survey-draft-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
    margin-top: 1rem;
  }

  .survey-draft-actions button {
    padding: 0.5rem 0.75rem;
    border-radius: 7px;
    font-size: 13px;
    cursor: pointer;
  }

  .survey-draft-open {
    color: var(--bg);
    background: var(--text);
    border: 1px solid var(--text);
  }

  .survey-draft-dismiss {
    color: var(--text-secondary);
    background: transparent;
    border: 1px solid var(--border);
  }
</style>
