import { describe, expect, test } from "vitest";
import { TERMINAL_ONLY_COMMANDS, windowModeAllowsCommand } from "./windowMode";

describe("terminal-only command gate", () => {
  test("allows the command launcher in standalone and control terminal windows", () => {
    expect(TERMINAL_ONLY_COMMANDS.has("app.launcher.toggle")).toBe(true);
    expect(TERMINAL_ONLY_COMMANDS.has("app.settings.open")).toBe(true);
    expect(TERMINAL_ONLY_COMMANDS.has("app.window.reload")).toBe(true);
    expect(TERMINAL_ONLY_COMMANDS.has("app.pane.mode")).toBe(true);
    expect(
      windowModeAllowsCommand("app.launcher.toggle", {
        terminalOnly: true,
        terminalControl: false,
      }),
    ).toBe(true);
    expect(
      windowModeAllowsCommand("app.launcher.toggle", {
        terminalOnly: true,
        terminalControl: true,
      }),
    ).toBe(true);
  });

  test("allows the window hide/close family in terminal-only and control windows", () => {
    // Standalone terminal (and control) windows are library windows with a
    // red-dot close prompt, so the self-hide command stays live there like
    // close/confirmClose.
    expect(TERMINAL_ONLY_COMMANDS.has("app.window.close")).toBe(true);
    expect(TERMINAL_ONLY_COMMANDS.has("app.window.hide")).toBe(true);
    expect(
      windowModeAllowsCommand("app.window.hide", {
        terminalOnly: true,
        terminalControl: true,
      }),
    ).toBe(true);
  });

  test("still blocks workspace-only commands in terminal-only windows", () => {
    expect(
      windowModeAllowsCommand("app.graph.toggle", {
        terminalOnly: true,
        terminalControl: false,
      }),
    ).toBe(false);
  });
});
