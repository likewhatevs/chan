import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";

import {
  isTauriDesktop,
  openWebInspector,
  reloadWindow,
  tauriInvoke,
} from "./desktop";

type W = Window & typeof globalThis & {
  __TAURI__?: unknown;
  __TAURI_INTERNALS__?: unknown;
};

function clearTauriGlobals(): void {
  delete (window as W).__TAURI__;
  delete (window as W).__TAURI_INTERNALS__;
}

function setTauriInternals(invoke: (cmd: string, args?: unknown) => Promise<unknown>): void {
  Object.defineProperty(window, "__TAURI_INTERNALS__", {
    value: { invoke },
    configurable: true,
  });
}

describe("isTauriDesktop", () => {
  afterEach(clearTauriGlobals);

  test("returns false when neither global is set (web build)", () => {
    expect(isTauriDesktop()).toBe(false);
  });

  test("returns true when __TAURI__ is present (old Tauri runtime)", () => {
    Object.defineProperty(window, "__TAURI__", { value: {}, configurable: true });
    expect(isTauriDesktop()).toBe(true);
  });

  test("returns true when __TAURI_INTERNALS__ is present (Tauri 2 runtime)", () => {
    Object.defineProperty(window, "__TAURI_INTERNALS__", { value: {}, configurable: true });
    expect(isTauriDesktop()).toBe(true);
  });
});

describe("tauriInvoke", () => {
  afterEach(clearTauriGlobals);

  test("throws when no Tauri runtime is present", async () => {
    await expect(tauriInvoke("anything")).rejects.toThrow(/not running under Tauri/);
  });

  test("dispatches via __TAURI_INTERNALS__.invoke", async () => {
    const spy = vi.fn().mockResolvedValue("ok");
    setTauriInternals(spy);
    await expect(tauriInvoke("ping")).resolves.toBe("ok");
    expect(spy).toHaveBeenCalledWith("ping", undefined);
  });
});

describe("reloadWindow dispatch", () => {
  let reloadSpy: ReturnType<typeof vi.fn>;
  let originalLocation: Location;

  beforeEach(() => {
    reloadSpy = vi.fn();
    originalLocation = window.location;
    // jsdom's `window.location.reload` is non-configurable, so swap
    // the whole `location` object instead of patching the field.
    Object.defineProperty(window, "location", {
      value: { ...originalLocation, reload: reloadSpy },
      configurable: true,
      writable: true,
    });
  });

  afterEach(() => {
    clearTauriGlobals();
    Object.defineProperty(window, "location", {
      value: originalLocation,
      configurable: true,
      writable: true,
    });
  });

  test("falls back to window.location.reload() on web", async () => {
    await reloadWindow();
    expect(reloadSpy).toHaveBeenCalledTimes(1);
  });

  test("invokes reload_window IPC on chan-desktop", async () => {
    const invokeSpy = vi.fn().mockResolvedValue(undefined);
    setTauriInternals(invokeSpy);
    await reloadWindow();
    expect(invokeSpy).toHaveBeenCalledWith("reload_window", undefined);
    expect(reloadSpy).not.toHaveBeenCalled();
  });

  test("falls back to window.location.reload() when reload_window IPC throws", async () => {
    const invokeSpy = vi.fn().mockRejectedValue(new Error("ipc fail"));
    setTauriInternals(invokeSpy);
    const consoleWarn = vi.spyOn(console, "warn").mockImplementation(() => {});
    await reloadWindow();
    expect(invokeSpy).toHaveBeenCalledWith("reload_window", undefined);
    expect(reloadSpy).toHaveBeenCalledTimes(1);
    consoleWarn.mockRestore();
  });
});

describe("openWebInspector dispatch", () => {
  afterEach(clearTauriGlobals);

  test("returns false on web (no Tauri runtime)", async () => {
    await expect(openWebInspector()).resolves.toBe(false);
  });

  test("invokes open_devtools IPC and returns true on chan-desktop", async () => {
    const invokeSpy = vi.fn().mockResolvedValue(undefined);
    setTauriInternals(invokeSpy);
    await expect(openWebInspector()).resolves.toBe(true);
    expect(invokeSpy).toHaveBeenCalledWith("open_devtools", undefined);
  });

  test("returns false when open_devtools IPC throws", async () => {
    const invokeSpy = vi.fn().mockRejectedValue(new Error("ipc fail"));
    setTauriInternals(invokeSpy);
    const consoleWarn = vi.spyOn(console, "warn").mockImplementation(() => {});
    await expect(openWebInspector()).resolves.toBe(false);
    consoleWarn.mockRestore();
  });
});
