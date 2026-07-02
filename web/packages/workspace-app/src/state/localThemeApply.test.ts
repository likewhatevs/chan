// @vitest-environment jsdom

// applyLocalTheme lets a local standalone terminal window follow the launcher's
// light/dark choice pushed over the local-theme watch: an explicit value locks
// that mode; null restores OS-follow. Reuses the same setThemeLocal path the
// config theme uses, with no write-back.

import { afterEach, describe, expect, test, vi } from "vitest";
import { applyLocalTheme, ui } from "./store.svelte";

afterEach(() => {
  vi.unstubAllGlobals();
  ui.themeChoice = "system";
});

describe("applyLocalTheme", () => {
  test("light locks the window to light", () => {
    applyLocalTheme("light");
    expect(ui.themeChoice).toBe("light");
    expect(ui.theme).toBe("light");
    expect(document.documentElement.getAttribute("data-theme")).toBe("light");
  });

  test("dark locks the window to dark", () => {
    applyLocalTheme("dark");
    expect(ui.themeChoice).toBe("dark");
    expect(ui.theme).toBe("dark");
  });

  test("null restores OS-follow (system), tracking the OS preference", () => {
    vi.stubGlobal(
      "matchMedia",
      (q: string) =>
        ({
          matches: q === "(prefers-color-scheme: light)",
          media: q,
          onchange: null,
          addEventListener() {},
          removeEventListener() {},
          addListener() {},
          removeListener() {},
          dispatchEvent() {
            return false;
          },
        }) as unknown as MediaQueryList,
    );
    applyLocalTheme(null);
    expect(ui.themeChoice).toBe("system");
    expect(ui.theme).toBe("light");
  });
});
