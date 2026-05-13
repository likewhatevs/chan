<script lang="ts">
  // Window-level floating bottom pill. Single instance per window;
  // hosts the navigation buttons (AccessoryPill) anchored at the
  // bottom-center of the viewport. The pill chrome (rounded bg,
  // border, shadow) lives inside AccessoryPill so the assistant
  // ensō can float above it without dragging the chrome along.
  //
  // Idle hide: when `idle.active` flips on (5s of no input), the
  // whole bar fades to transparent + drops pointer events. Any
  // scroll / click / keypress flips it back via the global tracker.
  // While the mouse is over the bar itself, pinAccessory() keeps it
  // visible so the user can't have it fade from under their cursor.

  import AccessoryPill from "./AccessoryPill.svelte";
  import { idle, pinAccessory, readMode } from "../state/idle.svelte";

  let release: (() => void) | null = null;
  function onEnter(): void {
    release?.();
    release = pinAccessory();
  }
  function onLeave(): void {
    release?.();
    release = null;
  }
</script>

<!-- svelte-ignore a11y_interactive_supports_focus -->
<!-- The container itself is non-tabbable; focusable controls live
     inside (the AccessoryPill buttons). The mouseenter/leave are
     pure visual hints (pin the bar so it doesn't fade under the
     cursor); tabindex on the wrapper would create a no-op focus
     stop. -->
<div
  class="bottom-pill"
  class:idle={idle.active}
  class:read-mode={readMode.active}
  role="toolbar"
  aria-label="Navigation"
  onmouseenter={onEnter}
  onmouseleave={onLeave}
>
  <AccessoryPill />
</div>

<style>
  /* Positioning + fade-state container. NO chrome here; the rounded
     bg + border + shadow live on AccessoryPill's .pill-chrome (so
     the chrome wraps only the side buttons, never the floating
     ensō). */
  .bottom-pill {
    position: fixed;
    left: 50%;
    bottom: calc(env(safe-area-inset-bottom, 0px) + 56px);
    /* translateX(-50%) centers the pill; the scale slot is what the
       idle/hover states animate. Keep both factors in every rule
       that touches transform or the centering jumps. */
    transform: translateX(-50%) scale(1);
    z-index: 4500;
    /* easeOutBack: ~10% overshoot on the way in so the reveal and
       hover read as alive rather than mechanical. */
    transition:
      opacity 220ms cubic-bezier(0.34, 1.56, 0.64, 1),
      transform 260ms cubic-bezier(0.34, 1.56, 0.64, 1);
  }
  .bottom-pill:hover {
    transform: translateX(-50%) scale(1.04);
  }
  /* Idle: fade out + drop pointer events so the pill doesn't catch
     the click that would otherwise bring it back through the global
     idle tracker. The first such click reactivates the tracker and
     the next click can interact with the pill normally. The slight
     scale-down pairs with the easeOutBack so the reveal pops back
     in with a tiny bounce. */
  .bottom-pill.idle {
    opacity: 0;
    pointer-events: none;
    transform: translateX(-50%) scale(0.94);
  }
  @media (prefers-reduced-motion: reduce) {
    .bottom-pill,
    .bottom-pill:hover,
    .bottom-pill.idle {
      transition: opacity 120ms linear;
      transform: translateX(-50%) scale(1);
    }
  }
  /* Read mode: fade the bar so it reads as ambient chrome rather
     than an active control surface. The grayscale filter desaturates
     the assistant attractor (orange ensō) into the same muted
     palette; combined with the lower opacity the entire bar reads
     as "you're reading; controls are still there if you reach for
     them". The .idle rule still wins when both are active. */
  .bottom-pill.read-mode {
    opacity: 0.55;
    filter: grayscale(0.8);
  }
  /* Side icons (Files / Search on the left, Graph / Settings on
     the right) collapse their width AND opacity to zero so the
     chrome itself shrinks down to just the ensō slot when idle.
     `pointer-events: none` while collapsed means the only
     hit-testable button is the ensō; once the user reaches the
     bar via the logo, the chrome :hover state propagates upward
     and the sides unfurl. */
  .bottom-pill :global(.fbtn.side) {
    opacity: 0;
    pointer-events: none;
    min-width: 0;
    width: 0;
    padding: 0;
    border-width: 0;
    overflow: hidden;
    transition:
      opacity 200ms ease,
      width 280ms cubic-bezier(0.34, 1.56, 0.64, 1),
      min-width 280ms cubic-bezier(0.34, 1.56, 0.64, 1),
      padding 280ms cubic-bezier(0.34, 1.56, 0.64, 1);
  }
  .bottom-pill:hover :global(.fbtn.side) {
    opacity: 1;
    pointer-events: auto;
    min-width: 38px;
    width: 38px;
    padding: 0 9px;
    border-width: 1px;
  }
  @media (prefers-reduced-motion: reduce) {
    .bottom-pill :global(.fbtn.side),
    .bottom-pill:hover :global(.fbtn.side) {
      transition: opacity 120ms linear;
      width: 38px;
      min-width: 38px;
      padding: 0 9px;
      border-width: 1px;
      opacity: 1;
      pointer-events: auto;
    }
  }
</style>
