<script lang="ts">
  // The window feed: the library's authoritative window set, ONE list grouped by
  // library (local first, then each remote devserver library). Visible and hidden
  // windows are listed together — the per-row Eye/EyeOff is the only hidden vs
  // visible indicator (no Open/Hidden section split). Rows are recomposed from
  // kind/ordinal/workspace_path, never from the opaque window_id or the
  // library-composed title. The same feed drives the desktop Window menu and
  // `cs window list`, so all three always agree.
  //
  // Each row (mutable surface) carries two icon actions: [FOCUS] (bring to focus
  // — un-hide + focus if hidden, take focus if visible) and [SHOW/HIDE] (the
  // visibility toggle, Eye when visible / EyeOff when hidden, keyed on the
  // server-persisted `hidden`). The toggle stays a bridge op; the desktop
  // persists `hidden` at the bury/unbury chokepoint, so a hide flips the row's
  // icon on the feed round-trip. The read-only surface (gateway/devserver, no
  // desktop bridge) has no actions and keeps the static connection dot.
  import { Eye, EyeOff, Focus } from "lucide-svelte";
  import {
    library,
    focusWindow,
    remoteLibraryName,
    toggleWindow,
    reportError,
    clearError,
  } from "../state/library.svelte";
  import { LOCAL_LIBRARY_ID, librarySectionLabel, windowRowLabel } from "../lib/windowLabel";
  import { readOnly } from "../state/capabilities";
  import type { WindowRecord } from "../api/library";

  // The FOCUS / SHOW-HIDE bridge ops reject on a surface with no desktop and on a
  // stale/reaped window (the native window is gone). Catch here and surface the
  // failure in the banner instead of letting a floating promise reject into the
  // console — the eye on a reaped row stays a clean no-op, not an error flood.
  async function run(action: Promise<void>): Promise<void> {
    clearError();
    try {
      await action;
    } catch (e) {
      reportError(e);
    }
  }

  interface Group {
    libraryId: string;
    label: string;
    windows: WindowRecord[];
  }

  function sortWindows(a: WindowRecord, b: WindowRecord): number {
    // The connect control terminal is pinned FIRST in its devserver group.
    if (a.control !== b.control) return a.control ? -1 : 1;
    if (a.kind !== b.kind) return a.kind === "terminal" ? -1 : 1;
    return a.ordinal - b.ordinal;
  }

  function groupByLibrary(windows: WindowRecord[]): Group[] {
    const map = new Map<string, WindowRecord[]>();
    for (const w of windows) {
      const arr = map.get(w.library_id) ?? [];
      // Defense-in-depth: a duplicate (library_id, window_id) must not reach the
      // keyed {#each} below — a repeated key throws Svelte each_key_duplicate and
      // freezes the entire feed. The library's keyed-pathspec mount prefix
      // prevents the collision at the source; dropping a stray duplicate here
      // keeps a slip a silent no-op instead of a render crash.
      if (arr.some((x) => x.window_id === w.window_id)) continue;
      arr.push(w);
      map.set(w.library_id, arr);
    }
    const groups: Group[] = [...map.entries()].map(([libraryId, ws]) => ({
      libraryId,
      label: librarySectionLabel(
        libraryId,
        libraryId === LOCAL_LIBRARY_ID ? null : remoteLibraryName(libraryId),
      ),
      windows: ws.slice().sort(sortWindows),
    }));
    groups.sort((a, b) => {
      if (a.libraryId === LOCAL_LIBRARY_ID) return -1;
      if (b.libraryId === LOCAL_LIBRARY_ID) return 1;
      return a.label.localeCompare(b.label);
    });
    return groups;
  }

  // ONE list, grouped by library (local first, then each remote devserver).
  // Visible and hidden windows sit together; the per-row Eye/EyeOff (keyed on the
  // server-persisted `hidden`) is the sole hidden/visible indicator.
  const groups = $derived(groupByLibrary(library.windows));
</script>

{#snippet windowRow(w: WindowRecord)}
  {#if readOnly}
    <!-- Read-only surface (gateway/devserver): the dot shows the connection
         state but can't drive a native window. -->
    <div class="row">
      <div class="row-main">
        <span class="row-name">{windowRowLabel(w)}</span>
        {#if w.workspace_path}
          <span class="row-sub" title={w.workspace_path}>{w.workspace_path}</span>
        {/if}
      </div>
      <span
        class="dot"
        class:live={w.connected}
        title={w.connected ? "Connected" : "Detached"}></span>
    </div>
  {:else}
    <!-- The mutable surface exposes two icon actions per row: [FOCUS] (un-hide +
         focus, or take focus) and [SHOW/HIDE] (visibility toggle; Eye visible /
         EyeOff hidden, keyed on the server-persisted `hidden`). -->
    <div class="row">
      <div class="row-main">
        <span class="row-name">{windowRowLabel(w)}</span>
        {#if w.workspace_path}
          <span class="row-sub" title={w.workspace_path}>{w.workspace_path}</span>
        {/if}
      </div>
      <div class="row-actions">
        <button
          class="icon-btn"
          type="button"
          title="Focus window"
          aria-label="Focus window"
          onclick={() => run(focusWindow(w))}>
          <Focus size={16} />
        </button>
        <button
          class="icon-btn"
          class:on={!w.hidden}
          type="button"
          title={w.hidden ? "Show window" : "Hide window"}
          aria-label={w.hidden ? "Show window" : "Hide window"}
          onclick={() => run(toggleWindow(w))}>
          {#if w.hidden}<EyeOff size={16} />{:else}<Eye size={16} />{/if}
        </button>
      </div>
    </div>
  {/if}
{/snippet}

{#snippet librarySection(groups: Group[])}
  {#each groups as g (g.libraryId)}
    <div class="group">
      <h3 class="group-title">{g.label}</h3>
      <ul class="rows">
        {#each g.windows as w (w.window_id)}
          <li>{@render windowRow(w)}</li>
        {/each}
      </ul>
    </div>
  {/each}
{/snippet}

{#if library.windows.length}
  <section class="feed">
    <h2 class="feed-heading">Open windows</h2>
    {@render librarySection(groups)}
  </section>
{/if}
