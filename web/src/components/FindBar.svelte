<script lang="ts">
  // In-document find bar. Sits above the editor body, toggled by
  // Cmd/Ctrl+F. Drives whichever editor mode is active (WYSIWYG
  // via the FindExtension; Source via @codemirror/search) through
  // the unified `find*` API surface each editor exports.
  //
  // Shape: thin top-anchored bar with a query input, "n of total"
  // counter, prev/next arrows, a case-sensitive toggle (Aa), and
  // a close button. Mirrors what Chrome / Firefox / VSCode show
  // for an in-document find. Esc closes; Enter steps next;
  // Shift+Enter steps previous.

  import { onMount, tick } from "svelte";

  type Snapshot = { matches: number; current: number };

  let {
    open = $bindable(false),
    onSetQuery,
    onStep,
    onClose,
  }: {
    open: boolean;
    /// Update the active query. The host adapts the snapshot back
    /// to a uniform `{ matches, current }` regardless of which
    /// editor is driving (WYSIWYG returns a richer FindState; we
    /// only care about the counts here).
    onSetQuery: (query: string, caseSensitive: boolean) => Snapshot;
    onStep: (delta: number) => Snapshot;
    onClose: () => void;
  } = $props();

  let inputEl: HTMLInputElement | undefined = $state();
  let query = $state("");
  let caseSensitive = $state(false);
  let snap = $state<Snapshot>({ matches: 0, current: 0 });

  // Cmd+F again while the bar is open: re-focus and select-all so
  // the user can type a new query immediately (browser behavior).
  export async function focusAndSelect(): Promise<void> {
    await tick();
    if (!inputEl) return;
    inputEl.focus();
    inputEl.select();
  }

  function onInput(): void {
    snap = onSetQuery(query, caseSensitive);
  }

  function toggleCase(): void {
    caseSensitive = !caseSensitive;
    snap = onSetQuery(query, caseSensitive);
  }

  function step(delta: number): void {
    if (snap.matches === 0) return;
    snap = onStep(delta);
  }

  function close(): void {
    snap = { matches: 0, current: 0 };
    onClose();
  }

  function onKeyDown(e: KeyboardEvent): void {
    // Capture-phase: this listener runs at element scope; only the
    // input element fires here, so we don't need to pre-empt other
    // editor handlers explicitly.
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      close();
    } else if (e.key === "Enter") {
      e.preventDefault();
      step(e.shiftKey ? -1 : 1);
    }
  }

  onMount(() => {
    // Auto-focus on mount: the host shows the bar on Cmd+F, so
    // we want the cursor in the input ready for the user to type.
    void focusAndSelect();
  });
</script>

{#if open}
  <div class="find-bar" role="search">
    <input
      bind:this={inputEl}
      bind:value={query}
      class="find-input"
      placeholder="Find in document"
      spellcheck="false"
      autocomplete="off"
      autocapitalize="off"
      autocorrect="off"
      oninput={onInput}
      onkeydown={onKeyDown}
    />
    <span
      class="find-count"
      class:none={query !== "" && snap.matches === 0}
      title="match count"
    >
      {#if query === ""}
        &nbsp;
      {:else if snap.matches === 0}
        no results
      {:else}
        {snap.current} of {snap.matches}
      {/if}
    </span>
    <button
      type="button"
      class="find-btn case"
      class:on={caseSensitive}
      title="match case (Aa)"
      onclick={toggleCase}
    >Aa</button>
    <button
      type="button"
      class="find-btn"
      title="previous match (Shift+Enter)"
      onclick={() => step(-1)}
      disabled={snap.matches === 0}
    >‹</button>
    <button
      type="button"
      class="find-btn"
      title="next match (Enter)"
      onclick={() => step(1)}
      disabled={snap.matches === 0}
    >›</button>
    <button
      type="button"
      class="find-btn close"
      title="close (Esc)"
      onclick={close}
    >✕</button>
  </div>
{/if}

<style>
  .find-bar {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 4px 6px;
    background: var(--bg-card);
    border-bottom: 1px solid var(--border);
    font-size: 12px;
    color: var(--text);
    flex-shrink: 0;
  }
  .find-input {
    flex: 1;
    min-width: 80px;
    padding: 3px 6px;
    background: var(--bg);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: 3px;
    outline: none;
    font: inherit;
  }
  .find-input:focus {
    border-color: var(--link);
  }
  .find-count {
    min-width: 64px;
    text-align: right;
    color: var(--text-secondary);
    font-variant-numeric: tabular-nums;
  }
  .find-count.none {
    color: var(--warn-text);
  }
  .find-btn {
    background: transparent;
    color: var(--text-secondary);
    border: 1px solid transparent;
    border-radius: 3px;
    padding: 2px 6px;
    cursor: pointer;
    font: inherit;
    line-height: 1;
  }
  .find-btn:hover:not(:disabled) {
    color: var(--text);
    border-color: var(--btn-border);
  }
  .find-btn:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .find-btn.case.on {
    color: var(--link);
    border-color: var(--link);
  }
  .find-btn.close {
    margin-left: 4px;
  }
</style>
