<script lang="ts">
  // Generic chat-style bubble. Visually matches the assistant /
  // user bubbles in InlineAssist (same CSS variables for the body
  // background, same role + timestamp row, same rounded body).
  // InlineAssist still ships its own inline markup because its
  // bubbles wrap a lot of streaming / mode-toggle / tool-chip /
  // edit-card behaviour that doesn't belong in a generic shell.
  // This component covers the read-only callers: search results
  // and the scope-history overlay.
  //
  // The shell intentionally exposes no interaction: click handling,
  // keyboard focus, and the `active` highlight are owned by the
  // caller (an outer <li>, <button>, etc.). The `active` prop only
  // toggles a visual ring so the parent's list-level keyboard nav
  // can light up the right bubble without reaching into our DOM.

  import type { Snippet } from "svelte";

  let {
    align = "left",
    role,
    timestampLabel,
    active = false,
    children,
    header,
  }: {
    /// Horizontal alignment. `right` mirrors the user-side bubble
    /// (right-aligned, tinted body); `left` matches the assistant /
    /// neutral bubble.
    align?: "left" | "right";
    /// Optional uppercase role label rendered above the body (e.g.
    /// "You", "Assistant", "Group"). Omit to skip the role row
    /// unless `timestampLabel` or `header` is supplied.
    role?: string;
    /// Pre-formatted relative-time string (e.g. "3m ago"). The
    /// caller owns the ticker so this component stays stateless;
    /// pass `undefined` to omit.
    timestampLabel?: string;
    /// Visual highlight for keyboard / list-driven focus. The
    /// caller still owns scroll-into-view and `aria-selected`.
    active?: boolean;
    /// Body content. Required.
    children: Snippet;
    /// Optional snippet appended to the right of the role/timestamp
    /// row (e.g. small action buttons next to the timestamp).
    header?: Snippet;
  } = $props();

  const showHeaderRow = $derived(
    role !== undefined || timestampLabel !== undefined || header !== undefined,
  );
</script>

<div class="bubble {align}" class:active>
  {#if showHeaderRow}
    <div class="role-line">
      {#if role}<span class="role">{role}</span>{/if}
      {#if timestampLabel}<span class="ts">{timestampLabel}</span>{/if}
      {#if header}{@render header()}{/if}
    </div>
  {/if}
  <div class="body">{@render children()}</div>
</div>

<style>
  /* Visual contract mirrors the inline bubbles in InlineAssist so
     the two surfaces read as the same component family. Same
     CSS variables, same max-width, same role + ts typography. */
  .bubble {
    max-width: 85%;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .bubble.left { align-self: flex-start; align-items: flex-start; }
  .bubble.right { align-self: flex-end; align-items: flex-end; }

  .role-line {
    display: flex;
    align-items: baseline;
    gap: 6px;
  }
  .bubble.right .role-line { flex-direction: row-reverse; }
  .role {
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-secondary);
  }
  .ts {
    font-size: 12px;
    color: var(--text-secondary);
    opacity: 0.65;
    font-variant-numeric: tabular-nums;
  }

  .body {
    background: var(--assistant-bubble-bg);
    padding: 6px 10px;
    border-radius: 8px;
    font-size: 15px;
    line-height: 1.5;
    word-break: break-word;
  }
  .bubble.right .body { background: var(--assistant-user-bubble-bg); }

  /* Active highlight for list-driven keyboard navigation. Soft
     ring around the body so the row reads as "selected" without
     fighting the body background. Uses --link to match the active
     border-left treatment search results had pre-bubble. */
  .bubble.active .body {
    box-shadow: 0 0 0 2px var(--link);
  }
</style>
