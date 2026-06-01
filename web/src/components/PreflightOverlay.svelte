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
  import { ApiError } from "../api/errors";
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

  // The non-blocking `cs` terminal-alias offer. It rides on the snapshot but
  // never gates `locked`, so it renders as a dismissible corner card once the
  // workspace is ready (or right away when nothing locked the boot). Dismissal
  // is persisted machine-wide in localStorage: the alias and PATH are global,
  // so once handled it should not nag again on the next load.
  const CS_DISMISS_KEY = "chan.csLinkDismissed";
  const csOffer = $derived(snapshot?.cs_link ?? null);
  let csDismissed = $state(readCsDismissed());
  let csBusy = $state(false);
  let csResult = $state<string | null>(null);
  let csError = $state<string | null>(null);
  // Flip to the manual `ln -s` hint when one-click create is unavailable or
  // failed (e.g. a root-owned bin dir).
  let manualMode = $state(false);
  const showCsCard = $derived(!!csOffer && !locked && !csDismissed);

  function readCsDismissed(): boolean {
    try {
      return localStorage.getItem(CS_DISMISS_KEY) === "1";
    } catch {
      return false;
    }
  }
  function persistCsDismissed(): void {
    try {
      localStorage.setItem(CS_DISMISS_KEY, "1");
    } catch {
      // Private-mode / disabled storage: dismissal stays session-local.
    }
  }
  function dismissCs(): void {
    csDismissed = true;
    persistCsDismissed();
  }
  function csErrorText(e: unknown): string {
    if (e instanceof ApiError) {
      // Some transports hand back the raw JSON body; unwrap { error }.
      try {
        const body = JSON.parse(e.message) as { error?: unknown };
        if (typeof body.error === "string" && body.error.trim()) return body.error;
      } catch {
        // Not JSON: the message is already human-readable.
      }
      return e.message;
    }
    return e instanceof Error ? e.message : String(e);
  }
  async function createCsLink(): Promise<void> {
    if (csBusy) return;
    csBusy = true;
    csError = null;
    try {
      const res = await api.createCsLink();
      csResult = res.message;
      // Succeeded (or already present): don't ask again on future loads.
      if (res.resolved) persistCsDismissed();
    } catch (e) {
      // Non-fatal: surface why and fall back to the manual hint so the user
      // can finish by hand, then continue.
      csError = csErrorText(e);
      manualMode = true;
    } finally {
      csBusy = false;
    }
  }

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

<!-- Non-blocking `cs` terminal-alias offer. A corner card, never modal: the
     workspace is already usable, this just helps a fresh install set up the
     terminal control alias. -->
{#if showCsCard && csOffer}
  <aside class="cs-card" role="status" aria-label="terminal shortcut setup">
    <div class="cs-head">
      <strong>Terminal shortcut</strong>
      <button class="cs-x" type="button" aria-label="Dismiss" onclick={dismissCs}>×</button>
    </div>
    {#if csResult}
      <p class="cs-msg done">{csResult}</p>
      <div class="cs-actions">
        <button type="button" class="primary" onclick={dismissCs}>Done</button>
      </div>
    {:else}
      <p class="cs-body">
        Add <code>cs</code> to your PATH to drive this window from the terminal
        (open files, split panes, run Team Work).
      </p>
      {#if csError}
        <p class="cs-msg warn">{csError}</p>
      {/if}
      {#if csOffer.can_create && !manualMode}
        <div class="cs-actions">
          <button type="button" class="primary" disabled={csBusy} onclick={createCsLink}>
            {csBusy ? "Creating…" : "Create link"}
          </button>
          <button type="button" disabled={csBusy} onclick={dismissCs}>Not now</button>
        </div>
        <p class="cs-target">{csOffer.target}</p>
      {:else}
        <p class="cs-hint">Run this once in a terminal whose PATH you control:</p>
        <code class="cs-cmd">ln -s "{csOffer.points_to}" ~/.local/bin/cs</code>
        {#if csOffer.note}
          <p class="cs-note">({csOffer.note})</p>
        {/if}
        <div class="cs-actions">
          <button type="button" onclick={dismissCs}>Got it</button>
        </div>
      {/if}
    {/if}
  </aside>
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

  /* Non-blocking corner card. Sits above the editor but well below the
     locked boot surface (z=40000), and only ever renders when nothing is
     locked, so the two never overlap. */
  .cs-card {
    position: fixed;
    right: 1rem;
    bottom: 1rem;
    z-index: 9000;
    width: min(340px, calc(100vw - 2rem));
    box-sizing: border-box;
    background: var(--bg-elev);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 0.85rem 0.95rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.18);
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
  }
  .cs-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: 13px;
  }
  .cs-x {
    border: none;
    background: transparent;
    color: var(--text-secondary);
    font-size: 18px;
    line-height: 1;
    padding: 0 0.2rem;
    cursor: pointer;
    border-radius: 4px;
  }
  .cs-x:hover {
    color: var(--text);
  }
  .cs-body {
    margin: 0;
    font-size: 12.5px;
    color: var(--text-secondary);
    line-height: 1.5;
  }
  .cs-body code,
  .cs-cmd {
    font-family: "Source Code Pro", ui-monospace, SFMono-Regular, Menlo, monospace;
  }
  .cs-hint {
    margin: 0;
    font-size: 12px;
    color: var(--text-secondary);
  }
  .cs-cmd {
    display: block;
    font-size: 11.5px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 0.4rem 0.5rem;
    overflow-x: auto;
    white-space: pre;
    user-select: all;
  }
  .cs-target {
    margin: 0;
    font-size: 11px;
    color: var(--text-secondary);
    font-family: "Source Code Pro", ui-monospace, SFMono-Regular, Menlo, monospace;
    overflow-wrap: anywhere;
  }
  .cs-note {
    margin: 0;
    font-size: 11.5px;
    color: var(--text-secondary);
    line-height: 1.4;
  }
  .cs-msg {
    margin: 0;
    font-size: 12px;
    line-height: 1.4;
  }
  .cs-msg.done {
    color: var(--text);
  }
  .cs-msg.warn {
    color: var(--warn-text);
  }
  .cs-actions {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
  }
  .cs-actions button {
    padding: 5px 11px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg);
    color: var(--text);
    cursor: pointer;
    font-size: 12.5px;
  }
  .cs-actions button.primary {
    background: var(--text);
    color: var(--bg);
    border-color: var(--text);
  }
  .cs-actions button:disabled {
    opacity: 0.5;
    cursor: default;
  }
</style>
