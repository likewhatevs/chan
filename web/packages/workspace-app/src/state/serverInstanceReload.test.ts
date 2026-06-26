import { describe, expect, test } from "vitest";
import storeSource from "./store.svelte.ts?raw";

// A remote `chan devserver` bouncing (^C + re-run) used to leave its
// desktop window stale: the watch socket reconnected fine, but the new
// process had none of the old PTYs, so terminals sat stuck until a
// manual Cmd+R. The store now reads /api/health's `instance` (random
// per-process id) on every watch-socket (re)connect and reloads the
// window when it changed. These pins lock that wiring.
describe("server-restart auto-reload", () => {
  const src = storeSource.replace(/\s+/g, " ");

  test("every watch (re)connect checks the server instance", () => {
    expect(src).toMatch(
      /function onWatchReady\(\): void \{.*?void checkServerInstance\(\);/,
    );
  });

  test("a changed instance reloads the window; the first read only seeds", () => {
    expect(src).toContain("const instance = (await api.health()).instance?.trim()");
    expect(src).toMatch(
      /if \(serverInstance === null\) \{ serverInstance = instance; return; \}/,
    );
    expect(src).toMatch(
      /if \(serverInstance !== instance\) \{ window\.location\.reload\(\); \}/,
    );
  });
});
