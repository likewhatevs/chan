// Read-only surface (gateway/devserver, no desktop bridge): the Library shows
// the on-state statically with NO mutation controls -- no new-workspace /
// new-terminal, no add-devserver, no checkboxes, no on/off or open-window, no
// connect/disconnect, and no edit-config affordance (the devserver header is a
// static identity, not a click target). A card can still expand to read its
// windows (static rows with the connection dot). `readOnly` is a boot-time
// const, so it is pinned via a module mock for the whole file. The mutable
// surface is covered in Library.test.ts.

import { describe, it, expect, afterEach, beforeEach, vi } from "vitest";
import { mount, unmount } from "svelte";
import Library from "./Library.svelte";
import { loadLibrary } from "../state/library.svelte";

// Force the read-only surface for the whole file (hoisted before the imports):
// no registry mutation, no desktop bridge, not self-managed.
vi.mock("../state/capabilities", () => ({
  readOnly: true,
  canMutateRegistry: false,
  hasDesktopBridge: false,
  selfManagedWindows: false,
  hostOs: "linux",
}));

vi.mock("../api/backend", async () => {
  const { mockApi } = await import("../api/mock");
  return { backend: mockApi };
});

let target: HTMLElement | null = null;
let app: Record<string, unknown> | null = null;

function mountList(): void {
  target = document.createElement("div");
  document.body.appendChild(target);
  app = mount(Library, { target });
}

function labels(): string[] {
  return [...(target?.querySelectorAll("button[aria-label]") ?? [])].map(
    (b) => b.getAttribute("aria-label") ?? "",
  );
}

beforeEach(async () => {
  const { resetMockRemoteWorkspaces } = await import("../api/mock");
  resetMockRemoteWorkspaces();
  await loadLibrary();
});

afterEach(() => {
  if (app) unmount(app);
  target?.remove();
  target = null;
  app = null;
});

describe("Library read-only parity", () => {
  it("hides every mutation control including edit-config, keeps the static on-state", () => {
    mountList();
    const l = labels();
    // No add entry points (new workspace/terminal, add devserver).
    expect(l.some((x) => x.startsWith("New local"))).toBe(false);
    expect(l.some((x) => x.startsWith("New terminal on"))).toBe(false);
    expect(
      [...target!.querySelectorAll("button")].some((b) => b.textContent?.includes("Add dev server")),
    ).toBe(false);
    // No selection checkboxes.
    expect(target!.querySelector('input[type="checkbox"]')).toBeNull();
    // No on/off, connect/disconnect, or open-window actions.
    expect(l.some((x) => x.startsWith("Turn off") || x.startsWith("Turn on"))).toBe(false);
    expect(l.some((x) => x.startsWith("Disconnect ") || x.startsWith("Connect "))).toBe(false);
    expect(l.some((x) => x.startsWith("New window of"))).toBe(false);
    // No edit-config affordance on read-only (the devserver header is static).
    expect(l.some((x) => x.startsWith("Edit config") || x.startsWith("Settings for"))).toBe(false);
    // The devserver identity still renders (name + address), just not clickable.
    expect(target!.textContent).toContain("box.example.com:8787");
    // The workspace on-state shows as a static pill.
    expect(target!.querySelector(".pill")).not.toBeNull();
  });

  it("renders window rows statically with the connection dot, no focus/hide", () => {
    mountList();
    expect(target!.querySelector(".dot")).not.toBeNull();
    expect(target!.querySelector('[aria-label="Focus window"]')).toBeNull();
    expect(target!.querySelector('[aria-label="Hide window"]')).toBeNull();
  });
});
