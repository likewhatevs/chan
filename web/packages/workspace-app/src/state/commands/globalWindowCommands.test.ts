// The additive Global window commands (Reload / Open Inspector) that
// mirror the WebView native menu. Importing the module is the registration
// side effect; allCommands()/availableCommands() then expose the catalog.

import { afterEach, describe, expect, it } from "vitest";
import {
  allCommands,
  availableCommands,
  type CommandContext,
} from "../commands";

import "./global";

type TauriWindow = typeof window & { __TAURI__?: unknown };

function ctx(): CommandContext {
  return {
    terminalOnly: false,
    terminalControl: false,
    activeSurface: null,
    activeSide: null,
    activeTabId: null,
  };
}

function categoryOf(id: string): string | undefined {
  return allCommands().find((c) => c.id === id)?.category;
}

function idsIn(c: CommandContext): Set<string> {
  return new Set(availableCommands(c).map((cmd) => cmd.id));
}

afterEach(() => {
  delete (window as TauriWindow).__TAURI__;
});

describe("Global window commands", () => {
  it("registers Reload and Open Inspector under Global", () => {
    expect(categoryOf("app.window.reload")).toBe("Global");
    expect(categoryOf("app.window.devtools")).toBe("Global");
  });

  it("offers Reload in every window", () => {
    expect(idsIn(ctx()).has("app.window.reload")).toBe(true);
  });

  it("gates Open Inspector to the desktop shell", () => {
    // Web: no Tauri runtime, so the browser's own DevTools stand in.
    expect(idsIn(ctx()).has("app.window.devtools")).toBe(false);
    // Desktop: a Tauri runtime is present, so the command is offered.
    (window as TauriWindow).__TAURI__ = {};
    expect(idsIn(ctx()).has("app.window.devtools")).toBe(true);
  });

  it("registers Hide window under Global, gated to the desktop shell", () => {
    expect(categoryOf("app.window.hide")).toBe("Global");
    // Web: the bury IPC is an explicit no-op, so the entry is not offered.
    expect(idsIn(ctx()).has("app.window.hide")).toBe(false);
    (window as TauriWindow).__TAURI__ = {};
    expect(idsIn(ctx()).has("app.window.hide")).toBe(true);
  });

  it("offers Hide window in a standalone terminal window on desktop", () => {
    // A terminal-only window is a library window with the same red-dot hide
    // semantics; the entry ignores the window mode (the id is also in
    // TERMINAL_ONLY_COMMANDS, so the chan:command bridge agrees).
    (window as TauriWindow).__TAURI__ = {};
    expect(
      idsIn({ ...ctx(), terminalOnly: true }).has("app.window.hide"),
    ).toBe(true);
  });
});
