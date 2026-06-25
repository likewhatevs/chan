<script lang="ts">
  // Shared single-icon light/dark toggle. Mirrors chan-desktop's
  // titlebar affordance: one button that shows the SUN while a dark
  // theme is active (click switches to light) and the MOON while a
  // light theme is active (click switches to dark). The displayed
  // glyph is the theme you switch TO, so the control reads as "tap to
  // get day / tap to get night".
  //
  // This replaces the per-surface "[Moon Dark][Sun Light]" segmented
  // TEXT toggle. The app-level tri-state (system / light / dark)
  // configurator keeps its labeled segmented form; it has a third
  // option a two-state icon can't express.
  import { Moon, Sun } from "lucide-svelte";

  let {
    theme,
    onToggle,
    label = "theme",
    size = 15,
  }: {
    /// The currently active (resolved) theme this control toggles.
    theme: "light" | "dark";
    /// Fired on click. The caller flips the underlying choice; this
    /// component stays presentation-only so it works for both the
    /// per-surface override and any future single-toggle consumer.
    onToggle: () => void;
    /// Prefix for the title / aria-label, e.g. "Editor body" yields
    /// "Switch Editor body to light theme".
    label?: string;
    size?: number;
  } = $props();

  const next = $derived(theme === "dark" ? "light" : "dark");
</script>

<button
  type="button"
  class="theme-toggle"
  onclick={onToggle}
  title={`Switch ${label} to ${next} theme`}
  aria-label={`Switch ${label} to ${next} theme`}
>
  {#if theme === "dark"}
    <Sun {size} strokeWidth={1.85} aria-hidden="true" />
  {:else}
    <Moon {size} strokeWidth={1.85} aria-hidden="true" />
  {/if}
</button>

<style>
  .theme-toggle {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 28px;
    min-height: 28px;
    padding: 3px;
    border: 1px solid var(--btn-border);
    border-radius: 5px;
    background: var(--bg);
    color: var(--text-secondary);
    cursor: pointer;
    flex-shrink: 0;
  }
  .theme-toggle:hover {
    color: var(--text);
    background: var(--hover-bg);
  }
</style>
