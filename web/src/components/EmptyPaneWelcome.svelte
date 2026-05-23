<script lang="ts">
  // `fullstack-a-75b`: static welcome surface for empty single-
  // pane lone-pane layouts. Carousel widget moved to the
  // Infographics tab (per @@Alex's `d4a3fc8` route); this
  // surface now renders a fixed spawn grid + Infographics tile
  // + footer hint — no rotation, no slides, no play/pause.
  //
  // Spawn rows mirror `Pane.svelte::spawnActions` so the user
  // sees the same set + ordering whether they enter from the
  // welcome, the pane hamburger, or the empty-pane right-click
  // menu. Clicks dispatch the same `chan:command` event the
  // chord layer fires.

  import {
    BarChart2,
    FilePlus,
    Folder,
    MessageSquare,
    Network,
    Terminal,
  } from "lucide-svelte";
  import {
    SHORTCUTS,
    currentOS,
    currentPlatform,
    formatChord,
  } from "../state/shortcuts";
  import { drive } from "../state/store.svelte";

  type Props = {
    /// Right-click forwarder. Pane.svelte wires this to the
    /// empty-pane menu handler so right-click still opens the
    /// welcome menu over the static surface.
    oncontextmenu?: (e: MouseEvent) => void;
  };
  let { oncontextmenu }: Props = $props();

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
  const spawnEntries: SpawnRow[] = [
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
      label: "Rich Prompt",
      icon: MessageSquare,
      command: "app.terminal.richPrompt",
      chordId: "app.terminal.richPrompt",
    },
    {
      label: "Graph",
      icon: Network,
      command: "app.graph.toggle",
      chordId: "app.graph.toggle",
    },
  ];
  const secondaryEntries: SpawnRow[] = [
    {
      label: "Infographics",
      icon: BarChart2,
      command: "app.infographics.open",
      chordId: "app.infographics.open",
    },
  ];

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
  {oncontextmenu}
>
  <div class="welcome-mark"></div>
  {#if drive.info}
    <div class="welcome-header" aria-label="drive summary">
      <div class="welcome-name">{drive.info.name ?? "(unnamed)"}</div>
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
  <div class="spawn-sep" role="separator" aria-hidden="true"></div>
  <div class="spawn-row spawn-row-secondary" aria-label="info">
    {#each secondaryEntries as row (row.command)}
      {@const Icon = row.icon}
      <button
        type="button"
        class="spawn-btn"
        onclick={() => dispatchSpawn(row.command)}
        title={row.label}
      >
        <Icon size={18} strokeWidth={1.75} aria-hidden="true" />
        <span class="spawn-label">{row.label}</span>
        <span class="spawn-chord"></span>
      </button>
    {/each}
  </div>
  <p class="welcome-hint">
    Each pane's visible tab is part of the scope<br />
    for Graph.
  </p>
</div>

<style>
  .welcome {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 1rem;
    padding: 2rem;
    outline: none;
  }
  .welcome-mark {
    width: 160px;
    height: 160px;
    background-color: var(--text-secondary);
    -webkit-mask: url('/chan-mark.png') center / contain no-repeat;
            mask: url('/chan-mark.png') center / contain no-repeat;
    opacity: 0.45;
  }
  .welcome-header {
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
  /* `fullstack-a-32` + `-a-67 slice 2`: 5-up grid (New Draft +
     Terminal / FB / RP / Graph). Width caps + tile shape mirror
     the carousel's old spawn-row so the visual reads identically
     across the welcome + the (now Infographics-tab-hosted)
     carousel slide 1's prior look. */
  .spawn-row {
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
    width: 70%;
    max-width: 320px;
    height: 1px;
    background: var(--border);
    margin: 0.5rem auto;
    opacity: 0.6;
  }
  .spawn-row-secondary {
    opacity: 0.85;
    grid-template-columns: minmax(120px, 240px);
    justify-content: center;
    width: auto;
  }
  .welcome-hint {
    margin: 0;
    text-align: center;
    color: var(--text-secondary);
    font-size: 13px;
    line-height: 1.4;
    max-width: 360px;
  }
</style>
