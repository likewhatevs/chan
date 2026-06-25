// Smoke test: the launcher root mounts and renders its top bar with the
// theme toggle and the New-workspace button. Registry/feed rendering loads
// asynchronously from the backend and is covered by the state + component
// tests; this keeps the mount path itself green. Also covers the error banner's
// dismiss [X] (clearError) — a real component mount, since a banner with no way
// to clear it short of a reload was the reported bug.

import { describe, it, expect, afterEach, vi } from "vitest";
import { mount, unmount, flushSync } from "svelte";
import App from "./App.svelte";
import { library } from "./state/library.svelte";

// Pin the in-memory mock as the backend so loadLibrary succeeds (no spurious
// error banner from a failed fetch) and the banner test controls library.error.
vi.mock("./api/backend", async () => {
  const { mockApi } = await import("./api/mock");
  return { backend: mockApi };
});

// A macrotask hop lets the onMount loadLibrary fully settle before we assert.
function settle(): Promise<void> {
  return new Promise((r) => setTimeout(r, 0));
}

describe("launcher root", () => {
  let target: HTMLElement | null = null;
  let app: Record<string, unknown> | null = null;

  afterEach(() => {
    if (app) unmount(app);
    target?.remove();
    target = null;
    app = null;
    library.error = null;
  });

  it("renders the top bar and its actions", () => {
    target = document.createElement("div");
    document.body.appendChild(target);
    app = mount(App, { target });

    expect(target.querySelector(".topbar")).not.toBeNull();
    expect(target.querySelector('[aria-label="New workspace"]')).not.toBeNull();
    expect(target.querySelector('[aria-label="Toggle theme"]')).not.toBeNull();
    expect(target.querySelector('[aria-label="Open terminal"]')).not.toBeNull();
  });

  it("shows a dismissable error banner that the [X] clears (no reload needed)", async () => {
    target = document.createElement("div");
    document.body.appendChild(target);
    app = mount(App, { target });
    // Let the mock loadLibrary settle (it nulls error on success), then inject a
    // banner-worthy error the way a failed action would.
    await settle();
    flushSync();

    library.error = "the control terminal was closed before the devserver connected";
    flushSync();

    const banner = target.querySelector('.banner[role="alert"]');
    expect(banner).not.toBeNull();
    expect(banner?.textContent).toContain("control terminal");
    const dismiss = target.querySelector('button[aria-label="Dismiss"]') as HTMLButtonElement;
    expect(dismiss).toBeTruthy();

    dismiss.click();
    flushSync();
    expect(library.error).toBeNull();
    expect(target.querySelector('.banner[role="alert"]')).toBeNull();
  });
});
