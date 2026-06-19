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
    // Category 2: list the windows this tenant knows about — the same
    // `{id, connected, saved, title?, kind?}` rows as `GET /api/windows`
    // (saved session blobs ∪ live `/ws` presence, enriched with the
    // desktop-supplied OS title/kind), returned as JSON in `Ok.message`
    // for the CLI to format. Works on both workspace and terminal tenants.
    WindowList,
    // Identify the serving process behind this control socket: `chan ps`
    // round-trips it to classify each served workspace's holder as a
    // standalone `serve`, a chan-desktop, or a devserver. The reply is an
    // [`Identity`] JSON in `Ok.message`.
    Identify,
    // Category 4 (desktop window lifecycle): drive the desktop's OS
    // windows from the terminal. These reach the Tauri app through the
    // in-process bridge the embedded server installs; a standalone
    // `chan serve` has no desktop attached and refuses them. `new` is the
    // only one without an id: the server derives the kind from the calling
    // tenant (a terminal tenant spawns a terminal window; a workspace
    // tenant spawns another window of that workspace) and returns the new
    // window id. The id-bearing verbs act on ANY window by id (the single
    // desktop AppHandle is global), so an id need not belong to this tenant.
    WindowNew,
    // Focus a live window, or un-hide a buried one; best-effort reopens a
    // closed-but-saved workspace window when its workspace is still running.
    WindowOpen {
        id: String,
    },
    // `cs window rm`: truly DESTROY a window (unlike the OS close button,
    // which buries it) and delete its saved layout. When the window has
    // live terminal shells and `force` is unset, the desktop raises a
    // confirmation dialog and this request BLOCKS until the user answers;
    // `force` skips the prompt and kills the shells.
    WindowClose {
        id: String,
        #[serde(default)]
        force: bool,
    },
    // `cs window hide`: replicate the OS close button — bury (hide) the
    // window, keeping its terminals and layout warm and reopenable.
    WindowHide {
        id: String,
    },
    // Set a custom OS window title that survives reload/reopen; an empty
    // string resets to the default `{base} Window {N}`.
    WindowTitle {
        id: String,
        title: String,
    },
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
    // Category 3 (blocking round-trip): query a SPA window's tab/pane
    // LAYOUT. The layout lives only in the frontend, so the server resolves
    // the target window, pushes a `pane_query` window_command, parks a
    // oneshot (the window bus), and BLOCKS until the SPA replies with the
    // layout snapshot via `POST /api/window/reply`. The CLI prints it
    // (markdown by default, `--json` for machine output). The target is
    // EITHER `window_id` (the caller's own window, $CHAN_WINDOW_ID) OR
    // `tab_name` (`--tab-name`, which the server resolves to the single
    // window owning that tab via `window_ids_matching`) so the command works
    // from a context with no $CHAN_WINDOW_ID (an unbound agent, a native
    // terminal).
    PaneQuery {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        window_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tab_name: Option<String>,
    },
    // Category 3 (blocking round-trip): EXECUTE a layout mutation on a SPA
    // window (focus / split / resize / close) over the same window bus as
    // `PaneQuery`, with the same `window_id` / `tab_name` target resolution.
    // The server pushes a `pane_exec` window_command, BLOCKS until the SPA
    // applies it and replies the result, and the CLI prints it. A close that
    // hits a dirty file or a live terminal WITHOUT `force` is a partial
    // failure (reported in the result, non-zero exit); `force` closes anyway.
    PaneExec {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        window_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tab_name: Option<String>,
        op: PaneOp,
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
        /// The caller's window id ($CHAN_WINDOW_ID), when `cs terminal team`
        /// runs from a chan terminal that belongs to an SPA window. The
        /// server binds each spawned agent session to it, so the agents
        /// carry $CHAN_WINDOW_ID too and `cs pane` / `cs open` work from
        /// inside an agent (the window-targeting commands resolve a window).
        /// Absent when the caller has no window (e.g. a native terminal):
        /// the agents spawn unbound, as before.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        window_id: Option<String>,
    },
    // Category 5 (process teardown): tear down the server serving `path`,
    // the transport behind `chan unserve <path>`. `cmd_unserve` discovers the
    // serving process from the workspace's `writer.lock` and sends this over
    // that process's control socket; THE SERVER DECIDES SCOPE from `path`: a
    // standalone `chan serve` of that root fires its own graceful shutdown
    // (the process exits, releasing the flock), while a multi-tenant host (a
    // `chan devserver` / chan-desktop) unmounts just that tenant and keeps
    // running. The client carries no scope hint — the server knows its own
    // kind. `path` is the canonical workspace root.
    Unserve {
        path: PathBuf,
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

/// A `cs pane` exec operation carried in [`ControlRequest::PaneExec`], pushed
/// to the SPA nested under the `pane_exec` window command's `op` field.
/// Internally tagged on `kind` (snake_case), so each variant is
/// `{ "kind": "focus", ... }` and the SPA discriminates on `frame.op.kind`.
/// The explicit `rename_all` pins the wire strings the SPA matches on, so a
/// Rust rename cannot silently drift the format with a green build.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PaneOp {
    /// Make `pane_id` the active (focused) pane.
    Focus { pane_id: String },
    /// Split a pane (the active one when `pane_id` is absent), placing the
    /// new empty pane to the `left` or `bottom`.
    Split {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pane_id: Option<String>,
        dir: SplitDir,
    },
    /// Resize the enclosing split of a pane (the active one when absent) by
    /// `delta` (a ratio step in -1.0..1.0; positive grows the pane). The SPA
    /// clamps the resulting ratio and no-ops a pane that has no parent split
    /// (the sole pane).
    Resize {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pane_id: Option<String>,
        delta: f64,
    },
    /// Close one tab: `tab_id` in `pane_id` (each defaults to the active
    /// tab / pane). `force` closes past a dirty file / live terminal.
    CloseTab {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pane_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tab_id: Option<String>,
        #[serde(default)]
        force: bool,
    },
    /// Close a whole pane (the active one when absent). `force` as above.
    ClosePane {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pane_id: Option<String>,
        #[serde(default)]
        force: bool,
    },
    /// Close every tab in every pane. `force` as above.
    CloseAll {
        #[serde(default)]
        force: bool,
    },
}

/// Which side a [`PaneOp::Split`] places the new pane. `right` splits the
/// pane horizontally (new pane to the right); `bottom` splits it vertically
/// (new pane below). Matches the hybrid pane hamburger (right / bottom). Bare
/// snake_case on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SplitDir {
    Right,
    Bottom,
}

/// A survey raised over terminal tab(s) by `cs terminal survey`. Carried in
/// [`ControlRequest::TermSurvey`] from the CLI, then pushed to the SPA in an
/// `open_survey` window command. The CLI builds it with an EMPTY `survey_id`;
/// the server mints the id before the SPA sees it, and the SPA echoes that id
/// back in its [`SurveyReply`] so the server matches the parked oneshot.
///
/// serde camelCase: this is the exact JSON the SPA reads; the SPA's
/// TypeScript types mirror this struct field for field.
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
    /// The user hit [F] (follow up). When the survey carried followup context,
    /// C created `{dir}/followups/followup-{from}-{to}-{n}.md` and
    /// `followup_path` is that workspace-relative path. Part C made [F]
    /// standard on every survey, so a survey raised WITHOUT followup context
    /// still offers it: that is a plain deferral and `followup_path` is `None`
    /// (no file).
    #[serde(rename = "followup", rename_all = "camelCase")]
    Followup {
        survey_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        followup_path: Option<String>,
    },
    /// The user hit Dismiss (Part C). A distinct reply with no option index
    /// and no file, so the asking agent can tell the host dropped the survey
    /// rather than answering it or deferring with [F].
    #[serde(rename = "dismissed", rename_all = "camelCase")]
    Dismissed { survey_id: String },
}

impl SurveyReply {
    /// The `survey_id` this reply echoes, used to match the parked oneshot
    /// regardless of which variant it is.
    pub fn survey_id(&self) -> &str {
        match self {
            SurveyReply::Option { survey_id, .. } => survey_id,
            SurveyReply::Followup { survey_id, .. } => survey_id,
            SurveyReply::Dismissed { survey_id } => survey_id,
        }
    }
}

#[cfg(test)]
mod survey_wire_tests {
    //! These pin the EXACT on-wire JSON of the survey types. The serde
    //! tags + camelCase are the contract between the CLI and the SPA's
    //! reply route; a Rust rename that drifts them breaks at runtime
    //! with a green build, so assert the bytes, not just round-trip.
    use super::*;

    #[test]
    fn survey_spec_is_camel_case_with_explicit_nulls() {
        let spec = SurveySpec {
            survey_id: "survey-3".into(),
            title: None,
            body_markdown: "pick one".into(),
            options: vec!["A".into(), "B".into()],
            followup: Some(SurveyFollowup {
                dir: "team".into(),
                from: "@@Alice".into(),
                to: "@@Bob".into(),
            }),
        };
        let v: serde_json::Value = serde_json::to_value(&spec).unwrap();
        assert_eq!(v["surveyId"], "survey-3");
        // title is null (not omitted), matching the contract's `string|null`.
        assert!(v.get("title").is_some_and(|t| t.is_null()));
        assert_eq!(v["bodyMarkdown"], "pick one");
        assert_eq!(v["options"], serde_json::json!(["A", "B"]));
        assert_eq!(v["followup"]["dir"], "team");
        assert_eq!(v["followup"]["from"], "@@Alice");
        assert_eq!(v["followup"]["to"], "@@Bob");
    }

    #[test]
    fn survey_spec_emits_null_followup_when_absent() {
        let spec = SurveySpec {
            survey_id: String::new(),
            title: Some("Heads up".into()),
            body_markdown: "x".into(),
            options: vec!["ok".into()],
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
            followup_path: Some("team/followups/followup-a-b-1.md".into()),
        };
        let v: serde_json::Value = serde_json::to_value(&reply).unwrap();
        assert_eq!(v["kind"], "followup");
        assert_eq!(v["surveyId"], "survey-9");
        assert_eq!(v["followupPath"], "team/followups/followup-a-b-1.md");
    }

    #[test]
    fn survey_reply_followup_without_path_is_a_bare_deferral() {
        // [F] is standard on every survey. A survey raised without
        // followup context defers with no file, so `followupPath` is omitted.
        let reply = SurveyReply::Followup {
            survey_id: "survey-9".into(),
            followup_path: None,
        };
        let v: serde_json::Value = serde_json::to_value(&reply).unwrap();
        assert_eq!(v["kind"], "followup");
        assert_eq!(v["surveyId"], "survey-9");
        assert!(v.get("followupPath").is_none());
    }

    #[test]
    fn survey_reply_dismissed_tag_and_fields() {
        // Dismiss is a distinct reply (no option index, no file) so
        // the asking agent can tell the host dropped the survey.
        let reply = SurveyReply::Dismissed {
            survey_id: "survey-7".into(),
        };
        let v: serde_json::Value = serde_json::to_value(&reply).unwrap();
        assert_eq!(v["kind"], "dismissed");
        assert_eq!(v["surveyId"], "survey-7");
        assert_eq!(reply.survey_id(), "survey-7");
    }

    #[test]
    fn term_survey_request_tag_and_spec_round_trip() {
        let req = ControlRequest::TermSurvey {
            tab_name: Some("@@Alice".into()),
            tab_group: None,
            spec: SurveySpec {
                survey_id: String::new(),
                title: None,
                body_markdown: "q".into(),
                options: vec!["yes".into()],
                followup: None,
            },
        };
        let v: serde_json::Value = serde_json::to_value(&req).unwrap();
        assert_eq!(v["type"], "term_survey");
        assert_eq!(v["tab_name"], "@@Alice");
        // tab_group None is skipped on the wire (matches the sibling variants).
        assert!(v.get("tab_group").is_none());
        assert_eq!(v["spec"]["bodyMarkdown"], "q");
        // Decodes back into the same variant (the server's path).
        let raw = serde_json::to_string(&req).unwrap();
        let back: ControlRequest = serde_json::from_str(&raw).unwrap();
        assert!(matches!(back, ControlRequest::TermSurvey { .. }));
    }

    #[test]
    fn window_list_request_tag() {
        // The wire tag is `window_list` (a bare unit variant; no fields).
        // A Rust rename that drifts it breaks the server's decode with a
        // green build.
        let v: serde_json::Value = serde_json::to_value(ControlRequest::WindowList).unwrap();
        assert_eq!(v, serde_json::json!({"type": "window_list"}));
        let back: ControlRequest = serde_json::from_str(r#"{"type":"window_list"}"#).unwrap();
        assert!(matches!(back, ControlRequest::WindowList));
    }

    #[test]
    fn window_new_request_tag() {
        // Bare unit variant; the server derives the kind from the tenant.
        let v: serde_json::Value = serde_json::to_value(ControlRequest::WindowNew).unwrap();
        assert_eq!(v, serde_json::json!({"type": "window_new"}));
        let back: ControlRequest = serde_json::from_str(r#"{"type":"window_new"}"#).unwrap();
        assert!(matches!(back, ControlRequest::WindowNew));
    }

    #[test]
    fn window_open_request_tag_and_id() {
        let req = ControlRequest::WindowOpen {
            id: "terminal-win-2".into(),
        };
        let v: serde_json::Value = serde_json::to_value(&req).unwrap();
        assert_eq!(v["type"], "window_open");
        assert_eq!(v["id"], "terminal-win-2");
        let back: ControlRequest =
            serde_json::from_str(&serde_json::to_string(&req).unwrap()).unwrap();
        assert!(matches!(back, ControlRequest::WindowOpen { .. }));
    }

    #[test]
    fn unserve_request_tag_and_path() {
        // Pin the `chan unserve` transport wire: tag `unserve`, a single
        // `path` (the canonical workspace root). A rename here would silently
        // break `cmd_unserve` ↔ the control-socket handler.
        let req = ControlRequest::Unserve {
            path: PathBuf::from("/home/u/notes"),
        };
        let v: serde_json::Value = serde_json::to_value(&req).unwrap();
        assert_eq!(v["type"], "unserve");
        assert_eq!(v["path"], "/home/u/notes");
        let back: ControlRequest =
            serde_json::from_str(&serde_json::to_string(&req).unwrap()).unwrap();
        assert!(matches!(back, ControlRequest::Unserve { .. }));
    }

    #[test]
    fn window_close_request_tag_id_and_force() {
        // `cs window rm`: wire tag `window_close`, `force` defaults false
        // and is omitted when false (matches the PaneOp force convention).
        let req = ControlRequest::WindowClose {
            id: "terminal-win-2".into(),
            force: false,
        };
        let v: serde_json::Value = serde_json::to_value(&req).unwrap();
        assert_eq!(v["type"], "window_close");
        assert_eq!(v["id"], "terminal-win-2");
        assert_eq!(v["force"], false);
        let forced = ControlRequest::WindowClose {
            id: "terminal-win-2".into(),
            force: true,
        };
        // Tolerates a missing `force` on decode (server-side default).
        let back: ControlRequest =
            serde_json::from_str(r#"{"type":"window_close","id":"terminal-win-2"}"#).unwrap();
        assert!(matches!(
            back,
            ControlRequest::WindowClose { force: false, .. }
        ));
        assert!(matches!(
            serde_json::from_str::<ControlRequest>(&serde_json::to_string(&forced).unwrap())
                .unwrap(),
            ControlRequest::WindowClose { force: true, .. }
        ));
    }

    #[test]
    fn window_hide_request_tag_and_id() {
        let req = ControlRequest::WindowHide {
            id: "workspace-aa-0".into(),
        };
        let v: serde_json::Value = serde_json::to_value(&req).unwrap();
        assert_eq!(v["type"], "window_hide");
        assert_eq!(v["id"], "workspace-aa-0");
        let back: ControlRequest =
            serde_json::from_str(&serde_json::to_string(&req).unwrap()).unwrap();
        assert!(matches!(back, ControlRequest::WindowHide { .. }));
    }

    #[test]
    fn window_title_request_tag_and_fields() {
        let req = ControlRequest::WindowTitle {
            id: "terminal-win-2".into(),
            title: "Build logs".into(),
        };
        let v: serde_json::Value = serde_json::to_value(&req).unwrap();
        assert_eq!(v["type"], "window_title");
        assert_eq!(v["id"], "terminal-win-2");
        assert_eq!(v["title"], "Build logs");
        let back: ControlRequest =
            serde_json::from_str(&serde_json::to_string(&req).unwrap()).unwrap();
        assert!(matches!(back, ControlRequest::WindowTitle { .. }));
    }

    #[test]
    fn term_scrollback_request_tag_and_field() {
        // The wire tag is `term_scrollback` and `tab_name` is a plain
        // required string (no group axis, no skip). A Rust rename that
        // drifts either breaks the server's decode with a green build.
        let req = ControlRequest::TermScrollback {
            tab_name: "@@Alice".into(),
        };
        let v: serde_json::Value = serde_json::to_value(&req).unwrap();
        assert_eq!(v["type"], "term_scrollback");
        assert_eq!(v["tab_name"], "@@Alice");
        // Decodes back into the same variant (the server's path).
        let raw = serde_json::to_string(&req).unwrap();
        let back: ControlRequest = serde_json::from_str(&raw).unwrap();
        assert!(matches!(back, ControlRequest::TermScrollback { .. }));
    }

    #[test]
    fn pane_query_request_tag_and_target() {
        // Wire tag `pane_query`; the target is window_id XOR tab_name (both
        // optional + skipped when None). A Rust rename that drifts either
        // breaks the server's decode with a green build.
        let by_window = ControlRequest::PaneQuery {
            window_id: Some("window-a".into()),
            tab_name: None,
        };
        let v: serde_json::Value = serde_json::to_value(&by_window).unwrap();
        assert_eq!(v["type"], "pane_query");
        assert_eq!(v["window_id"], "window-a");
        assert!(v.get("tab_name").is_none(), "None tab_name is skipped");
        let raw = serde_json::to_string(&by_window).unwrap();
        let back: ControlRequest = serde_json::from_str(&raw).unwrap();
        assert!(matches!(back, ControlRequest::PaneQuery { .. }));

        let by_tab = ControlRequest::PaneQuery {
            window_id: None,
            tab_name: Some("@@Alice".into()),
        };
        let v: serde_json::Value = serde_json::to_value(&by_tab).unwrap();
        assert_eq!(v["tab_name"], "@@Alice");
        assert!(v.get("window_id").is_none(), "None window_id is skipped");
    }

    #[test]
    fn pane_exec_request_tag_and_op_kind() {
        // `pane_exec` carries a `tab_name`/`window_id` target plus the op,
        // which is internally tagged on `kind` and nests under `op`.
        let req = ControlRequest::PaneExec {
            window_id: None,
            tab_name: Some("@@Alice".into()),
            op: PaneOp::Split {
                pane_id: Some("pane-1".into()),
                dir: SplitDir::Bottom,
            },
        };
        let v: serde_json::Value = serde_json::to_value(&req).unwrap();
        assert_eq!(v["type"], "pane_exec");
        assert_eq!(v["tab_name"], "@@Alice");
        assert_eq!(v["op"]["kind"], "split");
        assert_eq!(v["op"]["pane_id"], "pane-1");
        assert_eq!(v["op"]["dir"], "bottom");
        // Round-trips into the same variant (the server's decode path).
        let raw = serde_json::to_string(&req).unwrap();
        let back: ControlRequest = serde_json::from_str(&raw).unwrap();
        assert!(matches!(
            back,
            ControlRequest::PaneExec {
                op: PaneOp::Split { .. },
                ..
            }
        ));
    }

    #[test]
    fn pane_op_close_variants_carry_force() {
        // close_tab / close_pane / close_all all carry `force`; `force:false`
        // is emitted (default) so the SPA always reads a boolean.
        let v: serde_json::Value = serde_json::to_value(&PaneOp::CloseAll { force: true }).unwrap();
        assert_eq!(v["kind"], "close_all");
        assert_eq!(v["force"], true);
        let v: serde_json::Value = serde_json::to_value(&PaneOp::CloseTab {
            pane_id: None,
            tab_id: None,
            force: false,
        })
        .unwrap();
        assert_eq!(v["kind"], "close_tab");
        assert_eq!(v["force"], false);
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
            window_id: Some("window-a".into()),
        };
        let v: serde_json::Value = serde_json::to_value(&req).unwrap();
        assert_eq!(v["type"], "terminal_team");
        assert_eq!(v["dir"], "new-team-1");
        assert_eq!(v["op"], "new");
        assert_eq!(v["config_toml"], "team_name = \"alpha\"\n");
        assert_eq!(v["script"], true);
        assert_eq!(v["window_id"], "window-a");

        // `load` omits config_toml + window_id (skip_serializing_if) and
        // defaults script to false.
        let load = ControlRequest::TerminalTeam {
            dir: "teams/alpha".into(),
            op: TeamOp::Load,
            config_toml: None,
            script: false,
            window_id: None,
        };
        let v: serde_json::Value = serde_json::to_value(&load).unwrap();
        assert_eq!(v["op"], "load");
        assert!(
            v.get("config_toml").is_none(),
            "None config_toml is skipped"
        );
        assert!(v.get("window_id").is_none(), "None window_id is skipped");
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

    #[test]
    fn identify_request_and_identity_reply_wire() {
        // The request is a bare tagged unit; `chan ps` and the chan-server
        // handler must agree on the exact bytes.
        let v = serde_json::to_value(&ControlRequest::Identify).unwrap();
        assert_eq!(v, serde_json::json!({ "type": "identify" }));
        assert!(matches!(
            serde_json::from_str::<ControlRequest>(r#"{"type":"identify"}"#).unwrap(),
            ControlRequest::Identify
        ));

        // The reply payload (JSON in Ok.message): kind serializes to the
        // `standalone`/`desktop`/`devserver` strings `chan ps` shows.
        let id = Identity {
            kind: ServeKind::Devserver,
            version: "0.40.0".into(),
        };
        let v = serde_json::to_value(&id).unwrap();
        assert_eq!(
            v,
            serde_json::json!({ "kind": "devserver", "version": "0.40.0" })
        );
        assert_eq!(id, serde_json::from_value(v).unwrap());
        assert_eq!(
            serde_json::to_value(ServeKind::Standalone).unwrap(),
            serde_json::json!("standalone")
        );
        assert_eq!(
            serde_json::to_value(ServeKind::Desktop).unwrap(),
            serde_json::json!("desktop")
        );
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

/// What kind of process serves a workspace, for `chan ps`. A `serve` standalone,
/// a chan-desktop, or a headless devserver. Serializes to `standalone` /
/// `desktop` / `devserver`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServeKind {
    Standalone,
    Desktop,
    Devserver,
}

/// Reply payload for [`ControlRequest::Identify`], JSON-encoded into the
/// `Ok.message` of a [`ControlResponse`] (the convention for structured control
/// replies). `version` is the server's `CARGO_PKG_VERSION`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Identity {
    pub kind: ServeKind,
    pub version: String,
}
