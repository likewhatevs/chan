// Which screen the launcher's main area shows: Computers (the machine/library
// tree) or Gateways. The flip trigger is forward-only -- a monotonic counter
// the ScreenFlip shell watches, so every change plays the same turn. Leaving a
// screen drops select mode so no stale selection, highlight, or bulk bar
// survives on the other side.

import { setSelectMode } from "./selection.svelte";

export type LauncherScreen = "computers" | "gateways";

interface ScreenState {
  current: LauncherScreen;
  /** Monotonic flip trigger for the ScreenFlip shell. */
  flips: number;
}

export const screen = $state<ScreenState>({ current: "computers", flips: 0 });

export function showScreen(next: LauncherScreen): void {
  if (screen.current === next) return;
  screen.current = next;
  screen.flips += 1;
  setSelectMode(false);
}

export function toggleScreen(): void {
  showScreen(screen.current === "computers" ? "gateways" : "computers");
}
