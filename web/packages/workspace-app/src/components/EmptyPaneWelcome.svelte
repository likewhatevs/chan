<script lang="ts">
  // Empty single-pane welcome surface: the chan mark, the workspace's
  // absolute path, and the dotted wave field pinned to the bottom. It
  // carries no actions of its own; users can reopen the command launcher
  // from the pane menu or the global launcher chord.
  // Only mounted for a lone, non-terminal pane (see Pane.svelte), so it
  // needs no terminal-window branch.

  import { workspace } from "../state/store.svelte";
  import DottedSurface from "./DottedSurface.svelte";

  let welcomeEl: HTMLDivElement | undefined = $state();
  let headerEl: HTMLDivElement | undefined = $state();

  $effect(() => {
    const welcome = welcomeEl;
    const header = headerEl;
    if (!welcome) return;
    if (!header) {
      welcome.style.removeProperty("--dotted-surface-top");
      welcome.style.removeProperty("--dotted-surface-height");
      return;
    }

    let frame: number | null = null;
    const placeWave = (): void => {
      frame = null;
      const host = welcome.getBoundingClientRect();
      const label = header.getBoundingClientRect();
      const top = Math.max(0, Math.ceil(label.bottom - host.top + 10));
      welcome.style.setProperty("--dotted-surface-top", `${top}px`);
      welcome.style.removeProperty("--dotted-surface-height");
      welcome.style.removeProperty("--dotted-surface-bottom");
    };
    const schedule = (): void => {
      if (frame !== null) cancelAnimationFrame(frame);
      frame = requestAnimationFrame(placeWave);
    };
    const observer =
      typeof ResizeObserver !== "undefined" ? new ResizeObserver(schedule) : null;
    observer?.observe(welcome);
    observer?.observe(header);
    window.addEventListener("resize", schedule);
    schedule();
    return () => {
      observer?.disconnect();
      window.removeEventListener("resize", schedule);
      if (frame !== null) cancelAnimationFrame(frame);
    };
  });
</script>

<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<div class="welcome" role="region" aria-label="welcome" tabindex="0" bind:this={welcomeEl}>
  <DottedSurface />
  <div class="welcome-mark"></div>
  {#if workspace.info}
    <div class="welcome-header" aria-label="workspace summary" bind:this={headerEl}>
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
    box-sizing: border-box;
    width: min(1120px, 100%);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    margin-top: -0.5rem;
    padding: 0 clamp(16px, 5vw, 80px);
  }
  .welcome-name {
    width: 100%;
    font-size: 16px;
    color: var(--text);
    opacity: 0.85;
    letter-spacing: 0.01em;
    text-align: center;
    overflow-wrap: break-word;
    word-break: normal;
  }
</style>
