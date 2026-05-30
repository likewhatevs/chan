<script lang="ts">
  import { ChevronDown, ChevronUp, X } from "lucide-svelte";
  import type { BubbleOverlayMode } from "../api/types";
  import { workspace } from "../state/store.svelte";
  import { bubbleStubVisible, hideBubbleStub } from "../state/bubbleStub.svelte";

  // The overlay is a frontend-only STATIC EXAMPLE. It carries no
  // watcher state, no session id, and no reply / refresh callbacks.
  // It reads two reactive inputs only:
  //   1. `bubbleStubVisible()` - whether the example is showing
  //      (flipped by the Team Work right-click menu via
  //      `showBubbleStub()` in state/bubbleStub.svelte.ts).
  //   2. the persisted `bubble_overlay_mode` preference - stack vs
  //      tray LAYOUT.
  // Clicking anything dismisses the example (`hideBubbleStub()`);
  // there is no network and no filesystem.

  const mode = $derived<BubbleOverlayMode>(
    workspace.info?.preferences.bubble_overlay_mode === "tray" ? "tray" : "stack",
  );
  const visible = $derived(bubbleStubVisible());

  // Local "tray expanded" view state, owned in a plain rune.
  // Defaults to collapsed in tray mode so the chip demonstrates the
  // tray affordance.
  let trayExpanded = $state(false);
  const collapsed = $derived(mode === "tray" && !trayExpanded);

  // Static example payload. Demonstrates the three survey shapes
  // chan supports: a single-question survey, a multi-question
  // survey, and the "F" follow-up affordance (rendered on every
  // survey bubble). Each option carries the 1-based number key the
  // numbered-option UI shows; the example is presentational only,
  // so clicks dismiss rather than answer.
  type ExampleOption = { n: number; label: string };
  type ExampleQuestion = { header: string; text: string; options: ExampleOption[] };
  type ExampleBubble = {
    id: string;
    from: string;
    questions: ExampleQuestion[];
  };

  const EXAMPLES: ExampleBubble[] = [
    {
      id: "example-single",
      from: "@@Architect",
      questions: [
        {
          header: "Q1",
          text: "Single-question survey: ship the change now or hold for review?",
          options: [
            { n: 1, label: "Ship now" },
            { n: 2, label: "Hold for review" },
            { n: 3, label: "Need more detail first" },
          ],
        },
      ],
    },
    {
      id: "example-multi",
      from: "@@Architect",
      questions: [
        {
          header: "Scope",
          text: "Multi-question survey: how broad should the refactor be?",
          options: [
            { n: 1, label: "Minimal" },
            { n: 2, label: "Moderate" },
            { n: 3, label: "Full sweep" },
          ],
        },
        {
          header: "Timing",
          text: "When should it land?",
          options: [
            { n: 1, label: "This round" },
            { n: 2, label: "Next round" },
          ],
        },
      ],
    },
  ];

  // The multi-question example shows its first topic; the topic tabs
  // above it demonstrate the multi-topic affordance. A static cursor
  // keeps the focused topic on the first question.
  function focusedQuestion(bubble: ExampleBubble): ExampleQuestion {
    return bubble.questions[0]!;
  }

  function dismiss(): void {
    hideBubbleStub();
  }
</script>

{#if visible}
  <section class="bubble-overlay" class:tray={mode === "tray"} aria-label="bubble example">
    {#if collapsed}
      <button type="button" class="tray-chip" onclick={dismiss}>
        <ChevronDown size={15} strokeWidth={1.8} aria-hidden="true" />
        <span>{EXAMPLES.length} example bubble{EXAMPLES.length === 1 ? "" : "s"}</span>
      </button>
    {:else}
      <div class="bubble-list">
        {#each EXAMPLES as bubble (bubble.id)}
          {@const multi = bubble.questions.length > 1}
          {@const focused = focusedQuestion(bubble)}
          <article class="bubble">
            <div class="bubble-head">
              <span>{bubble.from}</span>
              <div class="bubble-head-actions">
                {#if mode === "tray"}
                  <button type="button" class="icon" onclick={() => (trayExpanded = false)} aria-label="Collapse tray" title="Collapse">
                    <ChevronUp size={14} strokeWidth={1.8} aria-hidden="true" />
                  </button>
                {/if}
                <button
                  type="button"
                  class="icon"
                  onclick={dismiss}
                  aria-label="Dismiss bubble"
                  title="Dismiss"
                >
                  <X size={14} strokeWidth={1.8} aria-hidden="true" />
                </button>
              </div>
            </div>
            <div class="survey" data-multitopic={multi}>
              {#if multi}
                <div class="topic-tabs" role="tablist" aria-label="survey topics">
                  {#each bubble.questions as topic, idx}
                    <button type="button" class:on={idx === 0} onclick={dismiss}>
                      <span>{topic.header}</span>
                    </button>
                  {/each}
                </div>
              {/if}
              <p class="question">{focused.text}</p>
              <div class="option-list">
                {#each focused.options as option (option.n)}
                  <button type="button" onclick={dismiss}>
                    <kbd>{option.n}</kbd>
                    <span>{option.label}</span>
                  </button>
                {/each}
              </div>
              <button type="button" class="follow-button" onclick={dismiss}>
                <kbd>F</kbd>
                <span>follow up</span>
              </button>
            </div>
          </article>
        {/each}
      </div>
    {/if}
  </section>
{/if}

<style>
  .bubble-overlay {
    position: absolute;
    z-index: 18;
    top: 10px;
    left: 12px;
    right: 12px;
    max-height: 48%;
    display: flex;
    flex-direction: column;
    gap: 8px;
    pointer-events: none;
  }
  .bubble-overlay :where(button, article) { pointer-events: auto; }
  .tray-chip,
  .icon,
  .option-list button,
  .topic-tabs button {
    border: 1px solid var(--border);
    background: color-mix(in srgb, var(--bg-card) 92%, transparent);
    color: var(--text);
    border-radius: 4px;
    font: inherit;
  }
  .tray-chip {
    align-self: flex-end;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 5px 9px;
  }
  .bubble-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
    overflow: auto;
  }
  .bubble {
    align-self: flex-end;
    width: min(520px, 100%);
    padding: 10px 12px;
    border: 1px solid var(--border);
    /* Round the chat / survey bubbles to match the Team Work
       prompt's floating-pill (14 px, floating with margin) so the
       column of floating chips reads as one design language. */
    border-radius: 12px;
    background: color-mix(in srgb, var(--bg-card) 88%, transparent);
    color: var(--text);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.22);
    outline: none;
  }
  .bubble-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
    margin-bottom: 6px;
    font-size: 12px;
    color: var(--text-secondary);
  }
  .bubble-head-actions {
    display: inline-flex;
    gap: 4px;
  }
  .icon {
    width: 24px;
    height: 22px;
    padding: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: var(--text-secondary);
  }
  .question {
    margin: 0 0 8px;
    line-height: 1.35;
  }
  .topic-tabs {
    display: flex;
    gap: 4px;
    margin-bottom: 8px;
    overflow-x: auto;
  }
  .topic-tabs button {
    min-width: 54px;
    min-height: 26px;
    padding: 0 8px;
    color: var(--text-secondary);
    white-space: nowrap;
  }
  .topic-tabs button.on {
    color: var(--link);
    border-color: var(--link);
  }
  .option-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .option-list button {
    min-height: 32px;
    display: grid;
    grid-template-columns: 28px minmax(0, 1fr);
    align-items: start;
    gap: 7px;
    padding: 6px 9px;
    text-align: left;
    justify-content: flex-start;
  }
  .option-list button span {
    min-width: 0;
    overflow-wrap: anywhere;
    line-height: 1.3;
  }
  kbd {
    min-width: 18px;
    height: 18px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: 1px solid var(--border);
    border-radius: 3px;
    background: var(--bg);
    color: var(--text);
    font: 11px/1 ui-monospace, SFMono-Regular, Menlo, monospace;
  }
  .follow-button {
    margin-top: 7px;
    min-height: 28px;
    display: inline-grid;
    grid-template-columns: 24px auto;
    align-items: center;
    gap: 7px;
    width: fit-content;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: color-mix(in srgb, var(--bg-card) 70%, transparent);
    color: var(--text-secondary);
    padding: 4px 8px;
    font: inherit;
    font-size: 12px;
  }
  .follow-button:hover,
  .follow-button:focus-visible {
    border-color: var(--link);
    color: var(--link);
  }
</style>
