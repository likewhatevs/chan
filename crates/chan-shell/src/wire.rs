//! The control-socket wire contract shared by the `cs` client (which
//! serializes a [`ControlRequest`] and deserializes a [`ControlResponse`])
//! and chan-server's control socket (which deserializes the request and
//! serializes the response). Defining the two enums once here is what
//! kills the historical client/server duplication: a tag or field rename
//! that only landed on one side used to break every `cs` command at
//! runtime with a green build (the serde tags are the wire format).
//!
//! These types carry no transport and no clap surface, so they are always
//! compiled (no `client` feature gate) and chan-server can depend on
//! chan-shell with `default-features = false` to pull just this module.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// A command from a `cs`-spawned terminal to the chan-server it belongs
/// to. The internal `type` tag plus `snake_case` variant names are the
/// wire strings the server matches on; do not rename without changing
/// both sides (they are the same type now, so a rename moves in lockstep).
///
/// Every `Option` field carries `default` (so the server tolerates an
/// omitted key) AND `skip_serializing_if` (so the client omits `None`):
/// both attributes on one field keep the emitted JSON byte-identical to
/// the pre-unification client while staying loss-tolerant on decode.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ControlRequest {
    // Category 1: open a UI tab in the originating window. The server
    // pushes a window_command keyed by window_id; only that window acts.
    OpenPath {
        window_id: String,
        path: PathBuf,
    },
    OpenGraph {
        window_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        path: Option<PathBuf>,
    },
    OpenTermNew {
        window_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        path: Option<PathBuf>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tab_name: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tab_group: Option<String>,
    },
    OpenDashboard {
        window_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        carousel_index: Option<u32>,
        // Always emitted by the client (no skip) so the wire shape matches
        // the pre-unification request byte-for-byte; `default` lets a
        // future caller omit it without a decode error.
        #[serde(default)]
        carousel_off: bool,
    },
    // Category 2: act on / inspect live PTY sessions the server owns. No
    // window_id; the server resolves sessions through its registry.
    TermWrite {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tab_name: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tab_group: Option<String>,
        data: String,
    },
    TermList,
    TermRestart {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tab_name: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tab_group: Option<String>,
    },
    // Category 2: dump one live session's replay ring (its scrollback) by
    // tab name and return the decoded bytes on the connection (like `term
    // list`). No group axis: scrollback reads exactly ONE terminal's
    // history, so `tab_name` is required and the server rejects a zero- or
    // multi-match. The CLI prints the raw bytes to stdout.
    TermScrollback {
        tab_name: String,
    },
    // Category 2: run the same content search the UI does and return the
    // results on the connection (like `term list`). The CLI formats the
    // JSON it gets back: markdown by default, compact `--json`, indented
    // `--json --pretty`.
    Search {
        query: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        limit: Option<u32>,
    },
    // Category 3 (blocking round-trip): query the originating SPA window's
    // tab/pane LAYOUT. The layout lives only in the frontend, so the server
    // pushes a `pane` window_command keyed by `window_id`, parks a oneshot
    // (the window bus), and BLOCKS until the SPA replies with the layout
    // snapshot via `POST /api/window/reply`. The CLI prints it (markdown by
    // default, `--json` for machine output). `window_id` is the caller's own
    // window ($CHAN_WINDOW_ID), like the `open_*` commands.
    PaneQuery {
        window_id: String,
    },
    // Category 2 (blocking): raise a survey overlay on the SPA window(s)
    // that own the matching terminal tab(s) and BLOCK until the user
    // answers. The server resolves the selector to those windows, mints
    // `spec.survey_id`, pushes the overlay, parks a oneshot keyed by that
    // id, and holds this connection open until the SPA's reply route
    // completes it. The CLI prints the chosen option (or the new followup
    // path) to stdout. Unlike `TermWrite`, the reply round-trip is the
    // whole point, so this is the one control request that does not return
    // immediately.
    TermSurvey {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tab_name: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tab_group: Option<String>,
        spec: SurveySpec,
    },
    // Category 2: create or load a Team Work team from the CLI (`cs
    // terminal team new|load`). `new` carries the team's config.toml text
    // to validate + write (config.toml + the server-regenerated
    // bootstrap.md + the tasks/journals/followups tree); `load` reads an
    // existing `{dir}/config.toml`. With `script`, the server returns the
    // paste-and-run bootstrap shell script instead of mutating anything.
    // The config travels as raw TOML text (not a typed TeamConfig) so this
    // wire module keeps its chan-workspace-free, serde-only footprint; the
    // server owns the parse / validate / generate.
    TerminalTeam {
        /// Workspace-relative team directory (the team lives at
        /// `{dir}/config.toml`).
        dir: String,
        /// `new` (write/generate from `config_toml`) or `load` (read
        /// `{dir}/config.toml`).
        op: TeamOp,
        /// The team config.toml text for `new`; absent for `load`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        config_toml: Option<String>,
        /// Emit the paste-and-run bootstrap script instead of writing
        /// (`new`) / summarizing (`load`).
        #[serde(default)]
        script: bool,
    },
}

/// Which `cs terminal team` operation a [`ControlRequest::TerminalTeam`]
/// carries. A bare snake_case string on the wire, matching the CLI
/// subcommand names. The explicit `rename_all` pins the wire strings so a
/// Rust rename cannot silently drift the format the server matches on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TeamOp {
    /// `cs terminal team new`: validate + write the config (and the dir
    /// tree), or with `--script` emit the bootstrap script.
    New,
    /// `cs terminal team load`: read + validate `{dir}/config.toml`, or
    /// with `--script` emit the bootstrap script for the stored config.
    Load,
}

/// A survey raised over terminal tab(s) by `cs terminal survey`. Carried in
/// [`ControlRequest::TermSurvey`] from the CLI, then pushed to the SPA in an
/// `open_survey` window command. The CLI builds it with an EMPTY `survey_id`;
/// the server mints the id before the SPA sees it, and the SPA echoes that id
/// back in its [`SurveyReply`] so the server matches the parked oneshot.
///
/// serde camelCase: this is the exact JSON the SPA reads
/// (`round-3-survey-contract.md` pins it; C's TypeScript mirrors that doc).
/// Nullable fields (`title`, `followup`) serialize as `null` rather than
/// being skipped, so the SPA-facing frame matches the contract's
/// `string | null` / `{...} | null` shape exactly.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SurveySpec {
    /// Server-minted. Empty on the CLI -> server request; filled in before
    /// the SPA sees it. The SPA echoes it in the reply.
    #[serde(default)]
    pub survey_id: String,
    /// Optional heading rendered above the body.
    #[serde(default)]
    pub title: Option<String>,
    /// The problem description, rendered as markdown.
    pub body_markdown: String,
    /// 1..=4 option labels; the SPA numbers them [1]..[4].
    pub options: Vec<String>,
    /// Render the [F] follow-up affordance.
    pub allow_followup: bool,
    /// Team context for the `[F]` path, so C's reply route can land the
    /// followup at `{dir}/followups/followup-{from}-{to}-{n}.md` without
    /// re-deriving the team-dir (a workspace may hold several teams). The
    /// CLI populates it ONLY when `--followup` is set; `null` otherwise
    /// (2026-06-01 contract amendment).
    #[serde(default)]
    pub followup: Option<SurveyFollowup>,
}

/// The team context a `[F]` follow-up needs, carried on [`SurveySpec`] from
/// the surveying agent (who read `bootstrap.md` and knows its own tab name)
/// through to C's reply route. serde camelCase to match the contract.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SurveyFollowup {
    /// The team directory (workspace-relative) under which
    /// `followups/followup-{from}-{to}-{n}.md` is created.
    pub dir: String,
    /// The surveying agent (the followup's author): `$CHAN_TAB_NAME`.
    pub from: String,
    /// The survey target (the tab name, or the group name for a group
    /// survey).
    pub to: String,
}

/// The reply the SPA sends back through the reply route to the blocked CLI.
/// Internally tagged on `kind` (`"option"` / `"followup"`), serde camelCase.
/// The explicit `tag` + variant renames pin the wire strings so a Rust
/// rename cannot silently drift the format the SPA POSTs.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SurveyReply {
    /// The user picked one of the numbered options.
    #[serde(rename = "option", rename_all = "camelCase")]
    Option {
        survey_id: String,
        option_index: u32,
        option_label: String,
    },
    /// The user hit [F]: C created the followup file and replies its path
    /// (workspace-relative).
    #[serde(rename = "followup", rename_all = "camelCase")]
    Followup {
        survey_id: String,
        followup_path: String,
    },
}

impl SurveyReply {
    /// The `survey_id` this reply echoes, used to match the parked oneshot
    /// regardless of which variant it is.
    pub fn survey_id(&self) -> &str {
        match self {
            SurveyReply::Option { survey_id, .. } => survey_id,
            SurveyReply::Followup { survey_id, .. } => survey_id,
        }
    }
}

#[cfg(test)]
mod survey_wire_tests {
    //! These pin the EXACT on-wire JSON of the survey types. The serde
    //! tags + camelCase are the C<->D contract (round-3-survey-contract.md);
    //! a Rust rename that drifts them breaks the SPA / reply route at
    //! runtime with a green build, so assert the bytes, not just round-trip.
    use super::*;

    #[test]
    fn survey_spec_is_camel_case_with_explicit_nulls() {
        let spec = SurveySpec {
            survey_id: "survey-3".into(),
            title: None,
            body_markdown: "pick one".into(),
            options: vec!["A".into(), "B".into()],
            allow_followup: true,
            followup: Some(SurveyFollowup {
                dir: "team".into(),
                from: "@@LaneD".into(),
                to: "@@LaneC".into(),
            }),
        };
        let v: serde_json::Value = serde_json::to_value(&spec).unwrap();
        assert_eq!(v["surveyId"], "survey-3");
        // title is null (not omitted), matching the contract's `string|null`.
        assert!(v.get("title").is_some_and(|t| t.is_null()));
        assert_eq!(v["bodyMarkdown"], "pick one");
        assert_eq!(v["options"], serde_json::json!(["A", "B"]));
        assert_eq!(v["allowFollowup"], true);
        assert_eq!(v["followup"]["dir"], "team");
        assert_eq!(v["followup"]["from"], "@@LaneD");
        assert_eq!(v["followup"]["to"], "@@LaneC");
    }

    #[test]
    fn survey_spec_emits_null_followup_when_absent() {
        let spec = SurveySpec {
            survey_id: String::new(),
            title: Some("Heads up".into()),
            body_markdown: "x".into(),
            options: vec!["ok".into()],
            allow_followup: false,
            followup: None,
        };
        let v: serde_json::Value = serde_json::to_value(&spec).unwrap();
        assert_eq!(v["title"], "Heads up");
        assert!(v.get("followup").is_some_and(|f| f.is_null()));
    }

    #[test]
    fn survey_reply_option_tag_and_fields() {
        let reply = SurveyReply::Option {
            survey_id: "survey-1".into(),
            option_index: 2,
            option_label: "Ship it".into(),
        };
        let v: serde_json::Value = serde_json::to_value(&reply).unwrap();
        assert_eq!(v["kind"], "option");
        assert_eq!(v["surveyId"], "survey-1");
        assert_eq!(v["optionIndex"], 2);
        assert_eq!(v["optionLabel"], "Ship it");
        // The SPA POSTs exactly this; round-trips back to the same variant.
        let back: SurveyReply = serde_json::from_value(v).unwrap();
        assert_eq!(back.survey_id(), "survey-1");
        assert!(matches!(
            back,
            SurveyReply::Option {
                option_index: 2,
                ..
            }
        ));
    }

    #[test]
    fn survey_reply_followup_tag_and_fields() {
        let reply = SurveyReply::Followup {
            survey_id: "survey-9".into(),
            followup_path: "team/followups/followup-a-b-1.md".into(),
        };
        let v: serde_json::Value = serde_json::to_value(&reply).unwrap();
        assert_eq!(v["kind"], "followup");
        assert_eq!(v["surveyId"], "survey-9");
        assert_eq!(v["followupPath"], "team/followups/followup-a-b-1.md");
    }

    #[test]
    fn term_survey_request_tag_and_spec_round_trip() {
        let req = ControlRequest::TermSurvey {
            tab_name: Some("@@LaneC".into()),
            tab_group: None,
            spec: SurveySpec {
                survey_id: String::new(),
                title: None,
                body_markdown: "q".into(),
                options: vec!["yes".into()],
                allow_followup: false,
                followup: None,
            },
        };
        let v: serde_json::Value = serde_json::to_value(&req).unwrap();
        assert_eq!(v["type"], "term_survey");
        assert_eq!(v["tab_name"], "@@LaneC");
        // tab_group None is skipped on the wire (matches the sibling variants).
        assert!(v.get("tab_group").is_none());
        assert_eq!(v["spec"]["bodyMarkdown"], "q");
        // Decodes back into the same variant (the server's path).
        let raw = serde_json::to_string(&req).unwrap();
        let back: ControlRequest = serde_json::from_str(&raw).unwrap();
        assert!(matches!(back, ControlRequest::TermSurvey { .. }));
    }

    #[test]
    fn term_scrollback_request_tag_and_field() {
        // The wire tag is `term_scrollback` and `tab_name` is a plain
        // required string (no group axis, no skip). A Rust rename that
        // drifts either breaks the server's decode with a green build.
        let req = ControlRequest::TermScrollback {
            tab_name: "@@LaneB".into(),
        };
        let v: serde_json::Value = serde_json::to_value(&req).unwrap();
        assert_eq!(v["type"], "term_scrollback");
        assert_eq!(v["tab_name"], "@@LaneB");
        // Decodes back into the same variant (the server's path).
        let raw = serde_json::to_string(&req).unwrap();
        let back: ControlRequest = serde_json::from_str(&raw).unwrap();
        assert!(matches!(back, ControlRequest::TermScrollback { .. }));
    }

    #[test]
    fn pane_query_request_tag_and_field() {
        // Wire tag `pane_query`, a required `window_id`. A Rust rename that
        // drifts either breaks the server's decode with a green build.
        let req = ControlRequest::PaneQuery {
            window_id: "window-a".into(),
        };
        let v: serde_json::Value = serde_json::to_value(&req).unwrap();
        assert_eq!(v["type"], "pane_query");
        assert_eq!(v["window_id"], "window-a");
        let raw = serde_json::to_string(&req).unwrap();
        let back: ControlRequest = serde_json::from_str(&raw).unwrap();
        assert!(matches!(back, ControlRequest::PaneQuery { .. }));
    }

    #[test]
    fn terminal_team_request_tag_and_op_strings() {
        // `new` carries the config TOML; the wire tag is `terminal_team`
        // and the op is the snake_case subcommand name.
        let req = ControlRequest::TerminalTeam {
            dir: "new-team-1".into(),
            op: TeamOp::New,
            config_toml: Some("team_name = \"alpha\"\n".into()),
            script: true,
        };
        let v: serde_json::Value = serde_json::to_value(&req).unwrap();
        assert_eq!(v["type"], "terminal_team");
        assert_eq!(v["dir"], "new-team-1");
        assert_eq!(v["op"], "new");
        assert_eq!(v["config_toml"], "team_name = \"alpha\"\n");
        assert_eq!(v["script"], true);

        // `load` omits config_toml (skip_serializing_if) and defaults
        // script to false.
        let load = ControlRequest::TerminalTeam {
            dir: "teams/alpha".into(),
            op: TeamOp::Load,
            config_toml: None,
            script: false,
        };
        let v: serde_json::Value = serde_json::to_value(&load).unwrap();
        assert_eq!(v["op"], "load");
        assert!(
            v.get("config_toml").is_none(),
            "None config_toml is skipped"
        );
        assert_eq!(v["script"], false);

        // Round-trips back into the same variant (the server's decode path).
        let raw = serde_json::to_string(&load).unwrap();
        let back: ControlRequest = serde_json::from_str(&raw).unwrap();
        assert!(matches!(
            back,
            ControlRequest::TerminalTeam {
                op: TeamOp::Load,
                ..
            }
        ));
    }
}

/// The single-line reply the server writes back on the control socket.
/// The internal `status` tag is the wire format; the client matches on it.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ControlResponse {
    Ok { message: String },
    Error { message: String },
}
