<script lang="ts">
  // Scope-history overlay. Lists every persisted assistant
  // conversation in the drive (per-file, per-group, drive) so the
  // user can revisit, resume, or export an old thread.
  //
  // Data comes from `scopeHistoryOverlay.entries`, populated by
  // `refreshScopeHistory` in the store. The overlay itself is a
  // thin renderer over that list: filter chips per scope kind,
  // sort toggle, scrollable list of bubbles.
  //
  // Per-entry actions live in the bubble header: Resume (in-place
  // when the scope can bind to the current window; falls back to
  // inline peek for group scopes), Open in new window (uses the
  // snapshotted layout URL), Export to .md (writes a self-
  // describing markdown transcript under the drive's answers_dir),
  // Delete (drops the on-disk blob + in-memory mirror).

  import {
    ExternalLink,
    Eye,
    FileDown,
    Maximize2,
    Minimize2,
    Trash2,
    X,
  } from "lucide-svelte";
  import {
    overlayMaximized,
    setOverlayMaximized,
  } from "../state/pageWidth.svelte";
  import { untrack } from "svelte";
  import Bubble from "./Bubble.svelte";
  import HamburgerMenu from "./HamburgerMenu.svelte";
  import OverlayShell from "./OverlayShell.svelte";
  import {
    clearAllScopeHistory,
    closeScopeHistory,
    deleteScopeHistoryEntry,
    exportScopeHistoryToDrive,
    fetchScopeHistoryTurns,
    openScopeHistoryInNewWindow,
    refreshScopeHistory,
    resumeScopeHistoryEntry,
    scopeHistoryOverlay,
    tree,
    uiConfirm,
    type AssistantTurn,
    type ScopeHistoryEntry,
    type ScopeHistoryKind,
  } from "../state/store.svelte";

  /// Live tick so the "X ago" labels refresh while the overlay is
  /// open. Same pattern as InlineAssist's chat timeline.
  let now = $state(Date.now());
  $effect(() => {
    if (!scopeHistoryOverlay.open) return;
    const id = setInterval(() => {
      now = Date.now();
    }, 30_000);
    return () => clearInterval(id);
  });

  /// Refresh on every open so a thread the user just created in
  /// the assistant overlay shows up the moment they pop the
  /// history. `untrack` keeps the call out of the reactivity
  /// graph; we only want it on the open transition.
  $effect(() => {
    if (!scopeHistoryOverlay.open) return;
    untrack(() => {
      void refreshScopeHistory();
    });
  });

  function formatRelative(ts: number | undefined): string {
    if (!ts) return "";
    const diffSec = Math.max(0, Math.floor((now - ts) / 1000));
    if (diffSec < 60) return "just now";
    if (diffSec < 3600) return `${Math.floor(diffSec / 60)}m ago`;
    if (diffSec < 86400) return `${Math.floor(diffSec / 3600)}h ago`;
    if (diffSec < 7 * 86400) return `${Math.floor(diffSec / 86400)}d ago`;
    return new Date(ts).toISOString().slice(0, 10);
  }

  /// Per-kind counts on the raw entry list (pre-filter). Drives
  /// the chip count badges so the user can see at a glance how
  /// much each filter would hide.
  const counts = $derived.by(() => {
    const c = { file: 0, group: 0, drive: 0 };
    for (const e of scopeHistoryOverlay.entries) c[e.kind] += 1;
    return c;
  });

  /// Filtered + sorted view of `entries`. Filters compose with AND:
  /// an entry shows only if its kind chip is on. Sort key is
  /// `last_touched` (or `created_at` as fallback) when sortByRecent
  /// is true, else `created_at`.
  const visible = $derived.by<ScopeHistoryEntry[]>(() => {
    const f = scopeHistoryOverlay.filters;
    const rows = scopeHistoryOverlay.entries.filter((e) => f[e.kind]);
    const sorted = [...rows];
    switch (scopeHistoryOverlay.sortBy) {
      case "recent":
        sorted.sort(
          (a, b) =>
            (b.last_touched ?? b.created_at ?? 0) -
            (a.last_touched ?? a.created_at ?? 0),
        );
        break;
      case "created":
        sorted.sort(
          (a, b) =>
            (b.created_at ?? b.last_touched ?? 0) -
            (a.created_at ?? a.last_touched ?? 0),
        );
        break;
      case "title":
        sorted.sort((a, b) =>
          a.title.localeCompare(b.title, undefined, { sensitivity: "base" }),
        );
        break;
      case "turns":
        sorted.sort((a, b) => b.turn_count - a.turn_count);
        break;
    }
    return sorted;
  });

  /// Pill color per kind, mirroring the search overlay palette so
  /// "scope" reads as the same visual language as "hit kind".
  const KIND_COLOR: Record<ScopeHistoryKind, string> = {
    file: "var(--g-doc)",
    group: "var(--g-tag)",
    drive: "var(--g-img)",
  };

  function kindLabel(k: ScopeHistoryKind): string {
    return k.toUpperCase();
  }

  /// Set of every path the drive currently knows about. Used to
  /// gate the "Resume" / "Open in new window" actions for scopes
  /// whose underlying files have been deleted; when nothing is on
  /// disk anymore, the only useful action is Export.
  const driveFilePaths = $derived.by<Set<string>>(() => {
    const s = new Set<string>();
    for (const e of tree.entries) {
      if (!e.is_dir) s.add(e.path);
    }
    return s;
  });

  /// (present, missing) tuple for a scope entry. Drive scopes
  /// report (0, 0) — they have no associated files. Single-file
  /// scopes report (0|1, 0|1).
  function fileAvailability(e: ScopeHistoryEntry): {
    present: number;
    missing: number;
  } {
    if (e.paths.length === 0) return { present: 0, missing: 0 };
    let present = 0;
    let missing = 0;
    for (const p of e.paths) {
      if (driveFilePaths.has(p)) present += 1;
      else missing += 1;
    }
    return { present, missing };
  }

  /// Inline peek toggle. Group scopes can't bind to the current
  /// window's assistant overlay (the context key is derived from
  /// visible files), so "Resume" toggles a read-only expansion
  /// here instead. Single-entry expand: opening another collapses
  /// the previous so the panel stays at a reasonable height. The
  /// `expandedId` / `expandedTurns` / `expandedLoading` fields
  /// live on `scopeHistoryOverlay` (not local state) so closing
  /// and reopening the overlay restores the previously-expanded
  /// bubble without refetching.
  async function togglePeek(entry: ScopeHistoryEntry): Promise<void> {
    if (scopeHistoryOverlay.expandedId === entry.id) {
      scopeHistoryOverlay.expandedId = null;
      scopeHistoryOverlay.expandedTurns = [];
      return;
    }
    scopeHistoryOverlay.expandedId = entry.id;
    scopeHistoryOverlay.expandedTurns = [];
    scopeHistoryOverlay.expandedLoading = true;
    try {
      scopeHistoryOverlay.expandedTurns = await fetchScopeHistoryTurns(entry);
    } finally {
      scopeHistoryOverlay.expandedLoading = false;
    }
  }

  async function onResume(entry: ScopeHistoryEntry): Promise<void> {
    const ok = await resumeScopeHistoryEntry(entry);
    if (!ok) {
      // Group scope (or file scope that couldn't bind for some
      // reason): show the read-only peek.
      await togglePeek(entry);
    }
  }

  function onOpenInNewWindow(entry: ScopeHistoryEntry): void {
    openScopeHistoryInNewWindow(entry);
  }

  async function onExport(entry: ScopeHistoryEntry): Promise<void> {
    try {
      const path = await exportScopeHistoryToDrive(entry);
      // Land a transient confirmation in the status line. Using
      // the overlay error slot for now (slice 6 didn't add a
      // toast / status surface); the prefix keeps it distinct.
      scopeHistoryOverlay.error = `exported to ${path}`;
      setTimeout(() => {
        if (scopeHistoryOverlay.error?.startsWith("exported to ")) {
          scopeHistoryOverlay.error = null;
        }
      }, 4000);
    } catch (e) {
      scopeHistoryOverlay.error = `export failed: ${(e as Error).message ?? String(e)}`;
    }
  }

  let menu: HamburgerMenu | undefined = $state();
  let menuOpen = $state(false);
  const POPOVER_WIDTH = 220;
  const POPOVER_HEIGHT = 60;

  function doToggleOverlayMaximized(): void {
    setOverlayMaximized(!overlayMaximized.on);
    menu?.close();
  }

  async function onClearAll(): Promise<void> {
    menu?.close();
    const total = scopeHistoryOverlay.entries.length;
    if (total === 0) return;
    const ok = await uiConfirm({
      title: "Clear all scope history?",
      message: `This drops all ${total} persisted thread${total === 1 ? "" : "s"} on disk and cannot be undone.`,
      confirmLabel: "Clear all",
      destructive: true,
    });
    if (!ok) return;
    try {
      await clearAllScopeHistory();
    } catch (e) {
      scopeHistoryOverlay.error = `clear failed: ${(e as Error).message ?? String(e)}`;
    }
  }

  async function onDelete(entry: ScopeHistoryEntry): Promise<void> {
    const ok = await uiConfirm({
      title: "Delete scope history?",
      message: `This drops the persisted thread for ${entry.title} and cannot be undone.`,
      confirmLabel: "Delete",
      destructive: true,
    });
    if (!ok) return;
    if (scopeHistoryOverlay.expandedId === entry.id) {
      scopeHistoryOverlay.expandedId = null;
      scopeHistoryOverlay.expandedTurns = [];
    }
    try {
      await deleteScopeHistoryEntry(entry);
    } catch (e) {
      scopeHistoryOverlay.error = `delete failed: ${(e as Error).message ?? String(e)}`;
    }
  }
</script>

<OverlayShell
  id="scope-history"
  open={scopeHistoryOverlay.open}
  onClose={closeScopeHistory}
>
  <div class="scope-history">
    <header>
      <button
        type="button"
        class="chrome-btn"
        onclick={doToggleOverlayMaximized}
        title={overlayMaximized.on ? "Restore size" : "Maximize"}
        aria-label={overlayMaximized.on ? "Restore size" : "Maximize"}
      >
        {#if overlayMaximized.on}
          <Minimize2 size={14} strokeWidth={1.75} aria-hidden="true" />
        {:else}
          <Maximize2 size={14} strokeWidth={1.75} aria-hidden="true" />
        {/if}
      </button>
      <span class="title">Scopes</span>
      <div class="filters">
        {#each ["file", "group", "drive"] as const as kind (kind)}
          <label class="chip" class:on={scopeHistoryOverlay.filters[kind]}>
            <input
              type="checkbox"
              bind:checked={scopeHistoryOverlay.filters[kind]}
            />
            <span class="dot" style="background:{KIND_COLOR[kind]}"></span>
            {kind}
            <span class="count">{counts[kind]}</span>
          </label>
        {/each}
      </div>
      <label class="sort-toggle">
        sort by
        <select
          class="sort-select"
          value={scopeHistoryOverlay.sortBy}
          onchange={(e) =>
            (scopeHistoryOverlay.sortBy = (e.currentTarget as HTMLSelectElement)
              .value as typeof scopeHistoryOverlay.sortBy)}
        >
          <option value="recent">recent activity</option>
          <option value="created">created</option>
          <option value="title">title</option>
          <option value="turns">turns</option>
        </select>
      </label>
      <span class="bar-menu">
        <HamburgerMenu
          bind:this={menu}
          bind:open={menuOpen}
          width={POPOVER_WIDTH}
          height={POPOVER_HEIGHT}
        >
          {@render menuItems()}
        </HamburgerMenu>
      </span>
      <button
        type="button"
        class="chrome-btn close"
        onclick={closeScopeHistory}
        title="Close"
        aria-label="Close"
      >
        <X size={14} strokeWidth={1.75} aria-hidden="true" />
      </button>
    </header>

    <ul class="rows">
      {#each visible as e (e.id)}
        {@const avail = fileAvailability(e)}
        {@const allMissing = e.paths.length > 0 && avail.present === 0}
        {@const canResume = e.kind === "drive" || (e.kind === "file" && !allMissing) || e.kind === "group"}
        {@const canOpenNew = !!e.url && (e.paths.length === 0 || !allMissing)}
        <li>
          <Bubble
            role={kindLabel(e.kind)}
            timestampLabel={e.last_touched
              ? `updated ${formatRelative(e.last_touched)}`
              : e.created_at
                ? `started ${formatRelative(e.created_at)}`
                : undefined}
          >
            <div class="row1">
              <span class="kind-pill" style="background:{KIND_COLOR[e.kind]}"
                >{e.kind}</span
              >
              <span class="title-text">{e.title}</span>
              <span class="meta"
                >{e.turn_count} turn{e.turn_count === 1 ? "" : "s"}</span
              >
            </div>
            {#if e.kind === "group" && e.paths.length > 1}
              <ul class="paths">
                {#each e.paths as p (p)}
                  <li class:missing={!driveFilePaths.has(p)}>
                    {p}
                    {#if !driveFilePaths.has(p)}<span class="path-warn">missing</span>{/if}
                  </li>
                {/each}
              </ul>
            {/if}
            {#if e.kind === "file" && allMissing}
              <div class="ts-line warn-line">file no longer on disk</div>
            {/if}
            {#if e.created_at}
              <div class="ts-line">
                created {formatRelative(e.created_at)}
                {#if e.last_touched && e.last_touched !== e.created_at}
                  · last activity {formatRelative(e.last_touched)}
                {/if}
                {#if !e.url}
                  · <span class="warn">no layout captured</span>
                {/if}
                {#if avail.missing > 0 && e.paths.length > 1}
                  · <span class="warn">{avail.missing} file{avail.missing === 1 ? "" : "s"} missing</span>
                {/if}
              </div>
            {/if}
            <div class="actions">
              {#if canResume}
                <button
                  class="act"
                  title={e.kind === "group"
                    ? "Read-only preview (group scope can't resume in place)"
                    : "Resume in current window"}
                  onclick={() => void onResume(e)}
                >
                  <Eye size={13} strokeWidth={1.75} aria-hidden="true" />
                  {e.kind === "group" ? "preview" : "resume"}
                </button>
              {/if}
              {#if canOpenNew}
                <button
                  class="act"
                  title="Open the saved pane / tab layout in a new window"
                  onclick={() => onOpenInNewWindow(e)}
                >
                  <ExternalLink size={13} strokeWidth={1.75} aria-hidden="true" />
                  new window
                </button>
              {/if}
              <button
                class="act"
                title="Export the transcript to a markdown file in the drive"
                onclick={() => void onExport(e)}
              >
                <FileDown size={13} strokeWidth={1.75} aria-hidden="true" />
                export
              </button>
              <button
                class="act danger"
                title="Delete this scope history"
                onclick={() => void onDelete(e)}
              >
                <Trash2 size={13} strokeWidth={1.75} aria-hidden="true" />
              </button>
            </div>
            {#if scopeHistoryOverlay.expandedId === e.id}
              <div class="peek">
                {#if scopeHistoryOverlay.expandedLoading}
                  <div class="peek-status">loading…</div>
                {:else if scopeHistoryOverlay.expandedTurns.length === 0}
                  <div class="peek-status muted">empty conversation</div>
                {:else}
                  <ul class="peek-turns">
                    {#each scopeHistoryOverlay.expandedTurns as t, i (i)}
                      {#if t.kind === "user"}
                        <li class="peek-turn user"><span class="who">you</span>{t.content}</li>
                      {:else if t.kind === "assistant"}
                        <li class="peek-turn assistant"><span class="who">agent</span>{t.content}</li>
                      {:else if t.kind === "tool"}
                        <li class="peek-turn tool"><span class="who">tool</span>{t.event.label}{#if t.event.result_summary} — {t.event.result_summary}{/if}</li>
                      {:else if t.kind === "assistant_switch"}
                        <li class="peek-turn tool"><span class="who">agent</span>changed to {t.backend}{#if t.model} — {t.model}{/if}</li>
                      {/if}
                    {/each}
                  </ul>
                {/if}
              </div>
            {/if}
          </Bubble>
        </li>
      {/each}
    </ul>

    <div class="status-line">
      {#if scopeHistoryOverlay.loading}
        <span>loading…</span>
      {:else if scopeHistoryOverlay.error}
        <span class="err">{scopeHistoryOverlay.error}</span>
      {:else if visible.length === 0}
        <span class="muted">no scope history yet</span>
      {:else}
        <span
          >{visible.length} scope{visible.length === 1 ? "" : "s"} ·
          {scopeHistoryOverlay.entries.length} total</span
        >
      {/if}
    </div>
  </div>
</OverlayShell>

{#snippet menuItems()}
  <li>
    <button
      role="menuitem"
      disabled={scopeHistoryOverlay.entries.length === 0}
      onclick={() => void onClearAll()}
    >
      <Trash2 size={16} strokeWidth={1.75} aria-hidden="true" />
      <span>Clear all history</span>
    </button>
  </li>
{/snippet}

<style>
  .scope-history {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
    min-height: 0;
  }

  header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.4rem 0.6rem;
    border-bottom: 1px solid var(--border);
    background: var(--bg-card);
    font-weight: 600;
    font-size: 15px;
    color: var(--text-heading);
    flex-shrink: 0;
    flex-wrap: wrap;
  }
  header .title {
    flex-shrink: 0;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    font-size: 13px;
    font-weight: 600;
    color: var(--text-secondary);
  }
  .filters {
    display: flex;
    gap: 0.35rem;
    align-items: center;
    flex-wrap: wrap;
  }
  .sort-toggle {
    margin-left: auto;
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
    font-size: 13px;
    color: var(--text-secondary);
    user-select: none;
  }
  .sort-select {
    background: var(--bg);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 2px 6px;
    font: inherit;
    font-size: 13px;
    cursor: pointer;
  }
  .sort-select:focus {
    outline: none;
    border-color: var(--link);
  }
  /* Hamburger sits right of the sort dropdown. The sort cluster
     already carries margin-left:auto so it floats right; this is
     just a tight gap to keep them visually paired. */
  .bar-menu {
    display: inline-flex;
    align-items: center;
  }
  /* Window-manager chrome: maximize/restore on the far left of the
     header, close on the far right. Matches the affordance used by
     every other overlay header. */
  .chrome-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 26px;
    height: 24px;
    padding: 0;
    background: var(--bg);
    color: var(--text-secondary);
    border: 1px solid var(--border);
    border-radius: 4px;
    cursor: pointer;
    transition: color 0.15s ease, border-color 0.15s ease;
    flex-shrink: 0;
  }
  .chrome-btn:hover {
    color: var(--text);
    border-color: var(--btn-hover);
  }

  /* Chip styling mirrors the graph overlay so the two surfaces
     share a vocabulary. Off-chips fade to btn-bg + secondary text;
     on-chips brighten to text + btn-hover. */
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    padding: 1px 6px;
    border: 1px solid var(--btn-border);
    border-radius: 12px;
    cursor: pointer;
    user-select: none;
    color: var(--text-secondary);
    background: var(--btn-bg);
    font-size: 13px;
    text-transform: uppercase;
    letter-spacing: 0.03em;
  }
  .chip.on {
    color: var(--text);
    border-color: var(--btn-hover);
  }
  .chip input {
    display: none;
  }
  .chip .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
  }
  .chip .count {
    font-variant-numeric: tabular-nums;
    opacity: 0.75;
  }

  .rows {
    list-style: none;
    margin: 0;
    padding: 8px 10px;
    overflow-y: auto;
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .rows > li {
    display: flex;
    align-self: stretch;
  }
  /* Match the search panel's bubble shape: stretch the body to
     fill the row width (Bubble itself shrinks the body to content
     so right-aligned chat bubbles stay tight; the list use case
     wants full-width cards). */
  .rows :global(.bubble) {
    max-width: none;
    width: 100%;
    align-items: stretch;
  }
  .rows :global(.bubble .body) {
    width: 100%;
    box-sizing: border-box;
  }

  .row1 {
    display: flex;
    gap: 6px;
    align-items: baseline;
    font-size: 14px;
    flex-wrap: wrap;
  }
  .kind-pill {
    display: inline-block;
    min-width: 48px;
    text-align: center;
    color: #fff;
    text-transform: uppercase;
    font-size: 10.5px;
    font-weight: 600;
    letter-spacing: 0.04em;
    padding: 1px 6px;
    border-radius: 3px;
    flex-shrink: 0;
  }
  .title-text {
    font-weight: 600;
    color: var(--text);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
    flex: 1;
  }
  .meta {
    color: var(--text-secondary);
    font-family: ui-monospace, monospace;
    font-size: 13px;
    margin-left: auto;
  }

  .paths {
    list-style: none;
    margin: 4px 0 0 0;
    padding: 0 0 0 12px;
    font-size: 13px;
    color: var(--text-secondary);
    border-left: 2px solid var(--border);
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .paths li {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .ts-line {
    margin-top: 4px;
    font-size: 12px;
    color: var(--text-secondary);
    opacity: 0.75;
    font-variant-numeric: tabular-nums;
  }
  .warn {
    color: #d80;
  }
  .warn-line {
    color: #d80;
    opacity: 0.9;
  }
  .paths li.missing {
    color: #d80;
  }
  .path-warn {
    margin-left: 0.4rem;
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    opacity: 0.8;
  }

  /* Action row sits at the bottom-right of each bubble, below
     the meta / timestamp lines so the eye finishes on the
     conversation summary and then lands on the buttons. The
     row stays right-aligned regardless of how many buttons the
     scope kind exposes (file / drive / group differ). */
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 4px;
    margin-top: 8px;
  }
  .act {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    padding: 1px 6px;
    height: 20px;
    background: transparent;
    color: var(--text-secondary);
    border: 1px solid var(--btn-border);
    border-radius: 3px;
    cursor: pointer;
    font: inherit;
    font-size: 11px;
    line-height: 1;
    text-transform: lowercase;
    transition: color 0.15s ease, border-color 0.15s ease, background 0.15s ease;
  }
  .act:hover {
    color: var(--text);
    border-color: var(--btn-hover);
  }
  .act.danger:hover {
    color: #d33;
    border-color: #d33;
  }

  /* Inline read-only peek. Group scopes can't bind to the
     assistant overlay in-place (the context key is derived from
     visible files), so we surface the saved turns here. Kept
     plain-text + bounded height so the panel stays scannable
     even on a long thread. */
  .peek {
    margin-top: 6px;
    border-top: 1px solid var(--border);
    padding-top: 6px;
    max-height: 40vh;
    overflow-y: auto;
  }
  .peek-status {
    font-size: 13px;
    color: var(--text-secondary);
    padding: 4px 0;
  }
  .peek-status.muted {
    opacity: 0.7;
  }
  .peek-turns {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .peek-turn {
    font-size: 13px;
    line-height: 1.45;
    white-space: pre-wrap;
    word-break: break-word;
  }
  .peek-turn .who {
    display: inline-block;
    margin-right: 6px;
    font-size: 10.5px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-secondary);
    opacity: 0.8;
    font-weight: 600;
  }
  .peek-turn.user .who { color: var(--link); opacity: 1; }
  .peek-turn.tool {
    color: var(--text-secondary);
    font-style: italic;
  }

  .status-line {
    padding: 4px 10px;
    font-size: 13px;
    color: var(--text-secondary);
    border-top: 1px solid var(--border);
  }
  .status-line .err {
    color: #d33;
  }
  .status-line .muted {
    opacity: 0.7;
  }
</style>
