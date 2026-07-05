<script lang="ts">
  // Empty single-pane welcome surface: the chan mark, the workspace's
  // absolute path, and the dotted wave field pinned to the bottom. It
  // carries no actions of its own; the command launcher auto-opens when
  // this surface appears (openCommandLauncher on mount) and is how the
  // user spawns anything from here, or reopens it from the pane menu.
  // Only mounted for a lone, non-terminal pane (see Pane.svelte), so it
  // needs no terminal-window branch.

  import { onMount } from "svelte";
  import { workspace, openCommandLauncher } from "../state/store.svelte";
  import DottedSurface from "./DottedSurface.svelte";

  onMount(() => {
    openCommandLauncher();
  });
</script>

<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<div class="welcome" role="region" aria-label="welcome" tabindex="0">
  <DottedSurface />
  <div class="welcome-mark"></div>
  {#if workspace.info}
    <div class="welcome-header" aria-label="workspace summary">
      <div class="welcome-name" title={workspace.info.root}>
        {workspace.info.root}
      </div>
    </div>
  {/if}
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
    max-width: min(720px, 90%);
    font-size: 16px;
    color: var(--text);
    opacity: 0.85;
    letter-spacing: 0.01em;
    text-align: center;
    overflow-wrap: anywhere;
  }
</style>
