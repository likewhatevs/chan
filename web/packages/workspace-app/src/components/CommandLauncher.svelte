<script lang="ts">
  // The Spotlight-style command launcher (Cmd+K / Ctrl+Alt+K). A centered
  // search capsule over OverlayShell: typing lifts it and opens a categorized,
  // type-ahead filtered list of every command available in the current window
  // and active surface. Keyboard-first: the input holds focus, arrows move a
  // highlight, Enter runs, Esc dismisses through the shared overlay stack.
  // Each row shows its resolved chord and doubles as an assign affordance.
  //
  // Focus discipline OverlayShell lacks: focus the input on open, trap
  // Tab inside the panel, and restore focus to the previously-focused
  // element on a dismissal (but not when a command runs, since that
  // command's action owns focus next).

  import { tick } from "svelte";
  import {
    BarChart2,
    ChevronRight,
    Command as CommandIcon,
    FilePlus,
    FileText,
    Folder,
    Network,
    Search as SearchIcon,
    Settings2,
    Shapes,
    SquareStack,
    Terminal,
  } from "lucide-svelte";
  import OverlayShell from "./OverlayShell.svelte";
  import CommandChordAssign from "./CommandChordAssign.svelte";
  import {
    launcherPanel,
    closeCommandLauncher,
    launcherReturnFocus,
  } from "../state/store.svelte";
  import {
    availableCommands,
    commandContext,
    type Command,
    type CommandCategory,
    type CommandSurface,
  } from "../state/commands";
  // Load the per-category registrations (side effect) so the catalog is
  // populated before the first read.
  import "../state/commands/install";

  const LIST_ID = "command-launcher-list";
  const optionId = (i: number): string => `command-launcher-opt-${i}`;
  type IconComponent = typeof SearchIcon;
  const categoryIcons: Record<CommandCategory, IconComponent> = {
    Global: CommandIcon,
    Workspace: Folder,
    Search: SearchIcon,
    Apps: FilePlus,
    Tabs: SquareStack,
    Panes: Shapes,
    Editor: FileText,
    "File Browser": Folder,
    Terminal,
    Dashboard: BarChart2,
    Graph: Network,
  };
  const namedIcons: Record<string, IconComponent> = {
    command: CommandIcon,
    dashboard: BarChart2,
    file: FileText,
    folder: Folder,
    graph: Network,
    panes: Shapes,
    search: SearchIcon,
    settings: Settings2,
    tabs: FilePlus,
    terminal: Terminal,
  };

  let inputEl = $state<HTMLInputElement>();
  let highlight = $state(0);
  let highlightedCommandKey: string | null = null;
  let lastQuery = "";

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
  // match. A non-null score ranks a command in the top "Results" section
  // (best first). A null score does NOT hide the command: it drops to the
  // discovery catalog below, so a query only promotes matches, never
  // removes anything available in the current context.
  function commandScore(cmd: Command, query: string): number | null {
    if (query === "") return 0;
    let best: number | null = null;
    for (const text of [cmd.title, ...(cmd.keywords ?? [])]) {
      const s = fuzzyScore(query, text);
      if (s !== null && (best === null || s > best)) best = s;
    }
    return best;
  }

  /// One runnable list entry: the command plus, for an acceptsArg match
  /// via the head-token split, the verbatim argument remainder that
  /// run() will receive.
  type Entry = { cmd: Command; arg?: string };
  type Row = Entry & { index: number };
  type Group = { label: string; rows: Row[] };
  type ScoredCommand = Entry & { score: number };

  const ctx = $derived(commandContext());
  const hasQuery = $derived(launcherPanel.query.trim().length > 0);

  function compareText(a: string, b: string): number {
    return (
      a.localeCompare(b, undefined, { sensitivity: "base" }) ||
      a.localeCompare(b)
    );
  }

  function compareCommands(a: ScoredCommand, b: ScoredCommand): number {
    return (
      compareText(a.cmd.title, b.cmd.title) ||
      compareText(a.cmd.category, b.cmd.category) ||
      compareText(a.cmd.id, b.cmd.id)
    );
  }

  function categoryForSurface(
    surface: CommandSurface | null,
  ): CommandCategory | null {
    switch (surface) {
      case "file":
        return "Editor";
      case "browser":
        return "File Browser";
      case "terminal":
        return "Terminal";
      case "dashboard":
        return "Dashboard";
      case "graph":
        return "Graph";
      default:
        return null;
    }
  }

  function compareCategories(
    active: CommandCategory | null,
    a: CommandCategory,
    b: CommandCategory,
  ): number {
    if (a === active && b !== active) return -1;
    if (b === active && a !== active) return 1;
    return compareText(a, b);
  }

  function iconFor(cmd: Command): IconComponent {
    if (cmd.icon) return namedIcons[cmd.icon] ?? categoryIcons[cmd.category];
    return categoryIcons[cmd.category];
  }

  function commandKey(cmd: Command): string {
    return `${cmd.id}\u001f${cmd.category}\u001f${cmd.title}`;
  }

  function setHighlight(index: number): void {
    // -1 is the explicit "no row highlighted" state: `flat[-1]` is
    // undefined, so Enter no-ops until the user picks a row.
    if (index < 0 || flat.length === 0) {
      highlight = -1;
      highlightedCommandKey = null;
      return;
    }
    const n = ((index % flat.length) + flat.length) % flat.length;
    highlight = n;
    highlightedCommandKey = commandKey(flat[n].cmd);
  }

  // The query never hides an available command; it only reorders. Matches
  // float to a top "Results" section (best score first). Below them the
  // full remaining catalog stays visible, grouped by category with the
  // active surface's category pinned first, so a partial query still lets
  // the user discover everything available in this context (the terminal
  // options on a terminal tab, the dashboard jumps on a dashboard tab, and
  // then the rest). Each row carries its flat index so arrow navigation and
  // aria-activedescendant span the sections in order.
  const groups = $derived.by<Group[]>(() => {
    const raw = launcherPanel.query.trim();
    const query = raw.toLowerCase();
    if (query === "") return [];
    // Head-token split for arg-accepting commands: "Open notes/x.md"
    // scores "open" against the command and carries "notes/x.md" to
    // run() VERBATIM (no lowercasing - paths are case-sensitive).
    const argSplit = /^(\S+)\s+(.+)$/.exec(raw);
    const argHead = argSplit ? argSplit[1].toLowerCase() : null;
    const argRest = argSplit ? argSplit[2] : null;
    const matches: ScoredCommand[] = [];
    const rest: Command[] = [];
    for (const cmd of availableCommands(ctx)) {
      const score = commandScore(cmd, query);
      if (score !== null) {
        matches.push({ cmd, score });
        continue;
      }
      if (cmd.acceptsArg && argHead !== null && argRest !== null) {
        const headScore = commandScore(cmd, argHead);
        if (headScore !== null) {
          matches.push({ cmd, score: headScore, arg: argRest });
          continue;
        }
      }
      rest.push(cmd);
    }
    const out: Group[] = [];
    let index = 0;
    // Top: the query matches, best score first (alphabetical tiebreak).
    if (matches.length) {
      matches.sort((a, b) => b.score - a.score || compareCommands(a, b));
      out.push({
        label: "Results",
        rows: matches.map((x) => ({ cmd: x.cmd, arg: x.arg, index: index++ })),
      });
    }
    // Below: the rest of the available catalog, nothing dropped, grouped by
    // category with the active surface pinned first.
    const byCategory = new Map<CommandCategory, Command[]>();
    for (const cmd of rest) {
      const inCat = byCategory.get(cmd.category) ?? [];
      inCat.push(cmd);
      byCategory.set(cmd.category, inCat);
    }
    const activeCategory = categoryForSurface(ctx.activeSurface);
    const categories = [...byCategory.keys()].sort((a, b) =>
      compareCategories(activeCategory, a, b),
    );
    for (const category of categories) {
      const inCat = byCategory.get(category);
      if (!inCat) continue;
      inCat.sort(
        (a, b) => compareText(a.title, b.title) || compareText(a.id, b.id),
      );
      out.push({
        label: category,
        rows: inCat.map((cmd) => ({ cmd, index: index++ })),
      });
    }
    return out;
  });

  const flat = $derived(
    groups.flatMap((g) => g.rows.map((r): Entry => ({ cmd: r.cmd, arg: r.arg }))),
  );

  // Whether the current query matched anything. Matches float into the
  // leading "Results" section (no CommandCategory shares that label), so
  // its presence is the signal.
  const hasMatches = $derived(groups.length > 0 && groups[0].label === "Results");

  // Query changes start at the top match. A query that matches nothing
  // keeps the discovery catalog visible but highlights NO row, so a blind
  // Enter can't run a command the user never picked; arrow keys still step
  // into the catalog. Catalog recomputes keep the same highlighted command
  // by id/title/category so appending discovery rows below Results does
  // not steal the active selection.
  $effect(() => {
    const query = launcherPanel.query;
    const rows = flat;
    const autoIndex = hasQuery && !hasMatches ? -1 : 0;
    if (query !== lastQuery) {
      lastQuery = query;
      setHighlight(autoIndex);
      return;
    }
    if (rows.length === 0) {
      highlight = -1;
      highlightedCommandKey = null;
      return;
    }
    if (highlightedCommandKey) {
      const index = rows.findIndex(
        (entry) => commandKey(entry.cmd) === highlightedCommandKey,
      );
      if (index >= 0) {
        highlight = index;
        return;
      }
    }
    setHighlight(autoIndex);
  });

  // Open/close focus management. The pre-launcher element is captured by
  // openCommandLauncher() (module-level, so a command's own dialog flow
  // can restore it after the launcher is gone); this component just reads
  // it back for the plain-dismissal restore.
  $effect(() => {
    const open = launcherPanel.open;
    if (open && !wasOpen) {
      restoreTarget = launcherReturnFocus();
      ranCommand = false;
      lastQuery = launcherPanel.query;
      setHighlight(0);
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

  function run(entry: Entry): void {
    // Close first so a command that opens another surface lands on top of
    // the dismissed launcher; the command's action owns focus next, so
    // suppress focus restore. The head-token remainder (if any) rides to
    // run() verbatim; a bare invocation passes undefined.
    ranCommand = true;
    closeCommandLauncher();
    entry.cmd.run(entry.arg);
  }

  function onKeydown(e: KeyboardEvent): void {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      e.stopPropagation();
      // From the unhighlighted state (no-match query) enter the catalog at
      // its first row; ArrowUp mirrors with the last.
      if (flat.length) setHighlight(highlight < 0 ? 0 : highlight + 1);
      scrollHighlightIntoView();
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      e.stopPropagation();
      if (flat.length) setHighlight(highlight <= 0 ? flat.length - 1 : highlight - 1);
      scrollHighlightIntoView();
    } else if (e.key === "Enter") {
      e.preventDefault();
      e.stopPropagation();
      const entry = flat[highlight];
      if (entry) run(entry);
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
  align="center"
  lifted={hasQuery}
  width="min(760px, calc(100vw - 32px))"
>
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="launcher" class:has-results={hasQuery} onkeydown={onKeydown}>
    <div class="search-row">
      <ChevronRight size={28} strokeWidth={1.45} aria-hidden="true" />
      <input
        class="search"
        bind:this={inputEl}
        bind:value={launcherPanel.query}
        type="text"
        role="combobox"
        aria-expanded={hasQuery}
        aria-controls={LIST_ID}
        aria-activedescendant={flat.length && highlight >= 0 ? optionId(highlight) : undefined}
        aria-autocomplete="list"
        aria-label="Command"
        placeholder="Command"
        autocomplete="off"
        autocorrect="off"
        autocapitalize="off"
        spellcheck="false"
      />
    </div>
    {#if hasQuery}
      <div class="results" id={LIST_ID} role="listbox" aria-label="Commands">
        {#if flat.length === 0}
          <div class="empty">No commands</div>
        {:else}
          {#each groups as group (group.label)}
            <div class="group" role="group" aria-label={group.label}>
              <div class="group-label">{group.label}</div>
              {#each group.rows as row (row.cmd.id + "␟" + row.cmd.title)}
                {@const Icon = iconFor(row.cmd)}
                <!-- svelte-ignore a11y_click_events_have_key_events -->
                <div
                  class="row"
                  class:active={row.index === highlight}
                  id={optionId(row.index)}
                  role="option"
                  tabindex="-1"
                  aria-selected={row.index === highlight}
                  onclick={() => run(row)}
                  onpointermove={() => setHighlight(row.index)}
                >
                  <span class="row-icon">
                    <Icon size={21} strokeWidth={1.6} aria-hidden="true" />
                  </span>
                  <span class="row-copy">
                    <span class="title"
                      >{row.cmd.title}{#if row.arg}<span class="arg"
                          >{" "}{row.arg}</span
                        >{/if}</span
                    >
                    <span class="description">{row.cmd.category}</span>
                  </span>
                  <CommandChordAssign
                    cmd={row.cmd}
                    onCaptureEnd={() => inputEl?.focus()}
                  />
                  <span class="chevron">
                    <ChevronRight size={20} strokeWidth={1.8} aria-hidden="true" />
                  </span>
                </div>
              {/each}
            </div>
          {/each}
        {/if}
      </div>
    {/if}
  </div>
</OverlayShell>

<style>
  .launcher {
    display: flex;
    flex-direction: column;
    min-height: 0;
    max-height: min(70vh, 560px);
    color: var(--text);
  }
  .search-row {
    flex: 0 0 auto;
    display: flex;
    align-items: center;
    gap: 12px;
    height: 66px;
    padding: 0 22px;
    background: color-mix(in srgb, var(--bg-elev) 86%, transparent);
    color: color-mix(in srgb, var(--text) 82%, transparent);
  }
  .search {
    flex: 1 1 auto;
    min-width: 0;
    box-sizing: border-box;
    width: 100%;
    height: 100%;
    padding: 0;
    border: none;
    background: transparent;
    color: var(--text);
    font-size: 24px;
    line-height: 1.3;
    outline: none;
  }
  .search::placeholder {
    color: color-mix(in srgb, var(--text-secondary) 82%, transparent);
  }
  .results {
    flex: 1 1 auto;
    min-height: 0;
    overflow-y: auto;
    padding: 8px;
    border-top: 1px solid color-mix(in srgb, var(--border) 70%, transparent);
    background: color-mix(in srgb, var(--bg-card) 82%, var(--bg-elev) 18%);
    animation: results-open 180ms ease-out;
  }
  .empty {
    padding: 22px;
    text-align: center;
    color: var(--text-secondary);
    font-size: 13px;
  }
  .group + .group {
    margin-top: 4px;
  }
  .group-label {
    padding: 7px 12px 4px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--text-secondary);
  }
  .row {
    display: flex;
    align-items: center;
    justify-content: flex-start;
    gap: 12px;
    min-height: 48px;
    padding: 7px 10px;
    border-radius: 14px;
    cursor: pointer;
    color: var(--text);
    transition:
      background-color 140ms ease,
      box-shadow 140ms ease,
      transform 140ms ease;
  }
  .row:hover {
    background: var(--bg-elev);
    box-shadow: 0 8px 22px rgba(0, 0, 0, 0.16);
  }
  .row.active {
    background: var(--bg-elev);
    color: var(--text);
    box-shadow:
      0 8px 22px rgba(0, 0, 0, 0.16),
      inset 0 0 0 1px color-mix(in srgb, var(--text) 18%, transparent);
  }
  .row-icon {
    flex: 0 0 auto;
    width: 34px;
    height: 34px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: 999px;
    color: var(--text);
    background: color-mix(in srgb, var(--text) 8%, transparent);
  }
  .row-copy {
    flex: 1 1 auto;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .title {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 14px;
    font-weight: 600;
  }
  /* The verbatim argument a head-token match carries ("Open notes/x.md"):
     mono + muted so it reads as the payload, not part of the title. */
  .title .arg {
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-weight: 400;
    color: var(--text-secondary);
    white-space: pre;
  }
  .description {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text-secondary);
    font-size: 12px;
  }
  .chevron {
    flex: 0 0 auto;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: var(--text-secondary);
    opacity: 0;
    transform: translateX(-4px);
    transition:
      opacity 140ms ease,
      transform 140ms ease;
  }
  .row:hover .chevron,
  .row.active .chevron {
    opacity: 1;
    transform: translateX(0);
  }
  @keyframes results-open {
    0% {
      opacity: 0;
      max-height: 0;
      transform: translateY(-8px);
    }
    100% {
      opacity: 1;
      max-height: min(70vh, 494px);
      transform: translateY(0);
    }
  }
  @media (max-width: 560px) {
    .search-row {
      height: 60px;
      padding: 0 16px;
    }
    .search {
      font-size: 21px;
    }
    .description {
      display: none;
    }
  }
</style>
