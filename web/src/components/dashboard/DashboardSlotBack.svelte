<script lang="ts">
  // Per-slot Dashboard flip-back. Replaces the monolithic
  // HybridDashboardConfig: the back now mirrors the front carousel's
  // current slot (About / Workspace / Search) and shows that slot's own
  // config body. A force-paused slot picker lets the user move between
  // slot configs without flipping back to the front; selecting a slot
  // moves the shared `tab.carouselSlide` cursor so the front carousel
  // follows on flip-back (the carousel is controlled off the same
  // field). The shell (title band, per-Hybrid Dark/Light override, OK)
  // is HybridSurfaceConfigShell, same as every other Hybrid back.

  import type { DashboardTab } from "../../state/tabs.svelte";
  import { scheduleSessionSave } from "../../state/store.svelte";
  import HybridSurfaceConfigShell from "../HybridSurfaceConfigShell.svelte";
  import AboutSlotConfig from "./AboutSlotConfig.svelte";
  import WorkspaceSlotConfig from "./WorkspaceSlotConfig.svelte";
  import SearchSlotConfig from "./SearchSlotConfig.svelte";

  type Props = { tab: DashboardTab; onDone?: () => void };
  let { tab, onDone }: Props = $props();

  const SLOTS = ["About", "Workspace", "Search"] as const;
  // Clamp to the valid slot range; the front carousel uses the same
  // clamp so the two faces never disagree on which slot is active.
  const slot = $derived(
    Math.min(Math.max(0, Math.floor(tab.carouselSlide ?? 0)), SLOTS.length - 1),
  );

  function selectSlot(i: number): void {
    if (tab.carouselSlide === i) return;
    tab.carouselSlide = i;
    scheduleSessionSave();
  }
</script>

<HybridSurfaceConfigShell
  title={SLOTS[slot]}
  surface="dashboard"
  ariaLabel="Dashboard settings"
  {onDone}
>
  <!-- Force-paused slot picker. The front dots auto-rotate; this one
       does not (you are configuring, not watching). Selecting a slot
       swaps the body below and moves tab.carouselSlide so the front
       lands on the same slot when flipped back. -->
  <div class="slot-picker" role="tablist" aria-label="Dashboard slot">
    {#each SLOTS as label, i (label)}
      <button
        type="button"
        class="slot-tab"
        class:on={slot === i}
        role="tab"
        aria-selected={slot === i}
        onclick={() => selectSlot(i)}
      >{label}</button>
    {/each}
  </div>

  {#if slot === 0}
    <AboutSlotConfig />
  {:else if slot === 1}
    <WorkspaceSlotConfig />
  {:else}
    <SearchSlotConfig />
  {/if}
</HybridSurfaceConfigShell>

<style>
  .slot-picker {
    display: inline-flex;
    align-self: flex-start;
    gap: 2px;
    padding: 2px;
    border: 1px solid var(--btn-border);
    border-radius: 6px;
    background: var(--bg);
  }
  .slot-tab {
    border: 0;
    border-radius: 4px;
    background: transparent;
    color: var(--text-secondary);
    font: inherit;
    font-size: 13px;
    padding: 4px 12px;
    cursor: pointer;
  }
  .slot-tab:hover {
    color: var(--text);
    background: var(--hover-bg);
  }
  .slot-tab.on {
    color: var(--text);
    background: var(--bg-card);
    box-shadow: inset 0 0 0 1px var(--border);
  }
</style>
