import { describe, expect, test } from "vitest";
import app from "../App.svelte?raw";
import store from "./store.svelte.ts?raw";

// First-boot File Browser UX: App.svelte does NOT spawn an FB tab on an
// empty layout; the docked File Browser defaults to left-docked instead.
// chan-server's BrowserSidePanes::default() mirrors this so a fresh
// preferences.toml lands with left: true; existing user preferences
// override it via the normal load path.

describe("App.svelte first-boot FB-tab spawn removed", () => {
  test("App.svelte no longer calls openBrowser() in the empty-layout branch", () => {
    expect(app).not.toMatch(/if \(!hasAnyTab\) openBrowser\(\)/);
  });

  test("App.svelte no longer imports openBrowser", () => {
    expect(app).not.toMatch(/^\s+openBrowser,\s*$/m);
  });
});

describe("browserSidePanes default is left-docked", () => {
  test("SPA default is {left: true, right: false}", () => {
    expect(store).toMatch(
      /export const browserSidePanes = \$state[\s\S]*?left: true,[\s\S]*?right: false/,
    );
  });

  test("rationale comment cites the chan-server side mirror", () => {
    expect(store).toMatch(
      /chan-server's `BrowserSidePanes::default\(\)` matches this/i,
    );
  });
});
