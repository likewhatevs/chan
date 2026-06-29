<script lang="ts">
  import { onMount } from "svelte";
  import App from "./App.svelte";
  import { createLauncherDemoApi } from "./api/demo";
  import { resetBackend, setBackend } from "./api/backend";
  import { loadLibrary, stopWatching } from "./state/library.svelte";
  import { closeDialog } from "./state/dialog.svelte";
  import { cancelConfirm } from "./state/confirm.svelte";
  import { clearSelection } from "./state/selection.svelte";
  import { clearAllControlAttention, markControlAttention } from "./state/controlAttention.svelte";
  import { setDemoReset } from "./state/demo.svelte";
  import { themeState } from "./state/theme.svelte";

  // The marketing mock always demos the dark theme to match the desktop
  // screenshots and the dark widget frame, regardless of the visitor's
  // prefers-color-scheme (which would otherwise start it light and render the
  // theme toggle as a moon instead of the desktop's sun).
  themeState.theme = "dark";

  const api = createLauncherDemoApi();
  setBackend(api);

  async function resetDemoData(): Promise<void> {
    stopWatching();
    api.reset();
    closeDialog();
    cancelConfirm();
    clearSelection();
    clearAllControlAttention();
    await loadLibrary();
    markControlAttention(api.attentionDevserverId);
  }

  setDemoReset(resetDemoData);

  onMount(() => {
    void resetDemoData();
    return () => {
      stopWatching();
      setDemoReset(null);
      clearAllControlAttention();
      resetBackend();
    };
  });
</script>

<div class="launcher-demo-frame" data-theme={themeState.theme}>
  <App />
</div>

<style>
  .launcher-demo-frame {
    background: var(--bg);
    color: var(--text);
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", system-ui, sans-serif;
    display: flex;
    flex-direction: column;
    font-size: 14px;
    height: 100%;
    isolation: isolate;
    min-height: 100%;
    overflow: hidden;
    position: relative;
    transform: translateZ(0);
  }

  :global(.launcher-demo-frame .topbar) {
    flex: 0 0 auto;
    position: static;
  }

  /* Match the desktop launcher: the content column shares the top bar's 1.25rem
     side padding, so each machine card's edge aligns under the "Library" title
     and the card internals (icon indent, gaps) fall back to the shipped launcher
     CSS instead of the embed-only nudges. Only the vertical padding is tightened
     for the shorter widget frame. */
  :global(.launcher-demo-frame .content) {
    background: var(--bg);
    flex: 1 1 auto;
    margin: 0;
    max-width: none;
    min-height: 0;
    overflow: auto;
    padding: 0.75rem 1.25rem 0.5rem;
  }

  :global(.launcher-demo-frame .content.with-bulk-bar) {
    padding-bottom: 4.25rem;
  }
</style>
