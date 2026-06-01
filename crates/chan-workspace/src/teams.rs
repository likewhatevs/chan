//! Team config schema. `TeamConfig` (members, host, prefix
//! policy) is persisted to a `config.toml` and consumed by the
//! path-based Team Work flow via chan-server's `/api/team-config`
//! route. The older name-based team registry (team-{name}/ dirs
//! under Drafts) was retired; only the config structs remain.

use serde::{Deserialize, Serialize};

/// Per-team config persisted to `config.toml`. Schema per the
/// addendum-b spec.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TeamConfig {
    pub team_name: String,
    pub host_name: String,
    pub host_handle: String,
    // Terminal tab group the team's tabs are grouped under. The SPA's
    // `TeamConfigWire` always carries this; `#[serde(default)]` keeps a
    // hand-edited config.toml that predates the field from 400-ing on
    // read (an empty group is harmless), rather than forcing a manual
    // edit on every pre-existing config.
    #[serde(default)]
    pub tab_group: String,
    #[serde(default = "default_auto_prefix_at")]
    pub auto_prefix_at: bool,
    // ISO 8601 UTC creation time. The SPA dialog always sends it; the CLI
    // `cs terminal team new` lets the input config.toml omit it (a
    // hand-written team spec should not carry a timestamp), and the server
    // stamps the current time on write. `#[serde(default)]` (empty string)
    // is the "not yet stamped" sentinel the server fills in.
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub members: Vec<Member>,
}

fn default_auto_prefix_at() -> bool {
    true
}

/// One member entry inside `TeamConfig.members`. `position` is
/// the airplane-style grid coordinate; `None` selects
/// tabs-in-current-Hybrid layout. `env` carries per-tab
/// environment overrides (e.g. `CHAN_TAB_NAME`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Member {
    pub handle: String,
    pub command: String,
    #[serde(default)]
    pub env: std::collections::BTreeMap<String, String>,
    #[serde(default)]
    pub is_lead: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<Position>,
    // Submit-encoding agent type for this member's terminal. Feeds the
    // generated bootstrap.md poke chord (claude reads ESC[27;9;13~,
    // codex/gemini read CR) and the lead composer's submit mode, both via
    // the shared submit map. `None` is a shell member with no submit chord;
    // valid Some values are "claude" / "codex" / "gemini". Stored as a plain
    // String to avoid a chan-workspace -> chan-shell layering dependency on
    // SubmitAgent; chan-server validates the value on read/write.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
}

/// Airplane-style grid coordinate. Row + column are zero-based.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct Position {
    pub row: u32,
    pub col: u32,
}
