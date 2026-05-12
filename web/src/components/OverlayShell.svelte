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
  // Size: every overlay fills the full viewport vertically with a
  // uniform margin (reserved by .overlay's padding) on every side.
  // The panel grows to take all the height available between the
  // top and bottom paddings rather than hugging the bottom of the
  // viewport. The width prop caps the cross-axis (search / settings
  // can stay narrower; assistant / graph go wide). Defaults are
  // wide enough for assistant + graph; overrides land in the
  // narrower panels.

  import { onDestroy, onMount } from "svelte";

  import type { Snippet } from "svelte";

  let {
    open,
    onClose,
    width = "min(1200px, calc(100vw - 48px))",
    children,
  }: {
    open: boolean;
    onClose: () => void;
    width?: string;
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
      style="width: {width};"
      onclick={(e) => e.stopPropagation()}
      role="dialog"
      tabindex="-1"
    >
      {@render children()}
    </div>
  </div>
{/if}

<style>
  /* Full-viewport panel with uniform margin on every side. The
     panel itself is bound by .overlay's padding; .panel's flex:1
     in the cross axis (height) lets it grow to fill the available
     vertical space between top and bottom paddings rather than
     hugging the viewport bottom. */
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.45);
    display: flex;
    justify-content: center;
    align-items: stretch;
    /* iOS draws the status bar OVER the WebView (viewport-fit=
       cover), so the panel's top edge needs to clear both the
       safe-area inset AND a comfortable buffer before any tap
       target sits there. Sum, not max(): even with a non-notched
       device the +44px floor keeps the always-on close button
       visible and reachable. */
    padding-top: calc(env(safe-area-inset-top, 0px) + 44px);
    padding-bottom: max(env(safe-area-inset-bottom, 0px), 24px);
    /* Side padding is set narrow on the .overlay; the panel's
       width prop (capped at 1200px) handles wide-viewport
       letterboxing. The padding here just guarantees a minimum
       gutter on phones / narrow windows where the width prop
       would otherwise hit 100vw. */
    padding-left: 16px;
    padding-right: 16px;
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
    /* Match the menu / tab-menu bounce so every overlay shares the
       same open feel. */
    transform-origin: center top;
    animation: overlay-pop 260ms cubic-bezier(0.34, 1.56, 0.64, 1);
  }
  @keyframes overlay-pop {
    0% {
      opacity: 0;
      transform: scale(0.92);
    }
    100% {
      opacity: 1;
      transform: scale(1);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .panel { animation: none; }
  }
</style>
