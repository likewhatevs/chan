// Clobber-safety source pins for the settings surface. A whole-block PATCH built
// from the form's buffer would drop a field a concurrent config write just
// landed, so every write must be a single-field slice through the shared serial
// config-write chain (which re-reads the latest config and overlays only that
// slice). These pins guard that invariant.

import { describe, expect, test } from "vitest";
import overlaySource from "./SettingsOverlay.svelte?raw";
import appearanceSource from "./settings/AppearanceSection.svelte?raw";
import terminalSource from "./settings/TerminalSection.svelte?raw";

describe("SettingsOverlay config writes (source pins)", () => {
  test("the default write path routes through updateGlobalConfigSerial", () => {
    expect(overlaySource).toMatch(
      /updateGlobalConfigSerial\(\(prefs\) => mutate\(prefs\)\)/,
    );
  });

  test("it never whole-block PATCHes the config directly", () => {
    expect(overlaySource).not.toMatch(/api\.updateConfig\(/);
  });

  test("the theme control reuses the store's setThemeChoice setter", () => {
    expect(appearanceSource).toMatch(/setThemeChoice\(/);
  });

  test("the per-surface theme control reuses the store set + clear setters", () => {
    // Light/Dark apply live + persist via setHybridSurfaceTheme; Inherit
    // drops the key via clearHybridSurfaceTheme, mirroring how the theme
    // control reuses setThemeChoice rather than a raw whole-block PATCH.
    expect(appearanceSource).toMatch(/setHybridSurfaceTheme\(/);
    expect(appearanceSource).toMatch(/clearHybridSurfaceTheme\(/);
  });

  test("the terminal font control downloads Source Code Pro before persist", () => {
    // The download endpoint is fired so the preference is only committed
    // after the woff2 lands (matching the terminal card invariant).
    expect(terminalSource).toMatch(/api\.fontsSourceCodeProDownload\(\)/);
  });
});
