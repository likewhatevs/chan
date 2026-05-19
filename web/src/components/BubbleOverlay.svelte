<script lang="ts">
  import { ChevronDown, ChevronUp, Loader2, RefreshCw } from "lucide-svelte";
  import type { BubbleOverlayMode } from "../api/types";
  import { openExternalUrl } from "../editor/external_links";
  import { drive } from "../state/store.svelte";
  import type { SurveyOption, TerminalWatcherState, WatcherEvent } from "../state/tabs.svelte";
  import { normalizeStandingOptions, writeSurveyReply } from "../state/watcherEvents";

  let {
    watcher,
    sessionId,
    onRefresh,
    onWatcherDetached,
    onOpenTerminal,
  }: {
    watcher: TerminalWatcherState;
    sessionId?: string;
    onRefresh: () => Promise<void> | void;
    onWatcherDetached?: () => void;
    onOpenTerminal?: (event: WatcherEvent) => void;
  } = $props();

  type NumberedOption = SurveyOption & { n: number };

  const mode = $derived<BubbleOverlayMode>(
    drive.info?.preferences.bubble_overlay_mode === "tray" ? "tray" : "stack",
  );
  const visibleEvents = $derived(watcher.events.filter((event) => event.type !== "survey-reply"));
  const orderedEvents = $derived([...visibleEvents].reverse());
  const collapsed = $derived(mode === "tray" && !watcher.trayExpanded);
  let answers = $state<Record<string, Record<number, string>>>({});
  let followUps = $state<Record<string, boolean>>({});
  let focusedQuestion = $state<Record<string, number>>({});
  let focusedBubbleId = $state<string | null>(null);
  let busyReply = $state<string | null>(null);
  let now = $state(Date.now());
  const dismissTimers = new Set<ReturnType<typeof setTimeout>>();

  $effect(() => {
    const ticker = setInterval(() => (now = Date.now()), 1000);
    return () => {
      clearInterval(ticker);
      for (const timer of dismissTimers) clearTimeout(timer);
      dismissTimers.clear();
    };
  });

  $effect(() => {
    const ids = new Set(orderedEvents.map((event) => event.id));
    if (!focusedBubbleId || !ids.has(focusedBubbleId)) {
      focusedBubbleId = orderedEvents[0]?.id ?? null;
    }
  });

  function textParts(text: string): Array<{ text: string; href?: string }> {
    const out: Array<{ text: string; href?: string }> = [];
    const re = /\bhttps?:\/\/[^\s<>)]+/g;
    let last = 0;
    for (const match of text.matchAll(re)) {
      const idx = match.index ?? 0;
      if (idx > last) out.push({ text: text.slice(last, idx) });
      out.push({ text: match[0]!, href: match[0] });
      last = idx + match[0]!.length;
    }
    if (last < text.length) out.push({ text: text.slice(last) });
    return out;
  }

  function optionsFor(event: WatcherEvent, questionIndex: number): NumberedOption[] {
    const base = optionCandidates(event, questionIndex);
    return base.slice(0, 3).map((option, idx) => ({ ...option, n: idx + 1 }));
  }

  function optionCandidates(event: WatcherEvent, questionIndex: number): SurveyOption[] {
    if (event.type === "pre-flight") return preFlightOptions();
    const question = event.questions?.[questionIndex];
    const base = question?.options ?? [];
    const standing = normalizeStandingOptions(event.standing_options);
    return [...base, ...standing];
  }

  function optionOverflowCount(event: WatcherEvent, questionIndex: number): number {
    return Math.max(0, optionCandidates(event, questionIndex).length - 3);
  }

  function visibleQuestions(event: WatcherEvent) {
    return (event.questions ?? []).slice(0, 4);
  }

  function questionOverflowCount(event: WatcherEvent): number {
    return event.type === "pre-flight" ? 0 : Math.max(0, (event.questions?.length ?? 0) - 4);
  }

  async function answer(event: WatcherEvent, questionIndex: number, option: SurveyOption): Promise<void> {
    if (event.type === "pre-flight" && option.key === "open-terminal") {
      onOpenTerminal?.(event);
    }
    const byQuestion = { ...(answers[event.id] ?? {}), [questionIndex]: option.key };
    answers[event.id] = byQuestion;
    const total = event.questions?.length ?? 0;
    if (total <= 1) {
      await commit(event, byQuestion, 600, false);
      return;
    }
    const next = nextUnanswered(total, byQuestion, questionIndex);
    if (next === null) {
      await commit(event, byQuestion, 600, false);
    } else {
      focusedQuestion[event.id] = next;
    }
  }

  async function skip(event: WatcherEvent): Promise<void> {
    await commit(event, {}, 0, false);
  }

  async function markFollowUp(event: WatcherEvent): Promise<void> {
    await commit(event, {}, -1, true);
  }

  async function commit(
    event: WatcherEvent,
    byQuestion: Record<number, string>,
    dismissDelayMs: number,
    followUp: boolean,
  ): Promise<void> {
    if (busyReply) return;
    if (!sessionId) {
      watcher.error = "reply failed: terminal session is not ready";
      return;
    }
    busyReply = event.id;
    watcher.error = undefined;
    try {
      const replyAnswers = Object.entries(byQuestion).map(([idx, key]) => ({
        question_index: Number(idx),
        key,
      }));
      await writeSurveyReply(sessionId, event, replyAnswers, "one-shot", followUp);
      if (followUp) {
        followUps[event.id] = true;
      } else {
        followUps[event.id] = false;
        dismissEvent(event.id, dismissDelayMs);
      }
    } catch (err) {
      watcher.error = replyError(err);
    } finally {
      busyReply = null;
    }
  }

  function replyError(err: unknown): string {
    const raw = (err as Error).message || "unknown error";
    if (/409|watcher|not attached|conflict/i.test(raw)) {
      onWatcherDetached?.();
      return "reply failed: watcher is no longer attached";
    }
    if (/400|invalid|bad request|schema/i.test(raw)) {
      return "reply failed: invalid survey reply";
    }
    return `reply failed: ${raw}`;
  }

  function dismissEvent(id: string, delayMs: number): void {
    let timer: ReturnType<typeof setTimeout>;
    const remove = () => {
      watcher.events = watcher.events.filter((candidate) => candidate.id !== id);
      dismissTimers.delete(timer);
    };
    if (delayMs <= 0) {
      watcher.events = watcher.events.filter((candidate) => candidate.id !== id);
      return;
    }
    timer = setTimeout(remove, delayMs);
    dismissTimers.add(timer);
  }

  function nextUnanswered(
    total: number,
    byQuestion: Record<number, string>,
    from: number,
  ): number | null {
    for (let offset = 1; offset <= total; offset += 1) {
      const idx = (from + offset) % total;
      if (byQuestion[idx] === undefined) return idx;
    }
    return null;
  }

  function setFocusedQuestion(event: WatcherEvent, idx: number): void {
    const total = event.questions?.length ?? 0;
    if (total === 0) return;
    focusedQuestion[event.id] = ((idx % total) + total) % total;
    focusedBubbleId = event.id;
  }

  function focusAdjacentBubble(delta: number): void {
    if (orderedEvents.length === 0) return;
    const current = Math.max(0, orderedEvents.findIndex((event) => event.id === focusedBubbleId));
    const next = (current + delta + orderedEvents.length) % orderedEvents.length;
    focusedBubbleId = orderedEvents[next]?.id ?? null;
  }

  function keyTarget(): WatcherEvent | null {
    if (collapsed) return null;
    return orderedEvents.find((event) => event.id === focusedBubbleId) ?? orderedEvents[0] ?? null;
  }

  function editableTarget(target: EventTarget | null): boolean {
    if (!(target instanceof Element)) return false;
    const el = target as HTMLElement;
    return Boolean(el?.closest("input, textarea, select, [contenteditable='true']"));
  }

  function onWindowKeydown(e: KeyboardEvent): void {
    if (editableTarget(e.target)) return;
    const event = keyTarget();
    if (!event || !isSurveyEvent(event)) return;
    if ((e.metaKey || e.ctrlKey) && e.key === "ArrowDown") {
      e.preventDefault();
      focusAdjacentBubble(1);
      return;
    }
    if ((e.metaKey || e.ctrlKey) && e.key === "ArrowUp") {
      e.preventDefault();
      focusAdjacentBubble(-1);
      return;
    }
    if (e.key === "Escape") {
      e.preventDefault();
      void skip(event);
      return;
    }
    if (e.key === "f" || e.key === "F") {
      e.preventDefault();
      void markFollowUp(event);
      return;
    }
    if (e.key === "Tab" || e.key === "ArrowRight" || e.key === "ArrowLeft") {
      const total = questionCount(event);
      if (total > 1) {
        e.preventDefault();
        const cur = focusedQuestion[event.id] ?? 0;
        setFocusedQuestion(event, cur + (e.shiftKey || e.key === "ArrowLeft" ? -1 : 1));
      }
      return;
    }
    if (/^[1-9]$/.test(e.key)) {
      const qi = focusedQuestion[event.id] ?? 0;
      const option = optionsFor(event, qi).find((candidate) => candidate.n === Number(e.key));
      if (option) {
        e.preventDefault();
        void answer(event, qi, option);
      }
    }
  }

  function isSurveyEvent(event: WatcherEvent): boolean {
    return event.type === "pre-flight" || (event.type === "survey" && Boolean(event.questions?.length));
  }

  function preFlightOptions(): SurveyOption[] {
    return [
      { key: "open-terminal", label: "Open the terminal" },
      { key: "kill", label: "Kill the spawn" },
      { key: "retry", label: "Retry now" },
    ];
  }

  function questionText(event: WatcherEvent, idx: number): string {
    if (event.type === "pre-flight") {
      return event.note || "Spawn needs attention. What now?";
    }
    return event.questions?.[idx]?.text ?? "";
  }

  function questionHeader(event: WatcherEvent, idx: number): string {
    if (event.type === "pre-flight") return "Spawn";
    return event.questions?.[idx]?.header || `Q${idx + 1}`;
  }

  function questionCount(event: WatcherEvent): number {
    return event.type === "pre-flight" ? 1 : visibleQuestions(event).length;
  }

  function elapsedLabel(event: WatcherEvent): string {
    const started = Number(event.topic);
    const base = Number.isFinite(started) && started > 0 ? started : event.id.match(/\d{10,}/)?.[0];
    const startMs = typeof base === "number" ? base : Number(base);
    const elapsed = Number.isFinite(startMs) && startMs > 0 ? Math.max(0, now - startMs) : 0;
    const seconds = Math.floor(elapsed / 1000);
    const mins = Math.floor(seconds / 60);
    const rem = seconds % 60;
    return `${mins}:${String(rem).padStart(2, "0")}`;
  }

  function preFlightTimedOut(event: WatcherEvent): boolean {
    const [m, s] = elapsedLabel(event).split(":").map(Number);
    return (m ?? 0) * 60 + (s ?? 0) >= 300;
  }
</script>

<svelte:window onkeydown={onWindowKeydown} />

{#if visibleEvents.length > 0 || watcher.loading || watcher.error}
  <section class="bubble-overlay" class:tray={mode === "tray"} aria-label="watcher events">
    {#if watcher.error}
      <div class="bubble error">{watcher.error}</div>
    {/if}
    {#if watcher.loading}
      <div class="bubble muted">Loading...</div>
    {:else if collapsed}
      <button type="button" class="tray-chip" onclick={() => (watcher.trayExpanded = true)}>
        <ChevronDown size={15} strokeWidth={1.8} aria-hidden="true" />
        <span>{visibleEvents.length} watcher event{visibleEvents.length === 1 ? "" : "s"}</span>
      </button>
    {:else}
      <div class="bubble-list">
        {#each orderedEvents as event (event.id)}
          <article
            class="bubble"
            class:focused={focusedBubbleId === event.id}
            onmouseenter={() => (focusedBubbleId = event.id)}
          >
            <div class="bubble-head">
              <span>{event.from}</span>
              <div class="bubble-head-actions">
                {#if followUps[event.id]}
                  <span class="follow-badge">follow up</span>
                {/if}
                {#if mode === "tray"}
                  <button type="button" class="icon" onclick={() => (watcher.trayExpanded = false)} aria-label="Collapse watcher tray" title="Collapse">
                    <ChevronUp size={14} strokeWidth={1.8} aria-hidden="true" />
                  </button>
                {/if}
                <button type="button" class="icon" onclick={() => void onRefresh()} aria-label="Refresh watcher events" title="Refresh">
                  <RefreshCw size={14} strokeWidth={1.8} aria-hidden="true" />
                </button>
              </div>
            </div>
            {#if isSurveyEvent(event)}
              {@const qi = focusedQuestion[event.id] ?? 0}
              {@const total = questionCount(event)}
              <div class="survey" data-multitopic={total > 1}>
                {#if total > 1}
                  <div class="topic-tabs" role="tablist" aria-label="survey topics">
                    {#each visibleQuestions(event) as topic, idx}
                      <button
                        type="button"
                        class:on={idx === qi}
                        class:answered={answers[event.id]?.[idx] !== undefined}
                        onclick={() => setFocusedQuestion(event, idx)}
                      >
                        <span>{questionHeader(event, idx)}</span>
                      </button>
                    {/each}
                  </div>
                {/if}
                {#if event.type === "pre-flight"}
                  <div class="preflight-status" class:timeout={preFlightTimedOut(event)}>
                    {#if preFlightTimedOut(event)}
                      <span>Spawn idle</span>
                    {:else}
                      <span class="spin">
                        <Loader2 size={13} strokeWidth={1.8} aria-hidden="true" />
                      </span>
                      <span>{elapsedLabel(event)}</span>
                    {/if}
                  </div>
                {/if}
                <p class="question">{event.type === "pre-flight" && preFlightTimedOut(event) ? "Spawn idle - retry now?" : questionText(event, qi)}</p>
                <div class="option-list">
                  {#each optionsFor(event, qi) as option (option.key)}
                    {#if event.type !== "pre-flight" || !preFlightTimedOut(event) || option.key === "retry"}
                    <button
                      type="button"
                      class:on={answers[event.id]?.[qi] === option.key}
                      disabled={busyReply === event.id}
                      onclick={() => void answer(event, qi, option)}
                    >
                      <kbd>{option.n}</kbd>
                      <span>{option.label}</span>
                    </button>
                    {/if}
                  {/each}
                </div>
                {#if questionOverflowCount(event) > 0 || optionOverflowCount(event, qi) > 0}
                  <p class="truncation">
                    {#if questionOverflowCount(event) > 0}
                      {questionOverflowCount(event)} extra topic{questionOverflowCount(event) === 1 ? "" : "s"} hidden.
                    {/if}
                    {#if optionOverflowCount(event, qi) > 0}
                      {optionOverflowCount(event, qi)} extra option{optionOverflowCount(event, qi) === 1 ? "" : "s"} hidden.
                    {/if}
                  </p>
                {/if}
                <button
                  type="button"
                  class="follow-button"
                  disabled={busyReply === event.id}
                  onclick={() => void markFollowUp(event)}
                >
                  <kbd>F</kbd>
                  <span>follow up</span>
                </button>
              </div>
            {:else}
              <p class="bubble-text">
                {#each textParts(event.note ?? `${event.type} from ${event.from}`) as part}
                  {#if part.href}
                    <button type="button" class="link" onclick={() => void openExternalUrl(part.href!)}>{part.text}</button>
                  {:else}{part.text}{/if}
                {/each}
              </p>
            {/if}
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
    padding: 9px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: color-mix(in srgb, var(--bg-card) 88%, transparent);
    color: var(--text);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.22);
    outline: none;
  }
  .bubble.focused {
    border-color: var(--link);
  }
  .bubble.error { color: var(--danger-text); }
  .bubble.muted { color: var(--text-secondary); }
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
  .preflight-status {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    margin-bottom: 6px;
    color: var(--text-secondary);
    font-size: 12px;
  }
  .preflight-status.timeout {
    color: var(--warn-text);
  }
  .spin {
    animation: spin 900ms linear infinite;
  }
  @keyframes spin {
    to { transform: rotate(360deg); }
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
  .topic-tabs button.answered::after {
    content: "*";
    margin-left: 4px;
    color: var(--success-text, var(--link));
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
  .option-list button.on {
    color: var(--link);
    border-color: var(--link);
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
  .follow-button:disabled {
    opacity: 0.65;
  }
  .follow-badge {
    min-height: 18px;
    display: inline-flex;
    align-items: center;
    padding: 0 6px;
    border: 1px solid var(--border);
    border-radius: 3px;
    color: var(--warn-text, var(--text-secondary));
    font-size: 11px;
  }
  .truncation {
    margin: 6px 0 0;
    color: var(--warn-text, var(--text-secondary));
    font-size: 12px;
  }
  .link {
    border: 0;
    background: transparent;
    color: var(--link);
    padding: 0;
    text-decoration: underline;
  }
  @media (prefers-reduced-motion: reduce) {
    .spin { animation: none; }
  }
</style>
