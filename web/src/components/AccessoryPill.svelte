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
    settingsDisabled,
  } from "../state/store.svelte";

  /// Master switch state. When off we keep the button visible but
  /// inert + greyed so the entry point stays discoverable and the
  /// tooltip points the user at the toggle in Settings. Hiding the
  /// button made the missing affordance unexplainable, especially
  /// for users who hit Cmd/Ctrl+P and saw nothing happen.
  const assistantEnabled = $derived(
    drive.info?.preferences.assistant.enabled ?? true,
  );

  /// Server-controlled lockdown of the Settings panel (tunnel mode
  /// with --tunnel-public). We grey the button rather than hide it
  /// so a returning owner sees the entry point and understands why
  /// it's inert: matches the issue #21 viewer / kiosk story.
  const settingsLocked = settingsDisabled;
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
<button
  class="fbtn enso"
  class:on={assistantOverlay.open && assistantEnabled}
  class:disabled={!assistantEnabled}
  title={assistantEnabled
    ? "Assistant (⌘P)"
    : "Assistant is off — enable it in Settings"}
  aria-label="Assistant"
  aria-disabled={!assistantEnabled}
  disabled={!assistantEnabled}
  onclick={openAssistant}
>
  <span class="enso-mark" aria-hidden="true"></span>
</button>
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
  class:disabled={settingsLocked}
  title={settingsLocked ? "Settings disabled by host" : "Settings (⌘,)"}
  aria-label="Settings"
  aria-disabled={settingsLocked}
  disabled={settingsLocked}
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
  /* Locked Settings entry: keep the affordance visible so the owner
     can still see where it lives, but kill hover feedback and the
     pointer hint so it reads as inert. The native `disabled`
     attribute already blocks onclick; this is purely visual. */
  .fbtn.disabled,
  .fbtn.disabled:hover {
    opacity: 0.35;
    background: transparent;
    cursor: not-allowed;
  }
  .fbtn svg {
    width: 19px;
    height: 19px;
    fill: currentColor;
    display: block;
  }
  /* The ensō uses the same chan-mark.png artwork as the empty-pane
     watermark, painted via CSS mask so the silhouette can take the
     theme accent (brand orange, matches chan.app). Bigger circle
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
