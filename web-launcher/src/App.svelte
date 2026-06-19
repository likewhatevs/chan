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
  import { library, loadLibrary } from "./state/library.svelte";
  import { dialog, openNewDialog } from "./state/dialog.svelte";
  import { applyTheme } from "./state/theme.svelte";

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
      <p>A workspace is a local folder with your markdown files. Add one to get started.</p>
      <button class="btn primary" type="button" onclick={() => openNewDialog("local")}>
        New workspace
      </button>
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
</style>
