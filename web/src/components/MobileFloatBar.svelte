<script lang="ts">
  // Persistent mobile bottom bar. Carries the same nav set as the
  // desktop BottomPill: Files / Search / Assistant / Graph / Settings.
  // Formatting (heading / bold / italic / etc.) is handled by the
  // editor's top fmt-bar (see FileEditorTab.svelte) which now also
  // shows on mobile and floats sticky at the top of the editor
  // canvas. The keyboard-aware position flip the bar used to do
  // is gone with that simplification.
  //
  // The bar is mounted on every mobile surface; idle hide fades it
  // out after 2.5 s of no input. When the soft keyboard is up,
  // position: fixed against the layout viewport puts the bar below
  // the keyboard area, so it disappears from view naturally without
  // any extra hide logic.

  import { idle } from "../state/idle.svelte";
  import {
    assistantOverlay,
    openAssistant,
    openBrowser,
    openGraph,
    openSettings,
    searchPanel,
  } from "../state/store.svelte";

  /// Whether the bar should be visually hidden because the
  /// assistant overlay is up. Other overlays (settings, search,
  /// graph) use OverlayShell which sits at z-index 25000 and
  /// already covers the bar with its dark scrim, so we leave the
  /// bar mounted for them: that way the moment the scrim is
  /// dismissed the bar is back, no re-render dance needed.
  ///
  /// The assistant overlay specifically has its own bottom-anchored
  /// input bar that would visually collide with the floating bar,
  /// so for that one we'd rather hide cleanly. The scrim on every
  /// overlay carries cursor:pointer (iOS WKWebView only fires
  /// `click` on plain divs that have it) so dismiss is reliable.
  const assistantSheetUp = $derived(assistantOverlay.open);
</script>

{#if !assistantSheetUp}
  <div
    class="float-bar"
    class:idle={idle.active}
    role="toolbar"
    aria-label="Mobile actions"
  >
    <!-- Same order as the desktop BottomPill so muscle memory is
         identical across surfaces: Files / Search / Assistant /
         Graph / Settings. The assistant ensō is the yellow brand
         attractor and renders a touch larger than its neighbours. -->
    <button
      class="action"
      onclick={openBrowser}
      aria-label="Files"
      title="Files"
    >
      <svg viewBox="0 0 16 16" aria-hidden="true" fill="currentColor">
        <path d="M1.75 1A1.75 1.75 0 0 0 0 2.75v10.5C0 14.216.784 15 1.75 15h12.5A1.75 1.75 0 0 0 16 13.25v-8.5A1.75 1.75 0 0 0 14.25 3H7.5l-1.4-1.55A1.75 1.75 0 0 0 4.81 1H1.75z" />
      </svg>
    </button>
    <button
      class="action"
      onclick={() => (searchPanel.open = true)}
      aria-label="Search"
      title="Search"
    >
      <svg viewBox="0 0 16 16" aria-hidden="true" fill="currentColor">
        <path
          d="M11.742 10.344a6.5 6.5 0 1 0-1.397 1.398h-.001c.03.04.062.078.097.114l3.85 3.85a1 1 0 0 0 1.415-1.414l-3.85-3.85a1.007 1.007 0 0 0-.114-.098zM12 6.5a5.5 5.5 0 1 1-11 0 5.5 5.5 0 0 1 11 0z"
        />
      </svg>
    </button>
    <button
      class="action enso"
      onclick={openAssistant}
      aria-label="Assistant"
      title="Assistant"
    >
      <svg viewBox="0 0 16 16" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round">
        <path d="M 10.75 3.24 A 5.5 5.5 0 1 1 5.25 3.24" />
      </svg>
    </button>
    <button
      class="action"
      onclick={openGraph}
      aria-label="Graph"
      title="Graph"
    >
      <svg viewBox="0 0 16 16" aria-hidden="true" fill="currentColor">
        <path d="M3.5 4a1.5 1.5 0 1 0 0-3 1.5 1.5 0 0 0 0 3zm0 11a1.5 1.5 0 1 0 0-3 1.5 1.5 0 0 0 0 3zm9-5.5a1.5 1.5 0 1 0 0-3 1.5 1.5 0 0 0 0 3z" />
        <path d="M4.27 3.05l7 4.5-.54.84-7-4.5.54-.84zM4.27 12.95l7-4.5-.54-.84-7 4.5.54.84z" />
      </svg>
    </button>
    <button
      class="action"
      onclick={openSettings}
      aria-label="Settings"
      title="Settings"
    >
      <svg viewBox="0 0 16 16" aria-hidden="true" fill="currentColor">
        <path d="M8 4.754a3.246 3.246 0 1 0 0 6.492 3.246 3.246 0 0 0 0-6.492zM5.754 8a2.246 2.246 0 1 1 4.492 0 2.246 2.246 0 0 1-4.492 0z M9.796 1.343c-.527-1.79-3.065-1.79-3.592 0l-.094.319a.873.873 0 0 1-1.255.52l-.292-.16c-1.64-.892-3.433.902-2.54 2.541l.159.292a.873.873 0 0 1-.52 1.255l-.319.094c-1.79.527-1.79 3.065 0 3.592l.319.094a.873.873 0 0 1 .52 1.255l-.16.292c-.892 1.64.901 3.434 2.541 2.54l.292-.159a.873.873 0 0 1 1.255.52l.094.319c.527 1.79 3.065 1.79 3.592 0l.094-.319a.873.873 0 0 1 1.255-.52l.292.16c1.64.893 3.434-.901 2.54-2.541l-.159-.292a.873.873 0 0 1 .52-1.255l.319-.094c1.79-.527 1.79-3.065 0-3.592l-.319-.094a.873.873 0 0 1-.52-1.255l.16-.292c.893-1.64-.901-3.433-2.541-2.54l-.292.159a.873.873 0 0 1-1.255-.52l-.094-.319zm-2.633.283c.246-.835 1.428-.835 1.674 0l.094.319a1.873 1.873 0 0 0 2.693 1.115l.291-.16c.764-.415 1.6.42 1.184 1.185l-.159.292a1.873 1.873 0 0 0 1.116 2.692l.318.094c.835.246.835 1.428 0 1.674l-.319.094a1.873 1.873 0 0 0-1.115 2.693l.16.291c.415.764-.42 1.6-1.185 1.184l-.291-.159a1.873 1.873 0 0 0-2.693 1.116l-.094.318c-.246.835-1.428.835-1.674 0l-.094-.319a1.873 1.873 0 0 0-2.692-1.115l-.292.16c-.764.415-1.6-.42-1.184-1.185l.159-.291A1.873 1.873 0 0 0 1.945 8.93l-.319-.094c-.835-.246-.835-1.428 0-1.674l.319-.094A1.873 1.873 0 0 0 3.06 4.377l-.16-.292c-.415-.764.42-1.6 1.185-1.184l.292.159a1.873 1.873 0 0 0 2.692-1.115l.094-.319z" />
      </svg>
    </button>
  </div>
{/if}

<style>
  .float-bar {
    position: fixed;
    left: 50%;
    bottom: calc(env(safe-area-inset-bottom, 0px) + 8px);
    transform: translateX(-50%);
    z-index: 4500;
    display: flex;
    gap: 4px;
    align-items: center;
    padding: 6px 8px;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: 999px;
    box-shadow: 0 6px 18px rgba(0, 0, 0, 0.18);
    font-size: 15px;
    color: var(--text);
    max-width: calc(100vw - 16px);
    overflow-x: auto;
    /* Native iOS feel: no horizontal scrollbar visible. */
    scrollbar-width: none;
    transition: opacity 200ms ease;
  }
  .float-bar::-webkit-scrollbar { display: none; }
  /* Idle: fade out + drop pointer events. Next tap anywhere
     reactivates via the global idle tracker. */
  .float-bar.idle {
    opacity: 0;
    pointer-events: none;
  }
  .action {
    width: 36px;
    height: 36px;
    background: transparent;
    border: 0;
    border-radius: 999px;
    color: var(--text);
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }
  .action svg { width: 20px; height: 20px; }
  .action:active { background: var(--hover-bg); }
  /* Assistant ensō: yellow tint as on desktop and visibly larger
     than its neighbours so the eye lands on it first. Stroke icon,
     so fill stays none; color drives the stroke through
     currentColor. */
  .action.enso {
    color: var(--assistant-accent);
    width: 44px;
    height: 44px;
    margin: 0 2px;
  }
  .action.enso svg { fill: none; width: 24px; height: 24px; }
</style>
