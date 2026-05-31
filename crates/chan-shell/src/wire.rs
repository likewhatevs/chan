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
    // Category 2: run the same content search the UI does and return the
    // results on the connection (like `term list`). The CLI formats the
    // JSON it gets back: markdown by default, compact `--json`, indented
    // `--json --pretty`.
    Search {
        query: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        limit: Option<u32>,
    },
}

/// The single-line reply the server writes back on the control socket.
/// The internal `status` tag is the wire format; the client matches on it.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ControlResponse {
    Ok { message: String },
    Error { message: String },
}
