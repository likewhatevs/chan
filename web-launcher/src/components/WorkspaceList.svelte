<script lang="ts">
  // The workspace feed, grouped like the window feed: a Local section (the local
  // registry) first, then one section per connected devserver (its served
  // workspaces, merged in by the library). Each row carries two icon actions —
  // [NEW WINDOW] (open a window onto it; greyed until the workspace is ON) and
  // [ON/OFF] (the tenant toggle). Local rows add a select checkbox feeding the
  // bulk bar (Turn On / Turn Off / Remove); remove is bulk-only there. A remote
  // row adds a [FORGET] action (unmount + drop) and routes its on/off/open/forget
  // to the owning devserver. The read-only surface shows the on-state statically.
  import { AppWindow, Power, Trash2 } from "lucide-svelte";
  import {
    library,
    toggleWorkspace,
    openWorkspaceWindow,
    openDevserverWorkspace,
    setDevserverWorkspaceOn,
    forgetDevserverWorkspace,
    reportError,
    clearError,
  } from "../state/library.svelte";
  import { liveTerminalsCount } from "../api/library";
  import { requestConfirm } from "../state/confirm.svelte";
  import { isSelected, toggleSelected } from "../state/selection.svelte";
  import SelectionBar from "./SelectionBar.svelte";
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
    return ds?.label || ds?.url || devserverId;
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

  // Toggle a remote (devserver) workspace. Turning ON is a plain action. Turning
  // OFF can hit live terminal sessions: the server answers 409 live_terminals
  // with the count, so on that specific error we open the in-SPA confirm (never
  // a native dialog) showing N and, on confirm, retry the same off forced
  // (force:true). Any other error — including a plain NO_DESKTOP 409, which
  // liveTerminalsCount maps to null — goes straight to the banner.
  async function toggleRemoteWorkspace(devserverId: string, prefix: string, on: boolean): Promise<void> {
    if (on) {
      await run(setDevserverWorkspaceOn(devserverId, prefix, true));
      return;
    }
    clearError();
    try {
      await setDevserverWorkspaceOn(devserverId, prefix, false);
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
        onConfirm: () => run(setDevserverWorkspaceOn(devserverId, prefix, false, true)),
      });
    }
  }
</script>

{#if localWorkspaces.length}
  <section class="group">
    <h2 class="group-title">🏠 Local</h2>

    {#if !readOnly}
      <SelectionBar kind="workspace" />
    {/if}

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
                title={ws.on ? "Turn off" : "Turn on"}
                aria-label={`${ws.on ? "Turn off" : "Turn on"} ${displayName(ws)}`}
                onclick={() => run(toggleWorkspace(ws.workspace_id, !ws.on))}>
                <Power size={16} />
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
        <li class="row">
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
                title={ws.on ? "Turn off" : "Turn on"}
                aria-label={`${ws.on ? "Turn off" : "Turn on"} ${displayName(ws)}`}
                onclick={() => toggleRemoteWorkspace(g.devserverId, ws.prefix, !ws.on)}>
                <Power size={16} />
              </button>
              <button
                class="icon-btn danger"
                type="button"
                title="Forget"
                aria-label={`Forget ${displayName(ws)}`}
                onclick={() => run(forgetDevserverWorkspace(g.devserverId, ws.prefix))}>
                <Trash2 size={16} />
              </button>
            {/if}
          </div>
        </li>
      {/each}
    </ul>
  </section>
{/each}
