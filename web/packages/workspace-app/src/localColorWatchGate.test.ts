import { describe, expect, test } from "vitest";
import app from "./App.svelte?raw";

// The per-library focus-colour watch route lives only on the root launcher
// router (desktop embedded host / headless devserver). A standalone
// `chan open` server never mounts it, so an unconditional subscription
// 404s the WS handshake into an endless 500ms -> 8s reconnect loop. The
// SPA must open the socket only on a launcher-hosted (desktop) surface.

describe("local-color watch surface gate", () => {
  test("openLocalColorWatch subscribes only under the desktop host", () => {
    expect(app).toMatch(
      /if \(isTauriDesktop\(\)\) \{[\s\S]{0,400}disposeLocalColorWatch = openLocalColorWatch\(/,
    );
  });

  test("no unconditional (standalone-surface) subscription remains", () => {
    // Exactly one call site, and it is the gated one above.
    const calls = app.match(/openLocalColorWatch\(/g) ?? [];
    expect(calls.length).toBe(1);
    expect(app).toMatch(
      /import \{[^}]*openLocalColorWatch[^}]*\} from "\.\/api\/client";/,
    );
  });

  test("the theme-watch twin keeps its terminal-only gate", () => {
    expect(app).toMatch(
      /if \(ui\.terminalOnly\) \{[\s\S]{0,300}disposeLocalThemeWatch = openLocalThemeWatch\(/,
    );
  });
});
