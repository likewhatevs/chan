<script lang="ts">
  // The Spotlight-style command launcher (Cmd+K / Ctrl+Alt+K). A top-anchored bubble
  // over OverlayShell: one search input plus a categorized, type-ahead
  // filtered list of every command available in the current window and
  // active surface. Keyboard-first: the input holds focus, arrows move a
  // highlight, Enter runs, Esc dismisses through the shared overlay
  // stack. Chorded commands show their current chord read-only.
  //
  // Focus discipline OverlayShell lacks: focus the input on open, trap
  // Tab inside the panel, and restore focus to the previously-focused
  // element on a dismissal (but not when a command runs, since that
  // command's action owns focus next).

  import { tick } from "svelte";
  import OverlayShell from "./OverlayShell.svelte";
  import { launcherPanel, closeCommandLauncher } from "../state/store.svelte";
  import {
    availableCommands,
    commandContext,
    COMMAND_CATEGORY_ORDER,
    type Command,
    type CommandCategory,
  } from "../state/commands";
  import { chordFor } from "../state/shortcuts";
  // Load the per-category registrations (side effect) so the catalog is
  // populated before the first read.
  import "../state/commands/install";

  const LIST_ID = "command-launcher-list";
  const optionId = (i: number): string => `command-launcher-opt-${i}`;

  let inputEl = $state<HTMLInputElement>();
  let highlight = $state(0);

  // Where focus was before opening, restored on a dismissal. `ranCommand`
  // suppresses that restore when a command ran (its action owns focus).
  // `wasOpen` latches the open/close transition; plain lets so they never
  // feed reactivity back into the effect that maintains them.
  let restoreTarget: HTMLElement | null = null;
  let ranCommand = false;
  let wasOpen = false;

  // Fuzzy subsequence score for `query` (already lowercased) against
  // `text`. Prefix beats substring beats a scattered subsequence;
  // contiguous runs and earlier positions score higher. null = no match.
  function fuzzyScore(query: string, text: string): number | null {
    if (query === "") return 0;
    const t = text.toLowerCase();
    const at = t.indexOf(query);
    if (at === 0) return 1000 - text.length;
    if (at > 0) return 600 - at;
    let ti = 0;
    let score = 0;
    let prev = -2;
    for (const c of query) {
      let found = -1;
      while (ti < t.length) {
        if (t[ti] === c) {
          found = ti;
          break;
        }
        ti++;
      }
      if (found === -1) return null;
      score += found === prev + 1 ? 8 : 2;
      score -= found;
      prev = found;
      ti = found + 1;
    }
    return score;
  }

  // Best score across a command's title and keywords, or null if none
  // match. An empty query matches everything (score 0), preserving the
  // registration order for a stable full list.
  function commandScore(cmd: Command, query: string): number | null {
    if (query === "") return 0;
    let best: number | null = null;
    for (const text of [cmd.title, ...(cmd.keywords ?? [])]) {
      const s = fuzzyScore(query, text);
      if (s !== null && (best === null || s > best)) best = s;
    }
    return best;
  }

  type Row = { cmd: Command; index: number };
  type Group = { category: CommandCategory; rows: Row[] };

  const ctx = $derived(commandContext());

  // Filtered, category-grouped rows. Each row carries its flat index so
  // arrow navigation and aria-activedescendant span groups in order.
  const groups = $derived.by<Group[]>(() => {
    const query = launcherPanel.query.trim().toLowerCase();
    const scored = availableCommands(ctx)
      .map((cmd) => ({ cmd, score: commandScore(cmd, query) }))
      .filter((x): x is { cmd: Command; score: number } => x.score !== null);
    const out: Group[] = [];
    let index = 0;
    for (const category of COMMAND_CATEGORY_ORDER) {
      const inCat = scored.filter((x) => x.cmd.category === category);
      if (inCat.length === 0) continue;
      if (query !== "") inCat.sort((a, b) => b.score - a.score);
      out.push({
        category,
        rows: inCat.map((x) => ({ cmd: x.cmd, index: index++ })),
      });
    }
    return out;
  });

  const flat = $derived(groups.flatMap((g) => g.rows.map((r) => r.cmd)));

  // Snap the highlight back to the top whenever the visible set changes.
  $effect(() => {
    void launcherPanel.query;
    void flat.length;
    highlight = 0;
  });

  // Open/close focus management.
  $effect(() => {
    const open = launcherPanel.open;
    if (open && !wasOpen) {
      restoreTarget = document.activeElement as HTMLElement | null;
      ranCommand = false;
      highlight = 0;
      void tick().then(() => inputEl?.focus());
    } else if (!open && wasOpen) {
      if (!ranCommand && restoreTarget && document.contains(restoreTarget)) {
        restoreTarget.focus();
      }
      restoreTarget = null;
    }
    wasOpen = open;
  });

  function scrollHighlightIntoView(): void {
    void tick().then(() => {
      document
        .getElementById(optionId(highlight))
        ?.scrollIntoView({ block: "nearest" });
    });
  }

  function run(cmd: Command): void {
    // Close first so a command that opens another surface lands on top of
    // the dismissed launcher; the command's action owns focus next, so
    // suppress focus restore.
    ranCommand = true;
    closeCommandLauncher();
    cmd.run();
  }

  function onKeydown(e: KeyboardEvent): void {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      e.stopPropagation();
      if (flat.length) highlight = (highlight + 1) % flat.length;
      scrollHighlightIntoView();
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      e.stopPropagation();
      if (flat.length) highlight = (highlight - 1 + flat.length) % flat.length;
      scrollHighlightIntoView();
    } else if (e.key === "Enter") {
      e.preventDefault();
      e.stopPropagation();
      const cmd = flat[highlight];
      if (cmd) run(cmd);
    } else if (e.key === "Tab") {
      // The input is the only focusable control, so keep focus on it
      // rather than letting Tab escape to the page behind the overlay.
      e.preventDefault();
      inputEl?.focus();
    }
    // Escape is intentionally unhandled here: it bubbles to App.svelte's
    // window keydown handler, which pops the topmost overlay (this
    // launcher) through the shared overlay-stack path.
  }
</script>

<OverlayShell
  id="launcher"
  open={launcherPanel.open}
  onClose={closeCommandLauncher}
  align="top"
  width="min(640px, calc(100vw - 32px))"
>
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="launcher" onkeydown={onKeydown}>
    <input
      class="search"
      bind:this={inputEl}
      bind:value={launcherPanel.query}
      type="text"
      role="combobox"
      aria-expanded="true"
      aria-controls={LIST_ID}
      aria-activedescendant={flat.length ? optionId(highlight) : undefined}
      aria-autocomplete="list"
      aria-label="Search commands"
      placeholder="Type a command"
      autocomplete="off"
      autocorrect="off"
      autocapitalize="off"
      spellcheck="false"
    />
    <div class="results" id={LIST_ID} role="listbox" aria-label="Commands">
      {#if flat.length === 0}
        <div class="empty">No commands</div>
      {:else}
        {#each groups as group (group.category)}
          <div class="group" role="group" aria-label={group.category}>
            <div class="group-label">{group.category}</div>
            {#each group.rows as row (row.cmd.id + "␟" + row.cmd.title)}
              {@const chord = chordFor(row.cmd.id)}
              <!-- svelte-ignore a11y_click_events_have_key_events -->
              <div
                class="row"
                class:active={row.index === highlight}
                id={optionId(row.index)}
                role="option"
                tabindex="-1"
                aria-selected={row.index === highlight}
                onclick={() => run(row.cmd)}
                onmouseenter={() => (highlight = row.index)}
              >
                <span class="title">{row.cmd.title}</span>
                {#if chord}<span class="chord">{chord}</span>{/if}
              </div>
            {/each}
          </div>
        {/each}
      {/if}
    </div>
  </div>
</OverlayShell>

<style>
  .launcher {
    display: flex;
    flex-direction: column;
    min-height: 0;
    max-height: min(70vh, 560px);
  }
  .search {
    flex: 0 0 auto;
    box-sizing: border-box;
    width: 100%;
    padding: 16px 18px;
    border: none;
    border-bottom: 1px solid var(--border);
    background: transparent;
    color: var(--text);
    font-size: 17px;
    line-height: 1.3;
    outline: none;
  }
  .search::placeholder {
    color: var(--text-secondary);
  }
  .results {
    flex: 1 1 auto;
    min-height: 0;
    overflow-y: auto;
    padding: 6px;
  }
  .empty {
    padding: 18px;
    text-align: center;
    color: var(--text-secondary);
    font-size: 13px;
  }
  .group + .group {
    margin-top: 4px;
  }
  .group-label {
    padding: 6px 10px 2px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--text-secondary);
  }
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 8px 10px;
    border-radius: 6px;
    cursor: pointer;
    color: var(--text);
  }
  .row:hover {
    background: var(--hover-bg);
  }
  /* Keyboard highlight is a solid accent bar, distinct from the subtler
     mouse hover so the arrow cursor is never ambiguous. */
  .row.active {
    background: var(--accent);
    color: #fff;
  }
  .title {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 14px;
  }
  .chord {
    flex: 0 0 auto;
    padding: 1px 7px;
    border: 1px solid var(--border);
    border-radius: 5px;
    background: var(--code-bg);
    color: var(--text-secondary);
    font-size: 11px;
    white-space: nowrap;
  }
  .row.active .chord {
    border-color: rgba(255, 255, 255, 0.5);
    background: rgba(255, 255, 255, 0.16);
    color: #fff;
  }
</style>
