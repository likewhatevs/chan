//! The per-agent submit-encoding map. A coding agent running inside a
//! chan terminal submits its compose buffer on a different byte sequence
//! depending on which agent it is, so a hands-free completion poke
//! (`cs terminal write --submit=<agent>`) has to append the right one.
//!
//! This is the Rust half of the shared map; the TypeScript half lives in
//! `web/src/terminal/submitMode.ts` (`AGENT_SUBMIT_CHORDS` /
//! `encodeForAgentSubmit`) and must stay in sync byte-for-byte. Keeping
//! the chords in one enum here means a new agent is added in one place.

#[cfg(feature = "client")]
use clap::ValueEnum;

/// A coding agent whose terminal submit encoding chan knows. Selected by
/// `cs terminal write --submit=<agent>`; absent means write pure bytes
/// (no chord), which is the historical default and stays the default.
///
/// The `ValueEnum` parse impl (for the client's `--submit` flag) is
/// `client`-gated so chan-server can read the chord map without linking
/// clap; the chord bytes themselves are clap-free.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "client", derive(ValueEnum))]
#[cfg_attr(feature = "client", value(rename_all = "lower"))]
pub enum SubmitAgent {
    /// Claude Code. Submits on the xterm modifyOtherKeys CSI for
    /// Cmd+Enter (`\x1b[27;9;13~`), live-probed 2026-05-20.
    Claude,
    /// OpenAI codex. Submits on a plain CR; it ignores the Claude chord
    /// silently (so the Claude chord parks the buffer unsubmitted).
    Codex,
    /// Google gemini. Submits on a plain CR (live-probed 2026-05-31 in a
    /// chan terminal: the Claude chord left the buffer unsubmitted).
    Gemini,
}

impl SubmitAgent {
    /// Resolve an agent NAME ("claude" | "codex" | "gemini") to its variant
    /// without clap's `ValueEnum::from_str` (so a caller that only has the
    /// string, e.g. the chan-server team spawner reading a member's `agent`,
    /// does not have to pull clap in). Returns `None` for an unknown name.
    pub fn from_agent_name(name: &str) -> Option<Self> {
        match name {
            "claude" => Some(SubmitAgent::Claude),
            "codex" => Some(SubmitAgent::Codex),
            "gemini" => Some(SubmitAgent::Gemini),
            _ => None,
        }
    }

    /// The byte sequence that makes this agent submit its compose buffer.
    /// These ARE the wire bytes written to the PTY; changing one changes
    /// runtime behavior with a green build, so the map is the single
    /// source of truth (mirrored in `submitMode.ts`).
    pub fn submit_chord(self) -> &'static str {
        match self {
            // xterm modifyOtherKeys CSI for Cmd+Enter. A bare newline
            // lands as a newline in Claude's multi-line draft, not a
            // submit.
            SubmitAgent::Claude => "\x1b[27;9;13~",
            // codex + gemini both read a plain CR as submit and ignore
            // the Claude chord.
            SubmitAgent::Codex | SubmitAgent::Gemini => "\r",
        }
    }
}

/// `cs terminal write --submit=<agent>`: strip trailing newlines from the
/// bytes then append the agent's submit chord, so a running agent submits
/// the input hands-free (the completion poke). `None` writes the bytes
/// verbatim. A trailing newline before the chord would land as a newline
/// inside the agent's draft, splitting the buffer before submit fires, so
/// we trim first. Mirrors `encodeForAgentSubmit` in `submitMode.ts`.
pub fn apply_submit_chord(data: String, submit: Option<SubmitAgent>) -> String {
    match submit {
        Some(agent) => format!("{}{}", data.trim_end_matches('\n'), agent.submit_chord()),
        None => data,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn submit_chord_strips_trailing_newlines_and_appends_per_agent() {
        // claude -> the modifyOtherKeys Cmd+Enter chord.
        assert_eq!(
            apply_submit_chord("poke\n\n".into(), Some(SubmitAgent::Claude)),
            "poke\x1b[27;9;13~"
        );
        // codex + gemini -> a plain CR.
        assert_eq!(
            apply_submit_chord("poke\n".into(), Some(SubmitAgent::Codex)),
            "poke\r"
        );
        assert_eq!(
            apply_submit_chord("poke".into(), Some(SubmitAgent::Gemini)),
            "poke\r"
        );
        // Unset -> bytes verbatim (no chord, trailing newline kept).
        assert_eq!(apply_submit_chord("poke\n".into(), None), "poke\n");
    }

    // ValueEnum parsing only exists with the `client` feature (the
    // `--submit` flag); the chord map below is tested unconditionally.
    #[cfg(feature = "client")]
    #[test]
    fn submit_agent_value_enum_parses_lowercase() {
        // The flag accepts the lower-case agent names the docs use.
        assert_eq!(
            SubmitAgent::from_str("claude", true).unwrap(),
            SubmitAgent::Claude
        );
        assert_eq!(
            SubmitAgent::from_str("codex", true).unwrap(),
            SubmitAgent::Codex
        );
        assert_eq!(
            SubmitAgent::from_str("gemini", true).unwrap(),
            SubmitAgent::Gemini
        );
        assert!(SubmitAgent::from_str("turbo", true).is_err());
    }

    #[test]
    fn from_agent_name_parses_the_known_names() {
        assert_eq!(
            SubmitAgent::from_agent_name("claude"),
            Some(SubmitAgent::Claude)
        );
        assert_eq!(
            SubmitAgent::from_agent_name("codex"),
            Some(SubmitAgent::Codex)
        );
        assert_eq!(
            SubmitAgent::from_agent_name("gemini"),
            Some(SubmitAgent::Gemini)
        );
        assert_eq!(SubmitAgent::from_agent_name("turbo"), None);
        // Same chord whether parsed by clap's ValueEnum or by name.
        assert_eq!(
            SubmitAgent::from_agent_name("claude").map(SubmitAgent::submit_chord),
            Some("\x1b[27;9;13~"),
        );
    }
}
