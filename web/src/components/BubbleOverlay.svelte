<script lang="ts">
  import { Check, ChevronDown, ChevronUp, CircleSlash, RefreshCw } from "lucide-svelte";
  import { api } from "../api/client";
  import type { BubbleOverlayMode } from "../api/types";
  import { openExternalUrl } from "../editor/external_links";
  import { drive } from "../state/store.svelte";
  import type { ScopeGrant, TerminalWatcherState, WatcherEvent } from "../state/tabs.svelte";
  import { normalizeStandingOptions, writeSurveyReply } from "../state/watcherEvents";

  let {
    watcher,
    onRefresh,
  }: {
    watcher: TerminalWatcherState;
    onRefresh: () => Promise<void> | void;
  } = $props();

  const mode = $derived<BubbleOverlayMode>(
    drive.info?.preferences.bubble_overlay_mode === "tray" ? "tray" : "stack",
  );
  const visibleEvents = $derived(watcher.events.filter((event) => event.type !== "survey-reply"));
  const collapsed = $derived(mode === "tray" && !watcher.trayExpanded);
  let selections = $state<Record<string, Record<number, string>>>({});
  let scopeGrants = $state<Record<string, ScopeGrant>>({});
  let busyReply = $state<string | null>(null);

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

  async function setMode(next: BubbleOverlayMode): Promise<void> {
    if (mode === next) return;
    if (drive.info) {
      drive.info = {
        ...drive.info,
        preferences: { ...drive.info.preferences, bubble_overlay_mode: next },
      };
    }
    await api.setBubbleOverlayMode(next);
  }

  function choose(event: WatcherEvent, questionIndex: number, key: string): void {
    const byQuestion = { ...(selections[event.id] ?? {}) };
    byQuestion[questionIndex] = key;
    selections[event.id] = byQuestion;
  }

  async function submit(event: WatcherEvent, skip = false): Promise<void> {
    busyReply = event.id;
    watcher.error = undefined;
    try {
      const answers = skip
        ? []
        : Object.entries(selections[event.id] ?? {}).map(([idx, key]) => ({
            question_index: Number(idx),
            key,
          }));
      await writeSurveyReply(
        watcher.path,
        event,
        answers,
        skip ? "one-shot" : scopeGrants[event.id] ?? "one-shot",
      );
      watcher.events = watcher.events.filter((candidate) => candidate.id !== event.id);
    } catch (err) {
      watcher.error = `reply failed: ${(err as Error).message}`;
    } finally {
      busyReply = null;
    }
  }
</script>

{#if visibleEvents.length > 0 || watcher.loading || watcher.error}
  <section class="bubble-overlay" class:tray={mode === "tray"} aria-label="watcher events">
    <div class="bubble-toolbar">
      <button type="button" class:on={mode === "stack"} onclick={() => void setMode("stack")}>stack</button>
      <button type="button" class:on={mode === "tray"} onclick={() => void setMode("tray")}>tray</button>
      <button type="button" class="icon" onclick={() => void onRefresh()} aria-label="Refresh watcher events" title="Refresh">
        <RefreshCw size={14} strokeWidth={1.8} aria-hidden="true" />
      </button>
    </div>
    {#if watcher.error}
      <div class="bubble error">{watcher.error}</div>
    {/if}
    {#if watcher.loading}
      <div class="bubble muted">Loading…</div>
    {:else if collapsed}
      <button type="button" class="tray-chip" onclick={() => (watcher.trayExpanded = true)}>
        <ChevronDown size={15} strokeWidth={1.8} aria-hidden="true" />
        <span>{visibleEvents.length} watcher event{visibleEvents.length === 1 ? "" : "s"}</span>
      </button>
    {:else}
      {#if mode === "tray"}
        <button type="button" class="tray-chip" onclick={() => (watcher.trayExpanded = false)}>
          <ChevronUp size={15} strokeWidth={1.8} aria-hidden="true" />
          <span>collapse</span>
        </button>
      {/if}
      <div class="bubble-list">
        {#each visibleEvents as event (event.id)}
          <article class="bubble">
            <div class="bubble-head">
              <span>{event.from}</span>
              {#if event.topic}<span>{event.topic}</span>{/if}
            </div>
            {#if event.type === "survey" && event.questions?.length}
              <div class="survey">
                {#each event.questions as question, qi}
                  <fieldset>
                    <legend>{question.header}</legend>
                    <p>{question.text}</p>
                    <div class="choices">
                      {#each question.options as option (option.key)}
                        <button
                          type="button"
                          class:on={selections[event.id]?.[qi] === option.key}
                          onclick={() => choose(event, qi, option.key)}
                        >
                          {#if selections[event.id]?.[qi] === option.key}
                            <Check size={13} strokeWidth={2} aria-hidden="true" />
                          {/if}
                          <span>{option.label}</span>
                        </button>
                      {/each}
                    </div>
                  </fieldset>
                {/each}
                <div class="standing">
                  {#each normalizeStandingOptions(event.standing_options) as option (option.key)}
                    <button type="button" onclick={() => choose(event, 0, option.key)}>
                      {option.label}
                    </button>
                  {/each}
                </div>
                <label class="scope">
                  <span>Scope</span>
                  <select
                    value={scopeGrants[event.id] ?? "one-shot"}
                    onchange={(e) => (scopeGrants[event.id] = (e.currentTarget as HTMLSelectElement).value as ScopeGrant)}
                  >
                    <option value="one-shot">one-shot</option>
                    <option value="topic-session">topic-session</option>
                    <option value="topic-phase">topic-phase</option>
                  </select>
                </label>
                <div class="survey-actions">
                  <button type="button" onclick={() => void submit(event)} disabled={busyReply === event.id}>
                    Submit
                  </button>
                  <button type="button" onclick={() => void submit(event, true)} disabled={busyReply === event.id}>
                    <CircleSlash size={13} strokeWidth={1.8} aria-hidden="true" />
                    <span>Skip / not now</span>
                  </button>
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
  .bubble-overlay :where(button, select) { pointer-events: auto; }
  .bubble-toolbar {
    display: flex;
    justify-content: flex-end;
    gap: 4px;
  }
  .bubble-toolbar button,
  .tray-chip,
  .choices button,
  .standing button,
  .survey-actions button {
    border: 1px solid var(--border);
    background: color-mix(in srgb, var(--bg-card) 92%, transparent);
    color: var(--text);
    border-radius: 4px;
    font: inherit;
  }
  .bubble-toolbar button {
    min-height: 24px;
    padding: 0 8px;
    color: var(--text-secondary);
  }
  .bubble-toolbar button.on,
  .choices button.on {
    color: var(--link);
    border-color: var(--link);
  }
  .bubble-toolbar .icon {
    width: 28px;
    padding: 0;
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
    width: min(560px, 100%);
    padding: 10px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: color-mix(in srgb, var(--bg-card) 88%, transparent);
    color: var(--text);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.22);
    pointer-events: auto;
  }
  .bubble.error { color: var(--danger-text); }
  .bubble.muted { color: var(--text-secondary); }
  .bubble-head {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 8px;
    font-size: 12px;
    color: var(--text-secondary);
  }
  fieldset {
    border: 0;
    padding: 0;
    margin: 0 0 9px;
  }
  legend {
    font-size: 12px;
    font-weight: 600;
    padding: 0;
  }
  p {
    margin: 3px 0 7px;
    line-height: 1.35;
  }
  .choices,
  .standing,
  .survey-actions,
  .scope {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
  }
  .choices button,
  .standing button,
  .survey-actions button {
    min-height: 28px;
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 0 9px;
  }
  .standing {
    margin-top: 6px;
    padding-top: 7px;
    border-top: 1px solid var(--border);
  }
  .scope {
    margin-top: 8px;
    font-size: 12px;
    color: var(--text-secondary);
  }
  .scope select {
    height: 26px;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--bg-card);
    color: var(--text);
  }
  .survey-actions {
    margin-top: 9px;
    justify-content: flex-end;
  }
  .link {
    border: 0;
    background: transparent;
    color: var(--link);
    padding: 0;
    text-decoration: underline;
  }
</style>
