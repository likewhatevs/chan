// Component test: the control-terminal-closed survey modal.
//
// It renders through the shared in-SPA Modal (role="dialog"), never a native
// dialog (the no_native_dialogs scan guards the surface), titled with the
// devserver name and offering Abandon / Edit / Re-run. Clicking Re-run drives
// the reconnect against the mock backend (seeded `ds-1` "prod", connected).

import { describe, it, expect, afterEach, beforeEach, vi } from "vitest";
import { mount, unmount, flushSync } from "svelte";
import ControlClosedSurvey from "./ControlClosedSurvey.svelte";
import { library, loadLibrary } from "../state/library.svelte";
import { controlClosed, handleControlClosed, dismissControlClosed } from "../state/controlClosed.svelte";

vi.mock("../api/backend", async () => {
  const { mockApi } = await import("../api/mock");
  return { backend: mockApi };
});

let target: HTMLElement | null = null;
let app: Record<string, unknown> | null = null;

function settle(): Promise<void> {
  return new Promise((r) => setTimeout(r, 0));
}

function button(label: string): HTMLButtonElement {
  const btn = [...(target?.querySelectorAll("button") ?? [])].find(
    (b) => b.textContent?.trim() === label,
  );
  if (!btn) throw new Error(`no button labelled "${label}"`);
  return btn as HTMLButtonElement;
}

beforeEach(async () => {
  const { resetMockRemoteWorkspaces } = await import("../api/mock");
  resetMockRemoteWorkspaces();
  // Restore the seeded connected state (a prior test may have disconnected it).
  const { backend } = await import("../api/backend");
  await backend.connectDevserver("ds-1");
  library.error = null;
  dismissControlClosed();
  await loadLibrary();
});

afterEach(() => {
  if (app) unmount(app);
  target?.remove();
  target = null;
  app = null;
  dismissControlClosed();
});

describe("control-closed survey modal", () => {
  it("renders via the in-SPA Modal (role=dialog) titled with the devserver name", () => {
    handleControlClosed("ds-1");
    flushSync();
    target = document.createElement("div");
    document.body.appendChild(target);
    app = mount(ControlClosedSurvey, { target });
    flushSync();

    const dlg = target.querySelector('[role="dialog"]');
    expect(dlg).not.toBeNull();
    expect(dlg?.getAttribute("aria-label")).toBe("prod disconnected");
    expect(target.textContent).toContain("no longer");
    // All three choices are present.
    expect(button("Abandon")).toBeTruthy();
    expect(button("Edit")).toBeTruthy();
    expect(button("Re-run")).toBeTruthy();
  });

  it("Re-run reconnects the devserver and closes the survey", async () => {
    handleControlClosed("ds-1");
    flushSync();
    target = document.createElement("div");
    document.body.appendChild(target);
    app = mount(ControlClosedSurvey, { target });
    flushSync();

    button("Re-run").click();
    await settle();
    flushSync();

    expect(library.devservers.find((d) => d.id === "ds-1")!.connected).toBe(true);
    expect(controlClosed.open).toBe(false);
    expect(library.error).toBeNull();
  });

  it("Abandon disconnects the devserver and closes the survey", async () => {
    handleControlClosed("ds-1");
    flushSync();
    target = document.createElement("div");
    document.body.appendChild(target);
    app = mount(ControlClosedSurvey, { target });
    flushSync();

    button("Abandon").click();
    await settle();
    flushSync();

    expect(library.devservers.find((d) => d.id === "ds-1")!.connected).toBe(false);
    expect(controlClosed.open).toBe(false);
  });
});
