import { describe, expect, test, vi, beforeEach, afterEach } from "vitest";
import shortcutsRaw from "./shortcuts.ts?raw";
import terminalRaw from "../components/TerminalTab.svelte?raw";
import {
  SHORTCUTS,
  chordFromEvent,
  shouldEscapeTerminal,
} from "./shortcuts";

// Chord-escape registry. Global App-group chords (Settings,
// TeamWork, Reload, FB, Graph, NewDraft, New Terminal, Hybrid
// Nav) carry `escapeTerminal: true`. The
// `handleTerminalKeyEvent` xterm-`customKeyEventHandler`
// callback consults the registry: matched events return false
// so the chord bubbles out of xterm to the App-level keymap.

describe("chord-escape registry shape", () => {
  test("Shortcut type carries an optional escapeTerminal flag", () => {
    expect(shortcutsRaw).toMatch(/escapeTerminal\?: boolean;/);
  });

  test("App-group chords flagged escapeTerminal=true", () => {
    const required = [
      "app.settings.toggle",
      "app.launcher.toggle",
      "app.terminal.teamWork",
      "app.files.toggle",
      "app.graph.toggle",
      "app.terminal.toggle",
      "app.pane.mode",
      "app.window.reload",
      "app.draft.new",
    ];
    for (const id of required) {
      const entry = SHORTCUTS.find((s) => s.id === id);
      expect(entry?.escapeTerminal).toBe(true);
    }
  });

  test("non-flagged chords default to undefined (xterm consumes)", () => {
    // app.tab.close (Ctrl+D) is intentionally not listed: it goes
    // through a different dispatch path and must still reach shells
    // as EOF when a terminal is focused.
    const close = SHORTCUTS.find((s) => s.id === "app.tab.close");
    expect(close?.escapeTerminal).toBeUndefined();
  });
});

describe("chordFromEvent normalisation", () => {
  beforeEach(() => {
    // Force macOS so `Mod` maps to metaKey deterministically.
    vi.stubGlobal("navigator", { userAgent: "Mac OS X" });
  });
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  test("Cmd+P on Mac yields `Mod+P`", () => {
    const e = new KeyboardEvent("keydown", { key: "p", metaKey: true });
    expect(chordFromEvent(e)).toBe("Mod+P");
  });

  test("Cmd+Shift+M on Mac yields `Mod+Shift+M`", () => {
    const e = new KeyboardEvent("keydown", {
      key: "m",
      metaKey: true,
      shiftKey: true,
    });
    expect(chordFromEvent(e)).toBe("Mod+Shift+M");
  });

  test("modifier-only events return null", () => {
    const e = new KeyboardEvent("keydown", { key: "Shift", shiftKey: true });
    expect(chordFromEvent(e)).toBeNull();
  });

  test("plain printable keys (no modifier) return null", () => {
    const e = new KeyboardEvent("keydown", { key: "a" });
    expect(chordFromEvent(e)).toBeNull();
  });
});

describe("shouldEscapeTerminal lookup", () => {
  beforeEach(() => {
    vi.stubGlobal("navigator", { userAgent: "Mac OS X" });
  });
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  test("Cmd+Alt+P (team work web Mac chord) escapes", () => {
    // Test env runs as web platform (no Tauri global). Web Mac
    // users fire the Cmd+Alt+P fallback (Cmd+P is browser-owned
    // for the print dialog). Native chan-desktop's
    // KEY_BRIDGE_JS handles Cmd+P → SPA itself; that path is
    // not exercised in this test surface.
    const e = new KeyboardEvent("keydown", {
      key: "p",
      metaKey: true,
      altKey: true,
    });
    expect(shouldEscapeTerminal(e)).toBe(true);
  });

  test("Cmd+R (reload window) escapes", () => {
    const e = new KeyboardEvent("keydown", { key: "r", metaKey: true });
    expect(shouldEscapeTerminal(e)).toBe(true);
  });

  test("Cmd+Shift+M (graph) escapes", () => {
    // The top-level Cmd+Shift+M graph chord bubbles out of a focused
    // terminal to the App keymap (escapeTerminal on app.graph.toggle).
    const e = new KeyboardEvent("keydown", {
      key: "m",
      metaKey: true,
      shiftKey: true,
    });
    expect(shouldEscapeTerminal(e)).toBe(true);
  });

  test("Cmd+, (settings) escapes", () => {
    const e = new KeyboardEvent("keydown", { key: ",", metaKey: true });
    expect(shouldEscapeTerminal(e)).toBe(true);
  });

  test("Ctrl+Alt+K (web command launcher) escapes", () => {
    const e = new KeyboardEvent("keydown", {
      key: "k",
      ctrlKey: true,
      altKey: true,
    });
    expect(shouldEscapeTerminal(e)).toBe(true);
  });

  test("Ctrl+Alt+K (off-mac command launcher) escapes", () => {
    vi.stubGlobal("navigator", { userAgent: "Windows" });
    const e = new KeyboardEvent("keydown", {
      key: "k",
      ctrlKey: true,
      altKey: true,
    });
    expect(shouldEscapeTerminal(e)).toBe(true);
  });

  test("Cmd+K (native macOS command launcher) escapes", () => {
    const w = window as typeof window & { __TAURI__?: unknown };
    w.__TAURI__ = {};
    const e = new KeyboardEvent("keydown", { key: "k", metaKey: true });
    expect(shouldEscapeTerminal(e)).toBe(true);
    delete w.__TAURI__;
  });

  test("Cmd+. (Hybrid Nav) escapes", () => {
    const e = new KeyboardEvent("keydown", { key: ".", metaKey: true });
    expect(shouldEscapeTerminal(e)).toBe(true);
  });

  test("plain alphabet keys (typing in terminal) do NOT escape", () => {
    const e = new KeyboardEvent("keydown", { key: "a" });
    expect(shouldEscapeTerminal(e)).toBe(false);
  });

  test("Ctrl+D (tab.close, not flagged) does NOT escape", () => {
    const e = new KeyboardEvent("keydown", { key: "d", ctrlKey: true });
    expect(shouldEscapeTerminal(e)).toBe(false);
  });
});

describe("TerminalTab consults shouldEscapeTerminal", () => {
  test("handleTerminalKeyEvent imports + calls shouldEscapeTerminal", () => {
    expect(terminalRaw).toMatch(
      /import \{[\s\S]*?\bshouldEscapeTerminal\b[\s\S]*?\} from "\.\.\/state\/shortcuts";/,
    );
    expect(terminalRaw).toMatch(
      /function handleTerminalKeyEvent\(e: KeyboardEvent\): boolean \{[\s\S]*?if \(shouldEscapeTerminal\(e\)\) return false;/,
    );
  });

  test("rationale comment cites the registry + the xterm-consumption issue", () => {
    expect(terminalRaw).toMatch(/chord-escape registry/i);
    expect(terminalRaw).toMatch(/Without this gate[\s\S]{1,200}swallowed by xterm/i);
  });
});
