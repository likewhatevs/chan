import { describe, expect, test } from "vitest";
import { AGENT_SUBMIT_CHORD, encodeForAgentSubmit } from "./submitMode";

describe("submitMode", () => {
  test("AGENT_SUBMIT_CHORD matches the probe-pinned Claude Code chord", () => {
    // Claude Code accepts this byte sequence as the "submit" chord
    // (xterm modifyOtherKeys CSI for Cmd+Enter). Probed live
    // 2026-05-20.
    expect(AGENT_SUBMIT_CHORD).toBe("\x1b[27;9;13~");
  });

  test("encodeForAgentSubmit strips trailing newline before chord", () => {
    expect(encodeForAgentSubmit("ship it\n")).toBe("ship it\x1b[27;9;13~");
  });

  test("encodeForAgentSubmit collapses multiple trailing newlines", () => {
    expect(encodeForAgentSubmit("hello\n\n\n")).toBe("hello\x1b[27;9;13~");
  });

  test("encodeForAgentSubmit preserves interior newlines", () => {
    // Multi-paragraph team-work drafts keep their structure;
    // only the trailing newline (which would land as a stray
    // line break inside the agent's draft before submit fires)
    // gets stripped.
    expect(encodeForAgentSubmit("line one\nline two\n")).toBe(
      "line one\nline two\x1b[27;9;13~",
    );
  });

  test("encodeForAgentSubmit no-ops on a buffer with no trailing newline", () => {
    expect(encodeForAgentSubmit("ship it")).toBe("ship it\x1b[27;9;13~");
  });

  test("encodeForAgentSubmit handles empty buffer", () => {
    expect(encodeForAgentSubmit("")).toBe("\x1b[27;9;13~");
  });
});
