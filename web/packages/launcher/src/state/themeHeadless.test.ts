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

function localStorageStub(): Storage {
  let entries: Record<string, string> = {};
  return {
    get length() {
      return Object.keys(entries).length;
    },
    clear() {
      entries = {};
    },
    getItem(key: string) {
      return entries[key] ?? null;
    },
    key(index: number) {
      return Object.keys(entries)[index] ?? null;
    },
    removeItem(key: string) {
      delete entries[key];
    },
    setItem(key: string, value: string) {
      entries[key] = value;
    },
  };
}

beforeEach(() => {
  if (!globalThis.localStorage) vi.stubGlobal("localStorage", localStorageStub());
  localStorage.clear();
  vi.resetModules();
});

afterEach(() => {
  localStorage.clear();
  vi.unstubAllGlobals();
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

describe("launcher local-theme desktop sync", () => {
  test("toggleTheme mirrors the new choice to the desktop config", async () => {
    vi.stubGlobal("matchMedia", matchMediaStub({ light: false })); // start dark
    const fetchMock = vi.fn(async () => new Response(null, { status: 204 }));
    vi.stubGlobal("fetch", fetchMock);
    const { themeState, toggleTheme } = await import("./theme.svelte");
    expect(themeState.theme).toBe("dark");

    toggleTheme();

    expect(themeState.theme).toBe("light");
    // The PUT fires synchronously (fire-and-forget) with the new theme.
    expect(fetchMock).toHaveBeenCalledWith(
      "/api/library/local-theme",
      expect.objectContaining({ method: "PUT", body: JSON.stringify({ theme: "light" }) }),
    );
  });

  test("toggleTheme still flips when the PUT fails (best-effort)", async () => {
    vi.stubGlobal("matchMedia", matchMediaStub({ light: false }));
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => {
        throw new Error("no store");
      }),
    );
    const { themeState, toggleTheme } = await import("./theme.svelte");
    toggleTheme();
    expect(themeState.theme).toBe("light");
  });

  test("reconcileLocalTheme adopts the authoritative config value", async () => {
    vi.stubGlobal("matchMedia", matchMediaStub({ light: false })); // boots dark
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => new Response(JSON.stringify({ theme: "light" }), { status: 200 })),
    );
    const { themeState, reconcileLocalTheme } = await import("./theme.svelte");
    expect(themeState.theme).toBe("dark");
    await reconcileLocalTheme();
    expect(themeState.theme).toBe("light");
  });

  test("reconcileLocalTheme keeps the current theme when the config is null", async () => {
    vi.stubGlobal("matchMedia", matchMediaStub({ light: false }));
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => new Response(JSON.stringify({ theme: null }), { status: 200 })),
    );
    const { themeState, reconcileLocalTheme } = await import("./theme.svelte");
    await reconcileLocalTheme();
    expect(themeState.theme).toBe("dark");
  });
});
