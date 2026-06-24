// Component test: the redesigned devserver registry rows. A connected devserver
// shows Disconnect + an enabled New-terminal action; a disconnected one shows
// Connect, which fires the connect action. jsdom has no read-only meta tag, so
// `readOnly` is false here — the mutable surface. Exercises the real Svelte 5
// runtime (a static check misses the reactive re-render after connect), per jsdom.

import { describe, it, expect, afterEach, beforeEach, vi } from "vitest";
import { mount, unmount, flushSync } from "svelte";
import DevserverList from "./DevserverList.svelte";
import { library, loadLibrary, saveDevserver } from "../state/library.svelte";
import { clearSelection } from "../state/selection.svelte";
import { beginPending, clearAllPending, dsKey, isPending } from "../state/pending.svelte";
import type { DevserverEntry } from "../api/library";

// Pin the in-memory mock as the backend so the list renders the seed devserver
// with no live server. The async-import factory dodges vi.mock's hoist trap.
vi.mock("../api/backend", async () => {
  const { mockApi } = await import("../api/mock");
  return { backend: mockApi };
});

let target: HTMLElement | null = null;
let app: Record<string, unknown> | null = null;

function mountList(): void {
  target = document.createElement("div");
  document.body.appendChild(target);
  app = mount(DevserverList, { target });
}

function byAria(prefix: string): HTMLButtonElement | undefined {
  return [...(target?.querySelectorAll("button[aria-label]") ?? [])].find((b) =>
    (b.getAttribute("aria-label") ?? "").startsWith(prefix),
  ) as HTMLButtonElement | undefined;
}

beforeEach(async () => {
  clearSelection();
  clearAllPending();
  await loadLibrary();
});

afterEach(() => {
  if (app) unmount(app);
  target?.remove();
  target = null;
  app = null;
  clearSelection();
  clearAllPending();
});

describe("DevserverList redesign", () => {
  it("shows Disconnect + an enabled New terminal for a connected devserver", () => {
    // The seed devserver "prod" is connected.
    mountList();
    expect(byAria("Disconnect prod")).toBeTruthy();
    const newTerm = byAria("New terminal on prod");
    expect(newTerm).toBeTruthy();
    expect(newTerm!.disabled).toBe(false);
    // A select checkbox feeds the bulk bar; the per-row Remove is gone.
    expect(target!.querySelector('input[type="checkbox"]')).toBeTruthy();
    // The row endpoint is rendered as host:port.
    expect(target!.querySelector(".row-sub")?.textContent).toBe("box.example.com:8787");
  });

  it("shows Connect (New terminal disabled) for a disconnected devserver and fires connect", async () => {
    // A freshly added devserver starts disconnected.
    await saveDevserver({ host: "fresh.example", port: 9100, label: "fresh" });
    mountList();

    const connect = byAria("Connect fresh");
    expect(connect).toBeTruthy();
    expect(connect!.disabled).toBe(false);
    // New terminal can't open until connected.
    expect(byAria("New terminal on fresh")!.disabled).toBe(true);

    connect!.click();
    // connect → backend tick → refreshDevservers → listDevservers tick is a
    // few microtask hops; a macrotask boundary drains them before asserting.
    await new Promise((r) => setTimeout(r, 0));
    flushSync();

    expect(library.error).toBeNull();
    const fresh = library.devservers.find((d) => d.host === "fresh.example" && d.port === 9100)!;
    expect(fresh.status).toBe("connected");
    // The row flips to Disconnect after connecting.
    expect(byAria("Disconnect fresh")).toBeTruthy();
  });

  it("swaps Connect/Disconnect for a disabled spinner while the devserver op is pending", () => {
    // The seed devserver "prod" (ds-1) is connected → shows Disconnect.
    mountList();
    expect(byAria("Disconnect prod")).toBeTruthy();

    // Begin a pending marker → the toggle becomes a disabled "Working on" spinner
    // (real Svelte reactivity), and the Disconnect action is gone meanwhile.
    beginPending(dsKey("ds-1"), "disconnected");
    flushSync();
    const spinning = byAria("Working on prod");
    expect(spinning).toBeTruthy();
    expect(spinning!.disabled).toBe(true);
    expect(spinning!.querySelector("svg.spin")).toBeTruthy();
    expect(byAria("Disconnect prod")).toBeUndefined();
  });

  it("spins from status:connecting alone, then clears the spinner on disconnect", () => {
    // Drive the spinner purely from the backend lifecycle status — no marker.
    mountList();
    expect(isPending(dsKey("ds-1"))).toBe(false);

    library.devservers = library.devservers.map(
      (d): DevserverEntry => (d.id === "ds-1" ? { ...d, status: "connecting" } : d),
    );
    flushSync();
    const spinning = byAria("Working on prod");
    expect(spinning).toBeTruthy();
    expect(spinning!.disabled).toBe(true);
    expect(byAria("Disconnect prod")).toBeUndefined();

    // A dropped tunnel lands `disconnected` on the next refetch: the spinner
    // clears with no manual reload and the row offers Connect again.
    library.devservers = library.devservers.map(
      (d): DevserverEntry => (d.id === "ds-1" ? { ...d, status: "disconnected" } : d),
    );
    flushSync();
    expect(byAria("Working on prod")).toBeUndefined();
    expect(byAria("Connect prod")).toBeTruthy();
  });
});
