import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";

// The launcher's `system` fallback must resolve to dark when the OS
// appearance cannot be determined (headless linux), matching the main SPA.
// "Undeterminable" = matchMedia unavailable OR prefers-color-scheme: light
// does not match. A stored choice always wins. `initialTheme()` runs at
// module load, so each scenario re-imports the module with matchMedia /
// localStorage primed first.

function matchMediaStub(opts: { light: boolean }) {
  return (query: string) =>
    ({
      matches: query === "(prefers-color-scheme: light)" ? opts.light : false,
      media: query,
      onchange: null,
      addEventListener() {},
      removeEventListener() {},
      addListener() {},
      removeListener() {},
      dispatchEvent() {
        return false;
      },
    }) as unknown as MediaQueryList;
}

beforeEach(() => {
  localStorage.clear();
  vi.resetModules();
});

afterEach(() => {
  vi.unstubAllGlobals();
  localStorage.clear();
});

describe("launcher initial theme headless fallback", () => {
  test("dark when matchMedia is unavailable", async () => {
    vi.stubGlobal("matchMedia", undefined);
    const { themeState } = await import("./theme.svelte");
    expect(themeState.theme).toBe("dark");
  });

  test("dark when prefers-color-scheme: light does not match (undeterminable)", async () => {
    vi.stubGlobal("matchMedia", matchMediaStub({ light: false }));
    const { themeState } = await import("./theme.svelte");
    expect(themeState.theme).toBe("dark");
  });

  test("light only when the OS explicitly prefers light", async () => {
    vi.stubGlobal("matchMedia", matchMediaStub({ light: true }));
    const { themeState } = await import("./theme.svelte");
    expect(themeState.theme).toBe("light");
  });

  test("a stored choice wins over the system fallback", async () => {
    localStorage.setItem("chan-launcher-theme", "light");
    vi.stubGlobal("matchMedia", undefined);
    const { themeState } = await import("./theme.svelte");
    expect(themeState.theme).toBe("light");
  });
});
