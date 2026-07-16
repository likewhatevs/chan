// The screen state: which side of the launcher's main area shows, the
// forward-only flip counter, and the select-mode drop on every flip (no stale
// selection or bulk bar survives on the other screen).

import { describe, expect, test, beforeEach } from "vitest";
import { screen, showScreen, toggleScreen } from "./screen.svelte";
import { selection, setSelectMode, toggleSelected } from "./selection.svelte";

beforeEach(() => {
  screen.current = "computers";
  screen.flips = 0;
  setSelectMode(false);
});

describe("launcher screen state", () => {
  test("toggling flips between the two screens, forward-only", () => {
    toggleScreen();
    expect(screen.current).toBe("gateways");
    expect(screen.flips).toBe(1);
    toggleScreen();
    expect(screen.current).toBe("computers");
    expect(screen.flips).toBe(2);
  });

  test("showing the current screen again is a no-op", () => {
    showScreen("computers");
    expect(screen.current).toBe("computers");
    expect(screen.flips).toBe(0);
  });

  test("flipping drops select mode and the selection", () => {
    setSelectMode(true);
    toggleSelected("devserver", "ds-1");
    toggleScreen();
    expect(selection.selectMode).toBe(false);
    expect(selection.selected).toEqual([]);
  });
});
