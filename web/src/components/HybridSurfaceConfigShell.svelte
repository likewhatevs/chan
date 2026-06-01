<script lang="ts">
  import type { Snippet } from "svelte";
  import ThemeToggleButton from "./ThemeToggleButton.svelte";
  import type { HybridSurfaceKind, SurfaceThemeChoice } from "../api/types";
  import {
    effectiveHybridSurfaceTheme,
    setHybridSurfaceTheme,
  } from "../state/store.svelte";

  type SaveStatus = "idle" | "saving" | "saved" | { error: string };

  let {
    title,
    surface,
    ariaLabel = `${title} configuration`,
    saveStatus = "idle",
    onDone,
    children,
  }: {
    title: string;
    surface: HybridSurfaceKind;
    ariaLabel?: string;
    saveStatus?: SaveStatus;
    onDone?: () => void;
    children?: Snippet;
  } = $props();

  const activeTheme = $derived(effectiveHybridSurfaceTheme(surface));

  function setTheme(choice: SurfaceThemeChoice): void {
    setHybridSurfaceTheme(surface, choice);
  }
</script>

<section class="hybrid-config" aria-label={ariaLabel}>
  <header class="config-header">
    <h2 class="config-title">{title}</h2>
    <div class="header-spacer" aria-hidden="true"></div>
    <div class="save-status" aria-live="polite">
      {#if saveStatus === "saving"}
        <span class="muted">saving...</span>
      {:else if saveStatus === "saved"}
        <span class="ok">saved</span>
      {:else if typeof saveStatus === "object"}
        <span class="err" title={saveStatus.error}>save failed</span>
      {/if}
    </div>
    <ThemeToggleButton
      theme={activeTheme}
      label={`${title} body`}
      onToggle={() => setTheme(activeTheme === "dark" ? "light" : "dark")}
    />
  </header>
  <div class="config-body">
    {#if children}
      {@render children()}
    {/if}
  </div>
  <footer class="config-footer">
    <button type="button" class="config-ok" onclick={() => onDone?.()}>OK</button>
  </footer>
</section>

<style>
  .hybrid-config {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
    min-height: 0;
  }
  .config-header {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 16px 20px;
    border-bottom: 1px solid var(--border);
  }
  .config-title {
    margin: 0;
    font-size: 18px;
    font-weight: 600;
    color: var(--text);
  }
  .header-spacer {
    flex: 1;
    min-width: 0;
  }
  .save-status {
    min-width: 60px;
    font-size: 14px;
    text-align: right;
  }
  .save-status .ok {
    color: var(--accent);
  }
  .save-status .err {
    color: #d33;
  }
  .save-status .muted {
    color: var(--text-secondary);
  }
  .config-body {
    flex: 1;
    overflow: auto;
    padding: 16px 20px;
    display: flex;
    flex-direction: column;
    gap: 1.25rem;
  }
  .config-body :global(section) {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  .config-body :global(section h3) {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
    color: var(--text);
  }
  .config-footer {
    display: flex;
    justify-content: flex-end;
    padding: 12px 20px 16px;
    border-top: 1px solid var(--border);
  }
  .config-ok {
    background: var(--btn-bg);
    color: var(--text);
    border: 1px solid var(--btn-border);
    border-radius: 4px;
    padding: 5px 12px;
    font: inherit;
    cursor: pointer;
  }
  .config-ok:hover {
    border-color: var(--btn-hover);
  }
  @media (max-width: 520px) {
    .config-header {
      align-items: flex-start;
      flex-wrap: wrap;
    }
    .header-spacer {
      flex-basis: 100%;
      height: 0;
    }
    .save-status {
      text-align: left;
    }
  }
</style>
