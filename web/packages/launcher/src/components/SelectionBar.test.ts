// Component test: the single global bulk bar. It shows while one or more rows of
// ANY kind are selected, displays a COMBINED count across local workspaces,
// served workspaces, and devservers, and exposes the global ops (Turn On / Turn
// Off / Remove → confirm / Clear). Rendering reads only the selection state — no
// backend — so this mounts the bar directly and drives the selection module;
// the bulk ops themselves are covered against the mock in selection.svelte.test.
// Exercises the real Svelte 5 runtime (a static check misses a non-reactive bar).

import { describe, it, expect, afterEach, beforeEach } from "vitest";
import { mount, unmount, flushSync } from "svelte";
import SelectionBar from "./SelectionBar.svelte";
import {
  selection,
  toggleSelected,
  clearSelection,
  requestBulkDelete,
} from "../state/selection.svelte";

let target: HTMLElement | null = null;
let app: Record<string, unknown> | null = null;

beforeEach(() => {
  clearSelection();
});

afterEach(() => {
  if (app) unmount(app);
  target?.remove();
  target = null;
  app = null;
  clearSelection();
});

function mountBar(): void {
  target = document.createElement("div");
  document.body.appendChild(target);
  app = mount(SelectionBar, { target });
}

describe("global SelectionBar", () => {
  it("is hidden with an empty selection", () => {
    mountBar();
    expect(target!.querySelector('[aria-label="Bulk actions"]')).toBeNull();
  });

  it("shows a combined count across kinds and the global action buttons", () => {
    mountBar();

    // One row of each kind: local workspace + served workspace + devserver.
    toggleSelected("workspace", "ws-1");
    toggleSelected("served", "w/api", "ds-1");
    toggleSelected("devserver", "ds-1");
    flushSync();

    expect(target!.querySelector('[aria-label="Bulk actions"]')).not.toBeNull();
    expect(target!.textContent).toContain("3 selected");
    expect(target!.textContent).toContain("Turn On");
    expect(target!.textContent).toContain("Turn Off");
    expect(target!.textContent).toContain("Remove");
    expect(target!.textContent).toContain("Clear");
  });

  it("flips to the delete-confirm prompt and back", () => {
    mountBar();
    toggleSelected("workspace", "ws-1");
    toggleSelected("devserver", "ds-1");
    flushSync();

    requestBulkDelete();
    flushSync();
    expect(selection.confirmingDelete).toBe(true);
    expect(target!.textContent).toContain("Remove 2?");
    expect(target!.textContent).toContain("Confirm remove");

    // Cancel (the bar's Cancel button) returns to the action row.
    const cancel = [...target!.querySelectorAll("button")].find((b) => b.textContent === "Cancel");
    expect(cancel).toBeTruthy();
    cancel!.click();
    flushSync();
    expect(selection.confirmingDelete).toBe(false);
    expect(target!.textContent).not.toContain("Confirm remove");
  });
});
