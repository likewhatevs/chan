import { describe, expect, test } from "vitest";
import {
  AGENT_SUBMIT_CHORD,
  AGENT_SUBMIT_CHORDS,
  encodeForAgentSubmit,
  inferSubmitAgentFromKeyboardProtocol,
  submitAgentForTerminal,
} from "./submitMode";
import { createTerminalKeyboardProtocolState } from "./keymap";

describe("submitMode", () => {
  test("AGENT_SUBMIT_CHORD matches the probe-pinned Claude Code chord", () => {
    // Claude Code accepts this byte sequence as the "submit" chord
    // (xterm modifyOtherKeys CSI for Cmd+Enter). Probed live
    // 2026-05-20.
    expect(AGENT_SUBMIT_CHORD).toBe("\x1b[27;9;13~");
  });

  test("per-agent chord map mirrors the Rust SubmitAgent::submit_chord", () => {
    // Must stay byte-identical to crates/chan-shell/src/submit.rs.
    expect(AGENT_SUBMIT_CHORDS).toEqual({
      claude: "\x1b[27;9;13~",
      codex: "\r",
      gemini: "\r",
      opencode: "\r",
    });
  });

  test("encodeForAgentSubmit appends the picked agent's chord", () => {
    // gemini submits on a plain CR suffix; claude on the Cmd+Enter chord.
    expect(encodeForAgentSubmit("ship it\n", "gemini")).toBe("ship it\r");
    expect(encodeForAgentSubmit("ship it\n", "claude")).toBe(
      "ship it\x1b[27;9;13~",
    );
  });

  test("encodeForAgentSubmit wraps codex in bracketed paste then CR", () => {
    // codex coalesces a bare text+CR write into a paste burst (the CR lands
    // as a literal newline and never submits). Wrapping in explicit
    // bracketed-paste delimiters makes the trailing CR a distinct Enter.
    // Must stay byte-identical to apply_submit_chord in submit.rs.
    expect(encodeForAgentSubmit("ship it\n", "codex")).toBe(
      "\x1b[200~ship it\x1b[201~\r",
    );
    // Interior newlines are preserved inside the paste; only trailing ones
    // are stripped before the wrap.
    expect(encodeForAgentSubmit("line one\nline two\n\n", "codex")).toBe(
      "\x1b[200~line one\nline two\x1b[201~\r",
    );
  });

  test("encodeForAgentSubmit wraps opencode in bracketed paste then CR", () => {
    expect(encodeForAgentSubmit("ship it\n", "opencode")).toBe(
      "\x1b[200~ship it\x1b[201~\r",
    );
    expect(encodeForAgentSubmit("line one\nline two\n\n", "opencode")).toBe(
      "\x1b[200~line one\nline two\x1b[201~\r",
    );
  });

  test("opencode paste-sized encoding is one exact Rust-parity payload", () => {
    const body = `HEAD${"x".repeat(20 * 1024)}TAIL`;
    expect(encodeForAgentSubmit(body, "opencode")).toBe(
      `\x1b[200~${body}\x1b[201~\r`,
    );
  });

  test("encodeForAgentSubmit defaults to the Claude chord", () => {
    // Existing single-agent callers omit the agent arg and keep the
    // prior Claude behavior.
    expect(encodeForAgentSubmit("ship it\n")).toBe("ship it\x1b[27;9;13~");
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

  test("server identity wins over keyboard-protocol fallback", () => {
    const protocol = createTerminalKeyboardProtocolState();
    protocol.xtermModifyOtherKeys = 1;
    expect(submitAgentForTerminal("opencode", protocol)).toBe("opencode");
    expect(submitAgentForTerminal(undefined, protocol)).toBe("claude");
  });

  test("protocol fallback keeps the existing claude/codex/gemini inference", () => {
    const protocol = createTerminalKeyboardProtocolState();
    expect(inferSubmitAgentFromKeyboardProtocol(protocol)).toBe("gemini");
    protocol.kitty.mainFlags = 8;
    expect(inferSubmitAgentFromKeyboardProtocol(protocol)).toBe("codex");
    protocol.xtermModifyOtherKeys = 1;
    expect(inferSubmitAgentFromKeyboardProtocol(protocol)).toBe("claude");
  });
});
