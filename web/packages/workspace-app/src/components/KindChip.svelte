<script lang="ts">
  // Single kind chip used across the inspector, the search overlay,
  // and the graph overlay's ghost-body card. Centralises the
  // uppercase / weight / letter-spacing typography plus the per-kind
  // palette so every chip in the app reads the same regardless of
  // which surface mounted it.
  //
  // Layout intentionally stays minimal: callers position the chip
  // via their own flex / grid containers. The only layout knobs are
  // `block` (flex:1, used by inspector headers that want the chip to
  // span the row) and `compact` (smaller font + fixed-width column,
  // used by the search-results list where many rows need to align in
  // a tabular column).

  import type { Kind } from "../state/kinds";
  import { labelFor, chipColorVar } from "../state/kinds";

  let {
    kind,
    path,
    block = false,
    compact = false,
    ghost = false,
    dim = false,
    onClick,
  }: {
    kind: Kind;
    /// Optional file path. When set for a concrete file kind, the chip
    /// colour follows the graph canvas's extension bucket so the
    /// inspector bubble matches the node fill (a `.rs` source node
    /// opens a blue bubble, a `.txt` an orange one). Ignored for
    /// non-file kinds and when absent. The LABEL is unaffected: the
    /// chip still reads the wire kind's text.
    path?: string;
    /// flex:1, used by inspector headers so the chip fills the row.
    block?: boolean;
    /// smaller font + fixed-width column. Used by the search-results
    /// list so doc / image / contact / tag align vertically.
    compact?: boolean;
    /// 0.55 opacity. Graph overlay uses this for the ghost-body
    /// card (paths that point at nodes outside the current scope).
    ghost?: boolean;
    /// 0.65 opacity. Search overlay uses this for filename-match
    /// rows so they read as "same family, less emphasis" than a
    /// content-chunk hit.
    dim?: boolean;
    /// When set, render as a <button> so the chip itself acts as the
    /// KIND-route affordance ("click the path chip = scope graph to
    /// this file"). Unset = presentational <span>, same as before.
    onClick?: () => void;
  } = $props();
</script>

{#if onClick}
  <button
    type="button"
    class="kind-chip clickable"
    class:block
    class:compact
    class:ghost
    class:dim
    style="background: {chipColorVar(kind, path)}"
    onclick={onClick}
  >{labelFor(kind)}</button>
{:else}
  <span
    class="kind-chip"
    class:block
    class:compact
    class:ghost
    class:dim
    style="background: {chipColorVar(kind, path)}"
  >{labelFor(kind)}</span>
{/if}

<style>
  .kind-chip {
    color: #fff;
    text-transform: uppercase;
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.05em;
    padding: 1px 6px;
    border-radius: 3px;
    text-align: center;
  }
  /* Inspector headers (FileInfoBody, GraphPanel ghost-body): the
     chip stretches to fill the row alongside other header items. */
  .kind-chip.block {
    flex: 1;
  }
  /* Search-results list: smaller type and a fixed-width column so
     a stack of rows aligns vertically even with kinds of different
     label lengths. min-width fits the longest label ("document",
     "contact", "mention" -- all 7-8 chars at 10.5px). */
  .kind-chip.compact {
    display: inline-block;
    min-width: 64px;
    font-size: 10.5px;
    letter-spacing: 0.04em;
    padding: 1px 4px;
    flex-shrink: 0;
  }
  .kind-chip.ghost { opacity: 0.55; }
  .kind-chip.dim   { opacity: 0.65; }
  /* Clickable chip: drop the default button chrome so the chip still
     reads as a chip, then add cursor + a keyboard focus ring. */
  button.kind-chip.clickable {
    border: 0;
    margin: 0;
    font-family: inherit;
    cursor: pointer;
  }
  button.kind-chip.clickable:focus-visible {
    outline: 2px solid var(--link);
    outline-offset: 1px;
  }
</style>
