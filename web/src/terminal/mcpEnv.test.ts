import { describe, expect, test, vi } from "vitest";
import { injectShowMcpEnvCommand, SHOW_MCP_ENV_COMMAND } from "./mcpEnv";

describe("terminal MCP env command injection", () => {
  test("writes the canonical grep command plus newline", () => {
    const sendInput = vi.fn();

    injectShowMcpEnvCommand(sendInput);

    expect(sendInput).toHaveBeenCalledWith(`${SHOW_MCP_ENV_COMMAND}\n`);
  });
});
