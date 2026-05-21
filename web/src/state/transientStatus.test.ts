// @vitest-environment jsdom

import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import { notify } from "./notify.svelte";
import { setTransientStatus, ui } from "./store.svelte";

beforeEach(() => {
  // Reset between tests; store.svelte.ts default is null/null.
  ui.status = null;
  ui.statusKind = null;
  vi.useFakeTimers();
});

afterEach(() => {
  vi.useRealTimers();
  ui.status = null;
  ui.statusKind = null;
});

describe("setTransientStatus", () => {
  test("sets ui.status + statusKind=transient and clears after the default window", () => {
    setTransientStatus("Copied path");
    expect(ui.status).toBe("Copied path");
    expect(ui.statusKind).toBe("transient");

    vi.advanceTimersByTime(2999);
    expect(ui.status).toBe("Copied path");

    vi.advanceTimersByTime(2);
    expect(ui.status).toBeNull();
    expect(ui.statusKind).toBeNull();
  });

  test("a newer transient stomps the prior timer (latest wins)", () => {
    setTransientStatus("Older");
    vi.advanceTimersByTime(1000);
    setTransientStatus("Newer");
    expect(ui.status).toBe("Newer");

    // The old 3-s timer would have fired here; verify it didn't
    // clear our "Newer" message.
    vi.advanceTimersByTime(2001);
    expect(ui.status).toBe("Newer");

    // Newer's own timer fires.
    vi.advanceTimersByTime(1000);
    expect(ui.status).toBeNull();
  });

  test("a direct persistent ui.status write during the window is NOT clobbered by the transient timer", () => {
    setTransientStatus("Copied path");
    vi.advanceTimersByTime(500);

    // Simulate a persistent write (e.g. "Moving…") landing
    // mid-flight.
    ui.status = "Moving…";
    ui.statusKind = "persistent";

    vi.advanceTimersByTime(5000);
    expect(ui.status).toBe("Moving…");
    expect(ui.statusKind).toBe("persistent");
  });

  test("respects custom ms argument", () => {
    setTransientStatus("Quick flash", 1000);
    vi.advanceTimersByTime(999);
    expect(ui.status).toBe("Quick flash");
    vi.advanceTimersByTime(2);
    expect(ui.status).toBeNull();
  });
});

describe("notify() routes through setTransientStatus", () => {
  test("notify() writes auto-dismiss on the same default window", () => {
    notify("Inline notification");
    expect(ui.status).toBe("Inline notification");
    expect(ui.statusKind).toBe("transient");

    vi.advanceTimersByTime(3001);
    expect(ui.status).toBeNull();
    expect(ui.statusKind).toBeNull();
  });
});
