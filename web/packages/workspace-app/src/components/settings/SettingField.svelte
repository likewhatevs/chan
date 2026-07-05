<script lang="ts">
  // Layout wrapper for one setting: a title + hint on the left, the
  // control on the right. Owns the descendant styling for the native
  // controls a section drops in (select, text/number input, range) so
  // each section stays declarative and the control look is defined once.

  import type { Snippet } from "svelte";

  let {
    label,
    hint,
    children,
  }: {
    label: string;
    hint?: string;
    children: Snippet;
  } = $props();
</script>

<section class="field">
  <div class="meta">
    <h3>{label}</h3>
    {#if hint}<p class="hint">{hint}</p>{/if}
  </div>
  <div class="control">
    {@render children()}
  </div>
</section>

<style>
  .field {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 16px 0;
    border-bottom: 1px solid color-mix(in srgb, var(--border) 60%, transparent);
  }
  .field:last-child {
    border-bottom: 0;
  }
  .meta {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  h3 {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
    color: var(--text);
  }
  .hint {
    margin: 0;
    color: var(--text-secondary);
    font-size: 13px;
    line-height: 1.4;
  }
  .control {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
  }
  /* Native controls a section nests here are styled once, so sections
     don't each re-declare select / input chrome. `:global` reaches the
     descendants the section renders into this scope. */
  .control :global(select),
  .control :global(input[type="text"]),
  .control :global(input[type="number"]) {
    background: var(--bg);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 5px 8px;
    font: inherit;
    min-width: 14em;
  }
  .control :global(input[type="range"]) {
    flex: 1;
    min-width: 12em;
    accent-color: var(--link);
  }
</style>
