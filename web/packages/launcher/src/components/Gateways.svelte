<script lang="ts">
  // The Gateways screen: one machine-styled badge per configured gateway --
  // the ArrowRightLeft glyph, the label over the URL, the status dot, and the
  // plug/unplug controls in the Library card idiom (spinner while connecting,
  // sign-in narration while a browser sign-in is pending, red lost dot +
  // disconnect-to-recover while unreachable). Below the list sits the dashed
  // "Add gateway" entry point. Select mode reveals per-badge checkboxes
  // feeding the same global bulk bar as the Computers screen; the screen is
  // reachable only on the desktop surface (the TopBar toggle gates on the
  // bridge), and the controls carry the same bridge gate.
  import { ArrowRightLeft, LoaderCircle, Plug, Plus, Unplug } from "lucide-svelte";
  import {
    library,
    connectGateway,
    disconnectGateway,
    reportError,
    clearError,
  } from "../state/library.svelte";
  import { checksVisible, isSelected, toggleSelected } from "../state/selection.svelte";
  import { openNewDialog } from "../state/dialog.svelte";
  import { readOnly, hasDesktopBridge } from "../state/capabilities";
  import type { GatewayEntry } from "../api/library";

  function hostOf(url: string): string {
    try {
      return new URL(url).host;
    } catch {
      return url;
    }
  }

  function gatewayName(gw: GatewayEntry): string {
    return gw.label || hostOf(gw.url);
  }

  const connected = (gw: GatewayEntry): boolean => gw.status === "connected";
  const lost = (gw: GatewayEntry): boolean => gw.status === "unreachable";
  const spinning = (gw: GatewayEntry): boolean => gw.status === "connecting";

  // Per-badge action failures surface as notice bubbles (uniform with the
  // Library rows: the actions throw, the caller catches here).
  async function run(action: Promise<void>): Promise<void> {
    clearError();
    try {
      await action;
    } catch (e) {
      reportError(e);
    }
  }
</script>

<div class="gateways-screen">
  {#each library.gateways as gw (gw.id)}
    <section class="gateway-card">
      <div class="gateway-header">
        {#if !readOnly && checksVisible()}
          <input
            class="row-check"
            type="checkbox"
            checked={isSelected("gateway", gw.id)}
            aria-label={`Select ${gatewayName(gw)}`}
            onchange={() => toggleSelected("gateway", gw.id)} />
        {/if}
        <div class="gw-id">
          <span class="gw-name-row">
            <span class="gw-glyph"><ArrowRightLeft size={16} /></span>
            <span class="gw-name">{gatewayName(gw)}</span>
            {#if lost(gw)}<span class="status-dot lost" title="Connection lost"></span>
            {:else if connected(gw)}<span class="status-dot live" title="Connected"></span>{/if}
            {#if connected(gw)}
              <span class="chip">
                {gw.devserver_count} devserver{gw.devserver_count === 1 ? "" : "s"}
              </span>
            {/if}
          </span>
          <span class="gw-addr-row">
            <span class="gw-glyph"></span>
            <span class="gw-url" title={gw.url}>{gw.url}</span>
          </span>
        </div>
        <div class="gateway-actions">
          {#if hasDesktopBridge}
            {#if spinning(gw)}
              <button
                class="icon-btn"
                type="button"
                disabled
                title="Working…"
                aria-label={`Working on ${gatewayName(gw)}`}>
                <LoaderCircle class="spin" size={16} />
              </button>
            {:else if connected(gw) || lost(gw)}
              <!-- `unreachable` keeps the disconnect affordance (the desktop
                   keeps retrying over the live connection record; a plug would
                   stack a second connect). Disconnect-then-reconnect is the
                   recovery, mirroring the devserver card. -->
              <button
                class="icon-btn"
                class:on={!lost(gw)}
                class:lost={lost(gw)}
                type="button"
                title={lost(gw) ? "Disconnect lost connection" : "Disconnect"}
                aria-label={`Disconnect gateway ${gatewayName(gw)}`}
                onclick={() => run(disconnectGateway(gw.id))}>
                <Unplug size={16} />
              </button>
            {:else}
              <!-- While a browser sign-in is pending the button stays live: a
                   re-click re-opens the sign-in page (latest-wins desktop-side). -->
              <button
                class="icon-btn"
                type="button"
                title={gw.pending_signin ? "Re-open sign-in in your browser" : "Connect"}
                aria-label={gw.pending_signin
                  ? `Re-open sign-in in your browser for ${gatewayName(gw)}`
                  : `Connect gateway ${gatewayName(gw)}`}
                onclick={() => run(connectGateway(gw.id))}>
                <Plug size={16} />
              </button>
            {/if}
          {/if}
        </div>
      </div>
      {#if gw.pending_signin}
        <!-- The connect handed off to a browser sign-in: narrate the wait. The
             desktop clears the state on the deep-link callback, its timeout,
             or teardown. -->
        <p class="gateway-prompt waiting">
          <LoaderCircle class="spin" size={14} aria-hidden="true" />
          Waiting for sign-in in your browser...
        </p>
      {:else if spinning(gw)}
        <p class="gateway-prompt">Connecting…</p>
      {:else if lost(gw)}
        <p class="gateway-prompt">
          Connection lost{gw.last_error ? `: ${gw.last_error}` : "."} Retrying; the last-known
          devservers stay listed.
        </p>
      {:else if connected(gw)}
        <p class="gateway-prompt">
          {gw.devserver_count === 0
            ? "No devservers on this gateway yet."
            : `${gw.devserver_count} devserver${gw.devserver_count === 1 ? "" : "s"} listed under Computers.`}
        </p>
      {:else}
        <p class="gateway-prompt">
          Not connected{gw.last_error ? ` (${gw.last_error})` : "."}
        </p>
      {/if}
    </section>
  {/each}

  {#if library.gateways.length === 0}
    <p class="empty-hint">
      No gateways yet. Add one to reach your devservers and devservers shared with you.
    </p>
  {/if}

  {#if hasDesktopBridge}
    <button class="add-gateway" type="button" onclick={() => openNewDialog("gateway")}>
      <Plus size={16} />
      Add gateway
    </button>
  {/if}
</div>

<style>
  /* Each gateway is a contained card, the machine-badge idiom: identity header
     over a one-line status narration. The whole badge wobbles on hover. */
  .gateway-card {
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

  .gateway-card:hover {
    transform: scale(1.015);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.24);
    z-index: 1;
  }

  .gateway-header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.4rem 0.25rem 0.35rem;
  }

  .gw-id {
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
    flex: 1;
    min-width: 0;
  }

  .gw-name-row,
  .gw-addr-row {
    display: flex;
    align-items: center;
    gap: 0.45rem;
    min-width: 0;
  }

  .gw-glyph {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 1rem;
    flex-shrink: 0;
    color: var(--text-secondary);
  }

  .gw-name {
    font-weight: 600;
    font-size: 0.92rem;
    color: var(--text);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .gw-url {
    font-size: 0.78rem;
    color: var(--text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

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

  /* Same dot, connection lost: the roster poll keeps failing. Steady red. */
  .status-dot.lost {
    background: var(--danger);
    opacity: 1;
    box-shadow: 0 0 6px color-mix(in srgb, var(--danger) 70%, transparent);
  }

  .gateway-actions {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    flex-shrink: 0;
  }

  .gateway-prompt {
    margin: 0.35rem 0 0 0.5rem;
    padding: 0.5rem 0.75rem;
    font-size: 0.82rem;
    color: var(--text-secondary);
  }

  /* The browser sign-in hand-off: the prompt gains the in-flight spinner
     (global .spin) beside the text. */
  .gateway-prompt.waiting {
    display: flex;
    align-items: center;
    gap: 0.45rem;
  }

  .empty-hint {
    margin: 0.35rem 0 0;
    font-size: 0.85rem;
    line-height: 1.5;
    color: var(--text-secondary);
  }

  /* The dashed add entry point, mirroring the Computers screen's Add devserver. */
  .add-gateway {
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

  .add-gateway:hover {
    border-color: color-mix(in srgb, var(--accent) 45%, var(--btn-border));
    color: var(--text);
    background: color-mix(in srgb, var(--text-secondary) 6%, transparent);
  }

  @media (prefers-reduced-motion: reduce) {
    .gateway-card,
    .gateway-card:hover {
      transform: none;
      transition: box-shadow 160ms ease;
    }
  }
</style>
