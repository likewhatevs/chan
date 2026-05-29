<script lang="ts">
  // `fullstack-a-75b`: Dashboard tab body. Per @@Alex's
  // `d4a3fc8` route on the slice-1 walk, the rotating carousel
  // moves OUT of the welcome surface (which becomes a static
  // spawn grid via EmptyPaneWelcome.svelte) and lives only
  // INSIDE this tab. The full carousel widget (rotation +
  // play/pause + pagination + 3 slides: About / Workspace
  // metadata / Indexing graph) renders here.
  //
  // `phase-13 lane-b` slice 3c: Cmd+, on a focused Hybrid
  // surface flips it to its back-side; Cmd+, again flips back.
  //
  // Phase-13 round-1 closing (B3): the back-of-card body
  // extracted to `HybridDashboardConfig.svelte` so Pane.svelte's
  // back-side switch mounts it via the canonical `active?.kind
  // === "dashboard"` arm (same shape as the Terminal / Editor /
  // Graph / File Browser arms). DashboardTab now only renders
  // the FRONT - the carousel - plus a right-click Reload row.
  // The redundant local `settingsOpen` path + its right-click
  // Settings entry retired in the same closing slice.

  import { RefreshCw } from "lucide-svelte";
  import { reloadWindow } from "../api/desktop";
  import {
    SHORTCUTS,
    currentOS,
    currentPlatform,
    formatChord,
  } from "../state/shortcuts";
  import {
    scheduleSessionSave,
    surfaceThemeOverride,
  } from "../state/store.svelte";
  import { type DashboardTab } from "../state/tabs.svelte";
  import EmptyPaneCarousel from "./EmptyPaneCarousel.svelte";
  import HamburgerMenu from "./HamburgerMenu.svelte";

  // Round-1 closing-10 (G3): the parent passes the live
  // DashboardTab so the carousel slide cursor persists across
  // window reloads. The tab is a $state proxy from
  // tabs.svelte.ts; mutating `tab.carouselSlide` reactively
  // updates the layout snapshot the next session save observes.
  type Props = { tab: DashboardTab };
  let { tab }: Props = $props();

  function onCarouselSlideChange(i: number): void {
    if (tab.carouselSlide === i) return;
    tab.carouselSlide = i;
    scheduleSessionSave();
  }

  let menu: HamburgerMenu | undefined = $state();
  let menuOpen = $state(false);

  // Phase-13 round-1 closing (B11): chord lookup so the Reload
  // row in the right-click menu renders its Cmd+R hint
  // alongside the row label, matching the pane top-bar pattern
  // in Pane.svelte.
  const platform = currentPlatform();
  const os = currentOS();
  function chordLabel(id: string): string {
    const s = SHORTCUTS.find((x) => x.id === id);
    if (!s) return "";
    const chord = s[platform];
    if (!chord) return "";
    return formatChord(chord, os);
  }

  function onContextMenu(e: MouseEvent): void {
    e.preventDefault();
    menu?.openAtCursor(e.clientX, e.clientY);
  }

  async function doReload(): Promise<void> {
    menu?.close();
    await reloadWindow();
  }
</script>

<div
  class="dashboard"
  aria-label="Dashboard"
  data-theme={surfaceThemeOverride("dashboard")}
  oncontextmenu={onContextMenu}
  role="region"
>
  <HamburgerMenu
    bind:this={menu}
    bind:open={menuOpen}
    showTrigger={false}
    width={220}
    height={58}
  >
    <!-- Phase-13 round-1 closing (B11): Reload mirrors the
         pane-top-bar paneContextMenu in Pane.svelte so the
         widget refresh affordance is reachable from the
         Dashboard body's own context menu, not just the tab
         strip. Both entry points route through `reloadWindow()`
         the same way Cmd+R does. The Settings entry retired in
         B3: Cmd+, is the canonical flip and Pane.svelte's
         back-side switch mounts HybridDashboardConfig directly. -->
    <li>
      <button role="menuitem" onclick={doReload}>
        <RefreshCw size={16} strokeWidth={1.75} aria-hidden="true" />
        <span class="menu-row-label">Reload</span>
        <span class="menu-row-chord">{chordLabel("app.window.reload")}</span>
      </button>
    </li>
  </HamburgerMenu>

  <EmptyPaneCarousel
    initialSlide={tab.carouselSlide ?? 0}
    onSlideChange={onCarouselSlideChange}
  />
</div>

<style>
  .dashboard {
    flex: 1;
    min-height: 0;
    min-width: 0;
    display: flex;
    flex-direction: column;
    background: var(--bg);
    color: var(--text);
  }
</style>
