import { describe, expect, test } from "vitest";
import { terminalWsPath } from "./session";

describe("terminalWsPath", () => {
  test("omits session query fields for a fresh terminal", () => {
    expect(
      terminalWsPath({
        cols: 100,
        rows: 31,
        tabName: "Terminal",
      }),
    ).toBe("/api/terminal/ws?cols=100&rows=31&tab_name=Terminal");
  });

  test("adds session and since fields when reattaching", () => {
    expect(
      terminalWsPath({
        cols: 80,
        rows: 24,
        tabName: "build log",
        sessionId: "term_abc",
        lastSeq: 42,
      }),
    ).toBe(
      "/api/terminal/ws?cols=80&rows=24&tab_name=build+log&session=term_abc&since=42&agent_echo_since=0",
    );
  });

  test("adds window id when provided", () => {
    expect(
      terminalWsPath({
        cols: 80,
        rows: 24,
        tabName: "shell",
        windowId: "workspace-notes-7",
      }),
    ).toBe(
      "/api/terminal/ws?cols=80&rows=24&tab_name=shell&window_id=workspace-notes-7",
    );
  });

  test("reattach starts from zero when no sequence was persisted", () => {
    expect(
      terminalWsPath({
        cols: 80,
        rows: 24,
        tabName: "shell",
        sessionId: "term_abc",
      }),
    ).toContain("&since=0");
    expect(
      terminalWsPath({
        cols: 80,
        rows: 24,
        tabName: "shell",
        sessionId: "term_abc",
      }),
    ).toContain("&agent_echo_since=0");
  });

  test("adds agent event echo replay cursor when reattaching", () => {
    expect(
      terminalWsPath({
        cols: 80,
        rows: 24,
        tabName: "shell",
        sessionId: "term_abc",
        agentEchoSince: 7,
      }),
    ).toBe(
      "/api/terminal/ws?cols=80&rows=24&tab_name=shell&session=term_abc&since=0&agent_echo_since=7",
    );
  });

  test("never emits a per-terminal mcp_env override (now a global setting)", () => {
    // The SPA no longer forces `?mcp_env=`; injection is governed by
    // the global `terminal.mcp_env` server config. Neither a fresh
    // spawn nor a reattach should carry the param.
    expect(
      terminalWsPath({ cols: 80, rows: 24, tabName: "plain shell" }),
    ).not.toContain("mcp_env");
    expect(
      terminalWsPath({
        cols: 80,
        rows: 24,
        tabName: "reattach",
        sessionId: "term_abc",
        lastSeq: 1,
      }),
    ).not.toContain("mcp_env");
  });

  test("adds tab_group only for non-default groups", () => {
    expect(
      terminalWsPath({
        cols: 80,
        rows: 24,
        tabName: "agent",
        tabGroup: "foobar",
      }),
    ).toBe("/api/terminal/ws?cols=80&rows=24&tab_name=agent&tab_group=foobar");
    // The default group is implicit; never on the wire.
    expect(
      terminalWsPath({
        cols: 80,
        rows: 24,
        tabName: "plain",
        tabGroup: "default",
      }),
    ).not.toContain("tab_group");
  });

  test("adds cwd only for fresh terminal sessions", () => {
    expect(
      terminalWsPath({
        cols: 80,
        rows: 24,
        tabName: "from here",
        cwd: "notes/work",
      }),
    ).toBe("/api/terminal/ws?cols=80&rows=24&tab_name=from+here&cwd=notes%2Fwork");
    expect(
      terminalWsPath({
        cols: 80,
        rows: 24,
        tabName: "reattach",
        sessionId: "term_abc",
        cwd: "notes/work",
      }),
    ).not.toContain("cwd");
  });
});
