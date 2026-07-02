<script lang="ts">
  // Near-fullscreen overlay hosting the frontend-only workspace demo on the
  // marketing site. A slim standalone take on the app's OverlayShell: dim
  // scrim, centered panel, pop animation, close button. Escape is NOT wired
  // to close: the app inside owns Escape (its own overlays, hybrid nav).
  //
  // Closing destroys the app ({#if open}), which resets the in-memory demo:
  // the next open boots the pristine snapshot again.

  import WorkspaceDemo from "@chan/workspace-app/demo";
  import type { MockWorkspaceData } from "@chan/workspace-app/demo-data";

  let { data }: { data: MockWorkspaceData } = $props();
  let open = $state(true);

  export function show(): void {
    open = true;
  }
</script>

{#if open}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="workspace-demo-overlay" onclick={() => (open = false)}>
    <div class="panel" onclick={(e) => e.stopPropagation()} role="dialog" tabindex="-1">
      <button class="close" type="button" title="Close demo" aria-label="Close demo" onclick={() => (open = false)}>
        &#10005;
      </button>
      <div class="app-host">
        <WorkspaceDemo {data} />
      </div>
    </div>
  </div>
{/if}

<style>
  .workspace-demo-overlay {
    position: fixed;
    inset: 0;
    z-index: 25000;
    background: rgba(0, 0, 0, 0.55);
    display: flex;
    justify-content: center;
    align-items: center;
    padding: 24px;
    box-sizing: border-box;
    cursor: pointer;
  }
  .panel {
    position: relative;
    width: 100%;
    height: 100%;
    background: #1c1c1e;
    border: 1px solid #3a3a3c;
    border-radius: 10px;
    box-shadow: 0 14px 44px rgba(0, 0, 0, 0.5);
    overflow: hidden;
    cursor: auto;
    transform-origin: center top;
    animation: workspace-demo-pop 260ms cubic-bezier(0.34, 1.56, 0.64, 1);
  }
  .close {
    position: absolute;
    top: 10px;
    right: 12px;
    z-index: 10;
    width: 28px;
    height: 28px;
    border: none;
    border-radius: 6px;
    background: rgba(255, 255, 255, 0.08);
    color: #f5f5f7;
    font-size: 14px;
    line-height: 1;
    cursor: pointer;
  }
  .close:hover {
    background: rgba(255, 255, 255, 0.16);
  }
  .app-host {
    width: 100%;
    height: 100%;
  }
  @keyframes workspace-demo-pop {
    0% {
      opacity: 0;
      transform: scale(0.94);
    }
    100% {
      opacity: 1;
      transform: scale(1);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .panel {
      animation: none;
    }
  }
</style>
