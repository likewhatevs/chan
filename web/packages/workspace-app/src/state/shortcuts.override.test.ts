import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import { chordFor, registerOverrideResolver } from "./shortcuts";

// The override-resolver hook: chordFor consults a runtime-injected
// resolver before the built-in SHORTCUTS. With none registered (the
// shortcuts-table generator and every other test) chordFor resolves
// only the compile-time chords, so the injection is inert by default.

describe("chordFor override-resolver hook", () => {
  beforeEach(() => {
    // Force macOS + web so Mod maps to Cmd and no per-OS divergence
    // muddies the built-in fallbacks under test.
    vi.stubGlobal("navigator", { userAgent: "Mac OS X" });
  });
  afterEach(() => {
    registerOverrideResolver(null);
    vi.unstubAllGlobals();
  });

  test("falls back to the built-in chord when no resolver is registered", () => {
    // app.launcher.toggle is Cmd+K on native mac; on web mac it is the
    // Ctrl+Alt+K fallback. Test env has no Tauri global -> web.
    expect(chordFor("app.launcher.toggle")).toBe("Ctrl+Alt+K");
  });

  test("a registered override wins over the built-in chord", () => {
    registerOverrideResolver((id) => (id === "app.search.toggle" ? "Mod+J" : null));
    // Mod formats to Cmd on mac.
    expect(chordFor("app.search.toggle")).toBe("Cmd+J");
    // Un-overridden ids still resolve to their built-in chord.
    expect(chordFor("app.launcher.toggle")).toBe("Ctrl+Alt+K");
  });

  test("an override resolves a chordless command with no SHORTCUTS entry", () => {
    // Chordless commands (only in the command catalog) return null today;
    // an assigned override makes chordFor resolve them.
    expect(chordFor("app.dashboard.slide.workspace")).toBeNull();
    registerOverrideResolver((id) =>
      id === "app.dashboard.slide.workspace" ? "Mod+Shift+1" : null,
    );
    expect(chordFor("app.dashboard.slide.workspace")).toBe("Cmd+Shift+1");
  });

  test("a nullish resolver result falls through to the built-in chord", () => {
    registerOverrideResolver(() => undefined);
    expect(chordFor("app.launcher.toggle")).toBe("Ctrl+Alt+K");
  });

  test("registering null restores the bare-registry behaviour", () => {
    registerOverrideResolver(() => "Mod+J");
    expect(chordFor("app.launcher.toggle")).toBe("Cmd+J");
    registerOverrideResolver(null);
    expect(chordFor("app.launcher.toggle")).toBe("Ctrl+Alt+K");
  });
});
