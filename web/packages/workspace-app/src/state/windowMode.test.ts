import { describe, expect, test } from "vitest";
import { TERMINAL_ONLY_COMMANDS, windowModeAllowsCommand } from "./windowMode";

describe("terminal-only command gate", () => {
  test("allows the command launcher in standalone and control terminal windows", () => {
    expect(TERMINAL_ONLY_COMMANDS.has("app.launcher.toggle")).toBe(true);
    expect(TERMINAL_ONLY_COMMANDS.has("app.settings.open")).toBe(true);
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

  test("still blocks workspace-only commands in terminal-only windows", () => {
    expect(
      windowModeAllowsCommand("app.graph.toggle", {
        terminalOnly: true,
        terminalControl: false,
      }),
    ).toBe(false);
  });
});
