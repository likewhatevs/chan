//! The `--service=chan` self-managed daemon: a cross-OS FOREGROUND devserver
//! guarded by a single-instance pidfile + flock ([`chan_workspace::daemon_lock`]).
//! It is the systemd/launchd analog where there is no OS supervisor (Windows,
//! other Unix), and the explicit portable choice everywhere.
//!
//! Unlike the systemd/launchd backends the daemon runs in the FOREGROUND on
//! every OS: chan-desktop's connect-script form needs the launching command to
//! stay attached (so the control terminal's PTY tracks the connection), and the
//! motivating `ssh -L ... chan devserver --service` case ties the devserver to
//! the long-lived session that holds the tunnel. Closing that session stops the
//! daemon; the walk-away "survives logout" model stays systemd/launchd's job via
//! `--service` (auto) or the explicit backends.

use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::Result;

use chan_workspace::daemon_lock::{
    is_record_live, read_daemon_record, signal_terminate, DaemonAcquire, DaemonLock,
};

/// `~/.chan/devserver/daemon.lock` -- the flock anchor (routed through the
/// chan-home authority so `CHAN_HOME` moves it, matching the token/log paths).
fn daemon_lock_path() -> PathBuf {
    chan_workspace::paths::config_dir()
        .join("devserver")
        .join("daemon.lock")
}

/// `~/.chan/devserver/daemon.json` -- the pidfile read by `--status` / `--stop`.
fn daemon_record_path() -> PathBuf {
    chan_workspace::paths::config_dir()
        .join("devserver")
        .join("daemon.json")
}

/// `chan devserver --service=chan` (or the per-OS auto pick on Windows/other):
/// become the foreground daemon, or with `--force` turn down a running one and
/// take over. Without `--force`, an already-running daemon is reported (the
/// watchdog re-attach is a later step).
pub async fn run_devserver_as_chan(addr: SocketAddr, force: bool, verbose: bool) -> Result<()> {
    let lock_path = daemon_lock_path();
    let record_path = daemon_record_path();
    if verbose {
        print_daemon_paths(&lock_path, &record_path);
    }
    if force {
        return take_over(&lock_path, &record_path, addr).await;
    }
    match DaemonLock::acquire(&lock_path, &record_path, &addr.to_string(), false)
        .map_err(|e| anyhow::anyhow!("acquiring the devserver daemon lock: {e}"))?
    {
        DaemonAcquire::Daemon(guard) => serve_as_daemon(guard, addr).await,
        DaemonAcquire::Running(record) => {
            if record.addr != addr.to_string() {
                anyhow::bail!(
                    "chan devserver: a self-managed daemon is already running on {} \
                     (pid {}); requested {addr}. Use --restart to rebind, --stop to stop \
                     it, or --force to replace it.",
                    record.addr,
                    record.pid,
                );
            }
            anyhow::bail!(
                "chan devserver: a self-managed daemon is already running (pid {}) on {}. \
                 Use --restart to bounce it, --stop to stop it, or --force to replace it.",
                record.pid,
                record.addr,
            );
        }
    }
}

/// `--restart`: turn down any running daemon, then serve. Starts one if none is
/// running.
pub async fn restart_devserver_chan(addr: SocketAddr, verbose: bool) -> Result<()> {
    run_devserver_as_chan(addr, true, verbose).await
}

/// `--stop`: terminate the running daemon and clear the pidfile. Idempotent --
/// a no-op (with a note) when nothing is running; a stale pidfile is cleared
/// without signalling an unrelated process.
pub async fn stop_devserver_chan(verbose: bool) -> Result<()> {
    let record_path = daemon_record_path();
    if verbose {
        print_daemon_paths(&daemon_lock_path(), &record_path);
    }
    let Some(record) = read_daemon_record(&record_path) else {
        eprintln!("chan devserver: no self-managed daemon is running.");
        return Ok(());
    };
    if !is_record_live(&record) {
        let _ = std::fs::remove_file(&record_path);
        eprintln!("chan devserver: no self-managed daemon is running (cleared a stale pidfile).");
        return Ok(());
    }
    signal_terminate(record.pid);
    if wait_for_record_gone(&record_path, Duration::from_secs(5)) {
        eprintln!(
            "chan devserver: stopped the self-managed daemon (pid {}).",
            record.pid
        );
        Ok(())
    } else {
        anyhow::bail!(
            "chan devserver: the self-managed daemon (pid {}) did not stop within 5s.",
            record.pid
        )
    }
}

/// `--status`: report whether a self-managed daemon is running, from the pidfile.
pub fn status_devserver_chan(verbose: bool) -> Result<()> {
    let record_path = daemon_record_path();
    if verbose {
        print_daemon_paths(&daemon_lock_path(), &record_path);
    }
    match read_daemon_record(&record_path) {
        Some(r) if is_record_live(&r) => println!(
            "chan devserver (chan): running -- pid {}, bind {}, since {}",
            r.pid, r.addr, r.started_at
        ),
        Some(r) => println!(
            "chan devserver (chan): not running (stale pidfile for pid {}).",
            r.pid
        ),
        None => println!("chan devserver (chan): not running."),
    }
    Ok(())
}

/// Acquire the lock and serve in the foreground, holding the guard for the
/// server's lifetime. The guard's Drop releases the flock and removes the
/// pidfile on exit; the foreground server prints the bearer-token marker itself
/// on startup, so a reconnecting desktop scrapes it from this terminal.
async fn serve_as_daemon(guard: DaemonLock, addr: SocketAddr) -> Result<()> {
    eprintln!(
        "chan devserver: self-managed daemon running in the foreground (bind={addr}); \
         Ctrl-C to stop."
    );
    let result = crate::run_devserver_foreground(addr, None, true).await;
    drop(guard);
    result
}

/// `--force` / `--restart`: stop a running daemon (if any), then take the lock
/// and serve. The terminate happens BEFORE re-acquiring so the new server does
/// not race the old one for the port.
async fn take_over(lock_path: &Path, record_path: &Path, addr: SocketAddr) -> Result<()> {
    if let Some(record) = read_daemon_record(record_path) {
        if is_record_live(&record) {
            eprintln!(
                "chan devserver: stopping the running daemon (pid {}) to take over.",
                record.pid
            );
            signal_terminate(record.pid);
            wait_for_record_gone(record_path, Duration::from_secs(5));
        }
    }
    match DaemonLock::acquire(lock_path, record_path, &addr.to_string(), true).map_err(|e| {
        anyhow::anyhow!("re-acquiring the devserver daemon lock after takeover: {e}")
    })? {
        DaemonAcquire::Daemon(guard) => serve_as_daemon(guard, addr).await,
        DaemonAcquire::Running(r) => anyhow::bail!(
            "chan devserver: could not take over the running daemon (pid {}); it is still \
             holding the lock.",
            r.pid
        ),
    }
}

/// Block (bounded) until the pidfile is gone or names a dead pid -- the signal
/// that a daemon we asked to stop has released its lock + pidfile.
fn wait_for_record_gone(record_path: &Path, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        match read_daemon_record(record_path) {
            None => return true,
            Some(r) if !is_record_live(&r) => return true,
            Some(_) => {}
        }
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// `-v`: print the daemon subsystem + the files it touches.
fn print_daemon_paths(lock_path: &Path, record_path: &Path) {
    eprintln!("chan devserver: subsystem=chan (self-managed foreground daemon)");
    eprintln!("  pidfile: {}", record_path.display());
    eprintln!("  lock:    {}", lock_path.display());
    if let Ok(log) = crate::devserver_log_path() {
        eprintln!("  log:     {}", log.display());
    }
    eprintln!(
        "  config:  {}",
        chan_workspace::paths::config_dir()
            .join("devserver")
            .join("config.json")
            .display()
    );
}
