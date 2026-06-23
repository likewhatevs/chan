<script lang="ts">
  // The workspace feed, grouped like the window feed: a Local section (the local
  // registry) first, then one section per connected devserver (its served
  // workspaces, merged in by the library). Each row carries two icon actions —
  // [NEW WINDOW] (open a window onto it; greyed until the workspace is ON) and
  // [ON/OFF] (the tenant toggle) — plus a select checkbox feeding the single
  // global bulk bar (App-level). Local rows select as kind "workspace"; served
  // (devserver-mounted) rows select as kind "served" carrying their devserverId,
  // so the bar's ordered Remove can forget them on the right devserver. Remove is
  // bulk-only (no per-row Forget); the row still routes its on/off/open to the
  // owning devserver. The read-only surface shows the on-state statically.
  import { AppWindow, LoaderCircle, Power } from "lucide-svelte";
  import {
    library,
    toggleWorkspace,
    openWorkspaceWindow,
    openDevserverWorkspace,
    setDevserverWorkspaceOn,
    reportError,
    clearError,
  } from "../state/library.svelte";
  import { liveTerminalsCount } from "../api/library";
  import { requestConfirm } from "../state/confirm.svelte";
  import { isSelected, toggleSelected } from "../state/selection.svelte";
  import { isPending, servedKey, wsKey } from "../state/pending.svelte";
  import { basename } from "../lib/windowLabel";
  import { readOnly } from "../state/capabilities";
  import type { WorkspaceEntry } from "../api/library";

  function displayName(ws: WorkspaceEntry): string {
    return ws.label || basename(ws.path) || ws.path;
  }

  interface RemoteGroup {
    devserverId: string;
    label: string;
    workspaces: WorkspaceEntry[];
  }

  function remoteName(devserverId: string): string {
    const ds = library.devservers.find((d) => d.id === devserverId);
    return ds?.label || (ds ? `${ds.host}:${ds.port}` : null) || devserverId;
  }

  // Group the merged workspace feed: local rows (no devserver_id) stay in the
  // Local section; remote rows group under their devserver.
  function buildRemoteGroups(workspaces: WorkspaceEntry[]): RemoteGroup[] {
    const map = new Map<string, WorkspaceEntry[]>();
    for (const w of workspaces) {
      if (!w.devserver_id) continue;
      const arr = map.get(w.devserver_id) ?? [];
      arr.push(w);
      map.set(w.devserver_id, arr);
    }
    return [...map.entries()]
      .map(([devserverId, ws]) => ({ devserverId, label: remoteName(devserverId), workspaces: ws }))
      .sort((a, b) => a.label.localeCompare(b.label));
  }

  const localWorkspaces = $derived(library.workspaces.filter((w) => w.devserver_id === null));
  const remoteGroups = $derived(buildRemoteGroups(library.workspaces));

  // Per-row actions surface their failure in the banner (the actions throw so
  // the bulk loop can count failures; the per-row caller catches here).
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
  // in-SPA confirm (never a native dialog) showing N and, on confirm, retry the
  // SAME off forced (force:true). Any other error — including a plain
  // NO_DESKTOP 409, which liveTerminalsCount maps to null — goes to the banner.
  // Shared by the LOCAL and the DEVSERVER workspace Off (identical UX).
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

  // Toggle a LOCAL workspace: On is a plain action; Off routes through the
  // live-terminal confirm (parity with the devserver Off).
  function toggleLocalWorkspace(id: string, on: boolean): void {
    if (on) {
      void run(toggleWorkspace(id, true));
      return;
    }
    void offWorkspaceWithConfirm((force) => toggleWorkspace(id, false, force));
  }

  // Toggle a remote (devserver) workspace: same On/Off-with-confirm shape.
  function toggleRemoteWorkspace(devserverId: string, prefix: string, on: boolean): void {
    if (on) {
      void run(setDevserverWorkspaceOn(devserverId, prefix, true));
      return;
    }
    void offWorkspaceWithConfirm((force) => setDevserverWorkspaceOn(devserverId, prefix, false, force));
  }
</script>

{#if localWorkspaces.length}
  <section class="group">
    <h2 class="group-title">🏠 Local</h2>

    <ul class="rows">
      {#each localWorkspaces as ws (ws.workspace_id)}
        <li class="row" class:selected={isSelected("workspace", ws.workspace_id)}>
          {#if !readOnly}
            <input
              class="row-check"
              type="checkbox"
              checked={isSelected("workspace", ws.workspace_id)}
              aria-label={`Select ${displayName(ws)}`}
              onchange={() => toggleSelected("workspace", ws.workspace_id)} />
          {/if}
          <div class="row-main">
            <span class="row-name">{displayName(ws)}</span>
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
                onclick={() => run(openWorkspaceWindow(ws.path))}>
                <AppWindow size={16} />
              </button>
              <button
                class="icon-btn"
                class:on={ws.on}
                type="button"
                disabled={isPending(wsKey(ws.workspace_id))}
                title={isPending(wsKey(ws.workspace_id))
                  ? "Working…"
                  : ws.on
                    ? "Turn off"
                    : "Turn on"}
                aria-label={isPending(wsKey(ws.workspace_id))
                  ? `Working on ${displayName(ws)}`
                  : `${ws.on ? "Turn off" : "Turn on"} ${displayName(ws)}`}
                onclick={() => toggleLocalWorkspace(ws.workspace_id, !ws.on)}>
                {#if isPending(wsKey(ws.workspace_id))}
                  <LoaderCircle class="spin" size={16} />
                {:else}
                  <Power size={16} />
                {/if}
              </button>
            {/if}
          </div>
        </li>
      {/each}
    </ul>
  </section>
{/if}

{#each remoteGroups as g (g.devserverId)}
  <section class="group">
    <h2 class="group-title">↗ {g.label}</h2>
    <ul class="rows">
      {#each g.workspaces as ws (ws.workspace_id)}
        <li class="row" class:selected={!readOnly && isSelected("served", ws.prefix, g.devserverId)}>
          {#if !readOnly}
            <input
              class="row-check"
              type="checkbox"
              checked={isSelected("served", ws.prefix, g.devserverId)}
              aria-label={`Select ${displayName(ws)}`}
              onchange={() => toggleSelected("served", ws.prefix, g.devserverId)} />
          {/if}
          <div class="row-main">
            <span class="row-name">{displayName(ws)}</span>
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
                onclick={() => run(openDevserverWorkspace(g.devserverId, ws.path))}>
                <AppWindow size={16} />
              </button>
              <button
                class="icon-btn"
                class:on={ws.on}
                type="button"
                disabled={isPending(servedKey(g.devserverId, ws.prefix))}
                title={isPending(servedKey(g.devserverId, ws.prefix))
                  ? "Working…"
                  : ws.on
                    ? "Turn off"
                    : "Turn on"}
                aria-label={isPending(servedKey(g.devserverId, ws.prefix))
                  ? `Working on ${displayName(ws)}`
                  : `${ws.on ? "Turn off" : "Turn on"} ${displayName(ws)}`}
                onclick={() => toggleRemoteWorkspace(g.devserverId, ws.prefix, !ws.on)}>
                {#if isPending(servedKey(g.devserverId, ws.prefix))}
                  <LoaderCircle class="spin" size={16} />
                {:else}
                  <Power size={16} />
                {/if}
              </button>
            {/if}
          </div>
        </li>
      {/each}
    </ul>
  </section>
{/each}
