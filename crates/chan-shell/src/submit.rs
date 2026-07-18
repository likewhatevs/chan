//! The per-agent submit-encoding map plus the command -> agent derivation.
//! A coding agent running inside a chan terminal submits its compose buffer
//! on a different byte sequence depending on which agent it is, so a
//! hands-free completion poke (`cs terminal write --submit=<agent>`) has to
//! append the right one.
//!
//! Two things live here, both shared by the `cs` CLI and chan-server:
//!
//!   - `SubmitAgent::derive`: map a spawn command (+ an optional `CHAN_AGENT`
//!     env override) to the agent whose submit encoding it uses. This is the
//!     single source of truth for "which agent is this", mirrored in
//!     `web/packages/workspace-app/src/state/teamDialog.svelte.ts` (`agentForMember`).
//!   - `apply_submit_chord`: turn a poke into the PTY bytes that submit it.
//!     Each agent has a `{}`-templated chord whose DEFAULT reproduces the
//!     live-probed bytes, but which is overridable at runtime (env var or
//!     `<config>/chan/submit.toml`) so a client changing its submit behavior
//!     does not need a rebuild. See `apply_submit_chord` / `set_chord_overrides`.
//!
//! Rich Prompt sends the agent NAME and the server applies the chord. The SPA's
//! `submitMode.ts` also pins the byte map for browser-side parity tests, so its
//! agent union, detection, and default encodings must stay in sync.

use std::collections::HashMap;
use std::sync::RwLock;

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
    /// suffix never submits. The default template wraps codex's text in
    /// bracketed paste so the trailing CR lands as a real Enter. It ignores
    /// both the Claude chord and the kitty CSI-u Enter (`\x1b[13u`) silently.
    /// Live-probed 2026-06-02 against codex-cli 0.136.0.
    Codex,
    /// Google gemini. Submits on a CR, but ONLY when the CR arrives as a
    /// DISTINCT write: gemini 0.51 converts Return received within 30 ms of
    /// inserted text into Shift+Return, including text delivered as bracketed
    /// paste. So gemini's chord is delivered as a SEPARATE write from the text
    /// - see `submit_writes`.
    Gemini,
    /// OpenCode. Its TUI accepts bracketed paste followed by CR in the same
    /// PTY write. The bracketed form is the default because it is proven for
    /// multiline and paste-sized input. Live-probed 2026-07-18 against
    /// OpenCode 1.18.3.
    OpenCode,
}

impl SubmitAgent {
    /// Resolve an agent NAME ("claude" | "codex" | "gemini" | "opencode")
    /// to its variant
    /// without clap's `ValueEnum::from_str` (so a caller that only has the
    /// string does not have to pull clap in). Returns `None` for an unknown
    /// name.
    pub fn from_agent_name(name: &str) -> Option<Self> {
        match name {
            "claude" => Some(SubmitAgent::Claude),
            "codex" => Some(SubmitAgent::Codex),
            "gemini" => Some(SubmitAgent::Gemini),
            "opencode" => Some(SubmitAgent::OpenCode),
            _ => None,
        }
    }

    /// The agent's lower-case name, the inverse of `from_agent_name`.
    pub fn name(self) -> &'static str {
        match self {
            SubmitAgent::Claude => "claude",
            SubmitAgent::Codex => "codex",
            SubmitAgent::Gemini => "gemini",
            SubmitAgent::OpenCode => "opencode",
        }
    }

    /// Derive the submit agent from a spawn command and an optional
    /// `CHAN_AGENT` override value. The single source of truth for the
    /// command -> agent mapping, mirrored in `agentForMember`
    /// (teamDialog.svelte.ts).
    ///
    /// `CHAN_AGENT` wins when it names a known agent ("claude"/"codex"/
    /// "gemini"/"opencode") or an explicit shell ("none"/"shell" ->
    /// `None`); an unrecognized value falls through to the command sniff (the escape
    /// hatch is opt-in, a typo should not silently disable submit). The
    /// command match is a LOOSE whole-word sniff: claude/codex/gemini/opencode
    /// recognized anywhere in the command as a word, so wrappers like
    /// `my-claude.sh`, `/usr/local/bin/codex-cli`, or `claude --resume` still
    /// resolve, while `claudette` does not. `None` means a shell member with
    /// no submit chord.
    pub fn derive(command: &str, chan_agent: Option<&str>) -> Option<SubmitAgent> {
        if let Some(raw) = chan_agent {
            match raw.trim().to_ascii_lowercase().as_str() {
                "claude" => return Some(SubmitAgent::Claude),
                "codex" => return Some(SubmitAgent::Codex),
                "gemini" => return Some(SubmitAgent::Gemini),
                "opencode" => return Some(SubmitAgent::OpenCode),
                "none" | "shell" => return None,
                // Unrecognized CHAN_AGENT: ignore it, sniff the command.
                _ => {}
            }
        }
        let c = command.to_ascii_lowercase();
        if word_match(&c, "claude") {
            Some(SubmitAgent::Claude)
        } else if word_match(&c, "codex") {
            Some(SubmitAgent::Codex)
        } else if word_match(&c, "gemini") {
            Some(SubmitAgent::Gemini)
        } else if word_match(&c, "opencode") {
            Some(SubmitAgent::OpenCode)
        } else {
            None
        }
    }

    /// The built-in submit template for this agent: a string with a single
    /// `{}` placeholder for the (trailing-newline-trimmed) text. These ARE
    /// the live-probed default bytes; an override (env / config file) replaces
    /// the whole template. claude appends the modifyOtherKeys Cmd+Enter CSI;
    /// gemini a bare CR; codex and opencode wrap the text in bracketed paste
    /// then CR. Codex needs the wrap to keep its paste-burst coalescing from
    /// eating the submit; opencode uses the same bytes as its multiline-safe
    /// default.
    fn default_template(self) -> &'static str {
        match self {
            SubmitAgent::Claude => "{}\x1b[27;9;13~",
            SubmitAgent::Codex => "\x1b[200~{}\x1b[201~\r",
            SubmitAgent::Gemini => "{}\r",
            SubmitAgent::OpenCode => "\x1b[200~{}\x1b[201~\r",
        }
    }

    /// Resolve this agent's submit template, applying overrides in priority
    /// order: env `CHAN_SUBMIT_<AGENT>` > the process-global override map
    /// (loaded from `<config>/chan/submit.toml` by the server) > the built-in
    /// default. Override strings are unescaped (`\e`, `\xHH`, `\r`, `\n`,
    /// `\t`, `\\`) so a config/env value can carry control bytes as text.
    fn template(self) -> String {
        let guard = CHORD_OVERRIDES.read().expect("CHORD_OVERRIDES poisoned");
        resolve_template(self, |k| std::env::var(k).ok(), guard.as_ref())
    }
}

/// Process-global per-agent chord template overrides, keyed by agent name
/// ("claude"/"codex"/"gemini"/"opencode"). The server loads these from
/// `<config>/chan/submit.toml` once at startup via `set_chord_overrides`;
/// env `CHAN_SUBMIT_<AGENT>` still takes precedence at apply time. Default
/// `None` means "no file overrides", which every chan-shell-only caller
/// (the `cs` CLI) sees, so it falls back to env + built-in.
static CHORD_OVERRIDES: RwLock<Option<HashMap<String, String>>> = RwLock::new(None);

/// Install the config-file chord template overrides (agent name -> template
/// string, escapes intact). Called once by the server at startup; a later
/// call replaces the map. Env vars still win over these at apply time.
pub fn set_chord_overrides(overrides: HashMap<String, String>) {
    *CHORD_OVERRIDES.write().expect("CHORD_OVERRIDES poisoned") = Some(overrides);
}

/// Pure resolution of an agent's template from an env lookup + an optional
/// override map, so the precedence logic is testable without touching the
/// process env or the global. env `CHAN_SUBMIT_<AGENT>` > override map >
/// built-in default; an empty/whitespace override value is ignored (falls
/// through), so a blank env var does not blank the chord.
fn resolve_template(
    agent: SubmitAgent,
    env: impl Fn(&str) -> Option<String>,
    overrides: Option<&HashMap<String, String>>,
) -> String {
    let name = agent.name();
    if let Some(v) = env(&format!("CHAN_SUBMIT_{}", name.to_ascii_uppercase())) {
        if !v.trim().is_empty() {
            return unescape(&v);
        }
    }
    if let Some(v) = overrides.and_then(|m| m.get(name)) {
        if !v.trim().is_empty() {
            return unescape(v);
        }
    }
    agent.default_template().to_string()
}

/// `cs terminal write --submit=<agent>`: encode `data` into the PTY bytes
/// that make a running agent submit it hands-free (the completion poke).
/// `None` writes the bytes verbatim. Trailing newlines are stripped first: a
/// newline before the submit would land inside the agent's draft, splitting
/// the buffer before submit fires.
///
/// The agent's resolved template (default or overridden) drives the bytes. A
/// template with a `{}` placeholder substitutes the text there (the codex and
/// opencode bracketed-paste wraps are expressed this way); a template WITHOUT
/// `{}` is treated as a pure suffix appended after the text, so a bare-chord override
/// like `CHAN_SUBMIT_GEMINI=$'\r'` still works.
///
/// Defaults mirror the live-probed bytes; the agent-name half is mirrored by
/// `submitMode.ts` (the SPA sends the name, the server applies the chord).
pub fn apply_submit_chord(data: String, submit: Option<SubmitAgent>) -> String {
    let Some(agent) = submit else {
        return data;
    };
    let text = data.trim_end_matches('\n');
    let template = agent.template();
    if template.contains("{}") {
        template.replacen("{}", text, 1)
    } else {
        format!("{text}{template}")
    }
}

/// The ordered PTY writes that deliver `data` to an agent and submit it. Most
/// agents need ONE write (the chord is part of it, via `apply_submit_chord`),
/// so a caller can write/enqueue the single element verbatim.
///
/// gemini is the exception. gemini 0.51 converts Return received within 30 ms
/// of inserted text into Shift+Return, including bracketed paste. Only a CR
/// delivered as its OWN later write submits gemini, so for gemini this
/// returns TWO writes (the text body, then the submit chord alone) which the
/// caller MUST deliver as separate events: separate write-queue items, whose
/// drainer idle-gates between them, or separate PTY writes with a gap. Empty
/// parts are dropped.
pub fn submit_writes(data: String, submit: Option<SubmitAgent>) -> Vec<String> {
    if submit != Some(SubmitAgent::Gemini) {
        return vec![apply_submit_chord(data, submit)];
    }
    let body = data.trim_end_matches('\n').to_string();
    // The submit chord alone is gemini's resolved template with the text
    // placeholder removed (default `\r`, or an override's trailing bytes).
    let template = SubmitAgent::Gemini.template();
    let chord = if template.contains("{}") {
        template.replacen("{}", "", 1)
    } else {
        template
    };
    let writes: Vec<String> = [body, chord]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect();
    if writes.is_empty() {
        vec![String::new()]
    } else {
        writes
    }
}

/// Whether `word` (ASCII) occurs in `haystack` bounded by non-word chars on
/// both sides, the `\b<word>\b` of `agentForCommand`. Word chars are ASCII
/// alphanumerics + `_`; anything else (including non-ASCII bytes and string
/// edges) is a boundary. `haystack` is expected pre-lowercased.
fn word_match(haystack: &str, word: &str) -> bool {
    let bytes = haystack.as_bytes();
    let mut from = 0;
    while let Some(pos) = haystack[from..].find(word) {
        let start = from + pos;
        let end = start + word.len();
        let before_ok = start == 0 || !is_word_byte(bytes[start - 1]);
        let after_ok = end == bytes.len() || !is_word_byte(bytes[end]);
        if before_ok && after_ok {
            return true;
        }
        from = start + 1;
        if from >= bytes.len() {
            break;
        }
    }
    false
}

fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Decode the backslash escapes a config/env override string may carry so a
/// template can express control bytes as plain text: `\e` (ESC), `\xHH` (a
/// hex byte, intended for ASCII/control), `\r`, `\n`, `\t`, `\0`, `\\`. An
/// unrecognized escape keeps both the backslash and the following char, so a
/// literal `\d` survives rather than being silently dropped.
fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('e') => out.push('\x1b'),
            Some('r') => out.push('\r'),
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('0') => out.push('\0'),
            Some('\\') => out.push('\\'),
            Some('x') => {
                let h1 = chars.next();
                let h2 = chars.next();
                match (h1, h2) {
                    (Some(a), Some(b)) => {
                        let hex: String = [a, b].iter().collect();
                        if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                            out.push(byte as char);
                        } else {
                            out.push('\\');
                            out.push('x');
                            out.push(a);
                            out.push(b);
                        }
                    }
                    (Some(a), None) => {
                        out.push('\\');
                        out.push('x');
                        out.push(a);
                    }
                    _ => {
                        out.push('\\');
                        out.push('x');
                    }
                }
            }
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    out
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
        // opencode -> bracketed paste and CR in the same PTY write.
        assert_eq!(
            apply_submit_chord("poke\n".into(), Some(SubmitAgent::OpenCode)),
            "\x1b[200~poke\x1b[201~\r"
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

    #[test]
    fn submit_writes_is_one_write_except_gemini() {
        // claude/codex/opencode/none: one write, identical to apply_submit_chord.
        assert_eq!(
            submit_writes("poke\n".into(), Some(SubmitAgent::Claude)),
            vec!["poke\x1b[27;9;13~".to_string()]
        );
        assert_eq!(
            submit_writes("poke".into(), Some(SubmitAgent::Codex)),
            vec!["\x1b[200~poke\x1b[201~\r".to_string()]
        );
        assert_eq!(
            submit_writes("poke".into(), Some(SubmitAgent::OpenCode)),
            vec!["\x1b[200~poke\x1b[201~\r".to_string()]
        );
        assert_eq!(submit_writes("raw".into(), None), vec!["raw".to_string()]);
        // gemini: TWO writes - the text body, then the bare submit chord -
        // so Return is not converted to Shift+Return with the insertion.
        assert_eq!(
            submit_writes("poke\n".into(), Some(SubmitAgent::Gemini)),
            vec!["poke".to_string(), "\r".to_string()]
        );
        // A text-only gemini body still splits off the chord write.
        assert_eq!(
            submit_writes("hi there".into(), Some(SubmitAgent::Gemini)),
            vec!["hi there".to_string(), "\r".to_string()]
        );
    }

    #[test]
    fn derive_sniffs_the_command_loosely() {
        let d = |c: &str| SubmitAgent::derive(c, None);
        assert_eq!(d("claude"), Some(SubmitAgent::Claude));
        assert_eq!(d("codex"), Some(SubmitAgent::Codex));
        assert_eq!(d("gemini"), Some(SubmitAgent::Gemini));
        assert_eq!(d("opencode"), Some(SubmitAgent::OpenCode));
        // past the first token / through a path / a wrapper
        assert_eq!(d("claude --resume"), Some(SubmitAgent::Claude));
        assert_eq!(d("/usr/local/bin/codex-cli"), Some(SubmitAgent::Codex));
        assert_eq!(d("my-claude.sh --flag"), Some(SubmitAgent::Claude));
        assert_eq!(d("env FOO=1 gemini chat"), Some(SubmitAgent::Gemini));
        assert_eq!(d("/usr/local/bin/opencode-ai"), Some(SubmitAgent::OpenCode));
        assert_eq!(d("OPENCODE"), Some(SubmitAgent::OpenCode));
        assert_eq!(d("CLAUDE"), Some(SubmitAgent::Claude)); // case-insensitive
                                                            // word boundaries keep near-misses out
        assert_eq!(d("claudette"), None);
        assert_eq!(d("codexterous"), None);
        assert_eq!(d("myopencode"), None);
        assert_eq!(d("opencoded"), None);
        // a plain shell -> no chord
        assert_eq!(d("bash"), None);
        assert_eq!(d(""), None);
    }

    #[test]
    fn derive_honors_chan_agent_override() {
        // CHAN_AGENT wins over the command sniff
        assert_eq!(
            SubmitAgent::derive("codex", Some("claude")),
            Some(SubmitAgent::Claude)
        );
        assert_eq!(
            SubmitAgent::derive("./run-my-agent.sh", Some("gemini")),
            Some(SubmitAgent::Gemini)
        );
        // explicit shell forces None despite an agent command
        assert_eq!(SubmitAgent::derive("claude", Some("none")), None);
        assert_eq!(SubmitAgent::derive("claude", Some("shell")), None);
        // an unrecognized value falls through to the command sniff
        assert_eq!(
            SubmitAgent::derive("claude", Some("banana")),
            Some(SubmitAgent::Claude)
        );
        assert_eq!(SubmitAgent::derive("bash", Some("banana")), None);
        // whitespace / case tolerated
        assert_eq!(
            SubmitAgent::derive("bash", Some("  Codex ")),
            Some(SubmitAgent::Codex)
        );
        assert_eq!(
            SubmitAgent::derive("claude", Some(" OpenCode ")),
            Some(SubmitAgent::OpenCode)
        );
    }

    #[test]
    fn resolve_template_precedence_env_over_file_over_default() {
        let none = |_: &str| None;
        // default when nothing is set
        assert_eq!(
            resolve_template(SubmitAgent::Claude, none, None),
            "{}\x1b[27;9;13~"
        );
        // file override (the process-global map), escapes decoded
        let mut file = HashMap::new();
        file.insert("claude".to_string(), "{}\\r".to_string());
        assert_eq!(
            resolve_template(SubmitAgent::Claude, none, Some(&file)),
            "{}\r"
        );
        // env beats the file
        let env = |k: &str| (k == "CHAN_SUBMIT_CLAUDE").then(|| "{}\\x0d".to_string());
        assert_eq!(
            resolve_template(SubmitAgent::Claude, env, Some(&file)),
            "{}\r"
        );
        // a blank env value is ignored (falls through to the file)
        let blank = |k: &str| (k == "CHAN_SUBMIT_CLAUDE").then(|| "  ".to_string());
        assert_eq!(
            resolve_template(SubmitAgent::Claude, blank, Some(&file)),
            "{}\r"
        );
    }

    #[test]
    fn template_without_placeholder_is_a_suffix() {
        // A bare-chord override (no `{}`) appends after the text.
        let mut file = HashMap::new();
        file.insert("gemini".to_string(), "\\r".to_string());
        let tmpl = resolve_template(SubmitAgent::Gemini, |_| None, Some(&file));
        // mimic apply_submit_chord's suffix branch
        let out = if tmpl.contains("{}") {
            tmpl.replacen("{}", "poke", 1)
        } else {
            format!("poke{tmpl}")
        };
        assert_eq!(out, "poke\r");
    }

    #[test]
    fn opencode_template_precedence_env_over_file_over_default() {
        let none = |_: &str| None;
        assert_eq!(
            resolve_template(SubmitAgent::OpenCode, none, None),
            "\x1b[200~{}\x1b[201~\r"
        );
        let mut file = HashMap::new();
        file.insert("opencode".to_string(), "{}\\r".to_string());
        assert_eq!(
            resolve_template(SubmitAgent::OpenCode, none, Some(&file)),
            "{}\r"
        );
        let env = |key: &str| {
            (key == "CHAN_SUBMIT_OPENCODE").then(|| "\\e[200~{}\\e[201~\\r".to_string())
        };
        assert_eq!(
            resolve_template(SubmitAgent::OpenCode, env, Some(&file)),
            "\x1b[200~{}\x1b[201~\r"
        );
    }

    #[test]
    fn opencode_encoding_pins_multiline_trimming_and_paste_sized_bytes() {
        assert_eq!(
            apply_submit_chord("one\ntwo\n\n".into(), Some(SubmitAgent::OpenCode)),
            "\x1b[200~one\ntwo\x1b[201~\r"
        );
        let body = format!("HEAD{}TAIL", "x".repeat(20 * 1024));
        let encoded = apply_submit_chord(body.clone(), Some(SubmitAgent::OpenCode));
        assert_eq!(
            encoded,
            format!("\x1b[200~{body}\x1b[201~\r"),
            "paste-sized input must remain one exact bracketed PTY write"
        );
    }

    #[test]
    fn unescape_decodes_control_escapes() {
        assert_eq!(unescape("\\e[27;9;13~"), "\x1b[27;9;13~");
        assert_eq!(unescape("\\x1b[200~"), "\x1b[200~");
        assert_eq!(unescape("a\\rb\\nc\\t"), "a\rb\nc\t");
        assert_eq!(unescape("\\\\"), "\\");
        // unknown escape keeps both chars
        assert_eq!(unescape("\\d"), "\\d");
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
        assert_eq!(
            SubmitAgent::from_str("opencode", true).unwrap(),
            SubmitAgent::OpenCode
        );
        assert!(SubmitAgent::from_str("turbo", true).is_err());
    }

    #[test]
    fn from_agent_name_round_trips_with_name() {
        for a in [
            SubmitAgent::Claude,
            SubmitAgent::Codex,
            SubmitAgent::Gemini,
            SubmitAgent::OpenCode,
        ] {
            assert_eq!(SubmitAgent::from_agent_name(a.name()), Some(a));
        }
        assert_eq!(SubmitAgent::from_agent_name("turbo"), None);
    }
}
