//! Runtime overrides for the per-agent submit chords. The DEFAULT chord
//! bytes live in chan-shell (`SubmitAgent::default_template`); this module
//! lets a user override them without a rebuild by editing
//! `<config>/chan/submit.toml`, so a client (claude/codex/gemini) changing
//! its submit behavior is a config edit, not a release. Env
//! `CHAN_SUBMIT_<AGENT>` still takes precedence over the file (resolved in
//! chan-shell at chord-application time).
//!
//! File shape (every table + field optional; omit one to keep its built-in):
//!
//! ```text
//! [claude]
//! template = "{}\e[27;9;13~"      # {} is the text; \e \xHH \r \n \t escapes
//! [codex]
//! template = "\e[200~{}\e[201~\r"
//! [gemini]
//! template = "{}\r"
//! ```
//!
//! A template without `{}` is treated as a pure suffix appended after the
//! text (so a bare-chord override works too).

use std::collections::HashMap;

use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
struct SubmitOverridesFile {
    claude: Option<AgentChord>,
    codex: Option<AgentChord>,
    gemini: Option<AgentChord>,
}

#[derive(Debug, Deserialize)]
struct AgentChord {
    template: String,
}

/// Load `<config>/chan/submit.toml` (if present) and install any per-agent
/// chord template overrides into chan-shell. Missing file: no-op (the
/// built-in defaults stand). Malformed file: logged and ignored, matching
/// the fall-back-on-malformed policy of the editor/server configs.
pub fn install() {
    let path = chan_workspace::paths::config_dir().join("submit.toml");
    let file: SubmitOverridesFile = match crate::store::load_toml(&path) {
        Ok(f) => f,
        Err(e) => {
            tracing::warn!("malformed submit.toml, ignoring chord overrides: {e}");
            return;
        }
    };
    let mut map = HashMap::new();
    if let Some(c) = file.claude {
        map.insert("claude".to_string(), c.template);
    }
    if let Some(c) = file.codex {
        map.insert("codex".to_string(), c.template);
    }
    if let Some(c) = file.gemini {
        map.insert("gemini".to_string(), c.template);
    }
    if !map.is_empty() {
        chan_shell::set_chord_overrides(map);
    }
}
