<script lang="ts">
  // Empty single-pane welcome surface: the chan mark over the dotted wave
  // field pinned to the bottom. It carries no actions of its own; app
  // spawns live in the pane hamburger's Apps rows and the command
  // launcher (menu or global chord).
  // Only mounted for a lone, non-terminal pane (see Pane.svelte), so it
  // needs no terminal-window branch.

  import DottedSurface from "./DottedSurface.svelte";
</script>

<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<div class="welcome" role="region" aria-label="welcome" tabindex="0">
  <DottedSurface />
  <div class="welcome-mark"></div>
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
</style>
