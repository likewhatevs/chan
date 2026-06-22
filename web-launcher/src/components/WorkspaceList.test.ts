// Component test: WorkspaceList renders the redesigned local rows (icon actions
// [New window] + [On/Off], no per-row Remove), the merged devserver workspace
// group (A4), and reveals the bulk bar when a local row is selected. Exercises
// the real Svelte 5 runtime reactivity of the kind-aware selection (a static
// check wouldn't catch a non-reactive selection), per jsdom.

import { describe, it, expect, afterEach, beforeEach, vi } from "vitest";
import { mount, unmount, flushSync } from "svelte";
import WorkspaceList from "./WorkspaceList.svelte";
import { library, loadLibrary } from "../state/library.svelte";
import { toggleSelected, clearSelection } from "../state/selection.svelte";

// Pin the in-memory mock as the backend so the mounted list renders real rows
// with no live server, independent of how the app composes its default backend.
// The async-import factory dodges vi.mock's hoist-over-imports trap.
vi.mock("../api/backend", async () => {
  const { mockApi } = await import("../api/mock");
  return { backend: mockApi };
});

let target: HTMLElement | null = null;
let app: Record<string, unknown> | null = null;

function ariaLabels(): string[] {
  return [...(target?.querySelectorAll("button[aria-label]") ?? [])].map(
    (b) => b.getAttribute("aria-label") ?? "",
  );
}

beforeEach(async () => {
  clearSelection();
  await loadLibrary();
});

afterEach(() => {
  if (app) unmount(app);
  target?.remove();
  target = null;
  app = null;
  clearSelection();
});

describe("WorkspaceList redesign", () => {
  it("renders local rows with icon actions and no per-row Remove", () => {
    target = document.createElement("div");
    document.body.appendChild(target);
    app = mount(WorkspaceList, { target });

    // No bulk bar before anything is selected, and no per-row Remove text.
    expect(target.querySelector('[aria-label="Bulk actions"]')).toBeNull();
    expect(target.textContent).not.toContain("Remove");
    // The redesigned local rows carry icon actions (aria-labelled), not the old
    // "Open" / "Turn on" text pills; the mutable surface shows no static pill.
    expect(target.querySelector(".pill")).toBeNull();
    const labels = ariaLabels();
    expect(labels.some((l) => l.startsWith("New window of"))).toBe(true);
    expect(labels.some((l) => l.startsWith("Turn off") || l.startsWith("Turn on"))).toBe(true);
  });

  it("renders the connected devserver's workspaces as their own group (A4)", () => {
    target = document.createElement("div");
    document.body.appendChild(target);
    app = mount(WorkspaceList, { target });

    // The seed devserver "prod" is connected, so its served workspaces merge in
    // under a "↗ prod" group header.
    expect(target.textContent).toContain("↗ prod");
    expect(target.textContent).toContain("/srv/api");
    // A remote row offers a Forget action (per-row; remote rows are not bulk).
    expect(ariaLabels().some((l) => l.startsWith("Forget"))).toBe(true);
  });

  it("reflects library.localColor in the local colour control and persists a change", async () => {
    target = document.createElement("div");
    document.body.appendChild(target);
    app = mount(WorkspaceList, { target });

    // loadLibrary seeded the mock's local colour (#3fb950) into library.localColor.
    expect(library.localColor).toBe("#3fb950");
    const swatch = target.querySelector('input[type="color"]') as HTMLInputElement | null;
    expect(swatch).not.toBeNull();
    expect(swatch!.value).toBe("#3fb950");

    // Changing the swatch routes through setLocalColor (the action + the mock's
    // tick); a macrotask boundary drains it before asserting the new state.
    swatch!.value = "#ff8800";
    swatch!.dispatchEvent(new Event("input", { bubbles: true }));
    await new Promise((r) => setTimeout(r, 0));
    flushSync();
    expect(library.localColor).toBe("#ff8800");
    expect(library.error).toBeNull();
  });

  it("clears the local colour to default through the Default button", async () => {
    target = document.createElement("div");
    document.body.appendChild(target);
    app = mount(WorkspaceList, { target });

    // A non-null localColor shows a Default clear affordance.
    const def = [...target.querySelectorAll("button")].find(
      (b) => b.textContent?.trim() === "Default",
    ) as HTMLButtonElement | undefined;
    expect(def).toBeTruthy();
    def!.click();
    await new Promise((r) => setTimeout(r, 0));
    flushSync();
    expect(library.localColor).toBeNull();
    expect(library.error).toBeNull();
  });

  it("reveals the bulk bar + checks the row when a local workspace is selected", () => {
    target = document.createElement("div");
    document.body.appendChild(target);
    app = mount(WorkspaceList, { target });

    const localId = library.workspaces.find((w) => w.devserver_id === null)!.workspace_id;
    toggleSelected("workspace", localId);
    flushSync();

    expect(target.querySelector('[aria-label="Bulk actions"]')).not.toBeNull();
    expect(target.textContent).toContain("1 selected");
    expect(target.textContent).toContain("Turn On");
    expect(target.textContent).toContain("Remove");
    const checks = [...target.querySelectorAll('input[type="checkbox"]')] as HTMLInputElement[];
    expect(checks.some((c) => c.checked)).toBe(true);
  });
});
