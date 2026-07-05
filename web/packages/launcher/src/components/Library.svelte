<script lang="ts">
  // The machine-first library tree: LOCAL plus each registered devserver as an
  // equal top-level machine, each owning its windows. A machine block is a header
  // (its machine-level actions) over the connected/local content: a TERMINALS
  // section (the control terminal pinned first, then standalone terminals) and a
  // WORKSPACES section of collapsible cards whose windows nest inside on expand.
  // A disconnected devserver shows only its header + a connect prompt. The
  // grouping is the pure lib/machineTree; this component is the presentation +
  // the per-row actions. The read-only surface (devserver/gateway) shows the
  // on-state statically with no mutation controls, but keeps the [edit config]
  // view and can still expand a card to read its windows.
  import {
    AppWindow,
    ChevronRight,
    CircleAlert,
    FolderPlus,
    Globe,
    House,
    LoaderCircle,
    Pencil,
    Plug,
    Plus,
    Power,
    SquareTerminal,
    Unplug,
  } from "lucide-svelte";
  import WindowRow from "./WindowRow.svelte";
  import OsIcon, { hasOsIcon } from "./OsIcon.svelte";
  import {
    library,
    toggleWorkspace,
    openWorkspaceWindow,
    openDevserverWorkspace,
    setDevserverWorkspaceOn,
    connectDevserver,
    disconnectDevserver,
    openDevserverTerminal,
    openTerminal,
    reportError,
    clearError,
  } from "../state/library.svelte";
  import { liveTerminalsCount } from "../api/library";
  import { requestConfirm } from "../state/confirm.svelte";
  import { checksVisible, isSelected, toggleSelected } from "../state/selection.svelte";
  import { isPending, servedKey, wsKey, dsKey } from "../state/pending.svelte";
  import { hasControlAttention } from "../state/controlAttention.svelte";
  import { openEditDevserver, openNewDialog } from "../state/dialog.svelte";
  import { basename } from "../lib/windowLabel";
  import { buildMachineTree, type MachineNode, type WorkspaceNode } from "../lib/machineTree";
  import { readOnly, hasDesktopBridge, selfManagedWindows, hostOs } from "../state/capabilities";
  import { canActOnTenant, ownsTenantLeader, tenantLeader } from "../state/leadership.svelte";
  import { mintWindow } from "../state/windowManager.svelte";
  import { demoState, resetDemo } from "../state/demo.svelte";
  import type { DevserverEntry, WorkspaceEntry } from "../api/library";

  // The whole tree, recomputed when any of the three feeds change (the two-array
  // interleave: workspaces + windows + devservers all drive it).
  const tree = $derived(buildMachineTree(library.devservers, library.workspaces, library.windows));

  function displayName(ws: WorkspaceEntry): string {
    return ws.label || basename(ws.path) || ws.path;
  }
  function devserverName(ds: DevserverEntry): string {
    return ds.label || `${ds.host}:${ds.port}`;
  }
  function endpoint(ds: DevserverEntry): string {
    return `${ds.host}:${ds.port}`;
  }

  // Per-workspace card expand state, keyed by the stable workspace_id so it
  // survives the live watch-push re-renders (never reset on a poll). Default
  // collapsed; the count badge shows how many windows are nested.
  const expanded = $state<Record<string, boolean>>({});
  function isExpanded(id: string): boolean {
    return expanded[id] === true;
  }
  function toggleExpand(id: string): void {
    expanded[id] = !expanded[id];
  }

  // Per-row / per-action failures surface in the banner (the actions throw so the
  // bulk loop can count failures; the per-row caller catches here).
  async function run(action: Promise<void>): Promise<void> {
    clearError();
    try {
      await action;
    } catch (e) {
      reportError(e);
    }
  }

  // The acting leader claim for minting onto a tenant: the leader window_id when
  // this launcher owns it, else undefined (leaderless allows; the server rejects
  // a stale claim against a live leader).
  function actingFor(prefix: string): string | undefined {
    return ownsTenantLeader(prefix) ? (tenantLeader(prefix) ?? undefined) : undefined;
  }

  // New window on a workspace. A self-managed surface browser-mints it (the
  // window manager opens it in-app, synchronously in this click gesture so the
  // browser does not block the popup); the desktop surface keeps the native mint
  // (the watcher opens it) and the served card keeps its devserver bridge route.
  function newWorkspaceWindow(
    ws: WorkspaceEntry,
    kind: "workspace" | "served",
    devserverId: string | null,
  ): Promise<void> {
    if (kind === "served") return openDevserverWorkspace(devserverId!, ws.path);
    if (selfManagedWindows) {
      return mintWindow("workspace", {
        workspacePath: ws.path,
        actingWindowId: actingFor(ws.prefix),
      }).then(() => {});
    }
    return openWorkspaceWindow(ws.path);
  }

  // New local terminal: browser-mint on a self-managed surface, else native mint.
  function newLocalTerminal(): Promise<void> {
    if (selfManagedWindows) return mintWindow("terminal").then(() => {});
    return openTerminal();
  }

  // Turning a workspace OFF can hit live terminal sessions: the server answers
  // 409 live_terminals with the count, so on that specific error we open the
  // in-SPA confirm (never a native dialog) and, on confirm, retry the SAME off
  // forced. Any other error goes to the banner. Shared by local + devserver off.
  async function offWorkspaceWithConfirm(off: (force: boolean) => Promise<void>): Promise<void> {
    clearError();
    try {
      await off(false);
    } catch (e) {
      const n = liveTerminalsCount(e);
      if (n === null) {
        reportError(e);
        return;
      }
      requestConfirm({
        title: "Turn off workspace?",
        message: `${n} live terminal session${n === 1 ? "" : "s"} ${n === 1 ? "is" : "are"} still running. Turn off anyway?`,
        confirmLabel: "Turn off",
        onConfirm: () => run(off(true)),
      });
    }
  }

  function toggleLocalWorkspace(id: string, on: boolean): void {
    if (on) {
      void run(toggleWorkspace(id, true));
      return;
    }
    void offWorkspaceWithConfirm((force) => toggleWorkspace(id, false, force));
  }

  function toggleRemoteWorkspace(devserverId: string, prefix: string, on: boolean): void {
    if (on) {
      void run(setDevserverWorkspaceOn(devserverId, prefix, true));
      return;
    }
    void offWorkspaceWithConfirm((force) =>
      setDevserverWorkspaceOn(devserverId, prefix, false, force),
    );
  }

  // The pending key for a workspace row (local by workspace_id, served by
  // devserver_id + prefix), matching the action handlers.
  function rowKey(ws: WorkspaceEntry): string {
    return ws.devserver_id === null ? wsKey(ws.workspace_id) : servedKey(ws.devserver_id, ws.prefix);
  }

  // Workspace spinner = backend reports a lifecycle transition OR the optimistic
  // bridge is open between a click and the first refetch.
  function spinning(ws: WorkspaceEntry): boolean {
    return (
      ws.status === "starting" ||
      ws.status === "closing" ||
      ws.status === "removing" ||
      isPending(rowKey(ws))
    );
  }

  function locked(ws: WorkspaceEntry): boolean {
    return ws.status === "locked";
  }

  const connected = (ds: DevserverEntry): boolean => ds.status === "connected";
  // The devserver's connection dropped out from under it: the control script
  // exited, or a connected transport stopped answering. The identity row's
  // status dot turns RED (same dot, same spot as the green connected one)
  // in place of a textual "connection lost" cue; the control row's eye flash
  // rides the same attention state. Both clear on recovery, reconnect, the
  // user acting on the control row, or the row leaving the feed.
  const connectionLost = (ds: DevserverEntry): boolean =>
    ds.library_id !== null && ds.library_id !== undefined && hasControlAttention(ds.library_id);
  // Devserver spinner = backend reports the dial in flight (`connecting`) OR the
  // optimistic bridge is open. A dropped tunnel lands `disconnected` + clears it.
  const dsSpinning = (ds: DevserverEntry): boolean =>
    ds.status === "connecting" || isPending(dsKey(ds.id));

  // A machine shows its content block while connected, while a fresh dial is
  // `connecting`, or while it still owns a control terminal. The last clause keeps
  // a disconnected devserver's DEAD "process exited" control row mounted (the
  // script died, connection torn down, control terminal kept alive) so it can
  // flash for attention and the user can open it to read the death reason; the row
  // clears once the control terminal is closed and its record leaves the feed.
  function hasContent(node: MachineNode): boolean {
    return (
      node.kind === "local" ||
      (node.devserver !== null &&
        (connected(node.devserver) ||
          node.devserver.status === "connecting" ||
          node.control.length > 0))
    );
  }

  function machineIsEmpty(node: MachineNode): boolean {
    return (
      node.control.length === 0 &&
      node.terminals.length === 0 &&
      node.workspaces.length === 0 &&
      node.looseWindows.length === 0
    );
  }
</script>

{#snippet workspaceCard(node: WorkspaceNode, kind: "workspace" | "served", devserverId: string | null)}
  {@const ws = node.ws}
  {@const hasWindows = node.count > 0}
  {@const checked =
    kind === "workspace"
      ? isSelected("workspace", ws.workspace_id)
      : isSelected("served", ws.prefix, devserverId ?? undefined)}
  <div class="ws-card">
    <div class="ws-head" class:selected={!readOnly && checked}>
      {#if !readOnly && checksVisible()}
        <input
          class="row-check"
          type="checkbox"
          {checked}
          aria-label={`Select ${displayName(ws)}`}
          onchange={() =>
            kind === "workspace"
              ? toggleSelected("workspace", ws.workspace_id)
              : toggleSelected("served", ws.prefix, devserverId ?? undefined)} />
      {/if}
      {#if hasWindows}
        <button
          class="chevron"
          class:expanded={isExpanded(ws.workspace_id)}
          type="button"
          aria-label={`${isExpanded(ws.workspace_id) ? "Collapse" : "Expand"} ${displayName(ws)}`}
          aria-expanded={isExpanded(ws.workspace_id)}
          onclick={() => toggleExpand(ws.workspace_id)}>
          <ChevronRight size={15} />
        </button>
      {:else}
        <span class="chevron-spacer" aria-hidden="true"></span>
      {/if}
      <div class="row-main">
        <span class="row-name">
          {displayName(ws)}
          {#if ws.status === "error"}
            <span class="row-error" title={ws.error ?? ""}>
              <CircleAlert size={14} />
            </span>
          {/if}
        </span>
        <span class="row-sub" title={ws.path}>{ws.path}</span>
      </div>
      {#if hasWindows}
        <button
          class="count-badge"
          type="button"
          title={`${node.count} window${node.count === 1 ? "" : "s"}`}
          aria-label={`${isExpanded(ws.workspace_id) ? "Collapse" : "Expand"} ${displayName(ws)} windows`}
          aria-expanded={isExpanded(ws.workspace_id)}
          onclick={() => toggleExpand(ws.workspace_id)}>
          <AppWindow size={12} />
          {node.count}
        </button>
      {/if}
      {#if readOnly}
        <span class="pill" class:on={ws.on} aria-disabled="true">
          {locked(ws) ? "Locked" : ws.on ? "On" : "Off"}
        </span>
      {:else}
        <button
          class="icon-btn"
          type="button"
          disabled={ws.status !== "running" ||
            spinning(ws) ||
            (selfManagedWindows && !canActOnTenant(ws.prefix))}
          title={selfManagedWindows && !canActOnTenant(ws.prefix)
            ? "The session leader manages this workspace's windows"
            : `New window of ${displayName(ws)}`}
          aria-label={`New window of ${displayName(ws)}`}
          onclick={() => run(newWorkspaceWindow(ws, kind, devserverId))}>
          <AppWindow size={16} />
        </button>
        <button
          class="icon-btn"
          class:on={ws.on}
          type="button"
          disabled={spinning(ws) || locked(ws)}
          title={locked(ws)
            ? "Workspace is open in another Chan process"
            : spinning(ws)
              ? "Working…"
              : ws.on
                ? "Turn off"
                : "Turn on"}
          aria-label={locked(ws)
            ? `${displayName(ws)} is open in another Chan process`
            : spinning(ws)
              ? `Working on ${displayName(ws)}`
              : `${ws.on ? "Turn off" : "Turn on"} ${displayName(ws)}`}
          onclick={() =>
            kind === "workspace"
              ? toggleLocalWorkspace(ws.workspace_id, !ws.on)
              : toggleRemoteWorkspace(devserverId!, ws.prefix, !ws.on)}>
          {#if spinning(ws)}
            <LoaderCircle class="spin" size={16} />
          {:else}
            <Power size={16} />
          {/if}
        </button>
      {/if}
    </div>
    {#if hasWindows && isExpanded(ws.workspace_id)}
      <div class="ws-windows">
        {#each node.windows as w (w.window_id)}
          <WindowRow {w} icon />
        {/each}
      </div>
    {/if}
  </div>
{/snippet}

{#snippet machineContent(node: MachineNode)}
  {@const kind = node.kind === "local" ? "workspace" : "served"}
  {@const devserverId = node.devserver?.id ?? null}
  <div class="machine-content">
    {#if node.control.length || node.terminals.length}
      <div class="section-label">Terminals</div>
      <div class="term-list">
        {#each node.control as w (w.window_id)}
          <WindowRow {w} icon />
        {/each}
        {#each node.terminals as w (w.window_id)}
          <WindowRow {w} icon />
        {/each}
      </div>
    {/if}
    {#if node.workspaces.length || node.looseWindows.length}
      <div class="section-label">Workspaces</div>
      {#each node.workspaces as wsNode (wsNode.ws.workspace_id)}
        {@render workspaceCard(wsNode, kind, devserverId)}
      {/each}
      {#if node.looseWindows.length}
        <div class="term-list">
          {#each node.looseWindows as w (w.window_id)}
            <WindowRow {w} icon />
          {/each}
        </div>
      {/if}
    {/if}
    {#if machineIsEmpty(node) && node.kind === "local"}
      {#if readOnly}
        <p class="empty-hint">
          No workspaces yet. Manage workspaces from the desktop app or the chan CLI.
        </p>
      {:else}
        <p class="empty-hint">
          No workspaces yet. Add one with the buttons above, or run
          <code>chan open /path/to/project</code> in a terminal.
        </p>
      {/if}
    {/if}
  </div>
{/snippet}

<!-- The devserver identity block: a left glyph column beside two text rows:
     name + status/token, then the address.
     On the mutable surface the whole block is the click target to open the
     edit-config form, with an inline pencil beside the address; the read-only
     surface renders it static with no edit affordance. -->
{#snippet dsIdentity(ds: DevserverEntry, withPencil: boolean)}
  <span class="ds-name-row">
    <span class="ds-glyph">
      {#if hasOsIcon(ds.os)}
        <OsIcon os={ds.os} prettyName={ds.pretty_name} size={16} />
      {:else}
        <Globe size={16} />
      {/if}
    </span>
    <span class="row-name">{devserverName(ds)}</span>
    {#if connectionLost(ds)}<span class="status-dot lost" title="Connection lost"></span>
    {:else if connected(ds)}<span class="status-dot live" title="Connected"></span>{/if}
    {#if ds.has_token}<span class="chip">🔒 token</span>{/if}
  </span>
  <span class="ds-addr-row">
    <span class="ds-glyph"></span>
    <span class="row-sub" title={endpoint(ds)}>{endpoint(ds)}</span>
    {#if withPencil}<Pencil size={12} class="addr-pencil" />{/if}
  </span>
{/snippet}

{#each tree.machines as node (node.kind === "local" ? "local" : node.devserver!.id)}
  <section class="machine">
    {#if node.kind === "local"}
      <div class="machine-header">
        <span class="machine-icon">
          {#if hasOsIcon(hostOs)}
            <OsIcon os={hostOs} size={16} />
          {:else}
            <House size={16} />
          {/if}
        </span>
        <span class="machine-name">Local machine</span>
        <span class="status-dot live" title="This machine"></span>
        <div class="machine-actions">
          {#if !readOnly}
            <button
              class="icon-btn"
              type="button"
              title="New terminal"
              aria-label="New local terminal"
              onclick={() => run(newLocalTerminal())}>
              <SquareTerminal size={16} />
            </button>
            <button
              class="icon-btn"
              type="button"
              title={demoState.enabled ? "Reset demo data" : "New workspace"}
              aria-label={demoState.enabled ? "Reset demo data" : "New local workspace"}
              onclick={() => (demoState.enabled ? run(resetDemo()) : openNewDialog("local"))}>
              <FolderPlus size={16} />
            </button>
          {/if}
        </div>
      </div>
      {@render machineContent(node)}
    {:else if node.devserver}
      {@const ds = node.devserver}
      <div class="machine-header">
        {#if !readOnly && checksVisible()}
          <input
            class="row-check"
            type="checkbox"
            checked={isSelected("devserver", ds.id)}
            aria-label={`Select ${devserverName(ds)}`}
            onchange={() => toggleSelected("devserver", ds.id)} />
        {/if}
        {#if readOnly}
          <div class="ds-id">{@render dsIdentity(ds, false)}</div>
        {:else}
          <button
            class="ds-id editable"
            type="button"
            title="Edit config"
            aria-label={`Edit config for ${devserverName(ds)}`}
            onclick={() => openEditDevserver(ds)}>
            {@render dsIdentity(ds, true)}
          </button>
        {/if}
        <div class="machine-actions">
          {#if hasDesktopBridge && connected(ds)}
            <button
              class="icon-btn"
              type="button"
              title="New terminal"
              aria-label={`New terminal on ${devserverName(ds)}`}
              onclick={() => run(openDevserverTerminal(ds.id))}>
              <SquareTerminal size={16} />
            </button>
          {/if}
          {#if hasDesktopBridge}
            {#if dsSpinning(ds)}
              <button
                class="icon-btn"
                class:on={connected(ds)}
                type="button"
                disabled
                title="Working…"
                aria-label={`Working on ${devserverName(ds)}`}>
                <LoaderCircle class="spin" size={16} />
              </button>
            {:else if connected(ds)}
              <button
                class="icon-btn"
                class:on={!connectionLost(ds)}
                class:lost={connectionLost(ds)}
                type="button"
                title={connectionLost(ds) ? "Disconnect lost connection" : "Disconnect"}
                aria-label={`Disconnect ${devserverName(ds)}`}
                onclick={() => run(disconnectDevserver(ds.id))}>
                <Unplug size={16} />
              </button>
            {:else}
              {@const controlOpen = node.control.length > 0}
              <!-- A disconnected devserver that still owns a control terminal (its
                   script died, sitting at "process exited") is not ready to
                   reconnect: the connect op errors until the control terminal is
                   closed. Gate the button and point the user at the control row
                   (open it to read the reason, close it) or the overlay Reconnect. -->
              <button
                class="icon-btn"
                type="button"
                disabled={controlOpen}
                title={controlOpen ? "Close the control terminal to reconnect" : "Connect"}
                aria-label={controlOpen
                  ? `Close the control terminal to reconnect ${devserverName(ds)}`
                  : `Connect ${devserverName(ds)}`}
                onclick={() => run(connectDevserver(ds.id))}>
                <Plug size={16} />
              </button>
            {/if}
          {/if}
        </div>
      </div>
      {#if hasContent(node)}
        {@render machineContent(node)}
      {:else if dsSpinning(ds)}
        <p class="connect-prompt">Connecting…</p>
      {:else}
        <p class="connect-prompt">Not connected.</p>
      {/if}
    {/if}
  </section>
{/each}

<!-- Fallback for windows whose library_id matches no machine yet (an unsynced
     control terminal minted at first connect, before the registry join lands).
     It empties on the next watch push once the devserver's library id resolves. -->
{#if tree.orphans.length}
  <section class="machine">
    <div class="section-label">Connecting…</div>
    <div class="term-list">
      {#each tree.orphans as w (w.window_id)}
        <WindowRow {w} icon />
      {/each}
    </div>
  </section>
{/if}

<!-- The decoupled add-devserver entry point: a full-width dashed button below
     the machine list (not a top-bar [+]). A devserver is dialed through the
     desktop bridge, so this is offered only where that bridge exists. -->
{#if hasDesktopBridge}
  <button class="add-devserver" type="button" onclick={() => openNewDialog("devserver")}>
    <Plus size={16} />
    Add dev server
  </button>
{/if}

<style>
  /* Each machine (LOCAL or a devserver) is a contained card whose background
     hosts the whole set: the header identity plus its terminal + workspace
     lists. The header is the top row; the lists sit below it as an indented peer
     list on the same surface. The whole badge wobbles on hover. */
  .machine {
    position: relative;
    margin-bottom: 0.8rem;
    padding: 0.3rem 0.5rem 0.7rem;
    border: 1px solid var(--border);
    border-radius: 14px;
    background: var(--bg-card);
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.28);
    transform-origin: center;
    transition:
      transform 240ms cubic-bezier(0.34, 1.56, 0.64, 1),
      box-shadow 160ms ease;
  }

  /* The whole machine badge wobbles as one, including while the pointer is on
     nested controls. Buttons keep their own hover highlights without scaling. */
  .machine:hover {
    transform: scale(1.015);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.24);
    z-index: 1;
  }

  /* The machine header: icon + name + status on the left, machine-level actions
     pushed to the far right. The top row of the card. */
  .machine-header {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    padding: 0.4rem 0.2rem;
  }

  .machine-icon {
    display: inline-flex;
    align-items: center;
    color: var(--text-secondary);
    flex-shrink: 0;
  }

  .machine-name {
    font-size: 0.95rem;
    font-weight: 600;
    color: var(--text);
  }

  /* A small accent dot: this machine is present / the devserver is connected. */
  .status-dot {
    width: 0.45rem;
    height: 0.45rem;
    border-radius: 50%;
    background: var(--text-secondary);
    opacity: 0.4;
    flex-shrink: 0;
  }

  .status-dot.live {
    background: var(--accent);
    opacity: 1;
    box-shadow: 0 0 6px color-mix(in srgb, var(--accent) 70%, transparent);
  }

  /* Same dot, connection lost: the script died or the transport stopped
     answering. Steady red, no flashing. */
  .status-dot.lost {
    background: var(--danger);
    opacity: 1;
    box-shadow: 0 0 6px color-mix(in srgb, var(--danger) 70%, transparent);
  }

  /* The devserver identity block (name row over address row). On the mutable
     surface it is a button: the whole block opens the edit-config form, lifting
     a hover tint; the read-only surface renders it as a static div. */
  .ds-id {
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
    min-width: 0;
    border-radius: 8px;
    padding: 0.2rem 0.45rem;
    margin: -0.2rem -0.45rem;
  }

  button.ds-id {
    border: none;
    background: transparent;
    text-align: left;
    font: inherit;
    color: inherit;
    cursor: pointer;
  }

  button.ds-id.editable:hover {
    background: color-mix(in srgb, var(--text-secondary) 8%, transparent);
  }

  /* Leading icon cell shared by both identity rows: the kind glyph (Globe) on the
     name row, the OS mark directly under it on the address row, so the two marks
     read as one left column. Fixed width keeps them aligned. */
  .ds-glyph {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 1.15rem;
    flex-shrink: 0;
    color: var(--text-secondary);
  }

  .ds-name-row {
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }

  .ds-addr-row {
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }

  /* The inline edit pencil beside the address (mutable surface only). */
  .ds-id :global(.addr-pencil) {
    flex-shrink: 0;
    color: var(--text-secondary);
    transition: color 160ms ease;
  }

  button.ds-id.editable:hover :global(.addr-pencil) {
    color: var(--brand);
  }

  .machine-actions {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    margin-left: auto;
    flex-shrink: 0;
  }

  /* The connected/local content (terminals + workspaces), tucked below the
     header on the same card surface with no left rule. */
  .machine-content {
    margin: 0.4rem 0.15rem 0 0.25rem;
  }

  .section-label {
    margin: 0.6rem 0 0.35rem 0.1rem;
    font-size: 0.66rem;
    font-weight: 600;
    letter-spacing: 0.09em;
    text-transform: uppercase;
    color: var(--text-secondary);
  }

  /* Flat (borderless) row lists for control + standalone terminals + loose
     windows: the rows read against the page, not boxed like the registry list. */
  .term-list :global(.row) {
    background: transparent;
    border-radius: 8px;
    padding: 0.45rem 0.5rem;
  }

  .term-list :global(.row:hover) {
    background: color-mix(in srgb, var(--text-secondary) 8%, transparent);
  }

  .connect-prompt {
    margin: 0.35rem 0 0 0.5rem;
    padding: 0.5rem 0.75rem;
    font-size: 0.82rem;
    color: var(--text-secondary);
  }

  .empty-hint {
    margin: 0.35rem 0 0;
    font-size: 0.85rem;
    line-height: 1.5;
    color: var(--text-secondary);
  }

  .empty-hint code {
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 0.85em;
    padding: 0.1em 0.35em;
    border-radius: 4px;
    background: color-mix(in srgb, var(--text-secondary) 16%, transparent);
    color: var(--text);
    white-space: nowrap;
  }

  /* A workspace card: a rounded panel whose header collapses/expands its nested
     windows. Workspace cards stay visually stable; the controls inside them keep
     only their hover highlights. */
  .ws-card {
    position: relative;
    margin-bottom: 0.4rem;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--bg-elev);
    overflow: hidden;
  }

  /* Reduced-motion: keep colour/background hover cues, drop the wobble. */
  @media (prefers-reduced-motion: reduce) {
    .machine {
      transition: box-shadow 160ms ease;
    }
    .machine:hover {
      transform: none;
    }
  }

  .ws-head {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.55rem 0.65rem;
  }

  .ws-head.selected {
    background: color-mix(in srgb, var(--brand) 10%, var(--bg-elev));
  }

  /* The expand chevron rotates from ► (collapsed) to ▼ (expanded). */
  .chevron {
    --chevron-rotation: 0deg;

    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 1.25rem;
    height: 1.25rem;
    padding: 0;
    border: none;
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
    flex-shrink: 0;
    transform: rotate(var(--chevron-rotation));
    transform-origin: center;
    transition:
      transform 120ms ease,
      color 160ms ease;
  }

  .chevron.expanded {
    --chevron-rotation: 90deg;
  }

  .chevron:hover {
    color: var(--text);
  }

  .chevron-spacer {
    width: 1.25rem;
    flex-shrink: 0;
  }

  /* The window-count badge on a card with nested windows. */
  .count-badge {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.1rem 0.4rem;
    border: 1px solid transparent;
    border-radius: 6px;
    background: color-mix(in srgb, var(--text-secondary) 12%, transparent);
    color: var(--text-secondary);
    font-size: 0.7rem;
    font-weight: 600;
    flex-shrink: 0;
    cursor: pointer;
    transition:
      background 160ms ease,
      color 160ms ease;
  }

  .count-badge:hover {
    background: color-mix(in srgb, var(--text-secondary) 18%, transparent);
    color: var(--text);
  }

  /* The nested windows panel (darker, inside the card). */
  .ws-windows {
    border-top: 1px solid var(--border);
    background: color-mix(in srgb, #000 14%, var(--bg-elev));
    padding: 0.25rem;
  }

  .ws-windows :global(.row) {
    background: transparent;
    border-radius: 8px;
    padding: 0.4rem 0.5rem;
  }

  .ws-windows :global(.row:hover) {
    background: color-mix(in srgb, var(--text-secondary) 8%, transparent);
  }

  /* The decoupled add-devserver entry: a full-width dashed button under the
     machine list, brightening to the accent on hover. */
  .add-devserver {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    width: 100%;
    margin-top: 1.5rem;
    padding: 0.75rem;
    border: 1px dashed var(--btn-border);
    border-radius: 11px;
    background: transparent;
    color: var(--text-secondary);
    font-size: 0.9rem;
    font-weight: 500;
    cursor: pointer;
    transition:
      border-color 160ms ease,
      background 160ms ease,
      color 160ms ease;
  }

  .add-devserver:hover {
    border-color: color-mix(in srgb, var(--accent) 45%, var(--btn-border));
    color: var(--text);
    background: color-mix(in srgb, var(--text-secondary) 6%, transparent);
  }

  @media (prefers-reduced-motion: reduce) {
    .count-badge,
    .add-devserver {
      transition:
        border-color 160ms ease,
        background 160ms ease,
        color 160ms ease;
    }
  }
</style>
