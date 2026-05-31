/// A coding agent whose terminal submit encoding chan knows. The Team
/// Work prompt carries the picked agent (`TeamWorkTab.agentTarget`) so a
/// submit appends the right chord. `"none"` (shell mode) never reaches
/// here.
export type SubmitAgent = "claude" | "codex" | "gemini";

/// Per-agent submit chords: the byte sequence each agent reads as "submit
/// this compose buffer". This is the TypeScript half of the shared map;
/// the Rust half lives in
/// `crates/chan-shell/src/submit.rs::SubmitAgent::submit_chord` and must
/// stay in sync byte-for-byte. Both halves feed the same PTY, so a drift
/// is a runtime bug with a green build.
///
///   - claude: xterm modifyOtherKeys CSI for Cmd+Enter, live-probed
///     2026-05-20. A bare `\n` lands as a newline in Claude's multi-line
///     draft and never submits.
///   - codex / gemini: a plain CR. Both submit on `\r` and ignore the
///     Claude chord silently (gemini live-probed 2026-05-31).
export const AGENT_SUBMIT_CHORDS: Record<SubmitAgent, string> = {
  claude: "\x1b[27;9;13~",
  codex: "\r",
  gemini: "\r",
};

/// Claude Code's submit chord, kept as a named export for callers that
/// submit to a Claude agent directly (the historical default before the
/// per-agent map). New code should prefer `encodeForAgentSubmit(buf,
/// agent)` so the agent's own encoding is used.
export const AGENT_SUBMIT_CHORD = AGENT_SUBMIT_CHORDS.claude;

/// Strip trailing newlines from the team-work buffer, then append the
/// picked agent's submit chord. A stray `\n` between the buffer text and
/// the chord lands as a newline inside the agent's multi-line draft,
/// splitting the buffer across lines before submit fires.
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
  return buffer.replace(/\n+$/, "") + AGENT_SUBMIT_CHORDS[agent];
}
