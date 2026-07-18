import type { TerminalKeyboardProtocolState } from "./keymap";

/// A coding agent whose terminal submit encoding chan knows. The Team
/// Work prompt carries the picked agent (`TeamWorkTab.agentTarget`) so a
/// submit appends the right chord. `"none"` (shell mode) never reaches
/// here.
export type SubmitAgent = "claude" | "codex" | "gemini" | "opencode";

/// Per-agent submit chords: the byte sequence each agent reads as "Enter /
/// submit this compose buffer". This is the TypeScript half of the shared
/// map; the Rust half lives in
/// `crates/chan-shell/src/submit.rs::SubmitAgent::submit_chord` and must
/// stay in sync byte-for-byte. Both halves feed the same PTY, so a drift
/// is a runtime bug with a green build.
///
///   - claude: xterm modifyOtherKeys CSI for Cmd+Enter, live-probed
///     2026-05-20. A bare `\n` lands as a newline in Claude's multi-line
///     draft and never submits.
///   - gemini: a plain CR suffix (live-probed 2026-05-31).
///   - codex: reads a plain CR as Enter, but only as a distinct keypress;
///     `encodeForAgentSubmit` wraps codex's text in bracketed paste so the
///     trailing CR is not coalesced into a paste burst (live-probed
///     2026-06-02 against codex-cli 0.136.0). See `encodeForAgentSubmit`.
///   - opencode: accepts bracketed paste followed by CR in the same PTY
///     write, live-probed 2026-07-18 against OpenCode 1.18.3.
export const AGENT_SUBMIT_CHORDS: Record<SubmitAgent, string> = {
  claude: "\x1b[27;9;13~",
  codex: "\r",
  gemini: "\r",
  opencode: "\r",
};

/// Claude Code's submit chord, kept as a named export for callers that
/// submit to a Claude agent directly (the historical default before the
/// per-agent map). New code should prefer `encodeForAgentSubmit(buf,
/// agent)` so the agent's own encoding is used.
export const AGENT_SUBMIT_CHORD = AGENT_SUBMIT_CHORDS.claude;

/// Strip trailing newlines from the team-work buffer, then encode it so the
/// picked agent submits hands-free. A stray `\n` between the buffer text and
/// the submit lands as a newline inside the agent's multi-line draft,
/// splitting the buffer across lines before submit fires.
///
/// claude/gemini take a plain suffix chord. codex and opencode use explicit
/// bracketed-paste delimiters (`\x1b[200~` .. `\x1b[201~`) followed by CR.
/// Codex needs the wrap so the CR is read as a distinct Enter keypress;
/// opencode uses the same bytes as its multiline-safe default. Interior
/// newlines are preserved inside the paste. Mirrors
/// `apply_submit_chord` in `crates/chan-shell/src/submit.rs` byte-for-byte.
///
/// `agent` defaults to `"claude"` so existing single-agent callers keep
/// the prior behavior; Team Work passes the prompt's `agentTarget`.
///
/// In shell mode this normalisation is skipped entirely (the editor's
/// trailing `\n` IS the Enter the shell needs); shell mode never calls
/// this function.
export function encodeForAgentSubmit(
  buffer: string,
  agent: SubmitAgent = "claude",
): string {
  const text = buffer.replace(/\n+$/, "");
  if (agent === "codex" || agent === "opencode") {
    return `\x1b[200~${text}\x1b[201~${AGENT_SUBMIT_CHORDS[agent]}`;
  }
  return text + AGENT_SUBMIT_CHORDS[agent];
}

/// Infer an agent from the keyboard protocol a running TUI announced. This is
/// only a fallback for old servers and agents launched manually from a shell;
/// a current server reports the spawn-derived identity in its session frame.
/// OpenCode may announce the kitty protocol and classify as codex here, which
/// is byte-compatible with OpenCode's default encoding.
export function inferSubmitAgentFromKeyboardProtocol(
  protocol?: TerminalKeyboardProtocolState,
): SubmitAgent {
  if (protocol) {
    if (protocol.xtermModifyOtherKeys > 0) return "claude";
    const kittyFlags =
      protocol.kitty.screen === "alternate"
        ? protocol.kitty.alternateFlags
        : protocol.kitty.mainFlags;
    if (kittyFlags > 0) return "codex";
  }
  return "gemini";
}

/// Prefer the spawn-derived server identity and fall back to protocol
/// inference only when the session frame omitted it.
export function submitAgentForTerminal(
  serverAgent: SubmitAgent | undefined,
  protocol?: TerminalKeyboardProtocolState,
): SubmitAgent {
  return serverAgent ?? inferSubmitAgentFromKeyboardProtocol(protocol);
}
