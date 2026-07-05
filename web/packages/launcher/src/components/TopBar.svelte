<script lang="ts">
  // The launcher's top bar: the "Computers" title with its subtitle, a Gmail-style
  // Select-mode toggle, and the theme toggle. The add-workspace and add-devserver
  // entry points live in the library tree (the LOCAL header's [new workspace] and
  // the bottom "Add dev server" dashed button), and open-terminal lives in each
  // machine header, so the top bar stays the global chrome: title + select + theme.
  import { Moon, SquareCheckBig, Sun } from "lucide-svelte";
  import { themeState, toggleTheme } from "../state/theme.svelte";
  import { selection, toggleSelectMode } from "../state/selection.svelte";
  import { readOnly } from "../state/capabilities";
</script>

<header class="topbar">
  <div class="title">
    <h1 class="brand">Computers</h1>
    <p class="subtitle">This machine &amp; dev servers</p>
  </div>
  <div class="actions">
    {#if !readOnly}
      <button
        class="icon-btn select"
        class:active={selection.selectMode}
        type="button"
        aria-label={selection.selectMode ? "Exit select mode" : "Select"}
        title={selection.selectMode ? "Exit select" : "Select"}
        onclick={toggleSelectMode}>
        <SquareCheckBig size={16} />
      </button>
    {/if}
    <button
      class="icon-btn"
      type="button"
      aria-label="Toggle theme"
      title="Toggle theme"
      onclick={toggleTheme}>
      {#if themeState.theme === "dark"}
        <Sun size={16} />
      {:else}
        <Moon size={16} />
      {/if}
    </button>
  </div>
</header>

<style>
  .topbar {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 1rem;
    padding: 0.75rem 1.25rem;
    border-bottom: 1px solid var(--border);
    position: sticky;
    top: 0;
    background: var(--bg);
    z-index: 10;
  }

  .title {
    min-width: 0;
  }

  .brand {
    display: block;
    font-size: 1.15rem;
    font-weight: 700;
    gap: 0;
    letter-spacing: -0.01em;
    line-height: 1.1;
    margin: 0;
    text-decoration: none;
  }

  .subtitle {
    margin: 0.15rem 0 0;
    font-size: 0.78rem;
    color: var(--text-secondary);
  }

  .actions {
    display: flex;
    gap: 0.5rem;
    flex-shrink: 0;
  }

  .icon-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 2rem;
    height: 2rem;
    border: 1px solid var(--btn-border);
    border-radius: 6px;
    background: var(--btn-bg);
    color: var(--text);
    font-size: 1.1rem;
    line-height: 1;
    cursor: pointer;
    transition:
      border-color 160ms ease,
      color 160ms ease;
  }

  .icon-btn:hover {
    border-color: var(--brand);
    color: var(--brand);
  }

  /* Select mode active: the toggle holds the accent so it reads as engaged. */
  .icon-btn.select.active {
    border-color: var(--accent);
    color: var(--accent);
    background: color-mix(in srgb, var(--accent) 14%, transparent);
  }

</style>
