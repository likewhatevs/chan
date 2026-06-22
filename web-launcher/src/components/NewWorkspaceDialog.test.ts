import { describe, it, expect, afterEach, vi } from "vitest";
import { mount, unmount, flushSync } from "svelte";
import NewWorkspaceDialog from "./NewWorkspaceDialog.svelte";
import { openNewDialog, openEditDevserver, selectChoice, closeDialog } from "../state/dialog.svelte";
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

  it("offers a single Devserver URL field (Host/Port dropped)", () => {
    openNewDialog("devserver");
    const el = render();
    expect(el.textContent).toContain("Devserver URL");
    expect(el.textContent).not.toContain("Host");
    expect(el.textContent).not.toContain("Port");
    // No number input remains (the old Port field).
    expect(el.querySelector('input[type="number"]')).toBeNull();
  });

  it("rejects a bare host:port that omits the scheme", () => {
    openNewDialog("devserver");
    const el = render();
    const urlField = el.querySelector('input[type="text"]') as HTMLInputElement;
    urlField.value = "box.example.com:8787";
    urlField.dispatchEvent(new Event("input", { bubbles: true }));
    flushSync();
    const addBtn = [...el.querySelectorAll("button")].find((b) =>
      b.textContent?.includes("Add devserver"),
    ) as HTMLButtonElement;
    addBtn.click();
    flushSync();
    expect(el.querySelector('[role="alert"]')?.textContent).toContain("scheme");
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
      url: "https://edit.example:8123",
      label: "staging",
      script: "",
      has_token: true,
      library_id: null,
      connected: false,
    };
    openEditDevserver(ds);
    const el = render();
    expect(el.textContent).toContain("Save changes");
    // The first text input is the Devserver URL field, seeded from the entry.
    const urlField = el.querySelector('input[type="text"]') as HTMLInputElement | null;
    expect(urlField?.value).toBe("https://edit.example:8123");
    const token = el.querySelector('input[type="password"]') as HTMLInputElement | null;
    expect(token?.value).toBe("");
    expect(token?.placeholder).toContain("leave blank to keep");
    // The edit form has no choice switcher.
    expect(el.querySelectorAll('[role="radio"]').length).toBe(0);
  });

  it("opens read-only (OK, no Save, disabled inputs) for a connected devserver", () => {
    const ds: DevserverEntry = {
      id: "ds-live",
      url: "https://live.example:8200",
      label: "live",
      script: "",
      has_token: false,
      library_id: "lib-abc",
      connected: true,
    };
    openEditDevserver(ds);
    const el = render();
    // A read-only notice, no Save, an OK dismiss instead.
    expect(el.textContent).toContain("read-only");
    expect(el.textContent).not.toContain("Save changes");
    const ok = [...el.querySelectorAll("button")].find((b) => b.textContent?.trim() === "OK");
    expect(ok).toBeTruthy();
    // The fields are disabled (no edits while connected).
    const urlField = el.querySelector('input[type="text"]') as HTMLInputElement;
    expect(urlField.disabled).toBe(true);
    expect((el.querySelector("textarea") as HTMLTextAreaElement).disabled).toBe(true);
  });
});
