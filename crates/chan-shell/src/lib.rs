//! `chan-shell`: the `cs` control-socket client, shared by the `chan`
//! binary and `chan-desktop` so desktop users get `cs` (and the MCP
//! discovery it carries) without a separate `chan` install.
//!
//! Two layers:
//!
//!   - The WIRE types ([`ControlRequest`] / [`ControlResponse`]) are
//!     always compiled (serde only). chan-server depends on chan-shell
//!     with `default-features = false` to share them, which is what kills
//!     the old client/server duplication.
//!   - The `client` feature (default) adds the clap surface
//!     ([`ShellAction`] / [`TerminalAction`]), the [`dispatch`] entry, the
//!     control transport, and the agent [`SubmitAgent`] submit map. The
//!     `chan` / `chan-desktop` binaries enable it; chan-server does not.

mod wire;
pub use wire::{
    ControlRequest, ControlResponse, PaneOp, SplitDir, SurveyFollowup, SurveyReply, SurveySpec,
    TeamOp,
};

#[cfg(feature = "client")]
mod cli;
#[cfg(feature = "client")]
mod control;
// The submit map is always compiled (serde-free, clap-free): chan-server's
// server-side team spawner reads the agent submit chords without pulling the
// `client` feature (clap). Only the `ValueEnum` parse impl for the
// `--submit` flag is `client`-gated, inside the module.
mod submit;

#[cfg(feature = "client")]
pub use cli::{dispatch, run_cs, ShellAction, TerminalAction};
#[cfg(feature = "client")]
pub use control::{
    absolutize, control_socket_env, open_env, open_env_from, send_control_request, OpenEnv,
};
pub use submit::{apply_submit_chord, set_chord_overrides, submit_writes, SubmitAgent};

/// Whether this process was invoked through a `cs` name (a `cs -> chan`
/// symlink on PATH, or chan-desktop launched as `cs`). Both `chan`'s
/// `parse_cli` and chan-desktop's entry use this so the `cs` alias rewrite
/// is defined once. The file stem comparison ignores any directory and
/// extension, so `/usr/local/bin/cs` and a bare `cs` both match.
#[cfg(feature = "client")]
pub fn invoked_as_cs(arg0: &std::ffi::OsStr) -> bool {
    std::path::Path::new(arg0)
        .file_stem()
        .map(|stem| stem == "cs")
        .unwrap_or(false)
}
