export const SHOW_MCP_ENV_COMMAND = "env | sort | grep '^CHAN_MCP_'";

export function injectShowMcpEnvCommand(sendInput: (data: string) => void): void {
  sendInput(`${SHOW_MCP_ENV_COMMAND}\n`);
}
