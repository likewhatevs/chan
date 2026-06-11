export type TerminalWsPathOpts = {
  cols: number;
  rows: number;
  tabName: string;
  tabGroup?: string | null;
  windowId?: string | null;
  sessionId?: string | null;
  agentEchoSince?: number | null;
  cwd?: string | null;
};

export function terminalWsPath(opts: TerminalWsPathOpts): string {
  const params = new URLSearchParams({
    cols: String(opts.cols),
    rows: String(opts.rows),
    tab_name: opts.tabName,
  });
  // Only non-default groups go on the wire; the server defaults the
  // per-session tab_group to "default" when absent, keeping the common
  // case's URL short.
  const tabGroup = opts.tabGroup?.trim();
  if (tabGroup && tabGroup !== "default") params.set("tab_group", tabGroup);
  const windowId = opts.windowId?.trim();
  if (windowId) params.set("window_id", windowId);
  const sessionId = opts.sessionId?.trim();
  if (sessionId) {
    params.set("session", sessionId);
    // A reattach always feeds a brand-new EMPTY xterm, so it always
    // wants the session's full replay ring: `since` is the constant 0,
    // not a byte cursor (the cursor was removed - tracking it across
    // remounts is what caused the "only the last line after a split"
    // bug). Explicit 0 rather than absent: `Some(0)` makes the server
    // report bytes lost to ring overflow via `missed_bytes` (the
    // "terminal replay missed N bytes" notice); `None` would silently
    // start at the ring head.
    params.set("since", "0");
    params.set(
      "agent_echo_since",
      String(Math.max(0, Math.floor(opts.agentEchoSince ?? 0))),
    );
  } else {
    const cwd = opts.cwd?.trim();
    if (cwd) params.set("cwd", cwd);
    // MCP env injection is now governed by the global `terminal.mcp_env`
    // server config (set from the Terminal Settings panel); the SPA no
    // longer forces a per-terminal `?mcp_env=` override. The backend
    // still honours an explicit query for `cs terminal new` / team
    // spawns.
  }
  return `/api/terminal/ws?${params.toString()}`;
}
