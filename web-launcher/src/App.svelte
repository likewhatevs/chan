<script lang="ts">
  // The launcher root: the top bar over the registry lists and the window
  // feed, with the New/Edit dialog mounted while open. Data loads on mount
  // and the window feed stays live through its watch subscription.
  import { onMount } from "svelte";
  import { X } from "lucide-svelte";
  import TopBar from "./components/TopBar.svelte";
  import SelectionBar from "./components/SelectionBar.svelte";
  import WorkspaceList from "./components/WorkspaceList.svelte";
  import DevserverList from "./components/DevserverList.svelte";
  import WindowFeed from "./components/WindowFeed.svelte";
  import NewWorkspaceDialog from "./components/NewWorkspaceDialog.svelte";
  import ConfirmDialog from "./components/ConfirmDialog.svelte";
  import ControlClosedSurvey from "./components/ControlClosedSurvey.svelte";
  import { library, loadLibrary, openTerminal, clearError } from "./state/library.svelte";
  import { dialog, openNewDialog } from "./state/dialog.svelte";
  import { confirm } from "./state/confirm.svelte";
  import { controlClosed, onControlClosedEvent } from "./state/controlClosed.svelte";
  import { onTauriEvent } from "./api/desktop";
  import { applyTheme } from "./state/theme.svelte";
  import { readOnly } from "./state/capabilities";

  onMount(() => {
    applyTheme();
    loadLibrary();
    // A connected devserver's control terminal exited: the desktop emits
    // `devserver-control-closed` with its id. Survey re-run / edit / abandon.
    // No-op off-desktop (the global Tauri event bridge is absent in a browser).
    let unlisten: (() => void) | null = null;
    void onTauriEvent("devserver-control-closed", onControlClosedEvent).then((un) => {
      unlisten = un;
    });
    return () => unlisten?.();
  });

  const isEmpty = $derived(
    !library.loading &&
      library.workspaces.length === 0 &&
      library.devservers.length === 0 &&
      library.windows.length === 0,
  );
</script>

<TopBar />

<main class="content">
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

  {#if isEmpty}
    <div class="empty">
      <h2>No workspaces yet</h2>
      <p>
        A workspace is just a directory — chan treats it as a project. Add your
        first one, or open a terminal and run
        <code>chan open /path/to/project</code>.
      </p>
      <div class="empty-actions">
        {#if !readOnly}
          <button class="btn primary" type="button" onclick={() => openNewDialog("local")}>
            New workspace
          </button>
        {/if}
        <button class="btn" type="button" onclick={() => openTerminal()}>
          Open terminal
        </button>
      </div>
      {#if readOnly}
        <p class="manage-hint">Manage workspaces from the desktop app or the chan CLI.</p>
      {/if}
    </div>
  {:else}
    {#if !readOnly}
      <SelectionBar />
    {/if}
    <WorkspaceList />
    <DevserverList />
    <WindowFeed />
  {/if}
</main>

{#if dialog.open}
  <NewWorkspaceDialog />
{/if}

{#if confirm.open}
  <ConfirmDialog />
{/if}

{#if controlClosed.open}
  <ControlClosedSurvey />
{/if}

<style>
  .content {
    max-width: 44rem;
    margin: 0 auto;
    padding: 1.5rem 1.25rem 4rem;
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

  .empty {
    max-width: 28rem;
    margin: 4rem auto;
    text-align: center;
    color: var(--text-secondary);
  }

  .empty h2 {
    color: var(--text);
    font-weight: 600;
  }

  .empty p {
    line-height: 1.5;
    margin-bottom: 1.25rem;
  }

  .empty p code {
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 0.85em;
    padding: 0.1em 0.35em;
    border-radius: 4px;
    background: color-mix(in srgb, var(--text-secondary) 16%, transparent);
    color: var(--text);
    white-space: nowrap;
  }

  .empty-actions {
    display: flex;
    gap: 0.6rem;
    justify-content: center;
    flex-wrap: wrap;
  }

  .manage-hint {
    margin-top: 1rem;
    font-size: 0.85rem;
    color: var(--text-secondary);
  }
</style>
