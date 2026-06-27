<script lang="ts">
  // The launcher root: the top bar over the machine-first library tree, with the
  // New/Edit dialog mounted while open. Data loads on mount and the tree stays
  // live through the window-watch subscription.
  import { onMount } from "svelte";
  import { X } from "lucide-svelte";
  import TopBar from "./components/TopBar.svelte";
  import SelectionBar from "./components/SelectionBar.svelte";
  import Library from "./components/Library.svelte";
  import NewWorkspaceDialog from "./components/NewWorkspaceDialog.svelte";
  import ConfirmDialog from "./components/ConfirmDialog.svelte";
  import {
    library,
    loadLibrary,
    clearError,
    disconnectDevserver,
    reportError,
  } from "./state/library.svelte";
  import { dialog } from "./state/dialog.svelte";
  import { confirm } from "./state/confirm.svelte";
  import { checksVisible } from "./state/selection.svelte";
  import { controlClosedId, onControlClosedEvent } from "./state/controlClosed.svelte";
  import { clearControlAttention } from "./state/controlAttention.svelte";
  import { onTauriEvent } from "./api/desktop";
  import { applyTheme } from "./state/theme.svelte";
  import { readOnly } from "./state/capabilities";

  onMount(() => {
    applyTheme();
    loadLibrary();
    // A connected devserver's control terminal exited: the desktop emits
    // `devserver-control-closed` with its id, and the launcher flashes that
    // control row's eye yellow for attention (the amber "reconnecting" cue).
    // No-op off-desktop (the global Tauri event bridge is absent in a browser).
    let unlisten: (() => void) | null = null;
    void onTauriEvent("devserver-control-closed", onControlClosedEvent).then((un) => {
      unlisten = un;
    });
    // A workspace window's disconnect overlay chose Abandon: the desktop resolved
    // its devserver and asks the launcher to disconnect it, reusing the connect
    // spinner lifecycle. No-op off-desktop (the event never fires in a browser).
    let unlistenAbandon: (() => void) | null = null;
    void onTauriEvent("devserver-abandon", (payload) => {
      const id = controlClosedId(payload);
      if (id) void disconnectDevserver(id).catch(reportError);
    }).then((un) => {
      unlistenAbandon = un;
    });
    return () => {
      unlisten?.();
      unlistenAbandon?.();
    };
  });

  // Clear a devserver's control-attention flash when it RECONNECTS (a
  // disconnected -> connected transition). Tracking the transition, not the
  // current state, avoids clearing the flash that the control-closed event just
  // set while the feed still reports the dying devserver as connected.
  const wasConnected = new Map<string, boolean>();
  $effect(() => {
    for (const ds of library.devservers) {
      const now = ds.status === "connected";
      if (now && !(wasConnected.get(ds.id) ?? false) && ds.library_id) {
        clearControlAttention(ds.library_id);
      }
      wasConnected.set(ds.id, now);
    }
  });
</script>

<TopBar />

<main class="content" class:with-bulk-bar={!readOnly && checksVisible()}>
  {#if library.error}
    <div class="banner" role="alert">
      <span class="banner-text">{library.error}</span>
      <button
        class="banner-dismiss"
        type="button"
        aria-label="Dismiss"
        title="Dismiss"
        onclick={() => clearError()}>
        <X size={16} />
      </button>
    </div>
  {/if}

  <Library />
</main>

{#if !readOnly}
  <SelectionBar />
{/if}

{#if dialog.open}
  <NewWorkspaceDialog />
{/if}

{#if confirm.open}
  <ConfirmDialog />
{/if}

<style>
  .content {
    max-width: 44rem;
    margin: 0 auto;
    padding: 1.5rem 1.25rem 4rem;
  }

  /* In select mode the bottom-docked bulk bar overlays the viewport; extra
     bottom padding keeps the last rows clear of it. */
  .content.with-bulk-bar {
    padding-bottom: 6rem;
  }

  .banner {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 1rem;
    padding: 0.5rem 0.5rem 0.5rem 0.8rem;
    border-radius: 8px;
    background: color-mix(in srgb, var(--danger) 16%, transparent);
    color: var(--danger);
    font-size: 0.9rem;
  }

  .banner-text {
    flex: 1;
  }

  /* Dismiss [X] — the icon-button posture, but transparent so it blends into the
     danger banner and inherits its colour. */
  .banner-dismiss {
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 1.6rem;
    height: 1.6rem;
    padding: 0;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: inherit;
    cursor: pointer;
  }

  .banner-dismiss:hover {
    background: color-mix(in srgb, var(--danger) 22%, transparent);
  }
</style>
