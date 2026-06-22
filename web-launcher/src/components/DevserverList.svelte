<script lang="ts">
  // The devserver registry: each remote library the desktop dials out to. On the
  // mutable (desktop loopback) surface each row carries three icon actions —
  // [NEW TERMINAL] (open a terminal on the connected devserver), [EDIT] (open the
  // edit form; read-only while connected), and [CONNECT/DISCONNECT] (the
  // connection toggle, gated on `connected`) — plus a select checkbox feeding the
  // bulk bar (Connect / Disconnect / Remove). Connect, disconnect, and new
  // terminal are desktop actions; the read-only devserver/gateway surface shows
  // the rows with Edit only (the registry CRUD is browser-testable; the desktop
  // actions and bulk are hidden where no desktop bridge exists). A stored token
  // shows as a lock chip (the value is never returned).
  import { Pencil, Plug, SquareTerminal, Unplug } from "lucide-svelte";
  import {
    library,
    connectDevserver,
    disconnectDevserver,
    openDevserverTerminal,
    reportError,
    clearError,
  } from "../state/library.svelte";
  import { isSelected, toggleSelected } from "../state/selection.svelte";
  import { openEditDevserver } from "../state/dialog.svelte";
  import SelectionBar from "./SelectionBar.svelte";
  import { readOnly } from "../state/capabilities";
  import type { DevserverEntry } from "../api/library";

  function endpoint(ds: DevserverEntry): string {
    return `${ds.host}:${ds.port}`;
  }
  function displayName(ds: DevserverEntry): string {
    return ds.label || endpoint(ds);
  }

  // The desktop actions throw on failure (uniform with the rest, so the bulk
  // loop can count); the per-row caller catches here and surfaces the banner.
  async function run(action: Promise<void>): Promise<void> {
    clearError();
    try {
      await action;
    } catch (e) {
      reportError(e);
    }
  }
</script>

{#if library.devservers.length}
  <section class="group">
    <h2 class="group-title">↗ Devservers</h2>

    {#if !readOnly}
      <SelectionBar kind="devserver" onLabel="Connect" offLabel="Disconnect" />
    {/if}

    <ul class="rows">
      {#each library.devservers as ds (ds.id)}
        <li class="row" class:selected={!readOnly && isSelected("devserver", ds.id)}>
          {#if !readOnly}
            <input
              class="row-check"
              type="checkbox"
              checked={isSelected("devserver", ds.id)}
              aria-label={`Select ${displayName(ds)}`}
              onchange={() => toggleSelected("devserver", ds.id)} />
          {/if}
          <div class="row-main">
            <span class="row-name">
              {displayName(ds)}
              {#if ds.connected}<span class="dot live" title="Connected"></span>{/if}
              {#if ds.has_token}<span class="chip" title="A connect token is stored">🔒 token</span>{/if}
            </span>
            <span class="row-sub" title={endpoint(ds)}>{endpoint(ds)}</span>
          </div>
          <div class="row-actions">
            {#if !readOnly}
              <button
                class="icon-btn"
                type="button"
                disabled={!ds.connected}
                title={ds.connected ? "New terminal" : "Connect to open a terminal"}
                aria-label={`New terminal on ${displayName(ds)}`}
                onclick={() => run(openDevserverTerminal(ds.id))}>
                <SquareTerminal size={16} />
              </button>
            {/if}
            <button
              class="icon-btn"
              type="button"
              title={ds.connected ? "View (read-only while connected)" : "Edit"}
              aria-label={`Edit ${displayName(ds)}`}
              onclick={() => openEditDevserver(ds)}>
              <Pencil size={16} />
            </button>
            {#if !readOnly}
              {#if ds.connected}
                <button
                  class="icon-btn on"
                  type="button"
                  title="Disconnect"
                  aria-label={`Disconnect ${displayName(ds)}`}
                  onclick={() => run(disconnectDevserver(ds.id))}>
                  <Unplug size={16} />
                </button>
              {:else}
                <button
                  class="icon-btn"
                  type="button"
                  title="Connect"
                  aria-label={`Connect ${displayName(ds)}`}
                  onclick={() => run(connectDevserver(ds.id))}>
                  <Plug size={16} />
                </button>
              {/if}
            {/if}
          </div>
        </li>
      {/each}
    </ul>
  </section>
{/if}
