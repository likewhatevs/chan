<script lang="ts">
  // Shared shell for the floating overlays (search, assistant,
  // graph). Owns the parts every overlay needs to be the same:
  //
  //   - dim backdrop layer that closes on click,
  //   - centered panel container that swallows clicks (so a click
  //     inside the panel doesn't bubble up and trigger close),
  //   - Escape closes (window-level listener, scoped to when the
  //     overlay is actually open),
  //   - z-index sits above every other UI layer.
  //
  // The overlay's body content goes through the `children` snippet
  // and renders inside the panel; the wrapped overlay owns its
  // own header + body + footer.
  //
  // Size is per-overlay because each has different content density:
  // search is a list, assistant is a conversation + composer, graph
  // is a sphere visualization. The defaults match the assistant's
  // current dimensions; pass `width` / `maxHeight` to override.

  import { onDestroy, onMount } from "svelte";

  import type { Snippet } from "svelte";

  let {
    open,
    onClose,
    width = "min(1200px, 96vw)",
    maxHeight = "92vh",
    children,
  }: {
    open: boolean;
    onClose: () => void;
    width?: string;
    maxHeight?: string;
    children: Snippet;
  } = $props();

  function onWindowKey(e: KeyboardEvent): void {
    if (!open) return;
    if (e.key === "Escape") {
      e.preventDefault();
      onClose();
    }
  }
  onMount(() => document.addEventListener("keydown", onWindowKey));
  onDestroy(() => document.removeEventListener("keydown", onWindowKey));
</script>

{#if open}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="overlay" onclick={onClose}>
    <!-- Always-on close button anchored to the top-right of the
         viewport, OUTSIDE the panel. Reachable even when the
         panel itself extends fully or its internal close header
         is covered. Sits below the iOS safe-area inset so it
         clears the notch / status bar. stopPropagation so a tap
         on the X does not also fire the scrim onClose (would be
         a double dismiss; harmless but cleaner to avoid). -->
    <button
      class="overlay-close"
      onclick={(e) => {
        e.stopPropagation();
        onClose();
      }}
      aria-label="Close"
    >×</button>
    <div
      class="panel"
      style="width: {width}; max-height: {maxHeight};"
      onclick={(e) => e.stopPropagation()}
      role="dialog"
      tabindex="-1"
    >
      {@render children()}
    </div>
  </div>
{/if}

<style>
  /* Centered, anchored near the bottom of the viewport to keep an
     input row close to the user's typing reach. The same recipe as
     search and assistant before the extraction, just shared. */
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.45);
    display: flex;
    justify-content: center;
    align-items: flex-end;
    /* iOS draws the status bar OVER the WebView (viewport-fit=
       cover), so the panel's top edge needs to clear both the
       safe-area inset AND a comfortable buffer before any tap
       target sits there. Sum, not max(): even with a non-notched
       device the +44px floor keeps the always-on close button
       visible and reachable. */
    padding-top: calc(env(safe-area-inset-top, 0px) + 44px);
    padding-bottom: max(env(safe-area-inset-bottom, 0px), 16px);
    z-index: 25000;
    box-sizing: border-box;
    /* iOS WKWebView only fires `click` on non-button elements that
       have `cursor: pointer` declared. Without this the scrim taps
       silently no-op and overlays look stuck. */
    cursor: pointer;
  }
  /* Always-on close affordance, fixed at the top-right of the
     viewport. Sits inside the .overlay's flex container but
     positions itself absolutely so the flex layout (panel
     centering / bottom alignment) is unaffected. */
  .overlay-close {
    position: absolute;
    top: calc(env(safe-area-inset-top, 0px) + 4px);
    right: 12px;
    width: 36px;
    height: 36px;
    border-radius: 18px;
    background: rgba(0, 0, 0, 0.65);
    border: 1px solid rgba(255, 255, 255, 0.2);
    color: #fff;
    font-size: 22px;
    line-height: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    z-index: 25002;
    padding: 0;
    /* Tap target stays comfortably above the status bar's bottom
       edge. The 4px from the safe-area inset keeps it visually
       inside the WebView area (not behind the iOS battery icon)
       while still being thumb-reachable. */
  }
  .overlay-close:hover { background: rgba(0, 0, 0, 0.8); }
  .panel {
    background: var(--bg-elev);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: 8px;
    box-shadow: 0 14px 44px rgba(0, 0, 0, 0.5);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
</style>
