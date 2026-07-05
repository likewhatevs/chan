<script lang="ts">
  // Static welcome surface for empty single-pane lone-pane layouts.
  // Renders a fixed spawn grid + Dashboard tile + footer hint (no
  // rotation, no slides, no play/pause; the rotating carousel lives
  // in the Dashboard tab).
  //
  // Spawn rows mirror the command launcher's top-level surface
  // entries so the welcome grid and launcher stay in the same
  // order. Clicks dispatch the same `chan:command` event the
  // chord layer fires.

  import {
    BarChart2,
    FilePlus,
    Folder,
    MessageSquare,
    Network,
    Search,
    Terminal,
  } from "lucide-svelte";
  import {
    SHORTCUTS,
    currentOS,
    currentPlatform,
    formatChord,
  } from "../state/shortcuts";
  import { ui, workspace } from "../state/store.svelte";
  import DottedSurface from "./DottedSurface.svelte";

  // EmptyPaneWelcome does not forward `oncontextmenu` to a parent
  // handler; the welcome surface is purely the click-driven spawn
  // grid. Menu-style access lives in the command launcher.

  const platform = currentPlatform();
  const os = currentOS();

  function chordLabel(id: string | undefined): string {
    if (!id) return "";
    const s = SHORTCUTS.find((x) => x.id === id);
    if (!s) return "";
    const chord = s[platform];
    if (!chord) return "";
    return formatChord(chord, os);
  }

  type SpawnRow = {
    label: string;
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    icon: any;
    command: string;
    chordId: string;
  };
  const FULL_SPAWN_ENTRIES: SpawnRow[] = [
    {
      label: "New Draft",
      icon: FilePlus,
      command: "app.draft.new",
      chordId: "app.draft.new",
    },
    {
      label: "Terminal",
      icon: Terminal,
      command: "app.terminal.toggle",
      chordId: "app.terminal.toggle",
    },
    {
      label: "File Browser",
      icon: Folder,
      command: "app.files.toggle",
      chordId: "app.files.toggle",
    },
    {
      // Label "Team Work"; chord id is app.terminal.teamWork.
      label: "Team Work",
      icon: MessageSquare,
      command: "app.terminal.teamWork",
      chordId: "app.terminal.teamWork",
    },
    {
      label: "Graph",
      icon: Network,
      command: "app.graph.toggle",
      chordId: "app.graph.toggle",
    },
  ];
  const FULL_SECONDARY_ENTRIES: SpawnRow[] = [
    // The secondary row carries every chord-bound spawn entry the
    // welcome grid offers (Search + Dashboard) so it matches the
    // command launcher.
    {
      label: "Search",
      icon: Search,
      command: "app.search.toggle",
      chordId: "app.search.toggle",
    },
    {
      label: "Dashboard",
      icon: BarChart2,
      command: "app.dashboard.open",
      chordId: "app.dashboard.open",
    },
  ];
  // In a `?kind=terminal` window the workspace surfaces (drafts / file
  // browser / team work / graph / search / dashboard) don't exist, so the
  // welcome grid collapses to just Terminal and the secondary info row
  // drops out entirely. Filter the full lists so the terminal entry stays
  // a single source of truth.
  const spawnEntries = $derived(
    ui.terminalOnly
      ? FULL_SPAWN_ENTRIES.filter((r) => r.command === "app.terminal.toggle")
      : FULL_SPAWN_ENTRIES,
  );
  const secondaryEntries = $derived(
    ui.terminalOnly ? [] : FULL_SECONDARY_ENTRIES,
  );

  function dispatchSpawn(command: string): void {
    window.dispatchEvent(
      new CustomEvent("chan:command", { detail: { name: command } }),
    );
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<div
  class="welcome"
  role="region"
  aria-label="welcome"
  tabindex="0"
>
  <DottedSurface />
  <div class="welcome-mark"></div>
  {#if workspace.info}
    <div class="welcome-header" aria-label="workspace summary">
      <div class="welcome-name" title={workspace.info.root}>{workspace.info.label ?? "(workspace)"}</div>
    </div>
  {/if}
  <div class="spawn-row" aria-label="spawn">
    {#each spawnEntries as row (row.command)}
      {@const Icon = row.icon}
      <button
        type="button"
        class="spawn-btn"
        onclick={() => dispatchSpawn(row.command)}
        title="{row.label} ({chordLabel(row.chordId)})"
      >
        <Icon size={18} strokeWidth={1.75} aria-hidden="true" />
        <span class="spawn-label">{row.label}</span>
        <span class="spawn-chord">{chordLabel(row.chordId)}</span>
      </button>
    {/each}
  </div>
  {#if secondaryEntries.length > 0}
    <div class="spawn-sep" role="separator" aria-hidden="true"></div>
    <div class="spawn-row spawn-row-secondary" aria-label="info">
      {#each secondaryEntries as row (row.command)}
        {@const Icon = row.icon}
        <button
          type="button"
          class="spawn-btn"
          onclick={() => dispatchSpawn(row.command)}
          title="{row.label} ({chordLabel(row.chordId)})"
        >
          <Icon size={18} strokeWidth={1.75} aria-hidden="true" />
          <span class="spawn-label">{row.label}</span>
          <span class="spawn-chord">{chordLabel(row.chordId)}</span>
        </button>
      {/each}
    </div>
  {/if}
  <!-- No per-tab graph-scope hint here. Graph scope is
       picker-driven (workspace / dir / file / tag / git_repo) and
       that picker lives in the graph overlay's chrome, not the
       welcome surface. -->
</div>

<style>
  .welcome {
    flex: 1;
    min-height: 0;
    align-self: stretch;
    width: 100%;
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 1rem;
    padding: 2rem;
    outline: none;
    overflow: hidden;
    isolation: isolate;
  }
  .welcome-mark {
    position: relative;
    z-index: 1;
    width: 160px;
    height: 160px;
    background-color: var(--text-secondary);
    -webkit-mask: url('/chan-mark.png') center / contain no-repeat;
            mask: url('/chan-mark.png') center / contain no-repeat;
    opacity: 0.45;
  }
  .welcome-header {
    position: relative;
    z-index: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    margin-top: -0.5rem;
  }
  .welcome-name {
    font-size: 18px;
    color: var(--text);
    opacity: 0.85;
    letter-spacing: 0.01em;
  }
  /* 5-up grid (New Draft + Terminal / FB / Team Work / Graph). Width caps
     + tile shape keep the spawn tiles visually consistent across
     the welcome surface. */
  .spawn-row {
    position: relative;
    z-index: 1;
    display: grid;
    grid-template-columns: repeat(5, minmax(96px, 1fr));
    gap: 8px;
    width: min(640px, 90%);
    margin: 0;
  }
  .spawn-btn {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 4px;
    padding: 10px 8px;
    background: var(--bg-card);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: 8px;
    cursor: pointer;
    font: inherit;
    transition: background-color 120ms ease, border-color 120ms ease,
      color 120ms ease;
  }
  .spawn-btn:hover {
    background: var(--hover-bg);
    border-color: var(--link);
  }
  .spawn-btn:focus-visible {
    outline: 2px solid var(--link);
    outline-offset: 1px;
  }
  .spawn-label {
    font-size: 13px;
    color: var(--text);
  }
  .spawn-chord {
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 11px;
    color: var(--text-secondary);
    opacity: 0.85;
    text-align: center;
    line-height: 1.2;
  }
  .spawn-sep {
    position: relative;
    z-index: 1;
    width: 70%;
    max-width: 320px;
    height: 1px;
    background: var(--border);
    margin: 0.5rem auto;
    opacity: 0.6;
  }
  .spawn-row-secondary {
    /* Two secondary tiles (Search + Dashboard) sit side by
       side. Width budget mirrors the carousel's old centered
       single-tile row so the overall surface still feels
       balanced under the 5-up primary grid. */
    opacity: 0.85;
    grid-template-columns: repeat(2, minmax(96px, 1fr));
    justify-content: center;
    width: min(320px, 80%);
  }
</style>
