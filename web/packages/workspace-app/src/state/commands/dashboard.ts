// Dashboard surface commands: available when a dashboard tab is the
// active surface. Slide commands move the controlled carousel index on
// the active DashboardTab (its fields are $state) and persist the session,
// matching the carousel's own arrows. See state/commands.ts for the
// Command shape and the onSurface helper.

import { registerCommands, onSurface } from "../commands";
import { scheduleSessionSave, setHybridSurfaceTheme } from "../store.svelte";
import {
  DASHBOARD_SLOT_COUNT,
  activeDashboardTab,
  dashboardSlotEnabled,
  nextEnabledSlot,
  type DashboardTab,
} from "../tabs.svelte";

/// Run an action against the active dashboard tab, a no-op when none is
/// active.
function onDashboard(fn: (tab: DashboardTab) => void): () => void {
  return () => {
    const tab = activeDashboardTab();
    if (tab) fn(tab);
  };
}

/// Previous enabled carousel slot, wrapping and skipping disabled slots.
/// nextEnabledSlot is exported for the forward walk; only the launcher's
/// "Previous slide" needs the reverse, so it lives here.
function prevEnabledSlot(tab: DashboardTab, from: number): number {
  for (let step = 1; step <= DASHBOARD_SLOT_COUNT; step++) {
    const cand = (from - step + DASHBOARD_SLOT_COUNT) % DASHBOARD_SLOT_COUNT;
    if (dashboardSlotEnabled(tab, cand)) return cand;
  }
  return from;
}

registerCommands([
  {
    id: "app.dashboard.surfaceTheme.light",
    title: "Dashboard theme: light",
    category: "Dashboard",
    keywords: ["theme", "light", "appearance"],
    available: (ctx) => onSurface(ctx, "dashboard"),
    run: () => setHybridSurfaceTheme("dashboard", "light"),
  },
  {
    id: "app.dashboard.surfaceTheme.dark",
    title: "Dashboard theme: dark",
    category: "Dashboard",
    keywords: ["theme", "dark", "appearance"],
    available: (ctx) => onSurface(ctx, "dashboard"),
    run: () => setHybridSurfaceTheme("dashboard", "dark"),
  },
  {
    id: "app.dashboard.nextSlide",
    title: "Next slide",
    category: "Dashboard",
    keywords: ["slide", "carousel", "next", "forward"],
    available: (ctx) => onSurface(ctx, "dashboard"),
    run: onDashboard((tab) => {
      tab.carouselSlide = nextEnabledSlot(tab, tab.carouselSlide ?? 0);
      scheduleSessionSave();
    }),
  },
  {
    id: "app.dashboard.prevSlide",
    title: "Previous slide",
    category: "Dashboard",
    keywords: ["slide", "carousel", "previous", "back"],
    available: (ctx) => onSurface(ctx, "dashboard"),
    run: onDashboard((tab) => {
      tab.carouselSlide = prevEnabledSlot(tab, tab.carouselSlide ?? 0);
      scheduleSessionSave();
    }),
  },
]);
