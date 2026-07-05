// Clobber-safety source pins for the settings surface. This is an
// additive round: the settings form and the back-of-pane cards both
// write the same config block. A whole-block PATCH built from the
// form's buffer would drop a field a concurrent card save just landed,
// so every write must be a single-field slice through the shared serial
// config-write chain (which re-reads the latest config and overlays
// only that slice). These pins guard that invariant.

import { describe, expect, test } from "vitest";
import overlaySource from "./SettingsOverlay.svelte?raw";
import appearanceSource from "./settings/AppearanceSection.svelte?raw";

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
});
