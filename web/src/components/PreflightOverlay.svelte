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
  import { workspace } from "../state/store.svelte";
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
  function errText(e: unknown): string {
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
      csError = errText(e);
      manualMode = true;
    } finally {
      csBusy = false;
    }
  }

  // First-run onboarding nudge (P2). Non-locking, like the cs card: it rides on
  // the ready snapshot's `summary` block and points the user at the Dashboard
  // to enable the optional Semantic / Reports layers (a thin nudge, NOT inline
  // toggles). Dismissal is persisted PER WORKSPACE so each new workspace gets
  // its own one-time nudge, keyed off the workspace identity the store already
  // holds.
  const ONBOARD_DISMISS_PREFIX = "chan.onboardDismissed:";
  const summary = $derived(snapshot?.summary ?? null);
  // Bumped on dismiss to force the localStorage-backed check below to re-read
  // (localStorage is not reactive on its own).
  let onboardDismissTick = $state(0);
  function workspaceKey(): string | null {
    return workspace.info?.metadata_key ?? workspace.info?.root ?? null;
  }
  // Reactive on both the workspace identity (loads after boot) and the tick, so
  // the nudge resolves correctly once `workspace.info` arrives and hides
  // immediately on dismiss. Reads only; never mutates state in the derivation.
  const onboardDismissed = $derived.by(() => {
    void onboardDismissTick;
    const key = workspaceKey();
    if (!key) return false;
    try {
      return localStorage.getItem(ONBOARD_DISMISS_PREFIX + key) === "1";
    } catch {
      return false;
    }
  });
  const showOnboardCard = $derived(!!summary && !locked && !onboardDismissed);

  function dismissOnboard(): void {
    const key = workspaceKey();
    if (key) {
      try {
        localStorage.setItem(ONBOARD_DISMISS_PREFIX + key, "1");
      } catch {
        // Private-mode storage: the tick still hides it for this session.
      }
    }
    onboardDismissTick += 1;
  }
  // Inline layer controls (round-1 wave-3: the nudge became actionable). The
  // displayed state seeds from the summary and then tracks the toggle results,
  // so a click reflects immediately without re-polling the snapshot.
  let reportsOverride = $state<boolean | null>(null);
  let semanticOverride = $state<boolean | null>(null);
  const reportsOn = $derived(reportsOverride ?? summary?.reports_enabled ?? false);
  const semanticOn = $derived(semanticOverride ?? summary?.semantic_enabled ?? false);
  let reportsBusy = $state(false);
  let reportsError = $state<string | null>(null);
  let semanticBusy = $state(false);
  let semanticError = $state<string | null>(null);
  // The embedding model is missing, so enabling needs a download first.
  let semanticNeedsModel = $state(false);
  let semanticDownloading = $state(false);

  async function toggleReports(): Promise<void> {
    if (reportsBusy) return;
    reportsBusy = true;
    reportsError = null;
    try {
      const next = !reportsOn;
      const state = next ? await api.reportsEnable() : await api.reportsDisable();
      reportsOverride = state.enabled;
    } catch (e) {
      reportsError = errText(e);
    } finally {
      reportsBusy = false;
    }
  }

  async function enableSemantic(): Promise<void> {
    if (semanticBusy) return;
    semanticBusy = true;
    semanticError = null;
    try {
      // The enable route guards on the embedding model being present; the
      // common case (model already fetched for another workspace, it is shared
      // machine-wide) succeeds in one click.
      const state = await api.semanticEnable();
      semanticOverride = state.semantic_enabled;
    } catch (e) {
      // Model missing: surface the download affordance instead of failing.
      semanticNeedsModel = true;
      semanticError = errText(e);
    } finally {
      semanticBusy = false;
    }
  }
  async function downloadAndEnableSemantic(): Promise<void> {
    if (semanticBusy) return;
    semanticBusy = true;
    semanticDownloading = true;
    semanticError = null;
    try {
      await api.semanticDownload();
      const state = await api.semanticEnable();
      semanticOverride = state.semantic_enabled;
      semanticNeedsModel = false;
    } catch (e) {
      semanticError = errText(e);
    } finally {
      semanticBusy = false;
      semanticDownloading = false;
    }
  }
  async function disableSemantic(): Promise<void> {
    if (semanticBusy) return;
    semanticBusy = true;
    semanticError = null;
    try {
      const state = await api.semanticDisable();
      semanticOverride = state.semantic_enabled;
      semanticNeedsModel = false;
    } catch (e) {
      semanticError = errText(e);
    } finally {
      semanticBusy = false;
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

<!-- Non-blocking boot cards: a corner stack shown once the workspace is ready
     (or right away when nothing locked the boot). Never modal, the workspace
     is already usable. Holds the first-run onboarding nudge and the cs offer,
     stacked when both apply. -->
{#if (showOnboardCard && summary) || (showCsCard && csOffer)}
  <div class="boot-cards">
    {#if showOnboardCard && summary}
      <aside class="boot-card onboard-card" role="status" aria-label="workspace ready">
        <div class="boot-head">
          <strong>{workspace.info?.label ?? "Workspace"} is ready</strong>
          <button class="boot-x" type="button" aria-label="Dismiss" onclick={dismissOnboard}>×</button>
        </div>
        <p class="onboard-summary">
          {summary.indexed_docs.toLocaleString()} indexed{#if summary.scm} · {summary.scm} repository{/if}
        </p>
        <p class="boot-body">
          Keyword search and the wiki-link graph are always on, the minimum
          needed to operate. Two optional layers you can toggle here:
        </p>
        <ul class="onboard-layers">
          <li>
            <div class="onboard-layer-top">
              <span class="onboard-layer-name">Semantic search</span>
              <span class="onboard-state" data-on={semanticOn}>{semanticOn ? "on" : "off"}</span>
              {#if semanticOn}
                <button class="onboard-toggle" disabled={semanticBusy} onclick={disableSemantic}>
                  Turn off
                </button>
              {:else if semanticNeedsModel}
                <button
                  class="onboard-toggle"
                  disabled={semanticBusy}
                  onclick={downloadAndEnableSemantic}
                >
                  {semanticDownloading ? "Downloading…" : "Download & enable"}
                </button>
              {:else}
                <button class="onboard-toggle" disabled={semanticBusy} onclick={enableSemantic}>
                  {semanticBusy ? "…" : "Turn on"}
                </button>
              {/if}
            </div>
            <span class="onboard-layer-hint">
              find by meaning; needs the BGE-small model (~63 MB, shared)
            </span>
            {#if semanticError}<span class="onboard-err">{semanticError}</span>{/if}
          </li>
          <li>
            <div class="onboard-layer-top">
              <span class="onboard-layer-name">Reports</span>
              <span class="onboard-state" data-on={reportsOn}>{reportsOn ? "on" : "off"}</span>
              <button class="onboard-toggle" disabled={reportsBusy} onclick={toggleReports}>
                {reportsBusy ? "…" : reportsOn ? "Turn off" : "Turn on"}
              </button>
            </div>
            <span class="onboard-layer-hint">
              per-file language, SLOC and COCOMO analysis
            </span>
            {#if reportsError}<span class="onboard-err">{reportsError}</span>{/if}
          </li>
        </ul>
        <div class="boot-actions boot-actions-end">
          <button type="button" onclick={dismissOnboard}>OK</button>
        </div>
      </aside>
    {/if}
    {#if showCsCard && csOffer}
      <aside class="boot-card cs-card" role="status" aria-label="terminal shortcut setup">
        <div class="boot-head">
          <strong>Terminal shortcut</strong>
          <button class="boot-x" type="button" aria-label="Dismiss" onclick={dismissCs}>×</button>
        </div>
        {#if csResult}
          <p class="boot-msg done">{csResult}</p>
          <div class="boot-actions">
            <button type="button" class="primary" onclick={dismissCs}>Done</button>
          </div>
        {:else}
          <p class="boot-body">
            Add <code>cs</code> to your PATH to drive this window from the terminal
            (open files, split panes, run Team Work).
          </p>
          {#if csError}
            <p class="boot-msg warn">{csError}</p>
          {/if}
          {#if csOffer.can_create && !manualMode}
            <div class="boot-actions">
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
            <div class="boot-actions">
              <button type="button" onclick={dismissCs}>Got it</button>
            </div>
          {/if}
        {/if}
      </aside>
    {/if}
  </div>
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

  /* Non-blocking corner stack. Sits above the editor but well below the
     locked boot surface (z=40000), and only renders when nothing is locked,
     so the two never overlap. Holds the onboarding nudge and the cs offer,
     stacked when both apply. */
  .boot-cards {
    position: fixed;
    right: 1rem;
    bottom: 1rem;
    z-index: 9000;
    width: min(340px, calc(100vw - 2rem));
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }
  .boot-card {
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
  .boot-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: 13px;
  }
  .boot-x {
    border: none;
    background: transparent;
    color: var(--text-secondary);
    font-size: 18px;
    line-height: 1;
    padding: 0 0.2rem;
    cursor: pointer;
    border-radius: 4px;
  }
  .boot-x:hover {
    color: var(--text);
  }
  .boot-body {
    margin: 0;
    font-size: 12.5px;
    color: var(--text-secondary);
    line-height: 1.5;
  }
  .boot-body code,
  .cs-cmd {
    font-family: "Source Code Pro", ui-monospace, SFMono-Regular, Menlo, monospace;
  }
  .boot-msg {
    margin: 0;
    font-size: 12px;
    line-height: 1.4;
  }
  .boot-msg.done {
    color: var(--text);
  }
  .boot-msg.warn {
    color: var(--warn-text);
  }
  .boot-actions {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
  }
  /* Onboarding card: single OK button, aligned to the right edge. */
  .boot-actions-end {
    justify-content: flex-end;
  }
  .boot-actions button {
    padding: 5px 11px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg);
    color: var(--text);
    cursor: pointer;
    font-size: 12.5px;
  }
  .boot-actions button.primary {
    background: var(--text);
    color: var(--bg);
    border-color: var(--text);
  }
  .boot-actions button:disabled {
    opacity: 0.5;
    cursor: default;
  }

  /* cs-offer specifics */
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

  /* onboarding-nudge specifics */
  .onboard-summary {
    margin: 0;
    font-size: 12px;
    color: var(--text-secondary);
  }
  .onboard-layers {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }
  .onboard-layers li {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }
  .onboard-layer-top {
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }
  .onboard-layer-name {
    font-size: 12.5px;
    color: var(--text);
  }
  .onboard-state {
    font-size: 11px;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .onboard-state[data-on="true"] {
    color: var(--text);
  }
  .onboard-toggle {
    margin-left: auto;
    padding: 3px 9px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg);
    color: var(--text);
    cursor: pointer;
    font-size: 12px;
    white-space: nowrap;
  }
  .onboard-toggle:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .onboard-layer-hint {
    font-size: 11px;
    color: var(--text-secondary);
    line-height: 1.4;
  }
  .onboard-err {
    font-size: 11px;
    color: var(--warn-text);
    line-height: 1.4;
  }
</style>
