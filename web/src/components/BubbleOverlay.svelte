<!-- Survey overlay for `cs terminal survey` (round-3 @@LaneC rebuild).

     Renders the active survey raised on this window: a markdown problem
     body, up to 4 vertically aligned numbered options, and an optional [F]
     follow-up. Picking an option (click or 1..N) or [F] (click or F) POSTs
     the reply, which unblocks the waiting CLI. The overlay is modal and has
     NO dismiss chrome: the CLI is blocked on the reply, so [F] (defer) is the
     only non-answer exit. Mounted once at the App root (window-level), driven
     by the singleton `surveyState`. -->
<script lang="ts">
  import { renderMarkdown } from "../api/markdown";
  import {
    surveyState,
    pickOption,
    requestFollowup,
  } from "../state/survey.svelte";

  const active = $derived(surveyState.active);

  // Steal focus to the card when a survey appears so option/F keys land here
  // and not in the terminal/editor underneath. Keyed on surveyId so a
  // replacing survey re-focuses.
  let card = $state<HTMLDivElement | null>(null);
  $effect(() => {
    const id = surveyState.active?.surveyId;
    if (id && card) card.focus();
  });

  // Number keys 1..N pick an option; F follows up. Routed at the window so a
  // focused terminal does not swallow the keystroke into its PTY.
  function onKeydown(e: KeyboardEvent): void {
    const s = surveyState.active;
    if (!s) return;
    if (e.key >= "1" && e.key <= "9") {
      const idx = Number(e.key) - 1;
      if (idx < s.options.length) {
        e.preventDefault();
        e.stopPropagation();
        void pickOption(idx);
      }
      return;
    }
    if ((e.key === "f" || e.key === "F") && s.allowFollowup && s.followup) {
      e.preventDefault();
      e.stopPropagation();
      void requestFollowup();
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />

{#if active}
  <div class="survey-overlay" role="dialog" aria-modal="true">
    <div class="survey-card" tabindex="-1" bind:this={card}>
      {#if active.title}
        <h2 class="survey-title">{active.title}</h2>
      {/if}
      <!-- renderMarkdown is DOMPurify-sanitized (web/src/api/markdown.ts),
           so {@html} is safe for the agent-supplied body. -->
      <div class="survey-body">{@html renderMarkdown(active.bodyMarkdown)}</div>
      <ul class="survey-options">
        {#each active.options as option, i (i)}
          <li>
            <button
              type="button"
              class="survey-option"
              disabled={surveyState.busy}
              onclick={() => pickOption(i)}
            >
              <span class="survey-option-key">{i + 1}</span>
              <span class="survey-option-label">{option}</span>
            </button>
          </li>
        {/each}
      </ul>
      {#if active.allowFollowup && active.followup}
        <button
          type="button"
          class="survey-followup"
          disabled={surveyState.busy}
          onclick={() => requestFollowup()}
        >
          [F] Follow up later
        </button>
      {/if}
    </div>
  </div>
{/if}

<style>
  .survey-overlay {
    position: fixed;
    inset: 0;
    z-index: 39000;
    background: color-mix(in srgb, var(--bg) 70%, transparent);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 2rem;
    box-sizing: border-box;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
  }
  .survey-card {
    width: min(520px, 92vw);
    max-height: 86vh;
    overflow-y: auto;
    background: var(--bg);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: 10px;
    box-shadow: 0 12px 40px rgba(0, 0, 0, 0.35);
    padding: 1.25rem 1.5rem;
    box-sizing: border-box;
    outline: none;
  }
  .survey-title {
    margin: 0 0 0.75rem;
    font-size: 16px;
    font-weight: 600;
  }
  .survey-body {
    font-size: 14px;
    line-height: 1.5;
    margin-bottom: 1rem;
  }
  .survey-body :global(p) {
    margin: 0 0 0.6rem;
  }
  .survey-body :global(code) {
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 0.9em;
  }
  .survey-options {
    list-style: none;
    margin: 0 0 0.75rem;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  .survey-option {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    width: 100%;
    text-align: left;
    padding: 0.6rem 0.75rem;
    background: var(--bg);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: 7px;
    cursor: pointer;
    font-size: 14px;
  }
  .survey-option:hover:not(:disabled),
  .survey-option:focus-visible {
    border-color: var(--text);
  }
  .survey-option:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .survey-option-key {
    flex: 0 0 auto;
    min-width: 1.4em;
    height: 1.4em;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: 1px solid var(--border);
    border-radius: 4px;
    font-size: 12px;
    color: var(--text-secondary);
  }
  .survey-option-label {
    flex: 1 1 auto;
  }
  .survey-followup {
    width: 100%;
    padding: 0.5rem 0.75rem;
    background: transparent;
    color: var(--text-secondary);
    border: 1px dashed var(--border);
    border-radius: 7px;
    cursor: pointer;
    font-size: 13px;
  }
  .survey-followup:hover:not(:disabled),
  .survey-followup:focus-visible {
    color: var(--text);
    border-color: var(--text);
  }
  .survey-followup:disabled {
    opacity: 0.5;
    cursor: default;
  }
</style>
