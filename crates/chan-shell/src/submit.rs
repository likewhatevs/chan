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
    /// OpenAI codex. Reads a plain CR as Enter, but ONLY as a distinct
    /// keypress: codex coalesces a single `text + CR` write into a paste
    /// burst and treats the trailing CR as a literal newline, so a bare-CR
    /// suffix never submits. `apply_submit_chord` wraps codex's text in
    /// bracketed paste so the trailing CR lands as a real Enter. It ignores
    /// both the Claude chord and the kitty CSI-u Enter (`\x1b[13u`) silently.
    /// Live-probed 2026-06-02 against codex-cli 0.136.0.
    Codex,
    /// Google gemini. Submits on a plain CR suffix (live-probed 2026-05-31 in
    /// a chan terminal: the Claude chord left the buffer unsubmitted).
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
            // codex + gemini both read a plain CR as Enter. gemini submits on
            // a bare CR suffix; codex needs that CR delivered as a distinct
            // keypress (apply_submit_chord wraps codex's text in bracketed
            // paste so the CR is not coalesced into a paste burst).
            SubmitAgent::Codex | SubmitAgent::Gemini => "\r",
        }
    }
}

/// `cs terminal write --submit=<agent>`: encode `data` into the PTY bytes
/// that make a running agent submit it hands-free (the completion poke).
/// `None` writes the bytes verbatim. Trailing newlines are stripped first: a
/// newline before the submit would land inside the agent's draft, splitting
/// the buffer before submit fires.
///
/// claude/gemini take a plain suffix chord (`submit_chord`). codex is the odd
/// one out: it coalesces a single `text + CR` write into a paste burst and
/// reads the trailing CR as a literal newline, so a bare-CR suffix never
/// submits. Wrapping the text in explicit bracketed-paste delimiters
/// (`\x1b[200~` .. `\x1b[201~`) makes codex insert it as a paste, so the CR
/// after the paste-end marker is read as a distinct Enter keypress and
/// submits. Interior newlines are preserved inside the paste (a multi-line
/// poke arrives as one message). Live-probed 2026-06-02 against codex-cli
/// 0.136.0.
///
/// Mirrors `encodeForAgentSubmit` in `submitMode.ts` byte-for-byte.
pub fn apply_submit_chord(data: String, submit: Option<SubmitAgent>) -> String {
    let Some(agent) = submit else {
        return data;
    };
    let text = data.trim_end_matches('\n');
    match agent {
        SubmitAgent::Codex => format!("\x1b[200~{text}\x1b[201~{}", agent.submit_chord()),
        SubmitAgent::Claude | SubmitAgent::Gemini => format!("{text}{}", agent.submit_chord()),
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
        // codex -> bracketed-paste wrap, then CR. The wrap defeats codex's
        // paste-burst coalescing of a bare text+CR write (which would land
        // the CR as a literal newline and never submit).
        assert_eq!(
            apply_submit_chord("poke\n".into(), Some(SubmitAgent::Codex)),
            "\x1b[200~poke\x1b[201~\r"
        );
        // gemini -> a plain CR suffix.
        assert_eq!(
            apply_submit_chord("poke".into(), Some(SubmitAgent::Gemini)),
            "poke\r"
        );
        // codex keeps interior newlines inside the paste (a multi-line poke is
        // one message) and trims only the trailing ones before the wrap.
        assert_eq!(
            apply_submit_chord("line one\nline two\n\n".into(), Some(SubmitAgent::Codex)),
            "\x1b[200~line one\nline two\x1b[201~\r"
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
