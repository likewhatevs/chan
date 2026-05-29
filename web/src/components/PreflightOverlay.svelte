<script lang="ts">
  // New-workspace pre-flight. chan-server runs first-boot readiness
  // (open / seed / index / embedding-model) and reports it on
  // GET /api/preflight; this surface renders that snapshot LOCKED until
  // `phase === "ready"`. Locked here means locked by construction: a
  // full-viewport layer with no close affordance, not part of the
  // dismissable overlay stack, so there is no close button and ESC has
  // nothing to dismiss (contracts.md section 2). It matches
  // MissingTokenOverlay's full-page boot-surface shape.
  //
  // The snapshot is derived server-side on every poll, so the only state
  // here is the latest snapshot plus a poll timer. A ready (or
  // unreachable) pre-flight never blocks the editor.

  import { onMount, onDestroy } from "svelte";
  import { api } from "../api/client";
  import type { PreflightSnapshot } from "../api/types";

  const POLL_MS = 750;
  // A pre-flight that can't be reached must never wedge the editor, so
  // give up after a few consecutive failures (e.g. an older server with
  // no /api/preflight) and leave the UI usable.
  const MAX_ERROR_STREAK = 5;

  let snapshot = $state<PreflightSnapshot | null>(null);
  let deciding = $state(false);
  let timer: ReturnType<typeof setTimeout> | null = null;
  let stopped = false;
  let errorStreak = 0;

  // Show only while the server reports the workspace is not yet ready.
  const locked = $derived(snapshot?.locked === true);

  function schedule(ms = POLL_MS): void {
    if (stopped) return;
    if (timer) clearTimeout(timer);
    timer = setTimeout(poll, ms);
  }

  async function poll(): Promise<void> {
    if (stopped) return;
    try {
      snapshot = await api.preflight();
      errorStreak = 0;
      if (snapshot.phase !== "ready") schedule();
    } catch {
      errorStreak += 1;
      if (errorStreak < MAX_ERROR_STREAK) schedule(POLL_MS * 2);
    }
  }

  async function decide(step: string, choice: string): Promise<void> {
    if (deciding) return;
    deciding = true;
    try {
      snapshot = await api.preflightDecision({ step, choice });
      if (snapshot.phase !== "ready") schedule();
    } catch {
      schedule();
    } finally {
      deciding = false;
    }
  }

  function percent(current?: number, total?: number): number {
    if (!total || total <= 0 || current === undefined) return 0;
    return Math.min(100, Math.round((current / total) * 100));
  }

  onMount(() => {
    void poll();
  });
  onDestroy(() => {
    stopped = true;
    if (timer) clearTimeout(timer);
  });
</script>

{#if locked && snapshot}
  <main class="preflight" aria-live="polite" aria-label="preparing workspace">
    <h1>Preparing workspace</h1>
    <ul class="steps">
      {#each snapshot.steps as step (step.id)}
        <li class="step" data-state={step.state}>
          <span class="label">{step.label}</span>
          {#if step.state === "running" && step.total}
            <div class="bar">
              <div class="fill" style="width: {percent(step.current, step.total)}%"></div>
            </div>
            <span class="meta">{step.current ?? 0} / {step.total}</span>
          {:else if step.state === "needs_decision" && step.decision}
            <p class="prompt">{step.decision.prompt}</p>
            <div class="choices">
              {#each step.decision.choices as choice (choice.id)}
                <button
                  type="button"
                  disabled={deciding}
                  onclick={() => decide(step.id, choice.id)}
                >
                  {choice.label}
                </button>
              {/each}
            </div>
          {:else if step.state === "failed"}
            <span class="meta failed">failed</span>
          {:else if step.state === "done"}
            <span class="meta done">done</span>
          {:else}
            <span class="meta">working...</span>
          {/if}
        </li>
      {/each}
    </ul>
    {#if snapshot.error}
      <p class="error">{snapshot.error.message}</p>
    {/if}
  </main>
{/if}

<style>
  /* Full-viewport boot surface above every other layer, mirroring
     MissingTokenOverlay so a not-ready workspace can't be interacted with
     until the pre-flight completes. No close chrome by construction. */
  .preflight {
    position: fixed;
    inset: 0;
    z-index: 40000;
    background: var(--bg);
    color: var(--text);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 1.25rem;
    padding: 2rem;
    box-sizing: border-box;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
  }
  h1 {
    margin: 0;
    font-size: 18px;
    font-weight: 600;
  }
  .steps {
    list-style: none;
    margin: 0;
    padding: 0;
    width: min(420px, 90vw);
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }
  .step {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }
  .label {
    font-size: 14px;
  }
  .bar {
    height: 6px;
    background: var(--border);
    border-radius: 3px;
    overflow: hidden;
  }
  .fill {
    height: 100%;
    background: var(--text);
    transition: width 200ms ease;
  }
  .meta {
    font-size: 12px;
    color: var(--text-secondary);
  }
  .meta.failed {
    color: var(--warn-text);
  }
  .prompt {
    margin: 0;
    font-size: 13px;
    color: var(--text-secondary);
  }
  .choices {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
  }
  button {
    padding: 6px 12px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg-elev);
    color: var(--text);
    cursor: pointer;
    font-size: 13px;
  }
  button:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .error {
    margin: 0;
    color: var(--warn-text);
    font-size: 13px;
    max-width: 44ch;
    text-align: center;
    line-height: 1.5;
  }
</style>
