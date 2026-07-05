import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import { chordFor } from "./shortcuts";
import type { Command } from "./commands";
import {
  assignOverride,
  clearOverride,
  commandIdForChord,
  currentSlot,
  hydrateOverrides,
  overrideChordFor,
  registerOverridePersist,
  resolvedKeymapEntries,
  serializeOverrides,
} from "./keymapOverrides.svelte";

// Importing the store registers its resolver with shortcuts.ts, so chordFor
// is override-aware for this whole test file (vitest isolates files).

function cmd(id: string): Command {
  return {
    id,
    title: id,
    category: "Global",
    available: () => true,
    run: () => {},
  };
}

function tauri(on: boolean): void {
  const w = window as typeof window & { __TAURI__?: unknown };
  if (on) w.__TAURI__ = {};
  else delete w.__TAURI__;
}

describe("keymap override store", () => {
  beforeEach(() => {
    // Web mac by default (no Tauri global -> web slot).
    vi.stubGlobal("navigator", { userAgent: "Mac OS X" });
    tauri(false);
  });
  afterEach(() => {
    hydrateOverrides(null);
    registerOverridePersist(null);
    tauri(false);
    vi.unstubAllGlobals();
  });

  test("currentSlot follows the client: browser -> web, desktop -> native OS", () => {
    expect(currentSlot()).toBe("web");
    tauri(true);
    expect(currentSlot()).toBe("macos");
    vi.stubGlobal("navigator", { userAgent: "Windows" });
    expect(currentSlot()).toBe("windows");
  });

  test("assign makes chordFor resolve the override over the built-in", () => {
    // app.search.toggle built-in is Cmd+S on web mac.
    expect(chordFor("app.search.toggle")).toBe("Cmd+S");
    assignOverride("app.search.toggle", "Mod+J");
    expect(overrideChordFor("app.search.toggle")).toBe("Mod+J");
    expect(chordFor("app.search.toggle")).toBe("Cmd+J");
  });

  test("clear restores the built-in chord", () => {
    assignOverride("app.search.toggle", "Mod+J");
    clearOverride("app.search.toggle");
    expect(overrideChordFor("app.search.toggle")).toBeUndefined();
    expect(chordFor("app.search.toggle")).toBe("Cmd+S");
  });

  test("an override targets only the current client's slot", () => {
    // Assigned as a browser (web slot). A desktop mac client must not see it.
    assignOverride("app.search.toggle", "Mod+J");
    expect(chordFor("app.search.toggle")).toBe("Cmd+J");
    tauri(true); // now a desktop mac client -> macos slot, unassigned
    expect(overrideChordFor("app.search.toggle")).toBeUndefined();
    expect(chordFor("app.search.toggle")).toBe("Cmd+S");
  });

  test("resolvedKeymapEntries is override-first over the SHORTCUTS baseline", () => {
    const commands = [cmd("app.search.toggle"), cmd("app.custom.chordless")];
    assignOverride("app.search.toggle", "Mod+J");
    assignOverride("app.custom.chordless", "Mod+Y");
    const byId = new Map(
      resolvedKeymapEntries(commands).map((e) => [e.id, e.chord]),
    );
    expect(byId.get("app.search.toggle")).toBe("Mod+J"); // override wins
    expect(byId.get("app.launcher.toggle")).toBe("Ctrl+Alt+K"); // baseline
    expect(byId.get("app.custom.chordless")).toBe("Mod+Y"); // chordless override
  });

  test("commandIdForChord matches an override but not a bare default", () => {
    assignOverride("app.terminal.toggle", "Mod+J");
    expect(commandIdForChord("Mod+J")).toBe("app.terminal.toggle");
    // Cmd+S is app.search.toggle's built-in default, not overridden: the
    // compile-time branch owns it, so the override lookup must miss.
    expect(commandIdForChord("Mod+S")).toBeUndefined();
  });

  test("an override equal to the command's own default does not double-fire", () => {
    // Assigning app.search.toggle back to its own Cmd+S must NOT surface in
    // the dispatch lookup, or the default branch and this path both fire.
    assignOverride("app.search.toggle", "Mod+S");
    expect(commandIdForChord("Mod+S")).toBeUndefined();
  });

  test("serialize / hydrate round-trips and drops junk slots", () => {
    assignOverride("app.search.toggle", "Mod+J", "web");
    assignOverride("app.search.toggle", "Cmd+K", "macos");
    const wire = serializeOverrides();
    expect(wire["app.search.toggle"]).toEqual({ web: "Mod+J", macos: "Cmd+K" });

    hydrateOverrides({
      "app.graph.toggle": { web: "Mod+G", bogus: "x" } as never,
      "app.empty": {},
    });
    const after = serializeOverrides();
    expect(after["app.graph.toggle"]).toEqual({ web: "Mod+G" });
    expect(after["app.empty"]).toBeUndefined();
    expect(after["app.search.toggle"]).toBeUndefined(); // hydrate replaces
  });

  test("persist writer receives the serialized wire on every mutation", () => {
    const seen: unknown[] = [];
    registerOverridePersist((wire) => seen.push(wire));
    assignOverride("app.search.toggle", "Mod+J");
    clearOverride("app.search.toggle");
    expect(seen).toHaveLength(2);
    expect(seen[0]).toEqual({ "app.search.toggle": { web: "Mod+J" } });
    expect(seen[1]).toEqual({});
  });
});
