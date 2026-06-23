<script lang="ts">
  // The window feed: the library's authoritative open-window set, grouped by
  // library (local first, then each remote devserver library). Rows are
  // recomposed from kind/ordinal/workspace_path, never from the opaque
  // window_id or the library-composed title. The same feed drives the desktop
  // Window menu and `cs window list`, so all three always agree.
  //
  // Each row carries two icon actions: [FOCUS] (bring the window to focus —
  // un-hide + focus if buried, take focus if visible) and [SHOW/HIDE] (the
  // visibility toggle, Eye when visible / EyeOff when hidden). The Eye/EyeOff
  // conveys the connection state, so the mutable surface drops the status dot.
  // The read-only surface (gateway/devserver, no desktop bridge) has no actions
  // and keeps the static dot indicator.
  import { Eye, EyeOff, Focus } from "lucide-svelte";
  import { library, focusWindow, remoteLibraryName, toggleWindow } from "../state/library.svelte";
  import { LOCAL_LIBRARY_ID, librarySectionLabel, windowRowLabel } from "../lib/windowLabel";
  import { readOnly } from "../state/capabilities";
  import type { WindowRecord } from "../api/library";

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

  const groups = $derived(groupByLibrary(library.windows));
</script>

{#if library.windows.length}
  <section class="feed">
    <h2 class="feed-heading">Open windows</h2>
    {#each groups as g (g.libraryId)}
      <div class="group">
        <h3 class="group-title">{g.label}</h3>
        <ul class="rows">
          {#each g.windows as w (w.window_id)}
            <li>
              {#if readOnly}
                <!-- Read-only surface (gateway/devserver): the dot shows the
                     connection state but can't drive a native window. -->
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
                <!-- The mutable surface exposes two icon actions per row:
                     [FOCUS] (un-hide + focus, or take focus) and [SHOW/HIDE]
                     (visibility toggle; Eye visible / EyeOff hidden). The
                     Eye/EyeOff conveys the connection state — no separate dot. -->
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
                      onclick={() => focusWindow(w)}>
                      <Focus size={16} />
                    </button>
                    <button
                      class="icon-btn"
                      class:on={w.connected}
                      type="button"
                      title={w.connected ? "Hide window" : "Show window"}
                      aria-label={w.connected ? "Hide window" : "Show window"}
                      onclick={() => toggleWindow(w)}>
                      {#if w.connected}<Eye size={16} />{:else}<EyeOff size={16} />{/if}
                    </button>
                  </div>
                </div>
              {/if}
            </li>
          {/each}
        </ul>
      </div>
    {/each}
  </section>
{/if}
