// Component test: mounting WorkspaceList and selecting a row reveals the bulk
// bar. This exercises the real Svelte 5 runtime reactivity of the selection
// Set (a static check wouldn't catch a non-reactive Set), per jsdom.

import { describe, it, expect, afterEach, beforeEach } from "vitest";
import { mount, unmount, flushSync } from "svelte";
import WorkspaceList from "./WorkspaceList.svelte";
import { library, loadLibrary } from "../state/library.svelte";
import { toggleSelected, clearSelection } from "../state/selection.svelte";

let target: HTMLElement | null = null;
let app: Record<string, unknown> | null = null;

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

describe("WorkspaceList multi-select rendering", () => {
  it("reveals the bulk bar + checks the row when a workspace is selected", () => {
    target = document.createElement("div");
    document.body.appendChild(target);
    app = mount(WorkspaceList, { target });

    // No bulk bar before anything is selected.
    expect(target.querySelector('[aria-label="Bulk actions"]')).toBeNull();
    // The per-row Remove button is gone; the On/Off pill stays.
    expect(target.textContent).not.toContain("Remove");
    expect(target.querySelector(".pill")).not.toBeNull();

    // Select a row -> the reactive Set drives the bulk bar + the checkbox.
    const id = library.workspaces[0]!.workspace_id;
    toggleSelected(id);
    flushSync();

    expect(target.querySelector('[aria-label="Bulk actions"]')).not.toBeNull();
    expect(target.textContent).toContain("1 selected");
    expect(target.textContent).toContain("Turn On");
    expect(target.textContent).toContain("Delete");
    const checks = [...target.querySelectorAll('input[type="checkbox"]')] as HTMLInputElement[];
    expect(checks.some((c) => c.checked)).toBe(true);
  });
});
