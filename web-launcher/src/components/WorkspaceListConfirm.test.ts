// Component test: the per-row remote-workspace OFF confirm-and-retry flow.
//
// Turning off a connected devserver's served workspace can hit live terminal
// sessions: the server answers 409 {error:"live_terminals", active_terminals:N}.
// The launcher must catch that specific 409, open the in-SPA confirm dialog (NOT
// a native one) showing N WITHOUT turning off yet, and on confirm retry the same
// route forced (→ turns off). A plain NO_DESKTOP 409 must NOT open the confirm —
// it is a generic banner error. This exercises the real Svelte 5 runtime + the
// mock's 409 simulation; the no_native_dialogs scan guards the dialog surface.

import { describe, it, expect, afterEach, beforeEach, vi } from "vitest";
import { mount, unmount, flushSync } from "svelte";
import WorkspaceList from "./WorkspaceList.svelte";
import ConfirmDialog from "./ConfirmDialog.svelte";
import { library, loadLibrary } from "../state/library.svelte";
import { clearSelection } from "../state/selection.svelte";
import { confirm, resolveConfirm, requestConfirm, cancelConfirm } from "../state/confirm.svelte";
import { ApiError } from "../api/library";

// Pin the in-memory mock as the backend (same idiom as WorkspaceList.test.ts).
vi.mock("../api/backend", async () => {
  const { mockApi } = await import("../api/mock");
  return { backend: mockApi };
});

let target: HTMLElement | null = null;
let app: Record<string, unknown> | null = null;

// Drain all pending microtasks (a macrotask hop settles every awaited promise in
// the click handler's chain, including the mock's rejected off and the catch).
function settle(): Promise<void> {
  return new Promise((r) => setTimeout(r, 0));
}

function offButton(label: string): HTMLButtonElement {
  const btn = [...(target?.querySelectorAll("button[aria-label]") ?? [])].find(
    (b) => b.getAttribute("aria-label") === label,
  );
  if (!btn) throw new Error(`no button labelled "${label}"`);
  return btn as HTMLButtonElement;
}

function apiOn(prefix: string): boolean {
  return library.workspaces.find((w) => w.devserver_id === "ds-1" && w.prefix === prefix)!.on;
}

beforeEach(async () => {
  const { resetMockRemoteWorkspaces } = await import("../api/mock");
  resetMockRemoteWorkspaces();
  clearSelection();
  library.error = null;
  cancelConfirm();
  await loadLibrary();
});

afterEach(() => {
  if (app) unmount(app);
  target?.remove();
  target = null;
  app = null;
  clearSelection();
});

describe("remote workspace OFF confirm-and-retry", () => {
  it("opens the confirm dialog showing N and does not turn off until confirmed", async () => {
    target = document.createElement("div");
    document.body.appendChild(target);
    app = mount(WorkspaceList, { target });

    // The seeded remote "api" workspace (ds-1:w/api) is ON with 2 live terminals.
    expect(apiOn("w/api")).toBe(true);
    expect(confirm.open).toBe(false);

    // Click its Power (Turn off) button → the unforced off 409s live_terminals.
    offButton("Turn off api").click();
    await settle();
    flushSync();

    // The confirm dialog is open, shows the count, and the workspace is NOT off.
    expect(confirm.open).toBe(true);
    expect(confirm.message).toContain("2 live terminal sessions");
    expect(confirm.message).toContain("still running");
    expect(confirm.confirmLabel).toBe("Turn off");
    expect(apiOn("w/api")).toBe(true);

    // Confirming retries forced → the workspace turns off.
    await resolveConfirm();
    flushSync();
    expect(confirm.open).toBe(false);
    expect(apiOn("w/api")).toBe(false);
    expect(library.error).toBeNull();
  });

  it("does NOT open the confirm for a non-live_terminals (NO_DESKTOP) 409", async () => {
    // Pin the off to a plain NO_DESKTOP-style 409 (body is not the live JSON).
    const { backend } = await import("../api/backend");
    const spy = vi
      .spyOn(backend, "setDevserverWorkspaceOn")
      .mockRejectedValueOnce(new ApiError(409, "NO_DESKTOP"));

    target = document.createElement("div");
    document.body.appendChild(target);
    app = mount(WorkspaceList, { target });

    offButton("Turn off api").click();
    await settle();
    flushSync();

    // No confirm dialog; the error went to the banner instead.
    expect(confirm.open).toBe(false);
    expect(library.error).toBe("NO_DESKTOP");
    spy.mockRestore();
  });

  it("opens the SAME confirm-and-retry for a LOCAL workspace off (B8 parity)", async () => {
    // Parity with the devserver path: turning off a LOCAL workspace with live
    // terminals must confirm + force-retry, not silently kill the terminals.
    const { setMockLocalLiveTerminals } = await import("../api/mock");
    const ws = library.workspaces.find((w) => w.devserver_id === null && w.on)!;
    setMockLocalLiveTerminals(ws.workspace_id, 3);

    target = document.createElement("div");
    document.body.appendChild(target);
    app = mount(WorkspaceList, { target });

    expect(ws.on).toBe(true);
    expect(confirm.open).toBe(false);

    // The seed on local workspace is "notes" (/Users/fiorix/notes).
    offButton("Turn off notes").click();
    await settle();
    flushSync();

    // The same confirm opens with N; the workspace is NOT off yet.
    expect(confirm.open).toBe(true);
    expect(confirm.message).toContain("3 live terminal sessions");
    expect(library.workspaces.find((w) => w.workspace_id === ws.workspace_id)!.on).toBe(true);

    // Confirming retries forced → the local workspace turns off.
    await resolveConfirm();
    flushSync();
    expect(confirm.open).toBe(false);
    expect(library.workspaces.find((w) => w.workspace_id === ws.workspace_id)!.on).toBe(false);
    expect(library.error).toBeNull();
  });

  it("renders the confirm via the in-SPA Modal (role=dialog), never a native one", () => {
    // ConfirmDialog is built on Modal — a role="dialog" overlay — so the prompt
    // is in-SPA, not window.confirm (the no_native_dialogs.test.ts scan enforces
    // no native dialog calls anywhere in shipped sources).
    requestConfirm({ title: "Turn off workspace?", message: "2 sessions still running.", onConfirm: () => {} });
    flushSync();

    target = document.createElement("div");
    document.body.appendChild(target);
    app = mount(ConfirmDialog, { target });
    flushSync();

    const dlg = target.querySelector('[role="dialog"]');
    expect(dlg).not.toBeNull();
    expect(dlg?.getAttribute("aria-label")).toBe("Turn off workspace?");
    expect(target.textContent).toContain("still running");

    cancelConfirm();
  });
});
