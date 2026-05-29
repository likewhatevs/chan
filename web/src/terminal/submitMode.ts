/// `fullstack-b-13`: chord byte sequence appended to the team-work
/// buffer in Agent submit-mode. Claude Code v2.1.145 reads this
/// (xterm modifyOtherKeys CSI for Cmd+Enter) as the "submit"
/// chord; bare `\n` lands as a newline in its multi-line draft
/// and never submits. Probed live 2026-05-20 against a Claude
/// Code session in a chan terminal; full data table at
/// `docs/journals/phase-8/fullstack-b/fullstack-b-13.md`.
///
/// codex v0.130.0 diverges (submits on `\r`, ignores this chord
/// silently). Per @@Alex's "if codex fails it's fine, just want
/// the signal" directive, we ship single-chord with Claude Code's
/// encoding. Per-agent encoding map is parked for Round-3 Track 5.
///
/// The server-side mirror lives in
/// `crates/chan-server/src/terminal_sessions.rs::SubmitMode::submit_chord`;
/// both consumers (team-work Cmd+Enter submit, server-side
/// `dispatch_agent_event` survey-reply echo) emit the same chord.
export const AGENT_SUBMIT_CHORD = "\x1b[27;9;13~";

/// `fullstack-b-13`: strip trailing newlines from the team-work
/// buffer before appending the agent chord. A stray `\n` between
/// the buffer text and the chord lands as a newline inside the
/// agent's multi-line draft, splitting the buffer across lines
/// before submit fires.
///
/// In shell mode this normalisation is skipped: the editor's
/// trailing `\n` IS the Enter the shell needs.
export function encodeForAgentSubmit(buffer: string): string {
  return buffer.replace(/\n+$/, "") + AGENT_SUBMIT_CHORD;
}
