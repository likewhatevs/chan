// @vitest-environment jsdom

import { mount, tick, unmount } from "svelte";
import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";

// Isolate from the real catalog (install pulls every lane's module).
vi.mock("../state/commands/install", () => ({}));

import CommandChordAssign from "./CommandChordAssign.svelte";
import { registerCommands, type Command } from "../state/commands";
import { chordFor } from "../state/shortcuts";
import {
  assignOverride,
  hydrateOverrides,
  overrideChordFor,
  overrideChordForSlot,
  type OverrideSlot,
} from "../state/keymapOverrides.svelte";

function cmd(id: string, title: string): Command {
  return { id, title, category: "Global", available: () => true, run: () => {} };
}

// A chorded command (real SHORTCUTS id) and a chordless one.
registerCommands([
  cmd("app.search.toggle", "Search"),
  cmd("app.custom.demo", "Demo"),
]);

const mounted: Array<Record<string, unknown>> = [];

async function flush(): Promise<void> {
  await tick();
  await tick();
}

function mountAssign(command: Command, slot?: OverrideSlot): HTMLElement {
  const target = document.createElement("div");
  document.body.append(target);
  const props = slot ? { cmd: command, slot } : { cmd: command };
  mounted.push(
    mount(CommandChordAssign, { target, props }) as Record<string, unknown>,
  );
  return target;
}

function key(target: HTMLElement, init: KeyboardEventInit): void {
  const el = target.querySelector(".capture") as HTMLElement;
  el.dispatchEvent(new KeyboardEvent("keydown", { bubbles: true, ...init }));
}

describe("CommandChordAssign", () => {
  beforeEach(() => {
    vi.stubGlobal("navigator", { userAgent: "Mac OS X" });
  });
  afterEach(() => {
    for (const c of mounted.splice(0)) unmount(c);
    document.body.innerHTML = "";
    hydrateOverrides(null);
    vi.unstubAllGlobals();
  });

  test("shows the built-in chord for a chorded command", () => {
    const target = mountAssign(cmd("app.search.toggle", "Search"));
    const btn = target.querySelector(".chord-btn") as HTMLElement;
    expect(btn.textContent?.trim()).toBe("Cmd+S");
    expect(target.querySelector(".reset")).toBeNull();
  });

  test("shows Assign for a command with no chord", () => {
    const target = mountAssign(cmd("app.custom.demo", "Demo"));
    expect((target.querySelector(".chord-btn") as HTMLElement).textContent?.trim()).toBe(
      "Assign",
    );
  });

  test("capturing a free chord assigns it and reveals a reset control", async () => {
    const command = cmd("app.custom.demo", "Demo");
    const target = mountAssign(command);
    (target.querySelector(".chord-btn") as HTMLElement).click();
    await flush();
    expect(target.querySelector(".capture")).not.toBeNull();

    key(target, { key: "j", metaKey: true });
    await flush();

    expect(overrideChordFor("app.custom.demo")).toBe("Mod+J");
    const btn = target.querySelector(".chord-btn") as HTMLElement;
    expect(btn.textContent?.trim()).toBe("Cmd+J");
    expect(target.querySelector(".reset")).not.toBeNull();
  });

  test("a conflicting chord is reported and not assigned", async () => {
    // Search already holds Cmd+S (its built-in); try to bind it to Demo.
    const command = cmd("app.custom.demo", "Demo");
    const target = mountAssign(command);
    (target.querySelector(".chord-btn") as HTMLElement).click();
    await flush();

    key(target, { key: "s", metaKey: true });
    await flush();

    const capture = target.querySelector(".capture") as HTMLElement;
    expect(capture).not.toBeNull(); // still capturing, not committed
    expect(capture.classList.contains("conflict")).toBe(true);
    expect(capture.textContent).toContain("In use by Search");
    expect(overrideChordFor("app.custom.demo")).toBeUndefined();
  });

  test("reset clears the override back to the built-in", async () => {
    assignOverride("app.search.toggle", "Mod+J");
    const target = mountAssign(cmd("app.search.toggle", "Search"));
    expect((target.querySelector(".chord-btn") as HTMLElement).textContent?.trim()).toBe(
      "Cmd+J",
    );
    (target.querySelector(".reset") as HTMLElement).click();
    await flush();
    expect(overrideChordFor("app.search.toggle")).toBeUndefined();
    expect(chordFor("app.search.toggle")).toBe("Cmd+S");
  });

  test("an explicit slot assigns that OS only, leaving the client slot alone", async () => {
    // A mac browser (web slot) editing the linux column via the grid.
    const command = cmd("app.custom.demo", "Demo");
    const target = mountAssign(command, "linux");
    // The linux cell shows the linux built-in? Demo is chordless -> Assign.
    expect((target.querySelector(".chord-btn") as HTMLElement).textContent?.trim()).toBe(
      "Assign",
    );
    (target.querySelector(".chord-btn") as HTMLElement).click();
    await flush();
    key(target, { key: "j", metaKey: true });
    await flush();
    expect(overrideChordForSlot("app.custom.demo", "linux")).toBe("Mod+J");
    // The web slot (this client) is untouched.
    expect(overrideChordForSlot("app.custom.demo", "web")).toBeUndefined();
    // The cell renders the linux label (Ctrl+J), not the mac Cmd+J.
    expect((target.querySelector(".chord-btn") as HTMLElement).textContent?.trim()).toBe(
      "Ctrl+J",
    );
  });
});
