//! The `cs` alias: invoking the chan binary through a `cs` symlink
//! rewrites argv so `cs <action>` parses as `chan shell <action>`. The
//! build never ships the symlink; the user creates it. These tests
//! create the symlink in a tempdir and exercise the dispatch.

#![cfg(unix)]

use std::os::unix::fs::symlink;
use std::process::Command;

/// Symlink `cs -> chan` in a fresh tempdir and return its path.
fn cs_symlink() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let cs = dir.path().join("cs");
    symlink(env!("CARGO_BIN_EXE_chan"), &cs).expect("symlink cs -> chan");
    (dir, cs)
}

#[test]
fn cs_terminal_list_dispatches_to_shell() {
    let (_dir, cs) = cs_symlink();
    // Outside a chan terminal there is no control socket, so `cs terminal
    // list` errors for the missing $CHAN_CONTROL_SOCKET. That the
    // `terminal` subcommand parses AT ALL proves the `cs -> shell`
    // rewrite: plain `chan terminal list` would be an unknown subcommand.
    let output = Command::new(&cs)
        .args(["terminal", "list"])
        .env_remove("CHAN_CONTROL_SOCKET")
        .env_remove("CHAN_WINDOW_ID")
        .output()
        .expect("run cs terminal list");

    assert!(
        !output.status.success(),
        "cs terminal list should fail without a control socket"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("CHAN_CONTROL_SOCKET"),
        "expected the missing-control-socket error, got: {stderr}"
    );
}

#[test]
fn cs_help_shows_shell_subcommands() {
    let (_dir, cs) = cs_symlink();
    // `cs --help` is `chan shell --help`, so its usage lists the shell
    // actions (terminal / graph / dashboard), not the top-level commands.
    let output = Command::new(&cs)
        .arg("--help")
        .output()
        .expect("run cs --help");

    assert!(output.status.success(), "cs --help should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("terminal"),
        "shell help should list `terminal`: {stdout}"
    );
    assert!(
        stdout.contains("dashboard"),
        "shell help should list `dashboard`: {stdout}"
    );
    // A top-level-only command must not appear: this is the shell group.
    // `upgrade` is top-level-only and (unlike `open`) is not also a `cs`
    // action, so its absence proves this is the shell surface.
    assert!(
        !stdout.contains("upgrade"),
        "shell help should not list the top-level `upgrade`: {stdout}"
    );
}

#[test]
fn plain_chan_rejects_terminal_subcommand() {
    // Control: WITHOUT the `cs` rewrite, `chan terminal` is unknown. This
    // is what makes the rewrite load-bearing.
    let output = Command::new(env!("CARGO_BIN_EXE_chan"))
        .args(["terminal", "list"])
        .output()
        .expect("run chan terminal list");

    assert!(!output.status.success(), "chan terminal list should fail");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unrecognized subcommand") || stderr.contains("unexpected"),
        "expected an unknown-subcommand error, got: {stderr}"
    );
}

#[test]
fn cs_prefix_match_resolves_terminal_list() {
    let (_dir, cs) = cs_symlink();
    // iproute2-style prefix matching: `cs t l` == `cs terminal list` via
    // clap infer_subcommands (t -> terminal, l -> list). That it resolves
    // far enough to fail on the missing control socket (rather than an
    // unknown-subcommand error) proves the prefix dispatch.
    let output = Command::new(&cs)
        .args(["t", "l"])
        .env_remove("CHAN_CONTROL_SOCKET")
        .env_remove("CHAN_WINDOW_ID")
        .output()
        .expect("run cs t l");

    assert!(
        !output.status.success(),
        "cs t l should fail without a control socket"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("CHAN_CONTROL_SOCKET"),
        "expected the missing-control-socket error (proving t->terminal, l->list), got: {stderr}"
    );
}
