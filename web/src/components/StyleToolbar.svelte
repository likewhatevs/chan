<script lang="ts">
  // Floating style toolbar pinned to the top-left of the editor
  // canvas. Two states:
  //
  //   - collapsed: a small "Aa" pill. Default at rest so the
  //     editor's first few centimeters of text aren't crowded.
  //   - expanded: full formatting toolbar (block kind selector
  //     plus B/I/S/code/link/lists/HR/image). Reveals on hover or
  //     keyboard focus.
  //
  // Mouseleave starts a 2-second timer to collapse back. The delay
  // matches the user's mental model ("I'm done formatting, fade
  // away") without flapping while they reach for an editor word
  // mid-sentence. Pointer entry / focus cancels the timer; any
  // formatting click extends the expanded state (the toolbar keeps
  // capturing the cursor while clicked).
  //
  // The component reads back active-mark state from the Wysiwyg ref
  // (passed through `wysiwyg`) and re-runs every time `selVer` ticks
  // up. Owner re-emits selVer from the editor's selectionChange
  // callback so this toolbar reflects the live cursor position.

  import type Wysiwyg from "../editor/Wysiwyg.svelte";
  import type { BlockKind } from "../editor/Wysiwyg.svelte";

  let {
    wysiwyg,
    selVer,
    disabled = false,
  }: {
    /// Live Wysiwyg ref. May be undefined during the first render
    /// pass before bind:this resolves; the buttons no-op until it's
    /// available.
    wysiwyg: Wysiwyg | undefined;
    /// Selection version tick. Reading this in derived expressions
    /// ties the active-state computations to the editor's selection
    /// updates so the toolbar reflects cursor moves.
    selVer: number;
    /// Greys out the controls when the tab is in source mode or the
    /// file is read-only. Kept as a single prop so the parent only
    /// needs to compute the gate once.
    disabled?: boolean;
  } = $props();

  // Expanded vs collapsed pill. Hover or focus expand; a debounced
  // mouseleave (`COLLAPSE_DELAY_MS`) collapses back. Click-pressed
  // state also keeps it open: the editor's onmousedown=preventDefault
  // for formatting buttons can swallow the focus event, so we set
  // `clickPinned` explicitly on mousedown and clear it on mouseup.
  const COLLAPSE_DELAY_MS = 2000;
  let expanded = $state(false);
  let clickPinned = $state(false);
  let collapseTimer: ReturnType<typeof setTimeout> | null = null;

  function cancelCollapse(): void {
    if (collapseTimer) {
      clearTimeout(collapseTimer);
      collapseTimer = null;
    }
  }

  function scheduleCollapse(): void {
    cancelCollapse();
    collapseTimer = setTimeout(() => {
      collapseTimer = null;
      // Late-stage click can re-pin the toolbar between
      // scheduleCollapse and the timer firing; the guard prevents
      // an in-flight click from being yanked out from under the
      // user.
      if (!clickPinned) expanded = false;
    }, COLLAPSE_DELAY_MS);
  }

  function onEnter(): void {
    cancelCollapse();
    expanded = true;
  }

  function onLeave(): void {
    scheduleCollapse();
  }

  function onFocusIn(): void {
    cancelCollapse();
    expanded = true;
  }

  function onFocusOut(e: FocusEvent): void {
    // Only collapse if focus left the toolbar entirely (not just
    // moved between buttons).
    const next = e.relatedTarget as Node | null;
    const root = e.currentTarget as HTMLElement | null;
    if (next && root && root.contains(next)) return;
    scheduleCollapse();
  }

  // Active-mark / current-block derivations. Reading `selVer` ties
  // them to the editor's selection-change callback so the on-state
  // tracks cursor moves. The void cast keeps the expression typed
  // without tripping no-unused-expressions.
  const isBold = $derived.by(() => {
    void selVer;
    return wysiwyg?.isActive("bold") ?? false;
  });
  const isItalic = $derived.by(() => {
    void selVer;
    return wysiwyg?.isActive("italic") ?? false;
  });
  const isStrike = $derived.by(() => {
    void selVer;
    return wysiwyg?.isActive("strike") ?? false;
  });
  const isInlineCode = $derived.by(() => {
    void selVer;
    return wysiwyg?.isActive("code") ?? false;
  });
  const isBulletList = $derived.by(() => {
    void selVer;
    return wysiwyg?.isActive("bulletList") ?? false;
  });
  const isOrderedList = $derived.by(() => {
    void selVer;
    return wysiwyg?.isActive("orderedList") ?? false;
  });
  const isTaskList = $derived.by(() => {
    void selVer;
    return wysiwyg?.isActive("taskList") ?? false;
  });
  const isLink = $derived.by(() => {
    void selVer;
    return wysiwyg?.isActive("link") ?? false;
  });
  const blockKind = $derived.by<BlockKind>(() => {
    void selVer;
    return wysiwyg?.currentBlockKind() ?? "normal";
  });

  function onBlockKindChange(e: Event): void {
    const v = (e.currentTarget as HTMLSelectElement).value as BlockKind;
    wysiwyg?.setBlockKind(v);
  }

  // Pin/unpin around the editor's preventDefault dance: holding the
  // mouse down on a button keeps the toolbar from collapsing even if
  // the collapse timer is in flight.
  function onMouseDownPin(e: MouseEvent): void {
    e.preventDefault();
    clickPinned = true;
    cancelCollapse();
  }
  function onMouseUpUnpin(): void {
    clickPinned = false;
    // Pointer is still over the toolbar at this moment (mouseup
    // fires before any subsequent leave), so we leave `expanded`
    // alone; the eventual mouseleave will schedule the collapse.
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="style-toolbar"
  class:expanded
  class:disabled
  role="toolbar"
  tabindex="-1"
  aria-label="Style toolbar"
  onmouseenter={onEnter}
  onmouseleave={onLeave}
  onfocusin={onFocusIn}
  onfocusout={onFocusOut}
>
  <!-- Aa always visible: signals the toolbar's presence at rest
       and keeps a fixed anchor as the buttons swing in. The
       expanded row pops out from the right of it with the same
       easeOutBack curve the tab-menu bubble + OverlayShell use,
       so the motion language stays consistent. -->
  <span class="pill" aria-hidden="true">Aa</span>
  {#if expanded}
    <div class="fbtn-row">
    <span class="vsep" aria-hidden="true"></span>
    <select
      class="block-kind"
      value={blockKind}
      onchange={onBlockKindChange}
      onmousedown={(e) => e.stopPropagation()}
      disabled={disabled}
      title="block style"
    >
      <option value="h1">h1</option>
      <option value="h2">h2</option>
      <option value="h3">h3</option>
      <option value="normal">text</option>
      <option value="code">code</option>
      <option value="quote">quote</option>
    </select>
    <button
      class="fbtn"
      class:on={isBold}
      title="bold (Cmd/Ctrl+B)"
      disabled={disabled}
      onmousedown={onMouseDownPin}
      onmouseup={onMouseUpUnpin}
      onclick={() => wysiwyg?.toggleBold()}
    ><b>B</b></button>
    <button
      class="fbtn"
      class:on={isItalic}
      title="italic (Cmd/Ctrl+I)"
      disabled={disabled}
      onmousedown={onMouseDownPin}
      onmouseup={onMouseUpUnpin}
      onclick={() => wysiwyg?.toggleItalic()}
    ><i>I</i></button>
    <button
      class="fbtn"
      class:on={isStrike}
      title="strikethrough (Cmd/Ctrl+Shift+S)"
      disabled={disabled}
      onmousedown={onMouseDownPin}
      onmouseup={onMouseUpUnpin}
      onclick={() => wysiwyg?.toggleStrike()}
    ><s>S</s></button>
    <button
      class="fbtn"
      class:on={isInlineCode}
      title="inline code (Cmd/Ctrl+E)"
      disabled={disabled}
      onmousedown={onMouseDownPin}
      onmouseup={onMouseUpUnpin}
      onclick={() => wysiwyg?.toggleInlineCode()}
    ><code>{`<>`}</code></button>
    <button
      class="fbtn"
      class:on={isLink}
      title="link"
      aria-label="toggle link"
      disabled={disabled}
      onmousedown={onMouseDownPin}
      onmouseup={onMouseUpUnpin}
      onclick={() => wysiwyg?.toggleLink()}
    >🔗</button>
    <button
      class="fbtn"
      class:on={isBulletList}
      title="bullet list"
      aria-label="bullet list"
      disabled={disabled}
      onmousedown={onMouseDownPin}
      onmouseup={onMouseUpUnpin}
      onclick={() => wysiwyg?.toggleBulletList()}
    >•</button>
    <button
      class="fbtn"
      class:on={isOrderedList}
      title="ordered list"
      aria-label="ordered list"
      disabled={disabled}
      onmousedown={onMouseDownPin}
      onmouseup={onMouseUpUnpin}
      onclick={() => wysiwyg?.toggleOrderedList()}
    >1.</button>
    <button
      class="fbtn"
      class:on={isTaskList}
      title="task list"
      aria-label="task list"
      disabled={disabled}
      onmousedown={onMouseDownPin}
      onmouseup={onMouseUpUnpin}
      onclick={() => wysiwyg?.toggleTaskList()}
    >☐</button>
    <button
      class="fbtn"
      title="horizontal rule (insert ---)"
      aria-label="insert horizontal rule"
      disabled={disabled}
      onmousedown={onMouseDownPin}
      onmouseup={onMouseUpUnpin}
      onclick={() => wysiwyg?.insertHorizontalRule()}
    >―</button>
    <button
      class="fbtn"
      title="insert image"
      aria-label="insert image"
      disabled={disabled}
      onmousedown={onMouseDownPin}
      onmouseup={onMouseUpUnpin}
      onclick={() => wysiwyg?.insertImage()}
    >🖼</button>
    </div>
  {/if}
</div>

<style>
  /* Floating chrome anchored to the top-left of the editor canvas.
     position: absolute inside the .editor-host wrapper means the
     toolbar stays put even as the user scrolls the editor body.
     The hover-scale wobble + box-shadow lift match the tab-menu
     bubble and bottom pill so the motion language stays consistent
     across the chrome (mouseenter overshoots ~2%, mouseleave settles
     back through the same easeOutBack curve). */
  .style-toolbar {
    position: absolute;
    top: 8px;
    left: 8px;
    /* Sit above the editor's own decoration layer (smart-node chevrons
       use ~z-index 1) without competing with the bottom pill (z-index
       4500) or any overlay (>= 25000). */
    z-index: 30;
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 4px 6px;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: 999px;
    box-shadow: 0 4px 14px rgba(0, 0, 0, 0.18);
    color: var(--text);
    font-size: 13px;
    transform-origin: top left;
    transition:
      transform 260ms cubic-bezier(0.34, 1.56, 0.64, 1),
      box-shadow 160ms ease;
  }
  .style-toolbar:hover {
    transform: scale(1.02);
    box-shadow: 0 6px 20px rgba(0, 0, 0, 0.22);
  }
  .style-toolbar.disabled .fbtn,
  .style-toolbar.disabled .block-kind {
    opacity: 0.55;
  }
  /* Wraps the block-kind + buttons so we can animate it as a unit
     with the same easeOutBack pop the tab-menu bubble uses. The
     `transform-origin: top left` anchors the scale to the toolbar's
     corner so the bounce grows away from the editor edge rather
     than from the center (which would visually drift the row). */
  .fbtn-row {
    display: flex;
    align-items: center;
    gap: 2px;
    transform-origin: top left;
  }
  /* Aa badge: always-on signal that the toolbar lives in this
     corner. Acts as the toolbar's left anchor; the formatting row
     opens to its right when expanded. */
  .pill {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 22px;
    height: 22px;
    padding: 0 4px;
    color: var(--text-secondary);
    font-weight: 600;
    user-select: none;
  }
  /* Vertical divider between the Aa badge and the heading selector
     when the toolbar is expanded. Thin, low-contrast hairline; sits
     inside the .fbtn-row so it scales with the wobble. */
  .vsep {
    align-self: stretch;
    width: 1px;
    margin: 2px 4px;
    background: var(--border);
  }
  .block-kind {
    background: transparent;
    color: var(--text);
    border: 1px solid var(--btn-border);
    border-radius: 3px;
    padding: 0 4px;
    margin-right: 2px;
    font: inherit;
    font-size: 12px;
    height: 22px;
  }
  .fbtn {
    min-width: 24px;
    height: 22px;
    text-align: center;
    background: transparent;
    border: 1px solid transparent;
    border-radius: 3px;
    color: var(--text);
    cursor: pointer;
    font: inherit;
    font-size: 13px;
    padding: 0 4px;
    line-height: 20px;
  }
  .fbtn:hover:not(:disabled) {
    background: var(--hover-bg);
    border-color: var(--btn-border);
  }
  .fbtn.on {
    background: var(--hover-bg);
    border-color: var(--btn-hover);
  }
  .fbtn:disabled { cursor: default; opacity: 0.55; }
  .fbtn b, .fbtn i, .fbtn s, .fbtn code { font-size: 13px; }
  .fbtn code { font-family: ui-monospace, monospace; }

  @media (prefers-reduced-motion: reduce) {
    .style-toolbar,
    .style-toolbar:hover {
      transition: none;
      transform: none;
    }
  }
</style>
