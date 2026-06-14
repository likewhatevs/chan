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
    invoked_as(arg0, "cs")
}

/// Whether this process was invoked through a `chan` name. chan-desktop
/// dispatches the `chan` CLI in-process (`chan::run(.., Personality::Desktop)`)
/// when launched through a `~/.local/bin/chan` shim, exactly as it already
/// does for `cs`. The same file-stem rule as [`invoked_as_cs`] applies, so a
/// real symlink, an AppImage wrapper, and a bare `chan` all match.
#[cfg(feature = "client")]
pub fn invoked_as_chan(arg0: &std::ffi::OsStr) -> bool {
    invoked_as(arg0, "chan")
}

/// Shared file-stem comparison behind [`invoked_as_cs`] / [`invoked_as_chan`]:
/// ignore any directory and extension so `/usr/local/bin/<name>` and a bare
/// `<name>` both match.
#[cfg(feature = "client")]
fn invoked_as(arg0: &std::ffi::OsStr, name: &str) -> bool {
    std::path::Path::new(arg0)
        .file_stem()
        .map(|stem| stem == name)
        .unwrap_or(false)
}

#[cfg(all(test, feature = "client"))]
mod arg0_tests {
    use super::{invoked_as_chan, invoked_as_cs};
    use std::ffi::OsStr;

    #[test]
    fn cs_matches_bare_path_and_extension() {
        assert!(invoked_as_cs(OsStr::new("cs")));
        assert!(invoked_as_cs(OsStr::new("/usr/local/bin/cs")));
        assert!(invoked_as_cs(OsStr::new("cs.exe")));
        assert!(!invoked_as_cs(OsStr::new("chan")));
        assert!(!invoked_as_cs(OsStr::new("/opt/chan-desktop")));
    }

    #[test]
    fn chan_matches_bare_path_and_appimage_wrapper() {
        assert!(invoked_as_chan(OsStr::new("chan")));
        assert!(invoked_as_chan(OsStr::new("/home/u/.local/bin/chan")));
        assert!(invoked_as_chan(OsStr::new("chan.exe")));
        // The desktop binary's own name must NOT be seen as `chan`, or a
        // normal GUI launch would re-dispatch into the CLI forever.
        assert!(!invoked_as_chan(OsStr::new("chan-desktop")));
        assert!(!invoked_as_chan(OsStr::new(
            "/Applications/Chan.app/Contents/MacOS/chan-desktop"
        )));
        assert!(!invoked_as_chan(OsStr::new("cs")));
    }
}
