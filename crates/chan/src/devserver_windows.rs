//! Windows `--service` backend: a detached background devserver tracked by a
//! PID/state file — the systemd/launchd analog on Windows.
//!
//! Unlike systemd/launchd there is no OS supervisor: the detached process is
//! per-login (it does **not** survive logout and does **not** auto-restart on
//! crash). It does survive the launching shell, which is enough for "start a
//! headless devserver and walk away for this login session." `--service --stop`
//! / `--restart` locate it by the recorded pid + process creation time (a
//! pid-reuse guard) and `TerminateProcess` it: a `DETACHED_PROCESS` child has no
//! console, so `CTRL_C_EVENT` cannot reach it. A hard stop is safe — the
//! devserver drains HTTP per request and releases its per-workspace flocks on
//! exit (same as `kill -9`, which the writer lock already self-heals).
//!
//! Assumes the console `chan.exe`: the detached child re-runs `current_exe()`
//! with `ARGV0=chan` so a multiplexed `chan-desktop.exe` still dispatches to the
//! CLI devserver path rather than launching the GUI.

use std::net::SocketAddr;
use std::os::windows::io::AsRawHandle;
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use windows_sys::Win32::Foundation::{CloseHandle, FILETIME, HANDLE};
use windows_sys::Win32::System::Threading::{
    GetProcessTimes, OpenProcess, TerminateProcess, CREATE_NEW_PROCESS_GROUP, DETACHED_PROCESS,
    PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_TERMINATE,
};

use chan_workspace::lock::{process_alive, ProcessLiveness};

/// Ephemeral supervision state for the detached devserver, at
/// `~/.chan/devserver/service.json` (sibling of the durable `config.json`).
/// Separate file because its lifecycle is "which pid is running right now" —
/// meaningless across reboots and freely cleanable, unlike the persisted token
/// + library id. `creation_time` is the FILETIME-derived pid-reuse guard: a
/// recorded pid is "ours" only if it is alive AND its process creation time
/// still matches.
#[derive(Debug, Serialize, Deserialize)]
struct ServiceState {
    pid: u32,
    creation_time: u64,
    addr: String,
    exe: String,
}

/// `~/.chan/devserver/service.json`. Routed through the chan-home authority
/// (`config_dir`) so `CHAN_HOME` moves it, matching the token/log paths.
fn service_state_path() -> PathBuf {
    chan_workspace::paths::config_dir()
        .join("devserver")
        .join("service.json")
}

fn read_state() -> Option<ServiceState> {
    let bytes = std::fs::read(service_state_path()).ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn write_state(state: &ServiceState) -> Result<()> {
    let path = service_state_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    let json = serde_json::to_vec_pretty(state).context("serializing devserver service state")?;
    std::fs::write(&path, json).with_context(|| format!("writing {}", path.display()))
}

fn remove_state() {
    let _ = std::fs::remove_file(service_state_path());
}

/// FILETIME as a single u64 (100ns ticks since 1601), for equality compares.
fn filetime_to_u64(ft: &FILETIME) -> u64 {
    ((ft.dwHighDateTime as u64) << 32) | (ft.dwLowDateTime as u64)
}

/// The process creation time for `pid`, or `None` if it cannot be read (no such
/// process, access denied, or the call fails). Used as the pid-reuse guard.
fn process_creation_time(pid: u32) -> Option<u64> {
    // SAFETY: plain Win32 FFI. We open with the minimal query right, never
    // inherit the handle, and CloseHandle before returning.
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            return None;
        }
        let mut creation: FILETIME = std::mem::zeroed();
        let mut ignored: FILETIME = std::mem::zeroed();
        let ok = GetProcessTimes(
            handle,
            &mut creation,
            &mut ignored,
            &mut ignored,
            &mut ignored,
        );
        CloseHandle(handle);
        (ok != 0).then(|| filetime_to_u64(&creation))
    }
}

/// The creation time of a freshly-spawned child via its process handle, so the
/// guard is recorded at spawn without a second `OpenProcess` race.
fn handle_creation_time(handle: HANDLE) -> Option<u64> {
    // SAFETY: `handle` is the live child handle owned by `std::process::Child`
    // for the duration of the call; we only read times and never close it (the
    // `Child` owns it).
    unsafe {
        let mut creation: FILETIME = std::mem::zeroed();
        let mut ignored: FILETIME = std::mem::zeroed();
        let ok = GetProcessTimes(
            handle,
            &mut creation,
            &mut ignored,
            &mut ignored,
            &mut ignored,
        );
        (ok != 0).then(|| filetime_to_u64(&creation))
    }
}

/// Is the recorded service still our live devserver? Alive AND (when a creation
/// time was recorded) the creation time matches, so a reused pid is never
/// mistaken for ours. A `0` recorded creation time (the spawn-time read failed)
/// degrades to liveness-only rather than refusing forever.
fn recorded_is_ours(state: &ServiceState) -> bool {
    if !matches!(process_alive(state.pid), ProcessLiveness::Alive) {
        return false;
    }
    if state.creation_time == 0 {
        return true;
    }
    process_creation_time(state.pid) == Some(state.creation_time)
}

/// `TerminateProcess(pid, 0)`. Returns true on success (or if the process is
/// already gone). Best-effort — the caller has already confirmed the pid is ours.
fn terminate(pid: u32) -> bool {
    // SAFETY: plain Win32 FFI; open with terminate right, close the handle.
    unsafe {
        let handle = OpenProcess(PROCESS_TERMINATE, 0, pid);
        if handle.is_null() {
            // No such process / access denied: treat as already-stopped.
            return true;
        }
        let ok = TerminateProcess(handle, 0);
        CloseHandle(handle);
        ok != 0
    }
}

/// Spawn the detached foreground devserver and return its `(pid, creation_time)`.
/// `DETACHED_PROCESS` cuts the console (truly headless, survives closing the
/// launching terminal); `CREATE_NEW_PROCESS_GROUP` keeps the launching console's
/// Ctrl-C from reaching it. stdout/stderr go to `~/.chan/devserver/devserver.log`
/// (no journal on Windows). The child runs WITHOUT `--service` so it is the plain
/// foreground server.
fn spawn_detached(addr: SocketAddr) -> Result<(u32, u64)> {
    let exe = std::env::current_exe().context("resolving the chan binary path to spawn")?;
    let log = crate::devserver_log_path()?;
    if let Some(parent) = log.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    let out = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log)
        .with_context(|| format!("opening the devserver log {}", log.display()))?;
    let err = out
        .try_clone()
        .context("cloning the devserver log handle for stderr")?;

    let mut cmd = Command::new(&exe);
    cmd.args([
        "devserver",
        "--bind",
        &addr.ip().to_string(),
        "--port",
        &addr.port().to_string(),
    ]);
    // A multiplexed chan-desktop.exe dispatches to the CLI only when ARGV0 is
    // `chan`; a real chan.exe ignores it. Set it so either binary serves.
    cmd.env("ARGV0", "chan");
    cmd.creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP);
    cmd.stdout(out);
    cmd.stderr(err);

    let child = cmd
        .spawn()
        .with_context(|| format!("spawning the detached devserver {}", exe.display()))?;
    let pid = child.id();
    // Read the creation time while the child handle is still owned by `child`.
    // Dropping `child` does NOT kill the process (std just closes the handle).
    let creation_time = handle_creation_time(child.as_raw_handle() as HANDLE).unwrap_or(0);
    Ok((pid, creation_time))
}

/// `chan devserver --service` on Windows: re-attach to a running detached
/// devserver (idempotent) or spawn one, then surface the bearer token and exit.
/// Unlike systemd/launchd this does NOT block-follow output — the detached
/// process owns the log file; the launching command returns once the token is
/// known.
pub async fn run_devserver_under_windows(addr: SocketAddr) -> Result<()> {
    if let Some(state) = read_state() {
        if recorded_is_ours(&state) {
            crate::emit_devserver_token_marker(crate::DEVSERVER_TOKEN_WAIT).await?;
            eprintln!(
                "chan devserver: re-attaching to the running detached devserver (pid {})",
                state.pid
            );
            return Ok(());
        }
    }

    let (pid, creation_time) = spawn_detached(addr)?;
    write_state(&ServiceState {
        pid,
        creation_time,
        addr: addr.to_string(),
        exe: std::env::current_exe()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default(),
    })?;
    // The detached child prints its own token marker to the log file, invisible
    // to this terminal — surface it from the persisted 0600 config so the
    // desktop reconnects, and fail loud if it never lands rather than claim
    // "started" on a token we cannot hand back.
    crate::emit_devserver_token_marker(crate::DEVSERVER_TOKEN_WAIT).await?;
    let log = crate::devserver_log_path()?;
    eprintln!(
        "chan devserver: started the detached devserver (pid {pid}, bind={addr}); \
         logs at {}",
        log.display()
    );
    Ok(())
}

/// `chan devserver --service --stop` on Windows: terminate the recorded detached
/// devserver and clear the state file. Idempotent — a no-op (with a note) when
/// nothing ours is running. A stale file whose pid is dead or reused is cleaned
/// without touching the unrelated process.
pub async fn stop_devserver_under_windows() -> Result<()> {
    let Some(state) = read_state() else {
        eprintln!("chan devserver: no detached devserver is running.");
        return Ok(());
    };
    if !recorded_is_ours(&state) {
        // Dead or pid-reused: do not signal a process that is not ours.
        remove_state();
        eprintln!("chan devserver: no detached devserver is running (cleared stale state).");
        return Ok(());
    }
    if terminate(state.pid) {
        remove_state();
        eprintln!(
            "chan devserver: stopped the detached devserver (pid {}).",
            state.pid
        );
        Ok(())
    } else {
        anyhow::bail!(
            "chan devserver: failed to terminate the detached devserver (pid {}).",
            state.pid
        )
    }
}

/// `chan devserver --service --restart` on Windows: stop the running detached
/// devserver (if any), then spawn a fresh one on `addr`.
pub async fn restart_devserver_under_windows(addr: SocketAddr) -> Result<()> {
    // Best-effort stop; a not-running instance is a no-op.
    stop_devserver_under_windows().await?;
    run_devserver_under_windows(addr).await
}
