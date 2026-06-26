<script lang="ts">
  // The library tree: the Local group first, then ONE group per registered
  // devserver (connected or not). A devserver IS its group header now -- the old
  // flat devserver list is gone. Each header carries the group-level actions:
  //   Local:     [home] Local            ... [new terminal]
  //   Devserver: [globe] {host, copies}  ... [new terminal] [settings] [on/off]
  // The devserver host label click-copies the hostname (lowercased). A connected
  // devserver's served workspaces nest under its header as rows identical to the
  // local ones -- [new window] + [on/off], plus a select checkbox feeding the one
  // global bulk bar (App-level SelectionBar). The two on/off controls keep their
  // distinct icons + semantics: a workspace toggles its tenant (Power); a
  // devserver connects/disconnects (Plug/Unplug). The read-only surface shows the
  // on-state statically with no actions.
  import {
    AppWindow,
    CircleAlert,
    Globe,
    House,
    LoaderCircle,
    Pencil,
    Plug,
    Power,
    SquareTerminal,
    Unplug,
  } from "lucide-svelte";
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
  import { isSelected, toggleSelected } from "../state/selection.svelte";
  import { isPending, servedKey, wsKey, dsKey } from "../state/pending.svelte";
  import { openEditDevserver } from "../state/dialog.svelte";
  import { basename } from "../lib/windowLabel";
  import { readOnly } from "../state/capabilities";
  import type { DevserverEntry, WorkspaceEntry } from "../api/library";

  function displayName(ws: WorkspaceEntry): string {
    return ws.label || basename(ws.path) || ws.path;
  }
  function devserverName(ds: DevserverEntry): string {
    return ds.label || `${ds.host}:${ds.port}`;
  }
  function endpoint(ds: DevserverEntry): string {
    return `${ds.host}:${ds.port}`;
  }

  const localWorkspaces = $derived(library.workspaces.filter((w) => w.devserver_id === null));
  // One group per registered devserver, connected or not, sorted by name. A
  // connected devserver's served workspaces nest under it (merged into the feed
  // by the library); a disconnected one shows only its header (with Connect).
  const devservers = $derived(
    [...library.devservers].sort((a, b) => devserverName(a).localeCompare(devserverName(b))),
  );
  function workspacesOf(devserverId: string): WorkspaceEntry[] {
    return library.workspaces.filter((w) => w.devserver_id === devserverId);
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
    void offWorkspaceWithConfirm((force) => setDevserverWorkspaceOn(devserverId, prefix, false, force));
  }

  // The pending key for a workspace row (local by workspace_id, served by
  // devserver_id + prefix), matching the action handlers.
  function rowKey(ws: WorkspaceEntry): string {
    return ws.devserver_id === null ? wsKey(ws.workspace_id) : servedKey(ws.devserver_id, ws.prefix);
  }

  // Workspace spinner = backend reports the mount in flight (`starting`) OR the
  // optimistic bridge is open between a click and the first refetch.
  function spinning(ws: WorkspaceEntry): boolean {
    return ws.status === "starting" || isPending(rowKey(ws));
  }

  const connected = (ds: DevserverEntry): boolean => ds.status === "connected";
  // Devserver spinner = backend reports the dial in flight (`connecting`) OR the
  // optimistic bridge is open. A dropped tunnel lands `disconnected` + clears it.
  const dsSpinning = (ds: DevserverEntry): boolean =>
    ds.status === "connecting" || isPending(dsKey(ds.id));

  // Select-all-local convenience for the Local group header checkbox: checked when
  // every local workspace is selected; toggling brings all to the same state.
  const allLocalSelected = $derived(
    localWorkspaces.length > 0 && localWorkspaces.every((w) => isSelected("workspace", w.workspace_id)),
  );
  function toggleAllLocal(): void {
    const target = !allLocalSelected;
    for (const w of localWorkspaces) {
      if (isSelected("workspace", w.workspace_id) !== target) {
        toggleSelected("workspace", w.workspace_id);
      }
    }
  }

  // Click the devserver host label to copy the hostname (lowercased) -- the handy
  // thing to paste into a shell. Best-effort: a surface without the async
  // clipboard API just no-ops.
  async function copyHost(ds: DevserverEntry): Promise<void> {
    try {
      await navigator.clipboard?.writeText(ds.host.toLowerCase());
    } catch {
      // Clipboard denied/unavailable: a non-fatal convenience, swallow it.
    }
  }
</script>

{#snippet workspaceRow(ws: WorkspaceEntry, kind: "workspace" | "served", devserverId: string | null)}
  <li
    class="row"
    class:selected={!readOnly &&
      (kind === "workspace"
        ? isSelected("workspace", ws.workspace_id)
        : isSelected("served", ws.prefix, devserverId ?? undefined))}>
    {#if !readOnly}
      <input
        class="row-check"
        type="checkbox"
        checked={kind === "workspace"
          ? isSelected("workspace", ws.workspace_id)
          : isSelected("served", ws.prefix, devserverId ?? undefined)}
        aria-label={`Select ${displayName(ws)}`}
        onchange={() =>
          kind === "workspace"
            ? toggleSelected("workspace", ws.workspace_id)
            : toggleSelected("served", ws.prefix, devserverId ?? undefined)} />
    {/if}
    <div class="row-main">
      <span class="row-name">
        {displayName(ws)}
        {#if ws.status === "error"}
          <span class="row-error" title={ws.error ?? "Mount failed"}>
            <CircleAlert size={14} />
          </span>
        {/if}
      </span>
      <span class="row-sub" title={ws.path}>{ws.path}</span>
    </div>
    <div class="row-actions">
      {#if readOnly}
        <span class="pill" class:on={ws.on} aria-disabled="true">{ws.on ? "On" : "Off"}</span>
      {:else}
        <button
          class="icon-btn"
          type="button"
          disabled={!ws.on}
          title={ws.on ? "New window" : "Turn on to open a window"}
          aria-label={`New window of ${displayName(ws)}`}
          onclick={() =>
            run(
              kind === "workspace"
                ? openWorkspaceWindow(ws.path)
                : openDevserverWorkspace(devserverId!, ws.path),
            )}>
          <AppWindow size={16} />
        </button>
        <button
          class="icon-btn"
          class:on={ws.on}
          type="button"
          disabled={spinning(ws)}
          title={spinning(ws) ? "Working…" : ws.on ? "Turn off" : "Turn on"}
          aria-label={spinning(ws)
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
  </li>
{/snippet}

<!-- Local group: a home-iconed header (new-terminal action) over the local rows. -->
<section class="group">
  <div class="group-header">
    {#if !readOnly}
      <input
        class="row-check"
        type="checkbox"
        checked={allLocalSelected}
        disabled={localWorkspaces.length === 0}
        aria-label="Select all local workspaces"
        onchange={toggleAllLocal} />
    {/if}
    <span class="group-icon" aria-hidden="true"><House size={15} /></span>
    <h2 class="group-name">Local</h2>
    {#if !readOnly}
      <div class="group-actions">
        <button
          class="icon-btn"
          type="button"
          title="New terminal"
          aria-label="New local terminal"
          onclick={() => run(openTerminal())}>
          <SquareTerminal size={16} />
        </button>
      </div>
    {/if}
  </div>
  {#if localWorkspaces.length}
    <ul class="rows">
      {#each localWorkspaces as ws (ws.workspace_id)}
        {@render workspaceRow(ws, "workspace", null)}
      {/each}
    </ul>
  {/if}
</section>

<!-- One group per registered devserver: a globe-iconed header (host click-copy,
     new-terminal, settings, connect/disconnect) over its served workspaces. -->
{#each devservers as ds (ds.id)}
  <section class="group">
    <div class="group-header">
      {#if !readOnly}
        <input
          class="row-check"
          type="checkbox"
          checked={isSelected("devserver", ds.id)}
          aria-label={`Select ${devserverName(ds)}`}
          onchange={() => toggleSelected("devserver", ds.id)} />
      {/if}
      <span class="group-icon" aria-hidden="true"><Globe size={15} /></span>
      <div class="row-main">
        <span class="row-name">
          <button
            class="host-copy"
            type="button"
            title="Copy host"
            aria-label={`Copy host ${ds.host}`}
            onclick={() => copyHost(ds)}>
            {devserverName(ds)}
          </button>
          {#if connected(ds)}<span class="dot live" title="Connected"></span>{/if}
          {#if ds.has_token}<span class="chip" title="A connect token is stored">🔒 token</span>{/if}
        </span>
        <span class="row-sub" title={endpoint(ds)}>{endpoint(ds)}</span>
      </div>
      <div class="group-actions">
        {#if !readOnly}
          <button
            class="icon-btn"
            type="button"
            disabled={!connected(ds)}
            title={connected(ds) ? "New terminal" : "Connect to open a terminal"}
            aria-label={`New terminal on ${devserverName(ds)}`}
            onclick={() => run(openDevserverTerminal(ds.id))}>
            <SquareTerminal size={16} />
          </button>
        {/if}
        <button
          class="icon-btn"
          type="button"
          title={ds.status === "disconnected" ? "Settings" : "Settings (read-only while connected)"}
          aria-label={`Settings for ${devserverName(ds)}`}
          onclick={() => openEditDevserver(ds)}>
          <Pencil size={16} />
        </button>
        {#if !readOnly}
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
              class="icon-btn on"
              type="button"
              title="Disconnect"
              aria-label={`Disconnect ${devserverName(ds)}`}
              onclick={() => run(disconnectDevserver(ds.id))}>
              <Unplug size={16} />
            </button>
          {:else}
            <button
              class="icon-btn"
              type="button"
              title="Connect"
              aria-label={`Connect ${devserverName(ds)}`}
              onclick={() => run(connectDevserver(ds.id))}>
              <Plug size={16} />
            </button>
          {/if}
        {/if}
      </div>
    </div>
    {#if workspacesOf(ds.id).length}
      <ul class="rows">
        {#each workspacesOf(ds.id) as ws (ws.workspace_id)}
          {@render workspaceRow(ws, "served", ds.id)}
        {/each}
      </ul>
    {/if}
  </section>
{/each}

<style>
  /* The group header is an interactive row: the select check + icon + name on the
     left, the group-level actions on the right. Sits above the .rows list (shared
     global styles). */
  .group-header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin: 0 0 0.6rem;
  }

  .group-icon {
    display: inline-flex;
    align-items: center;
    color: var(--text-secondary);
  }

  .group-name {
    margin: 0;
    font-size: 0.8rem;
    font-weight: 600;
    letter-spacing: 0.03em;
    text-transform: uppercase;
    color: var(--text-secondary);
  }

  /* The devserver host label is a button (click-to-copy) but reads as the row
     name -- strip the button chrome and inherit the surrounding text type. */
  button.host-copy {
    border: none;
    background: transparent;
    padding: 0;
    cursor: pointer;
    font: inherit;
    color: inherit;
  }

  button.host-copy:hover {
    color: var(--text);
    text-decoration: underline;
  }

  /* Push the group actions to the far right. */
  .group-actions {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    margin-left: auto;
  }
</style>
