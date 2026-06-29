// Component test: the machine-first Library tree. The LOCAL block (home header +
// new-terminal + new-workspace) over its workspace cards; then one block per
// registered devserver -- connected or not -- whose globe header carries the
// name/address edit-config click target, new-terminal (connected only), and
// connect/disconnect, with its served workspaces nested as collapsible cards.
// Exercises the real Svelte 5 runtime (reactive re-render after connect /
// pending / status), per jsdom; readOnly is false here (the mutable surface).

import { describe, it, expect, afterEach, beforeEach, vi } from "vitest";
import { mount, unmount, flushSync } from "svelte";
import Library from "./Library.svelte";
import ConfirmDialog from "./ConfirmDialog.svelte";
import { library, loadLibrary, saveDevserver } from "../state/library.svelte";
import { isSelected, toggleSelected, clearSelection, setSelectMode } from "../state/selection.svelte";
import { beginPending, clearAllPending, dsKey, isPending, wsKey } from "../state/pending.svelte";
import { confirm, requestConfirm, resolveConfirm, cancelConfirm } from "../state/confirm.svelte";
import { ApiError, type DevserverEntry, type WorkspaceEntry } from "../api/library";
import { controlAttention, clearAllControlAttention } from "../state/controlAttention.svelte";

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

function ariaLabels(): string[] {
  return [...(target?.querySelectorAll("button[aria-label], input[aria-label]") ?? [])].map(
    (b) => b.getAttribute("aria-label") ?? "",
  );
}

function byAria(prefix: string): HTMLButtonElement | undefined {
  return [...(target?.querySelectorAll("button[aria-label]") ?? [])].find((b) =>
    (b.getAttribute("aria-label") ?? "").startsWith(prefix),
  ) as HTMLButtonElement | undefined;
}

function settle(): Promise<void> {
  return new Promise((r) => setTimeout(r, 0));
}

beforeEach(async () => {
  const { resetMockRemoteWorkspaces } = await import("../api/mock");
  resetMockRemoteWorkspaces();
  clearSelection();
  clearAllPending();
  clearAllControlAttention();
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
  clearAllPending();
  clearAllControlAttention();
});

describe("Library: Local group", () => {
  it("renders local rows with icon actions, no per-row Remove, no pill", () => {
    mountList();
    expect(target!.textContent).not.toContain("Remove");
    expect(target!.querySelector(".pill")).toBeNull();
    const labels = ariaLabels();
    expect(labels.some((l) => l.startsWith("New window of"))).toBe(true);
    expect(labels.some((l) => l.startsWith("Turn off") || l.startsWith("Turn on"))).toBe(true);
  });

  it("has a home header with new-terminal + new-workspace actions and no select-all", () => {
    mountList();
    expect(byAria("New local terminal")).toBeTruthy();
    expect(byAria("New local workspace")).toBeTruthy();
    // The select-all-local checkbox is gone (the mock has none); selection is
    // per-row, revealed by the top-bar Select toggle.
    expect(target!.querySelector('input[aria-label="Select all local workspaces"]')).toBeNull();
  });

  it("spins a local row from a pending marker, disabled, with the spinner svg", () => {
    mountList();
    const id = library.workspaces.find((w) => w.devserver_id === null)!.workspace_id;
    beginPending(wsKey(id), "off");
    flushSync();
    const spinning = target!.querySelector('button[aria-label^="Working on"]') as HTMLButtonElement;
    expect(spinning).toBeTruthy();
    expect(spinning.disabled).toBe(true);
    expect(spinning.querySelector("svg.spin")).toBeTruthy();
  });

  it("spins a local row from backend status:starting alone (no marker)", () => {
    mountList();
    const id = library.workspaces.find((w) => w.devserver_id === null)!.workspace_id;
    expect(isPending(wsKey(id))).toBe(false);
    library.workspaces = library.workspaces.map(
      (w): WorkspaceEntry => (w.workspace_id === id ? { ...w, status: "starting" } : w),
    );
    flushSync();
    expect(target!.querySelector('button[aria-label^="Working on"]')).toBeTruthy();
  });

  it("surfaces status:error with the reason and keeps the toggle enabled (retry)", () => {
    mountList();
    const id = library.workspaces.find((w) => w.devserver_id === null)!.workspace_id;
    library.workspaces = library.workspaces.map(
      (w): WorkspaceEntry =>
        w.workspace_id === id ? { ...w, status: "error", error: "foreign lock held" } : w,
    );
    flushSync();
    expect(target!.querySelector('.row-error[title="foreign lock held"]')).toBeTruthy();
    const toggle = target!.querySelector(
      'button[aria-label^="Turn off"], button[aria-label^="Turn on"]',
    ) as HTMLButtonElement;
    expect(toggle).toBeTruthy();
    expect(toggle.disabled).toBe(false);
  });

  it("checks the row when a local workspace is selected", () => {
    mountList();
    const localId = library.workspaces.find((w) => w.devserver_id === null)!.workspace_id;
    toggleSelected("workspace", localId);
    flushSync();
    const checks = [...target!.querySelectorAll('input[type="checkbox"]')] as HTMLInputElement[];
    expect(checks.some((c) => c.checked)).toBe(true);
  });
});

describe("Library: devserver groups", () => {
  it("renders a connected devserver as its own group: Disconnect + enabled New terminal + Edit config + endpoint", () => {
    mountList();
    // Seed devserver "prod" (ds-1) is connected.
    expect(byAria("Disconnect prod")).toBeTruthy();
    const newTerm = byAria("New terminal on prod");
    expect(newTerm).toBeTruthy();
    expect(newTerm!.disabled).toBe(false);
    // The header name/address block is the edit-config click target.
    expect(byAria("Edit config for prod")).toBeTruthy();
    // The header carries the endpoint as host:port.
    expect(target!.textContent).toContain("box.example.com:8787");
    // The devserver is bulk-selectable once the checkboxes are revealed.
    setSelectMode(true);
    flushSync();
    expect(target!.querySelector('input[aria-label="Select prod"]')).not.toBeNull();
  });

  it("nests the connected devserver's served workspaces as rows with a checkbox and no Forget", () => {
    mountList();
    expect(target!.textContent).toContain("/srv/api");
    expect(ariaLabels().some((l) => l.startsWith("Forget"))).toBe(false);
    // The served row carries the workspace on/off + new-window actions.
    expect(byAria("New window of api")).toBeTruthy();
    expect(byAria("Turn off api")).toBeTruthy();
    // ...and reveals a select checkbox in select mode.
    setSelectMode(true);
    flushSync();
    expect(target!.querySelector('input[aria-label="Select api"]')).not.toBeNull();
  });

  it("still shows a DISCONNECTED devserver as a header with Connect, a prompt, and no New terminal", async () => {
    await saveDevserver({ host: "fresh.example", port: 9100, label: "fresh" });
    mountList();
    const connect = byAria("Connect fresh");
    expect(connect).toBeTruthy();
    expect(connect!.disabled).toBe(false);
    // New terminal is hidden until connected (it appears on connect).
    expect(byAria("New terminal on fresh")).toBeUndefined();
    expect(byAria("Edit config for fresh")).toBeTruthy();
    // A disconnected devserver shows the connect prompt, no content.
    expect(target!.textContent).toContain("Not connected");
  });

  it("renders a disconnected devserver's retained control row when it needs attention", () => {
    const ds = library.devservers.find((d) => d.id === "ds-1")!;
    library.devservers = library.devservers.map(
      (d): DevserverEntry => (d.id === "ds-1" ? { ...d, status: "disconnected" } : d),
    );
    controlAttention.libs[ds.library_id!] = true;

    mountList();

    const machines = [...target!.querySelectorAll("section.machine")];
    const prod = machines.find((m) => m.textContent?.includes("box.example.com:8787"));
    expect(prod).toBeTruthy();
    expect(prod!.textContent).toContain("Control terminal");
    expect(prod!.textContent).toContain("disconnected...");
    expect(prod!.querySelector("button.icon-btn.attention")).not.toBeNull();
    expect(byAria("Connect prod")).toBeTruthy();
  });

  it("does not retain a reaped control row from stale attention alone", () => {
    const ds = library.devservers.find((d) => d.id === "ds-1")!;
    library.devservers = library.devservers.map(
      (d): DevserverEntry => (d.id === "ds-1" ? { ...d, status: "disconnected" } : d),
    );
    library.windows = library.windows.filter((w) => !(w.control && w.library_id === ds.library_id));
    controlAttention.libs[ds.library_id!] = true;

    mountList();

    const machines = [...target!.querySelectorAll("section.machine")];
    const prod = machines.find((m) => m.textContent?.includes("box.example.com:8787"));
    expect(prod).toBeTruthy();
    expect(prod!.textContent).not.toContain("Control terminal");
    expect(prod!.textContent).not.toContain("disconnected...");
    expect(prod!.textContent).toContain("Not connected");
    expect(prod!.querySelector("button.icon-btn.attention")).toBeNull();
  });

  it("fires connect and flips the disconnected devserver to Disconnect", async () => {
    await saveDevserver({ host: "fresh2.example", port: 9101, label: "fresh2" });
    mountList();
    byAria("Connect fresh2")!.click();
    await settle();
    flushSync();
    expect(library.error).toBeNull();
    const fresh = library.devservers.find((d) => d.host === "fresh2.example")!;
    expect(fresh.status).toBe("connected");
    expect(byAria("Disconnect fresh2")).toBeTruthy();
  });

  it("swaps Connect/Disconnect for a disabled spinner while a devserver op is pending", () => {
    mountList();
    expect(byAria("Disconnect prod")).toBeTruthy();
    beginPending(dsKey("ds-1"), "disconnected");
    flushSync();
    const spinning = byAria("Working on prod");
    expect(spinning).toBeTruthy();
    expect(spinning!.disabled).toBe(true);
    expect(spinning!.querySelector("svg.spin")).toBeTruthy();
    expect(byAria("Disconnect prod")).toBeUndefined();
  });

  it("spins a devserver from status:connecting alone, then clears on disconnect", () => {
    mountList();
    library.devservers = library.devservers.map(
      (d): DevserverEntry => (d.id === "ds-1" ? { ...d, status: "connecting" } : d),
    );
    flushSync();
    expect(byAria("Working on prod")).toBeTruthy();
    library.devservers = library.devservers.map(
      (d): DevserverEntry => (d.id === "ds-1" ? { ...d, status: "disconnected" } : d),
    );
    flushSync();
    expect(byAria("Working on prod")).toBeUndefined();
    expect(byAria("Connect prod")).toBeTruthy();
  });

  it("checks the devserver row when selected", () => {
    mountList();
    toggleSelected("devserver", "ds-1");
    flushSync();
    const check = target!.querySelector('input[aria-label="Select prod"]') as HTMLInputElement;
    expect(check.checked).toBe(true);
  });
});

describe("Library: workspace OFF confirm-and-retry", () => {
  function apiOn(prefix: string): boolean {
    return library.workspaces.find((w) => w.devserver_id === "ds-1" && w.prefix === prefix)!.on;
  }
  function offButton(label: string): HTMLButtonElement {
    const btn = [...(target?.querySelectorAll("button[aria-label]") ?? [])].find(
      (b) => b.getAttribute("aria-label") === label,
    );
    if (!btn) throw new Error(`no button labelled "${label}"`);
    return btn as HTMLButtonElement;
  }

  it("opens the confirm with N and turns off only on confirm (remote workspace)", async () => {
    mountList();
    expect(apiOn("w/api")).toBe(true);
    expect(confirm.open).toBe(false);
    offButton("Turn off api").click();
    await settle();
    flushSync();
    expect(confirm.open).toBe(true);
    expect(confirm.message).toContain("2 live terminal sessions");
    expect(apiOn("w/api")).toBe(true);
    await resolveConfirm();
    flushSync();
    expect(confirm.open).toBe(false);
    expect(apiOn("w/api")).toBe(false);
    expect(library.error).toBeNull();
  });

  it("does NOT open the confirm for a non-live_terminals (NO_DESKTOP) 409", async () => {
    const { backend } = await import("../api/backend");
    const spy = vi
      .spyOn(backend, "setDevserverWorkspaceOn")
      .mockRejectedValueOnce(new ApiError(409, "NO_DESKTOP"));
    mountList();
    offButton("Turn off api").click();
    await settle();
    flushSync();
    expect(confirm.open).toBe(false);
    expect(library.error).toBe("NO_DESKTOP");
    spy.mockRestore();
  });

  it("opens the SAME confirm-and-retry for a LOCAL workspace off", async () => {
    const { setMockLocalLiveTerminals } = await import("../api/mock");
    const ws = library.workspaces.find((w) => w.devserver_id === null && w.on)!;
    setMockLocalLiveTerminals(ws.workspace_id, 3);
    mountList();
    offButton("Turn off notes").click();
    await settle();
    flushSync();
    expect(confirm.open).toBe(true);
    expect(confirm.message).toContain("3 live terminal sessions");
    expect(library.workspaces.find((w) => w.workspace_id === ws.workspace_id)!.on).toBe(true);
    await resolveConfirm();
    flushSync();
    expect(confirm.open).toBe(false);
    expect(library.workspaces.find((w) => w.workspace_id === ws.workspace_id)!.on).toBe(false);
    expect(library.error).toBeNull();
  });

  it("renders the confirm via the in-SPA Modal (role=dialog), never a native one", () => {
    // ConfirmDialog is built on Modal -- a role="dialog" overlay -- so the prompt
    // is in-SPA, not window.confirm (no_native_dialogs.test.ts enforces no native
    // dialog calls anywhere in shipped sources).
    requestConfirm({
      title: "Turn off workspace?",
      message: "2 sessions still running.",
      onConfirm: () => {},
    });
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

describe("Library: nested machine tree", () => {
  it("renders the LOCAL machine block with Terminals + Workspaces sections", () => {
    mountList();
    expect(target!.textContent).toContain("Local machine");
    expect(target!.textContent).toContain("Terminals");
    expect(target!.textContent).toContain("Workspaces");
    // The local standalone terminal renders as a window row with a focus action.
    expect(target!.querySelector('[aria-label="Focus window"]')).not.toBeNull();
  });

  it("pins the control terminal first in a connected devserver's terminals", () => {
    mountList();
    // ds-1 ("prod") is connected and owns a control terminal; it sorts first.
    const machines = [...target!.querySelectorAll("section.machine")];
    const prod = machines.find((m) => m.textContent?.includes("box.example.com:8787"));
    expect(prod).toBeTruthy();
    const firstRowName = prod!.querySelector(".term-list .row-name");
    expect(firstRowName?.textContent?.trim()).toBe("Control terminal");
  });

  it("shows a clickable window-count badge and expands a card to reveal its nested windows", () => {
    mountList();
    // The connected ds-1's "api" workspace owns one window (its window survives
    // the shared mock across tests, unlike a local one that an off discards);
    // collapsed by default with a count badge and no nested-windows panel.
    const badge = target!.querySelector(".count-badge") as HTMLButtonElement;
    expect(badge?.textContent).toContain("1");
    expect(badge?.getAttribute("aria-expanded")).toBe("false");
    expect(target!.querySelector(".ws-windows")).toBeNull();
    badge.click();
    flushSync();
    // Expanded: the nested-windows panel appears with the window row, labelled
    // just "Window N" (the card already names the workspace, no path repeated).
    const panel = target!.querySelector(".ws-windows");
    expect(panel).not.toBeNull();
    expect(panel!.textContent).toContain("Window 1");
    expect(target!.querySelector(".count-badge")?.getAttribute("aria-expanded")).toBe("true");
    (target!.querySelector(".count-badge") as HTMLButtonElement).click();
    flushSync();
    expect(target!.querySelector(".ws-windows")).toBeNull();
  });
});

// Selection across all three kinds is covered above (local + devserver) and in
// selection.svelte.test.ts; the served-kind check is exercised by the remote
// confirm test selecting/acting on api rows.
void isSelected;
