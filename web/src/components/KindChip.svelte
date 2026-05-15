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
  import { labelFor, colorVarFor } from "../state/kinds";

  let {
    kind,
    block = false,
    compact = false,
    ghost = false,
    dim = false,
  }: {
    kind: Kind;
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
  } = $props();
</script>

<span
  class="kind-chip"
  class:block
  class:compact
  class:ghost
  class:dim
  style="background: {colorVarFor(kind)}"
>{labelFor(kind)}</span>

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
</style>
