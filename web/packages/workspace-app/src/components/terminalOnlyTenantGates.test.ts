import { describe, expect, test } from "vitest";
import app from "../App.svelte?raw";

// Terminal-only windows are served by the slim terminal tenant, which has no
// workspace: it mounts neither /api/preflight (workspace onboarding) nor the
// /api/screensaver routes (per-workspace config). The SPA must not call
// workspace-concept endpoints there -- before these gates, every terminal
// window logged a 404 for each at boot (local and devserver alike).
//
// Static `?raw` source-pin (repo convention): pins both gates so an
// unconditional mount/call can't silently creep back. Real behaviour is
// browser-smoked.
describe("terminal-only windows skip workspace-concept endpoints", () => {
  test("PreflightOverlay mounts only outside terminal-only mode", () => {
    expect(app).toMatch(/\{#if !ui\.terminalOnly\}\s*<PreflightOverlay \/>/);
    expect(app).not.toMatch(/^<PreflightOverlay \/>$/m);
  });

  test("screensaver state loads only outside terminal-only mode", () => {
    expect(app).toMatch(
      /if \(!ui\.terminalOnly\) \{\s*void loadScreensaverState\(\);/,
    );
  });
});
