// Component test: the devserver registry renders a Connect button that, on the
// mutable (desktop loopback) surface, is enabled and fires the connect action.
// jsdom has no read-only meta tag, so `readOnly` is false here — the mutable
// surface. Exercises the real Svelte 5 runtime (a static check misses the
// reactive feed re-render after the connect push), per jsdom.

import { describe, it, expect, afterEach, beforeEach, vi } from "vitest";
import { mount, unmount, flushSync } from "svelte";
import DevserverList from "./DevserverList.svelte";
import { library, loadLibrary } from "../state/library.svelte";

// Pin the in-memory mock as the backend so the list renders the seed devserver
// with no live server. The async-import factory dodges vi.mock's hoist trap.
vi.mock("../api/backend", async () => {
  const { mockApi } = await import("../api/mock");
  return { backend: mockApi };
});

let target: HTMLElement | null = null;
let app: Record<string, unknown> | null = null;

beforeEach(async () => {
  await loadLibrary();
});

afterEach(() => {
  if (app) unmount(app);
  target?.remove();
  target = null;
  app = null;
});

describe("DevserverList Connect", () => {
  it("renders an enabled Connect button on the mutable surface and fires connect", async () => {
    target = document.createElement("div");
    document.body.appendChild(target);
    app = mount(DevserverList, { target });

    const connect = [...target.querySelectorAll("button")].find(
      (b) => b.textContent?.trim() === "Connect",
    ) as HTMLButtonElement | undefined;
    expect(connect).toBeTruthy();
    expect(connect!.disabled).toBe(false);

    const ds = library.devservers.find((d) => d.library_id)!;
    connect!.click();
    await Promise.resolve();
    flushSync();
    // The mock marks that library's windows connected and pushes the feed; no
    // error surfaces on the happy path.
    expect(library.error).toBeNull();
    const remote = library.windows.filter((w) => w.library_id === ds.library_id);
    expect(remote.length).toBeGreaterThan(0);
    expect(remote.every((w) => w.connected)).toBe(true);
  });
});
