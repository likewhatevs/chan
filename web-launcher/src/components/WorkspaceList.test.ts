// Component test: WorkspaceList renders the redesigned local rows (icon actions
// [New window] + [On/Off], no per-row Remove), the merged devserver workspace
// group (A4) with a select checkbox and NO per-row Forget, and checks a row when
// it is selected. The single global bulk bar now lives App-level (SelectionBar),
// not inside the list — covered by SelectionBar.test.ts. Exercises the real
// Svelte 5 runtime reactivity of the kind-aware selection (a static check
// wouldn't catch a non-reactive selection), per jsdom.

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

  it("renders the connected devserver's workspaces as their own group (A4) with a checkbox, no Forget", () => {
    target = document.createElement("div");
    document.body.appendChild(target);
    app = mount(WorkspaceList, { target });

    // The seed devserver "prod" is connected, so its served workspaces merge in
    // under a "↗ prod" group header.
    expect(target.textContent).toContain("↗ prod");
    expect(target.textContent).toContain("/srv/api");
    // Served rows are now bulk-managed like local ones: no per-row Forget, and a
    // select checkbox (the ordered Remove lives in the global bar).
    expect(ariaLabels().some((l) => l.startsWith("Forget"))).toBe(false);
    expect(target.querySelector('input[aria-label="Select api"]')).not.toBeNull();
  });

  it("checks the row when a local workspace is selected (the bulk bar is App-level, not here)", () => {
    target = document.createElement("div");
    document.body.appendChild(target);
    app = mount(WorkspaceList, { target });

    const localId = library.workspaces.find((w) => w.devserver_id === null)!.workspace_id;
    toggleSelected("workspace", localId);
    flushSync();

    // The single bulk bar lives App-level (SelectionBar), not inside the list.
    expect(target.querySelector('[aria-label="Bulk actions"]')).toBeNull();
    const checks = [...target.querySelectorAll('input[type="checkbox"]')] as HTMLInputElement[];
    expect(checks.some((c) => c.checked)).toBe(true);
  });
});
