<script lang="ts">
  import { ChevronDown, ChevronUp, RefreshCw } from "lucide-svelte";
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
  }: {
    watcher: TerminalWatcherState;
    sessionId?: string;
    onRefresh: () => Promise<void> | void;
    onWatcherDetached?: () => void;
  } = $props();

  type NumberedOption = SurveyOption & { n: number };

  const mode = $derived<BubbleOverlayMode>(
    drive.info?.preferences.bubble_overlay_mode === "tray" ? "tray" : "stack",
  );
  const visibleEvents = $derived(watcher.events.filter((event) => event.type !== "survey-reply"));
  const orderedEvents = $derived([...visibleEvents].reverse());
  const collapsed = $derived(mode === "tray" && !watcher.trayExpanded);
  let answers = $state<Record<string, Record<number, string>>>({});
  let focusedQuestion = $state<Record<string, number>>({});
  let focusedBubbleId = $state<string | null>(null);
  let busyReply = $state<string | null>(null);
  const dismissTimers = new Set<ReturnType<typeof setTimeout>>();

  $effect(() => {
    return () => {
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
    const question = event.questions?.[questionIndex];
    const base = question?.options ?? [];
    const standing = normalizeStandingOptions(event.standing_options);
    return [...base, ...standing].slice(0, 9).map((option, idx) => ({
      ...option,
      n: idx + 1,
    }));
  }

  async function answer(event: WatcherEvent, questionIndex: number, option: SurveyOption): Promise<void> {
    const byQuestion = { ...(answers[event.id] ?? {}), [questionIndex]: option.key };
    answers[event.id] = byQuestion;
    const total = event.questions?.length ?? 0;
    if (total <= 1) {
      await commit(event, byQuestion, 600);
      return;
    }
    const next = nextUnanswered(total, byQuestion, questionIndex);
    if (next === null) {
      await commit(event, byQuestion, 600);
    } else {
      focusedQuestion[event.id] = next;
    }
  }

  async function skip(event: WatcherEvent): Promise<void> {
    await commit(event, {}, 0);
  }

  async function commit(
    event: WatcherEvent,
    byQuestion: Record<number, string>,
    dismissDelayMs: number,
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
      await writeSurveyReply(sessionId, event, replyAnswers, "one-shot");
      dismissEvent(event.id, dismissDelayMs);
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
    if (!event || event.type !== "survey" || !event.questions?.length) return;
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
    if (e.key === "Tab" || e.key === "ArrowRight" || e.key === "ArrowLeft") {
      const total = event.questions.length;
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
            {#if event.type === "survey" && event.questions?.length}
              {@const qi = focusedQuestion[event.id] ?? 0}
              {@const question = event.questions[qi] ?? event.questions[0]}
              <div class="survey" data-multitopic={event.questions.length > 1}>
                {#if event.questions.length > 1}
                  <div class="topic-tabs" role="tablist" aria-label="survey topics">
                    {#each event.questions as topic, idx}
                      <button
                        type="button"
                        class:on={idx === qi}
                        class:answered={answers[event.id]?.[idx] !== undefined}
                        onclick={() => setFocusedQuestion(event, idx)}
                      >
                        <span>{topic.header || `Q${idx + 1}`}</span>
                      </button>
                    {/each}
                  </div>
                {/if}
                <p class="question">{question?.text ?? ""}</p>
                <div class:option-row={event.questions.length === 1} class:option-stack={event.questions.length > 1}>
                  {#each optionsFor(event, qi) as option (option.key)}
                    <button
                      type="button"
                      class:on={answers[event.id]?.[qi] === option.key}
                      disabled={busyReply === event.id}
                      onclick={() => void answer(event, qi, option)}
                    >
                      <kbd>{option.n}</kbd>
                      <span>{option.label}</span>
                    </button>
                  {/each}
                </div>
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
  .option-row button,
  .option-stack button,
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
  .option-row {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .option-stack {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .option-row button,
  .option-stack button {
    min-height: 28px;
    display: inline-flex;
    align-items: center;
    gap: 7px;
    padding: 0 9px;
    text-align: left;
  }
  .option-stack button {
    justify-content: flex-start;
  }
  .option-row button.on,
  .option-stack button.on {
    color: var(--link);
    border-color: var(--link);
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
  .link {
    border: 0;
    background: transparent;
    color: var(--link);
    padding: 0;
    text-decoration: underline;
  }
</style>
