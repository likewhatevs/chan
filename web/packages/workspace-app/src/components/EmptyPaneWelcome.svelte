<script lang="ts">
  // Empty single-pane welcome surface: the chan mark over the dotted wave
  // field pinned to the bottom, plus a floating Apps button (top-left)
  // whose menu spawns every app surface through the same `chan:command`
  // dispatch the chords and the launcher use.
  // Only mounted for a lone, non-terminal pane (see Pane.svelte), so it
  // needs no terminal-window branch.

  import {
    BarChart2,
    FileText,
    Folder,
    LayoutGrid,
    Network,
    Presentation,
    Shapes,
    Terminal,
    Users,
  } from "lucide-svelte";
  import DottedSurface from "./DottedSurface.svelte";
  import HamburgerMenu from "./HamburgerMenu.svelte";
  import { chordFor } from "../state/shortcuts";

  type IconComponent = typeof LayoutGrid;
  type AppRow = { id: string; title: string; icon: IconComponent };

  // One row per spawnable app surface, alphabetical by title. Six ids are
  // the chorded spawn commands; New diagram / New slide deck are the
  // chordless catalog entries runCommand also routes (App.svelte).
  const appRows: AppRow[] = [
    { id: "app.dashboard.open", title: "New dashboard", icon: BarChart2 },
    { id: "app.diagram.new", title: "New diagram", icon: Shapes },
    { id: "app.draft.new", title: "New draft", icon: FileText },
    { id: "app.files.toggle", title: "New file browser", icon: Folder },
    { id: "app.graph.toggle", title: "New graph", icon: Network },
    { id: "app.slides.new", title: "New slide deck", icon: Presentation },
    { id: "app.terminal.teamWork", title: "New team", icon: Users },
    { id: "app.terminal.toggle", title: "New terminal", icon: Terminal },
  ];

  let appsBtn: HTMLButtonElement | undefined = $state();
  let appsMenu: HamburgerMenu | undefined = $state();
  let appsMenuOpen = $state(false);

  /// Resolve a command's chord for the menu rows, override-aware (user
  /// assignment first, then the built-in), empty when unbound so the
  /// chord column stays aligned.
  function chordLabel(id: string): string {
    return chordFor(id) ?? "";
  }

  /// Fire the same `chan:command` event the keymap layer uses so every
  /// row routes through the existing dispatcher in App.svelte. Avoids
  /// re-implementing the spawn actions here.
  function dispatchCommand(id: string): void {
    window.dispatchEvent(
      new CustomEvent("chan:command", { detail: { name: id } }),
    );
  }

  function runRow(id: string): void {
    appsMenuOpen = false;
    dispatchCommand(id);
  }

  // Anchor the bubble under the button. The button carries the
  // `hamburger-trigger` class so HamburgerMenu's outside-mousedown
  // dismissal skips it and this click toggles instead of
  // close-then-reopen.
  function toggleAppsMenu(): void {
    if (appsMenuOpen) {
      appsMenuOpen = false;
      return;
    }
    const r = appsBtn?.getBoundingClientRect();
    if (!r) return;
    appsMenu?.openAtCursor(r.left, r.bottom + 6);
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<div class="welcome" role="region" aria-label="welcome" tabindex="0">
  <DottedSurface />
  <div class="welcome-mark"></div>
  <button
    bind:this={appsBtn}
    class="welcome-apps hamburger-trigger"
    type="button"
    aria-haspopup="menu"
    aria-expanded={appsMenuOpen}
    onclick={toggleAppsMenu}
  >
    <LayoutGrid size={15} strokeWidth={1.75} aria-hidden="true" />
    <span>Apps</span>
  </button>
  <HamburgerMenu
    bind:this={appsMenu}
    bind:open={appsMenuOpen}
    showTrigger={false}
    width={240}
    height={330}
  >
    {#each appRows as row (row.id)}
      {@const Icon = row.icon}
      <li>
        <button role="menuitem" onclick={() => runRow(row.id)}>
          <Icon size={16} strokeWidth={1.75} aria-hidden="true" />
          <span class="menu-row-label">{row.title}</span>
          <span class="menu-row-chord">{chordLabel(row.id)}</span>
        </button>
      </li>
    {/each}
  </HamburgerMenu>
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
    /* Pane-aware sizing for the mark: the surface is its own query
       container so the mark hides per pane in splits, not per window. */
    container-type: size;
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
  /* Short panes drop the mark so the wave field keeps breathing room;
     it reappears the moment the pane grows back. */
  @container (max-height: 420px) {
    .welcome-mark {
      display: none;
    }
  }
  .welcome-apps {
    position: absolute;
    top: 10px;
    left: 10px;
    z-index: 2;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 4px 10px;
    border: 1px solid var(--btn-border);
    border-radius: 6px;
    background: color-mix(in srgb, var(--bg) 65%, transparent);
    color: var(--text);
    font: inherit;
    font-size: 13px;
    cursor: pointer;
  }
  .welcome-apps:hover {
    background: var(--hover-bg);
  }
  .welcome-apps[aria-expanded="true"] {
    background: var(--hover-bg);
    border-color: var(--btn-hover);
  }
</style>
