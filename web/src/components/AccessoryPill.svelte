<script lang="ts">
  // Floating navigation pill: Files / Search / Assistant / Graph /
  // Settings. The single visual entry point for jumping between
  // window-level overlays from anywhere this is mounted. Callers
  // rely on the global keybindings still firing whether or not the
  // pill is visible (Cmd/Ctrl+P, Cmd/Ctrl+Shift+G, Cmd/Ctrl+H,
  // Cmd/Ctrl+Shift+E, Cmd/Ctrl+,); the pill just makes the
  // affordance discoverable.
  //
  // The Assistant button sits in the middle of the row and renders
  // a touch larger than its neighbours: it's the brand attractor
  // and the most actionable surface in the pill, so the size +
  // yellow stroke double up to draw the eye there first.
  //
  // The component renders only the buttons; chrome (rounded
  // background, shadow, positioning) is the caller's responsibility
  // since the surrounding container varies.

  import {
    assistantOverlay,
    browserOverlay,
    drive,
    openAssistant,
    openBrowser,
    openGraph,
    openSettings,
    searchPanel,
  } from "../state/store.svelte";

  /// Hide the assistant button when the master switch is off so
  /// the pill doesn't promise an empty panel.
  const assistantEnabled = $derived(
    drive.info?.preferences.assistant.enabled ?? true,
  );
</script>

<button
  class="fbtn"
  class:on={browserOverlay.open}
  title="Files (⌘⇧O)"
  aria-label="Files"
  onclick={openBrowser}
>
  <svg viewBox="0 0 16 16" aria-hidden="true">
    <path d="M1.75 1A1.75 1.75 0 0 0 0 2.75v10.5C0 14.216.784 15 1.75 15h12.5A1.75 1.75 0 0 0 16 13.25v-8.5A1.75 1.75 0 0 0 14.25 3H7.5l-1.4-1.55A1.75 1.75 0 0 0 4.81 1H1.75z" />
  </svg>
</button>
<button
  class="fbtn"
  title="Search (⌘K)"
  aria-label="Search"
  onclick={() => (searchPanel.open = true)}
>
  <svg viewBox="0 0 16 16" aria-hidden="true">
    <path
      d="M11.742 10.344a6.5 6.5 0 1 0-1.397 1.398h-.001c.03.04.062.078.097.114l3.85 3.85a1 1 0 0 0 1.415-1.414l-3.85-3.85a1.007 1.007 0 0 0-.114-.098zM12 6.5a5.5 5.5 0 1 1-11 0 5.5 5.5 0 0 1 11 0z"
    />
  </svg>
</button>
{#if assistantEnabled}
  <button
    class="fbtn enso"
    class:on={assistantOverlay.open}
    title="Assistant (⌘P)"
    aria-label="Assistant"
    onclick={openAssistant}
  >
    <span class="enso-mark" aria-hidden="true"></span>
  </button>
{/if}
<button
  class="fbtn"
  title="Graph (⌘⇧G)"
  aria-label="Graph"
  onclick={openGraph}
>
  <svg viewBox="0 0 16 16" aria-hidden="true">
    <path d="M3.5 4a1.5 1.5 0 1 0 0-3 1.5 1.5 0 0 0 0 3zm0 11a1.5 1.5 0 1 0 0-3 1.5 1.5 0 0 0 0 3zm9-5.5a1.5 1.5 0 1 0 0-3 1.5 1.5 0 0 0 0 3z" />
    <path d="M4.27 3.05l7 4.5-.54.84-7-4.5.54-.84zM4.27 12.95l7-4.5-.54-.84-7 4.5.54.84z" />
  </svg>
</button>
<button
  class="fbtn"
  title="Settings (⌘,)"
  aria-label="Settings"
  onclick={openSettings}
>
  <svg viewBox="0 0 16 16" aria-hidden="true">
    <path d="M8 4.754a3.246 3.246 0 1 0 0 6.492 3.246 3.246 0 0 0 0-6.492zM5.754 8a2.246 2.246 0 1 1 4.492 0 2.246 2.246 0 0 1-4.492 0z M9.796 1.343c-.527-1.79-3.065-1.79-3.592 0l-.094.319a.873.873 0 0 1-1.255.52l-.292-.16c-1.64-.892-3.433.902-2.54 2.541l.159.292a.873.873 0 0 1-.52 1.255l-.319.094c-1.79.527-1.79 3.065 0 3.592l.319.094a.873.873 0 0 1 .52 1.255l-.16.292c-.892 1.64.901 3.434 2.541 2.54l.292-.159a.873.873 0 0 1 1.255.52l.094.319c.527 1.79 3.065 1.79 3.592 0l.094-.319a.873.873 0 0 1 1.255-.52l.292.16c1.64.893 3.434-.901 2.54-2.541l-.159-.292a.873.873 0 0 1 .52-1.255l.319-.094c1.79-.527 1.79-3.065 0-3.592l-.319-.094a.873.873 0 0 1-.52-1.255l.16-.292c.893-1.64-.901-3.433-2.541-2.54l-.292.159a.873.873 0 0 1-1.255-.52l-.094-.319zm-2.633.283c.246-.835 1.428-.835 1.674 0l.094.319a1.873 1.873 0 0 0 2.693 1.115l.291-.16c.764-.415 1.6.42 1.184 1.185l-.159.292a1.873 1.873 0 0 0 1.116 2.692l.318.094c.835.246.835 1.428 0 1.674l-.319.094a1.873 1.873 0 0 0-1.115 2.693l.16.291c.415.764-.42 1.6-1.185 1.184l-.291-.159a1.873 1.873 0 0 0-2.693 1.116l-.094.318c-.246.835-1.428.835-1.674 0l-.094-.319a1.873 1.873 0 0 0-2.692-1.115l-.292.16c-.764.415-1.6-.42-1.184-1.185l.159-.291A1.873 1.873 0 0 0 1.945 8.93l-.319-.094c-.835-.246-.835-1.428 0-1.674l.319-.094A1.873 1.873 0 0 0 3.06 4.377l-.16-.292c-.415-.764.42-1.6 1.185-1.184l.292.159a1.873 1.873 0 0 0 2.692-1.115l.094-.319z" />
  </svg>
</button>

<style>
  .fbtn {
    min-width: 38px;
    height: 38px;
    text-align: center;
    background: transparent;
    border: 1px solid transparent;
    border-radius: 19px;
    color: var(--text);
    cursor: pointer;
    font: inherit;
    padding: 0 9px;
    line-height: 1;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }
  .fbtn:hover { background: var(--hover-bg); }
  .fbtn.on {
    background: var(--hover-bg);
    border-color: var(--btn-hover);
  }
  .fbtn svg {
    width: 19px;
    height: 19px;
    fill: currentColor;
    display: block;
  }
  /* The ensō uses the same chan-mark.png artwork as the empty-pane
     watermark, painted via CSS mask so the silhouette can take the
     theme accent. Yellow tint = Notes-style accent. Bigger circle
     than the neighbours so it still reads as the primary attractor,
     but everything in the bar scales together so nothing looks like
     an afterthought. */
  .fbtn.enso {
    width: 52px;
    height: 52px;
    min-width: 52px;
    border-radius: 50%;
    margin: 0 4px;
    padding: 0;
  }
  .fbtn.enso .enso-mark {
    width: 38px;
    height: 38px;
    background-color: var(--assistant-accent);
    -webkit-mask: url('/chan-mark.png') center / contain no-repeat;
            mask: url('/chan-mark.png') center / contain no-repeat;
  }
  .fbtn.enso.on {
    border-color: var(--assistant-accent);
    background: rgba(229, 140, 77, 0.12);
  }
</style>
