<script lang="ts">
  // The launcher's top bar: the "Library" title, a theme toggle, and the
  // New-workspace button (+). The open-terminal action moved into the Local
  // group header (the library tree), so the top bar stays the global chrome:
  // theme + add.
  import { themeState, toggleTheme } from "../state/theme.svelte";
  import { openNewDialog } from "../state/dialog.svelte";
  import { readOnly } from "../state/capabilities";
</script>

<header class="topbar">
  <h1 class="brand">Library</h1>
  <div class="actions">
    <button
      class="icon-btn"
      type="button"
      aria-label="Toggle theme"
      title="Toggle theme"
      onclick={toggleTheme}>{themeState.theme === "dark" ? "☀" : "☾"}</button>
    {#if !readOnly}
      <button
        class="icon-btn"
        type="button"
        aria-label="New workspace"
        title="New workspace"
        onclick={() => openNewDialog("local")}>+</button>
    {/if}
  </div>
</header>

<style>
  .topbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.75rem 1.25rem;
    border-bottom: 1px solid var(--border);
    position: sticky;
    top: 0;
    background: var(--bg);
    z-index: 10;
  }

  .brand {
    font-size: 1.05rem;
    font-weight: 600;
    margin: 0;
  }

  .actions {
    display: flex;
    gap: 0.5rem;
  }

  .icon-btn {
    width: 2rem;
    height: 2rem;
    border: 1px solid var(--btn-border);
    border-radius: 6px;
    background: var(--btn-bg);
    color: var(--text);
    font-size: 1.1rem;
    line-height: 1;
    cursor: pointer;
  }

  .icon-btn:hover {
    border-color: var(--brand);
    color: var(--brand);
  }
</style>
