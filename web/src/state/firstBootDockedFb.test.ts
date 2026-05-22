import { describe, expect, test } from "vitest";
import app from "../App.svelte?raw";
import store from "./store.svelte.ts?raw";

// `fullstack-a-88`: first-boot UX swap.
//
// Pre-fix: App.svelte spawned an FB tab when the layout came
// up empty (`if (!hasAnyTab) openBrowser();`), and
// `browserSidePanes` defaulted to `{left: false, right: false}`.
//
// Post-fix: the App.svelte tab-spawn is removed; the docked FB
// defaults to LEFT-docked. chan-server's
// `BrowserSidePanes::default()` mirrors this so a fresh
// preferences.toml lands with `left: true`. Existing user
// preferences override the default via the normal load path.

describe("fullstack-a-88: App.svelte first-boot FB-tab spawn removed", () => {
  test("App.svelte no longer calls openBrowser() in the empty-layout branch", () => {
    expect(app).not.toMatch(/if \(!hasAnyTab\) openBrowser\(\)/);
  });

  test("App.svelte no longer imports openBrowser (only spawn-context users left)", () => {
    // App.svelte imports a destructured block from
    // ../state/store.svelte; openBrowser should be gone after
    // the first-boot rule's removal.
    expect(app).not.toMatch(/^\s+openBrowser,\s*$/m);
  });

  test("App.svelte references `fullstack-a-88` in the replacement comment", () => {
    expect(app).toMatch(/`fullstack-a-88`/);
    expect(app).toMatch(/docked FB on left by[\s\S]{1,40}default/i);
  });
});

describe("fullstack-a-88: browserSidePanes default is left-docked", () => {
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
