import { describe, expect, test } from "vitest";
import settings from "./SettingsPanel.svelte?raw";

describe("Wave 2: Settings feature ownership", () => {
  test("global Settings no longer renders the Features quick-toggle section", () => {
    expect(settings).not.toMatch(/<section class="features">/);
    expect(settings).not.toMatch(/<h3>Features<\/h3>/);
  });

  test("chan-reports controls are absent from SettingsPanel", () => {
    expect(settings).not.toContain("chan-reports");
    expect(settings).not.toMatch(/api\.reports(State|Enable|Disable)\(/);
    expect(settings).not.toMatch(/toggleReports/);
    expect(settings).not.toMatch(/reportsEnabled|reportsBusy|reportsError/);
  });

  test("BGE semantic search controls are absent from SettingsPanel", () => {
    expect(settings).not.toContain("BGE semantic search");
    expect(settings).not.toMatch(/SemanticState/);
    expect(settings).not.toMatch(/api\.semantic(State|Enable|Disable|Download)\(/);
    expect(settings).not.toMatch(/toggleSemantic/);
    expect(settings).not.toMatch(/semanticState|semanticBusy|semanticError/);
  });
});

describe("Wave 2: Settings screen lock remains global", () => {
  test("screen lock section remains in SettingsPanel", () => {
    expect(settings).toMatch(/<section class="screen-lock">[\s\S]*?<h3>Screen lock<\/h3>/);
    expect(settings).toMatch(/api\.screensaverState\(\)/);
    expect(settings).toMatch(/api\.screensaverPatch/);
  });

  test("Settings mounts screen-lock state loader", () => {
    expect(settings).toMatch(
      /onMount\(\(\) => \{[\s\S]*?void loadScreenLockState\(\);[\s\S]*?\}\);/,
    );
  });
});

describe("Settings About attributions", () => {
  test("Matrix screen lock credits the upstream visual reference", () => {
    expect(settings).toContain("dcragusa/MatrixScreensaver");
    expect(settings).toContain("https://github.com/dcragusa/MatrixScreensaver");
    expect(settings).toMatch(/matrix screen lock[\s\S]*?MIT/);
  });
});
