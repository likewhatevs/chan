<script lang="ts">
  // Shared shell for the floating overlays (search, graph, files,
  // settings). Owns the parts every overlay needs to be the same:
  //
  //   - dim backdrop layer that closes on click,
  //   - centered panel container that swallows clicks (so a click
  //     inside the panel doesn't bubble up and trigger close),
  //   - z-index sits above every other UI layer and rises with the
  //     overlay's depth in the global stack so a freshly-opened
  //     overlay paints over the one it was opened from.
  //
  // Escape handling lives in App.svelte, not here: a per-shell
  // listener fired once per mounted-open overlay and closed every
  // open overlay on a single press. The window-level handler closes
  // the topmost overlay only (see `topOverlay`).
  //
  // The overlay's body content goes through the `children` snippet
  // and renders inside the panel; the wrapped overlay owns its
  // own header + body + footer.
  //
  // Size: every overlay fills the full viewport vertically with a
  // uniform margin (reserved by .overlay's padding) on every side.
  // The panel grows to take all the height available between the
  // top and bottom paddings rather than hugging the bottom of the
  // viewport. The width prop caps the cross-axis; overrides land in
  // narrower panels.

  import type { Snippet } from "svelte";
  import { overlayDepth, type OverlayId } from "../state/store.svelte";
  import { overlayMaximized } from "../state/pageWidth.svelte";
  // Each wrapped overlay renders its own maximize/restore + close
  // chrome inside its header (left and right edges respectively),
  // so this shell only provides the backdrop, the panel container,
  // and the depth-based z-index.

  let {
    id,
    open,
    onClose,
    onBackdropContextMenu,
    width,
    align = "stretch",
    children,
  }: {
    id: OverlayId;
    open: boolean;
    onClose: () => void;
    onBackdropContextMenu?: (e: MouseEvent) => void;
    width?: string;
    // Vertical anchoring. "stretch" fills the viewport height (the
    // full-height overlays). "top" pins an auto-height panel near the
    // top, for a Spotlight-style bubble that grows with its content.
    align?: "stretch" | "top";
    children: Snippet;
  } = $props();

  // 10-step gap per depth so any same-overlay sub-layers (popovers,
  // dropdowns) still have room above their parent without spilling
  // into the next overlay's slot. Closed (depth -1) collapses to the
  // base z-index, but the `{#if open}` below means we never paint
  // anything in that state anyway.
  const zIndex = $derived(25000 + Math.max(0, overlayDepth(id)) * 10);

  // Resolved panel width. An explicit `width` prop from the caller
  // wins (no overlay sets one today, but the override stays useful
  // for narrower future surfaces). Otherwise we honor the global
  // overlay-maximize toggle: 1200px cap by default, full viewport
  // minus a symmetric 44px gutter when maximized so the side margin
  // matches the top safe-area + chrome buffer.
  const resolvedWidth = $derived(
    width ??
      (overlayMaximized.on
        ? "calc(100vw - 88px)"
        : "min(1200px, calc(100vw - 48px))"),
  );

  function onContextMenu(e: MouseEvent): void {
    if (!onBackdropContextMenu) return;
    e.preventDefault();
    e.stopPropagation();
    onBackdropContextMenu(e);
  }
</script>

{#if open}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="overlay"
    class:top={align === "top"}
    style="z-index: {zIndex};"
    onclick={onClose}
    oncontextmenu={onContextMenu}
  >
    <div
      class="panel"
      style="width: {resolvedWidth};"
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
    /* Symmetric top/bottom gutter. iOS draws the status bar OVER
       the WebView (viewport-fit=cover); the safe-area-inset-top
       branch of max() clears the notch on those devices while the
       24px floor matches the bottom gutter on desktop / non-notched
       viewports. The in-panel chrome (maximize on the left, close
       on the right of each overlay's own header) lives below this
       padding so it's never under the status bar. */
    padding-top: max(env(safe-area-inset-top, 0px), 24px);
    padding-bottom: max(env(safe-area-inset-bottom, 0px), 24px);
    /* Side padding is set narrow on the .overlay; the panel's
       width prop (capped at 1200px) handles wide-viewport
       letterboxing. The padding here just guarantees a minimum
       gutter on phones / narrow windows where the width prop
       would otherwise hit 100vw. */
    padding-left: 16px;
    padding-right: 16px;
    /* z-index is bound inline from the overlay stack depth so a
       freshly-opened overlay paints above the one underneath. */
    box-sizing: border-box;
    /* iOS WKWebView only fires `click` on non-button elements that
       have `cursor: pointer` declared. Without this the scrim taps
       silently no-op and overlays look stuck. */
    cursor: pointer;
  }
  /* Top-anchored variant: the panel takes its content height and pins
     near the top (below the notch on iOS, a comfortable drop on
     desktop). Used by the Spotlight-style command launcher. */
  .overlay.top {
    align-items: flex-start;
    padding-top: max(env(safe-area-inset-top, 0px), 10vh);
  }
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
       same open feel. The hover transition takes over once the
       open animation finishes, mirroring the floating chrome
       (BottomPill / WikiStatusBar / AppStatusBar) so the panels
       feel alive instead of inert. */
    transform-origin: center top;
    animation: overlay-pop 260ms cubic-bezier(0.34, 1.56, 0.64, 1);
    transition:
      transform 260ms cubic-bezier(0.34, 1.56, 0.64, 1),
      box-shadow 160ms ease;
  }
  /* Much subtler scale than the pills (1.04). Panels are big; even
     a 1% factor shoves the whole modal more than it needs to. Half
     a percent is enough to read as a lift on cursor enter without
     making the panel feel restless. */
  .panel:hover {
    transform: scale(1.005);
    box-shadow: 0 16px 50px rgba(0, 0, 0, 0.52);
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
    .panel,
    .panel:hover {
      animation: none;
      transition: none;
      transform: none;
    }
  }
</style>
