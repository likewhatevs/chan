import { describe, it, expect, afterEach } from "vitest";
import { mount, unmount, flushSync } from "svelte";
import NewWorkspaceDialog from "./NewWorkspaceDialog.svelte";
import { openNewDialog, openEditDevserver, selectChoice, closeDialog } from "../state/dialog.svelte";
import type { DevserverEntry } from "../api/library";

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

  it("shows the masked token field + connect command for the devserver choice", () => {
    openNewDialog("devserver");
    const el = render();
    expect(el.querySelector('input[type="password"]')).not.toBeNull();
    expect(el.querySelector("textarea")).not.toBeNull();
    expect(el.textContent).toContain("Token");
    expect(el.textContent).toContain("Add devserver");
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
    };
    openEditDevserver(ds);
    const el = render();
    expect(el.textContent).toContain("Save changes");
    const host = el.querySelector('input[type="text"]') as HTMLInputElement | null;
    expect(host?.value).toBe("edit.example");
    const token = el.querySelector('input[type="password"]') as HTMLInputElement | null;
    expect(token?.value).toBe("");
    expect(token?.placeholder).toContain("leave blank to keep");
    // The edit form has no choice switcher.
    expect(el.querySelectorAll('[role="radio"]').length).toBe(0);
  });
});
