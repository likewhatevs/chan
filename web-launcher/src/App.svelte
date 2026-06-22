<script lang="ts">
  // The launcher root: the top bar over the registry lists and the window
  // feed, with the New/Edit dialog mounted while open. Data loads on mount
  // and the window feed stays live through its watch subscription.
  import { onMount } from "svelte";
  import TopBar from "./components/TopBar.svelte";
  import WorkspaceList from "./components/WorkspaceList.svelte";
  import DevserverList from "./components/DevserverList.svelte";
  import WindowFeed from "./components/WindowFeed.svelte";
  import NewWorkspaceDialog from "./components/NewWorkspaceDialog.svelte";
  import { library, loadLibrary, openTerminal } from "./state/library.svelte";
  import { dialog, openNewDialog } from "./state/dialog.svelte";
  import { applyTheme } from "./state/theme.svelte";
  import { readOnly } from "./state/capabilities";

  onMount(() => {
    applyTheme();
    loadLibrary();
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
    <div class="banner" role="alert">{library.error}</div>
  {/if}

  {#if isEmpty}
    <div class="empty">
      <h2>No workspaces yet</h2>
      <p>
        A workspace is just a directory — chan treats it as a project. Add your
        first one, or open a terminal and run
        <code>chan serve /path/to/project</code>.
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
    <WorkspaceList />
    <DevserverList />
    <WindowFeed />
  {/if}
</main>

{#if dialog.open}
  <NewWorkspaceDialog />
{/if}

<style>
  .content {
    max-width: 44rem;
    margin: 0 auto;
    padding: 1.5rem 1.25rem 4rem;
  }

  .banner {
    margin-bottom: 1rem;
    padding: 0.6rem 0.8rem;
    border-radius: 8px;
    background: color-mix(in srgb, var(--danger) 16%, transparent);
    color: var(--danger);
    font-size: 0.9rem;
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
