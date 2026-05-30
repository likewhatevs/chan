/// Chord byte sequence appended to the team-work buffer in Agent
/// submit-mode. Claude Code reads this (xterm modifyOtherKeys CSI
/// for Cmd+Enter) as the "submit" chord; bare `\n` lands as a
/// newline in its multi-line draft and never submits. Probed live
/// 2026-05-20 against a Claude Code session in a chan terminal.
///
/// codex diverges (submits on `\r`, ignores this chord silently);
/// we ship single-chord with Claude Code's encoding. Per-agent
/// encoding map is deferred.
///
/// The server-side mirror lives in
/// `crates/chan-server/src/terminal_sessions.rs::SubmitMode::submit_chord`;
/// both consumers (team-work Cmd+Enter submit, server-side
/// `dispatch_agent_event` survey-reply echo) emit the same chord.
export const AGENT_SUBMIT_CHORD = "\x1b[27;9;13~";

/// Strip trailing newlines from the team-work buffer before
/// appending the agent chord. A stray `\n` between the buffer
/// text and the chord lands as a newline inside the agent's
/// multi-line draft, splitting the buffer across lines before
/// submit fires.
///
/// In shell mode this normalisation is skipped: the editor's
/// trailing `\n` IS the Enter the shell needs.
export function encodeForAgentSubmit(buffer: string): string {
  return buffer.replace(/\n+$/, "") + AGENT_SUBMIT_CHORD;
}
