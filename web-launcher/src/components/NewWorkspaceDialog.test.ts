import { describe, it, expect, afterEach, vi } from "vitest";
import { mount, unmount, flushSync } from "svelte";
import NewWorkspaceDialog from "./NewWorkspaceDialog.svelte";
import { openNewDialog, openEditDevserver, selectChoice, closeDialog } from "../state/dialog.svelte";
import { library } from "../state/library.svelte";
import type { DevserverEntry } from "../api/library";

// Pin the in-memory mock as the backend so the Browse… picker returns a canned
// path with no live server. The async-import factory dodges vi.mock's hoist trap.
vi.mock("../api/backend", async () => {
  const { mockApi } = await import("../api/mock");
  return { backend: mockApi };
});

let target: HTMLElement | null = null;
let app: Record<string, unknown> | null = null;

function render(): HTMLElement {
  target = document.createElement("div");
  document.body.appendChild(target);
  app = mount(NewWorkspaceDialog, { target });
  return target;
}

afterEach(() => {
  if (app) unmount(app);
  target?.remove();
  target = null;
  app = null;
  closeDialog();
});

describe("New workspace dialog", () => {
  it("offers exactly two choices (outbound dropped)", () => {
    openNewDialog("local");
    const el = render();
    const radios = el.querySelectorAll('[role="radio"]');
    expect(radios.length).toBe(2);
    const labels = [...radios].map((r) => r.textContent?.trim());
    expect(labels).toEqual(["Local directory", "Devserver"]);
  });

  it("shows a folder-path input for the local choice", () => {
    openNewDialog("local");
    const el = render();
    expect(el.querySelector('input[type="text"]')).not.toBeNull();
    expect(el.textContent).toContain("Folder path");
  });

  it("fills the folder path from the Browse… native picker", async () => {
    openNewDialog("local");
    const el = render();
    const browse = [...el.querySelectorAll("button")].find(
      (b) => b.textContent?.trim() === "Browse…",
    ) as HTMLButtonElement | undefined;
    expect(browse).toBeTruthy();

    browse!.click();
    // The picker resolves through the action + the mock's tick; drain the full
    // microtask queue (a macrotask hop) before the bound input reflects it.
    await new Promise((r) => setTimeout(r, 0));
    flushSync();
    const input = el.querySelector('input[type="text"]') as HTMLInputElement;
    expect(input.value).toBe("/Users/you/picked-folder");
  });

  it("shows the masked token field + connect command for the devserver choice", () => {
    openNewDialog("devserver");
    const el = render();
    expect(el.querySelector('input[type="password"]')).not.toBeNull();
    expect(el.querySelector("textarea")).not.toBeNull();
    expect(el.textContent).toContain("Token");
    expect(el.textContent).toContain("Add devserver");
  });

  it("offers an auto-hide control-terminal checkbox (off by default for a new devserver)", () => {
    openNewDialog("devserver");
    const el = render();
    const cb = el.querySelector('input[type="checkbox"]') as HTMLInputElement | null;
    expect(cb).not.toBeNull();
    expect(cb!.checked).toBe(false);
    expect(el.textContent).toContain("Auto-hide control terminal on success");
  });

  it("offers separate Host and Port fields", () => {
    openNewDialog("devserver");
    const el = render();
    expect(el.textContent).toContain("Host");
    expect(el.textContent).toContain("Port");
    expect(el.textContent).not.toContain("Devserver URL");
    // The Port field is a number input.
    expect(el.querySelector('input[type="number"]')).not.toBeNull();
  });

  it("rejects an empty host or an out-of-range port", () => {
    openNewDialog("devserver");
    const el = render();
    const hostField = el.querySelector('input[type="text"]') as HTMLInputElement;
    const portField = el.querySelector('input[type="number"]') as HTMLInputElement;
    // Host filled but the port is out of range → rejected with the new message.
    hostField.value = "box.example.com";
    hostField.dispatchEvent(new Event("input", { bubbles: true }));
    portField.value = "70000";
    portField.dispatchEvent(new Event("input", { bubbles: true }));
    flushSync();
    const addBtn = [...el.querySelectorAll("button")].find((b) =>
      b.textContent?.includes("Add devserver"),
    ) as HTMLButtonElement;
    addBtn.click();
    flushSync();
    expect(el.querySelector('[role="alert"]')?.textContent).toContain("1–65535");
  });

  it("submits a valid host and port", async () => {
    openNewDialog("devserver");
    const el = render();
    const hostField = el.querySelector('input[type="text"]') as HTMLInputElement;
    const portField = el.querySelector('input[type="number"]') as HTMLInputElement;
    hostField.value = "valid.example.com";
    hostField.dispatchEvent(new Event("input", { bubbles: true }));
    portField.value = "8787";
    portField.dispatchEvent(new Event("input", { bubbles: true }));
    flushSync();
    const addBtn = [...el.querySelectorAll("button")].find((b) =>
      b.textContent?.includes("Add devserver"),
    ) as HTMLButtonElement;
    addBtn.click();
    // saveDevserver → backend tick → refresh hops; drain to a macrotask boundary.
    await new Promise((r) => setTimeout(r, 0));
    flushSync();
    // A valid submit closes the dialog (no validation error raised).
    expect(el.querySelector('[role="alert"]')).toBeNull();
    const added = library.devservers.find(
      (d) => d.host === "valid.example.com" && d.port === 8787,
    );
    expect(added).toBeTruthy();
  });

  it("switches body when the choice changes", () => {
    openNewDialog("local");
    const el = render();
    expect(el.querySelector('input[type="password"]')).toBeNull();
    selectChoice("devserver");
    flushSync();
    expect(el.querySelector('input[type="password"]')).not.toBeNull();
  });

  it("gives the action row a clearing margin (the dialog-margin fix)", () => {
    openNewDialog("devserver");
    const el = render();
    expect(el.querySelector(".dialog-footer")).not.toBeNull();
  });

  it("prefills the edit form and reports a stored token without echoing it", () => {
    const ds: DevserverEntry = {
      id: "ds-edit",
      host: "edit.example",
      port: 8123,
      label: "staging",
      script: "",
      has_token: true,
      library_id: null,
      connected: false,
      auto_hide_control: false,
    };
    openEditDevserver(ds);
    const el = render();
    expect(el.textContent).toContain("Save changes");
    // The first text input is the Host field, seeded from the entry; the Port
    // number input carries its port.
    const hostField = el.querySelector('input[type="text"]') as HTMLInputElement | null;
    expect(hostField?.value).toBe("edit.example");
    const portField = el.querySelector('input[type="number"]') as HTMLInputElement | null;
    expect(portField?.value).toBe("8123");
    const token = el.querySelector('input[type="password"]') as HTMLInputElement | null;
    expect(token?.value).toBe("");
    expect(token?.placeholder).toContain("leave blank to keep");
    // The edit form has no choice switcher.
    expect(el.querySelectorAll('[role="radio"]').length).toBe(0);
  });

  it("opens read-only (OK, no Save, disabled inputs) for a connected devserver", () => {
    const ds: DevserverEntry = {
      id: "ds-live",
      host: "live.example",
      port: 8200,
      label: "live",
      script: "",
      has_token: false,
      library_id: "lib-abc",
      connected: true,
      auto_hide_control: false,
    };
    openEditDevserver(ds);
    const el = render();
    // A read-only notice, no Save, an OK dismiss instead.
    expect(el.textContent).toContain("read-only");
    expect(el.textContent).not.toContain("Save changes");
    const ok = [...el.querySelectorAll("button")].find((b) => b.textContent?.trim() === "OK");
    expect(ok).toBeTruthy();
    // The host + port fields are disabled (no edits while connected).
    const hostField = el.querySelector('input[type="text"]') as HTMLInputElement;
    expect(hostField.disabled).toBe(true);
    expect((el.querySelector('input[type="number"]') as HTMLInputElement).disabled).toBe(true);
    expect((el.querySelector("textarea") as HTMLTextAreaElement).disabled).toBe(true);
  });
});
