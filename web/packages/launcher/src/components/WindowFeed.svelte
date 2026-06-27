<script lang="ts">
  // The window feed: the library's authoritative window set, ONE list grouped by
  // library (local first, then each remote devserver library). Visible and hidden
  // windows are listed together -- the per-row Eye/EyeOff is the only hidden vs
  // visible indicator. Rows render through the shared WindowRow; the
  // grouping/sort/dedupe come from the pure lib/machineTree helper, the same
  // source the nested machine tree uses, so the feed and `cs window list` agree.
  import WindowRow from "./WindowRow.svelte";
  import { library, remoteLibraryName } from "../state/library.svelte";
  import { groupWindowsByLibrary } from "../lib/machineTree";

  // ONE list, grouped by library (local first, then each remote devserver).
  const groups = $derived(groupWindowsByLibrary(library.windows, remoteLibraryName));
</script>

{#if library.windows.length}
  <section class="feed">
    <h2 class="feed-heading">Open windows</h2>
    {#each groups as g (g.libraryId)}
      <div class="group">
        <h3 class="group-title">{g.label}</h3>
        <ul class="rows">
          {#each g.windows as w (w.window_id)}
            <li><WindowRow {w} /></li>
          {/each}
        </ul>
      </div>
    {/each}
  </section>
{/if}
