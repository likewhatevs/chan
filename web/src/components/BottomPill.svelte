<script lang="ts">
  // Window-level floating bottom pill. Single instance per window;
  // hosts the navigation button list (AccessoryPill) inside a
  // rounded-pill chrome anchored at the bottom-center of the
  // viewport.
  //
  // Idle hide: when `idle.active` flips on (5s of no input), the
  // pill fades to transparent + drops pointer events. Any scroll /
  // click / keypress flips it back via the global tracker. While
  // the mouse is over the pill itself, pinAccessory() keeps it
  // visible so the user can't have it fade from under their cursor.

  import AccessoryPill from "./AccessoryPill.svelte";
  import { idle, pinAccessory } from "../state/idle.svelte";

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
  role="toolbar"
  aria-label="Navigation"
  onmouseenter={onEnter}
  onmouseleave={onLeave}
>
  <AccessoryPill />
</div>

<style>
  .bottom-pill {
    position: fixed;
    left: 50%;
    bottom: calc(env(safe-area-inset-bottom, 0px) + 12px);
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
    font-size: 14px;
    color: var(--text);
    max-width: calc(100vw - 16px);
    overflow-x: auto;
    scrollbar-width: none;
    transition: opacity 200ms ease;
  }
  .bottom-pill::-webkit-scrollbar { display: none; }
  /* Idle: fade out + drop pointer events so the pill doesn't catch
     the click that would otherwise bring it back through the global
     idle tracker. The first such click reactivates the tracker and
     the next click can interact with the pill normally. */
  .bottom-pill.idle {
    opacity: 0;
    pointer-events: none;
  }
</style>
