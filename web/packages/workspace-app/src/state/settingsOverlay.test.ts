// The settings surface's overlay state: its open/close/toggle helpers
// and its participation in the shared overlay stack (z-order + the
// single-Escape close). Pure state, no rendering.

import { afterEach, describe, expect, test } from "vitest";
import {
  closeOverlay,
  closeSettings,
  launcherPanel,
  openSettings,
  overlayDepth,
  overlayStack,
  searchPanel,
  settingsPanel,
  syncOverlayStack,
  toggleSettings,
  topOverlay,
} from "./store.svelte";

afterEach(() => {
  settingsPanel.open = false;
  launcherPanel.open = false;
  searchPanel.open = false;
  overlayStack.ids = [];
});

describe("settings overlay state", () => {
  test("open / close / toggle drive settingsPanel.open", () => {
    expect(settingsPanel.open).toBe(false);
    openSettings();
    expect(settingsPanel.open).toBe(true);
    closeSettings();
    expect(settingsPanel.open).toBe(false);
    toggleSettings();
    expect(settingsPanel.open).toBe(true);
    toggleSettings();
    expect(settingsPanel.open).toBe(false);
  });

  test("syncOverlayStack tracks settings; topOverlay + closeOverlay agree", () => {
    openSettings();
    syncOverlayStack();
    expect(overlayDepth("settings")).toBeGreaterThanOrEqual(0);
    expect(topOverlay()).toBe("settings");

    closeOverlay("settings");
    expect(settingsPanel.open).toBe(false);
    syncOverlayStack();
    expect(overlayDepth("settings")).toBe(-1);
    expect(topOverlay()).toBeNull();
  });

  test("an overlay opened over settings sits on top (Escape closes it first)", () => {
    // App.svelte syncs the stack on every .open change, so an overlay
    // opened after settings is appended above it.
    openSettings();
    syncOverlayStack();
    launcherPanel.open = true;
    syncOverlayStack();
    expect(overlayDepth("settings")).toBe(0);
    expect(overlayDepth("launcher")).toBe(1);
    expect(topOverlay()).toBe("launcher");
  });
});
