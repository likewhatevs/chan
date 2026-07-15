// @vitest-environment jsdom
//
// PathPromptModal "open" mode (item 8): existing entries are the normal
// case (opens / opens directory), a missing path gets the ruling-6
// "creates and opens" disclosure, a chan://graph link bypasses path
// validation (parseGraphLink judges it), NO .md is ever auto-appended,
// and the resolve carries the raw typed path so extensionless names
// reach the server's content sniff verbatim.

import { mount, tick, unmount } from "svelte";
import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";

import PathPromptModal from "./PathPromptModal.svelte";
import { resolvePathPrompt, tree, uiPathPrompt } from "../state/store.svelte";

const mounted: Array<Record<string, unknown>> = [];

function mountModal(): HTMLElement {
  const target = document.createElement("div");
  document.body.append(target);
  mounted.push(mount(PathPromptModal, { target }) as Record<string, unknown>);
  return target;
}

/// Open the dialog in open mode and type `text`. The resolve promise rides
/// back INSIDE an object: returning it bare would let the caller's `await`
/// chain (flatten) it and block on the still-open dialog.
async function openDialog(
  target: HTMLElement,
  text: string,
): Promise<{ promise: Promise<string | null> }> {
  const promise = uiPathPrompt({
    title: "Open path or chan://graph link",
    kind: "either",
    mode: "open",
    allowAbsolute: true,
  });
  await tick();
  const input = target.querySelector("input")!;
  input.value = text;
  input.dispatchEvent(new Event("input", { bubbles: true }));
  await tick();
  return { promise };
}

function statusText(target: HTMLElement): string {
  return target.querySelector(".status")!.textContent!.replace(/\s+/g, " ").trim();
}

function okButton(target: HTMLElement): HTMLButtonElement {
  return target.querySelector(".actions .ok")!;
}

beforeEach(() => {
  tree.entries = [
    { path: "docs", is_dir: true, mtime: null, size: 0 },
    { path: "notes.md", is_dir: false, mtime: null, size: 3 },
  ];
  tree.loadedDirs = { "": true, docs: true };
  tree.loadingDirs = {};
});

afterEach(() => {
  resolvePathPrompt(null);
  for (const c of mounted.splice(0)) unmount(c);
  document.body.innerHTML = "";
  tree.entries = [];
  tree.loadedDirs = {};
  vi.clearAllMocks();
});

describe("PathPromptModal open mode", () => {
  test("an existing file reads as opens, not already-exists", async () => {
    const target = mountModal();
    const { promise } = await openDialog(target, "notes.md");
    expect(statusText(target)).toBe("→ opens notes.md");
    expect(okButton(target).disabled).toBe(false);
    okButton(target).click();
    await expect(promise).resolves.toBe("notes.md");
  });

  test("an existing directory reads as opens directory", async () => {
    const target = mountModal();
    await openDialog(target, "docs");
    expect(statusText(target)).toBe("→ opens directory docs/");
    expect(okButton(target).disabled).toBe(false);
  });

  test("a missing path discloses creates-and-opens and resolves RAW (no .md append)", async () => {
    const target = mountModal();
    const { promise } = await openDialog(target, "readme");
    // Ruling 6 disclosure: the server will create it empty and open it.
    expect(statusText(target)).toContain("creates and opens");
    expect(statusText(target)).toContain("readme");
    // No auto-extension anywhere: the create-mode `.md` append must not
    // run in open mode, or extensionless names would never reach the
    // server's content sniff.
    expect(statusText(target)).not.toContain("readme.md");
    okButton(target).click();
    await expect(promise).resolves.toBe("readme");
  });

  test("a valid graph link bypasses path validation and resolves verbatim", async () => {
    const target = mountModal();
    // `:` and `?` would fail validatePath; the open-mode graph branch must
    // judge the link with parseGraphLink instead.
    const link = "chan://graph?s=ws&d=2";
    const { promise } = await openDialog(target, link);
    expect(statusText(target)).toBe("→ opens graph link");
    expect(okButton(target).disabled).toBe(false);
    okButton(target).click();
    await expect(promise).resolves.toBe(link);
  });

  test("a malformed graph link is rejected inline", async () => {
    const target = mountModal();
    // No scope param -> parseGraphLink returns null.
    await openDialog(target, "chan://graph?d=2");
    expect(statusText(target)).toContain("not a valid chan://graph link");
    expect(okButton(target).disabled).toBe(true);
  });

  test("cancel resolves null (the caller's focus-restore branch)", async () => {
    const target = mountModal();
    const { promise } = await openDialog(target, "notes.md");
    (target.querySelector(".actions .cancel") as HTMLButtonElement).click();
    await expect(promise).resolves.toBeNull();
  });
});
