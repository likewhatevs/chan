<script lang="ts">
  // Thin draggable bar used to resize a side panel. Mounted as a
  // sibling of the panel it controls, on whichever edge is "inward"
  // toward the rest of the workspace.
  //
  // Pointer events (not mouse): a finger drag on iOS / Android only
  // dispatches synthesised mouse events after a delay and with quirks
  // that drop the gesture. Pointer events unify mouse / touch / pen
  // and `setPointerCapture` keeps the move/up firing on the handle
  // even when the pointer leaves the 4px bar mid-drag, so we don't
  // need window listeners.
  //
  // `side` is the side of the workspace the panel sits on:
  //   - "right": handle is on the panel's left edge; dragging left
  //     widens the panel (used by file editor inspector + graph
  //     details).
  //   - "left": handle is on the panel's right edge; dragging right
  //     widens it. Reserved for a future left-side panel.
  //
  // `onChange` is called for every move with the new clamped width,
  // so the parent persists immediately rather than only on release.
  // That keeps the URL hash + localStorage in sync with what the
  // user sees, even if the page closes mid-drag.

  let {
    width = $bindable(),
    min = 140,
    max = 600,
    side = "right",
    onChange,
  }: {
    width: number;
    min?: number;
    max?: number;
    side?: "right" | "left";
    onChange?: (w: number) => void;
  } = $props();

  let activePointer: number | null = null;
  let startX = 0;
  let startW = 0;

  function onPointerDown(e: PointerEvent): void {
    if (activePointer !== null) return;
    e.preventDefault();
    activePointer = e.pointerId;
    startX = e.clientX;
    startW = width;
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";
  }

  function onPointerMove(e: PointerEvent): void {
    if (e.pointerId !== activePointer) return;
    const delta = e.clientX - startX;
    const next = side === "right" ? startW - delta : startW + delta;
    const clamped = Math.max(min, Math.min(max, next));
    width = clamped;
    onChange?.(clamped);
  }

  function endDrag(e: PointerEvent): void {
    if (e.pointerId !== activePointer) return;
    activePointer = null;
    try {
      (e.currentTarget as HTMLElement).releasePointerCapture(e.pointerId);
    } catch {}
    document.body.style.removeProperty("cursor");
    document.body.style.removeProperty("user-select");
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="handle"
  onpointerdown={onPointerDown}
  onpointermove={onPointerMove}
  onpointerup={endDrag}
  onpointercancel={endDrag}
></div>

<style>
  .handle {
    position: relative;
    width: 4px;
    flex-shrink: 0;
    background: var(--separator);
    cursor: col-resize;
    /* touch-action:none so iOS doesn't swallow horizontal drags as
       page scrolls. Required for pointer events to fire reliably on
       a finger drag. */
    touch-action: none;
    transition: width 0.1s, background 0.1s;
  }
  .handle:hover {
    width: 6px;
    background: var(--separator-hover);
  }
</style>
