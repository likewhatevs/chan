// @vitest-environment jsdom

import { afterEach, describe, expect, test, vi } from "vitest";
import { __testSystemTheme } from "./store.svelte";

// When the OS appearance cannot be determined (e.g. headless linux), the
// `system` theme must resolve to dark. "Undeterminable" means matchMedia is
// unavailable OR neither prefers-color-scheme query matches. An explicit
// light/dark preference is still honoured.

function matchMediaStub(opts: { dark: boolean; light: boolean }) {
  return (query: string) =>
    ({
      matches:
        query === "(prefers-color-scheme: dark)"
          ? opts.dark
          : query === "(prefers-color-scheme: light)"
            ? opts.light
            : false,
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

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("systemTheme headless fallback", () => {
  test("dark when matchMedia is unavailable", () => {
    vi.stubGlobal("matchMedia", undefined);
    expect(__testSystemTheme()).toBe("dark");
  });

  test("dark when neither query matches (OS appearance undeterminable)", () => {
    vi.stubGlobal("matchMedia", matchMediaStub({ dark: false, light: false }));
    expect(__testSystemTheme()).toBe("dark");
  });

  test("dark when the OS explicitly prefers dark", () => {
    vi.stubGlobal("matchMedia", matchMediaStub({ dark: true, light: false }));
    expect(__testSystemTheme()).toBe("dark");
  });

  test("light only when the OS explicitly prefers light", () => {
    vi.stubGlobal("matchMedia", matchMediaStub({ dark: false, light: true }));
    expect(__testSystemTheme()).toBe("light");
  });
});
