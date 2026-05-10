//! Process-wide shutdown signals + idle-timeout watcher.
//!
//! The serve loop wants to react to SIGINT/SIGTERM (or Ctrl-C on Windows)
//! and to an optional idle-timeout window. Both feed a single
//! `tokio::sync::watch::Sender<bool>` that axum's `with_graceful_shutdown`
//! awaits; whichever fires first wins. The idle watcher only spawns when
//! `ServeConfig::idle_timeout` is set.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Current wall-clock unix timestamp in seconds. Saturates at 0 on
/// the impossible-but-cheap-to-handle case where the system clock
/// is set before 1970.
pub fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Print a Dense1x2 QR for `url` to stderr when stderr is a TTY.
/// Suppressed under redirection so the banner stays grep-friendly
/// in logs. Encoder failure (URL too long for ECC H) is silent: the
/// preceding plain-text URL line is still authoritative.
pub fn print_qr_if_tty(url: &str) {
    use std::io::IsTerminal;
    if !std::io::stderr().is_terminal() {
        return;
    }
    if let Some(s) = crate::qr::render(url) {
        eprintln!("\n{s}");
    }
}

/// Spawn the idle-timeout watcher. Compares `last_activity` against
/// `now` on each tick; on a window expiry, signals shutdown.
///
/// Tick interval is `min(timeout / 4, 30s)` so short timeouts (the
/// systemd socket-activation case) get tight precision while long
/// timeouts don't burn CPU on a hot loop.
pub fn spawn_idle_watcher(
    timeout: Duration,
    last_activity: Arc<AtomicU64>,
    signal_tx: Arc<tokio::sync::watch::Sender<bool>>,
) {
    tokio::spawn(async move {
        let tick = std::cmp::min(timeout / 4, Duration::from_secs(30)).max(Duration::from_secs(1));
        let timeout_secs = timeout.as_secs().max(1);
        loop {
            tokio::time::sleep(tick).await;
            let last = last_activity.load(Ordering::Relaxed);
            let now = now_unix_secs();
            if now.saturating_sub(last) >= timeout_secs {
                eprintln!("chan: idle for {timeout_secs}s; shutting down");
                let _ = signal_tx.send(true);
                return;
            }
        }
    });
}

/// Spawn a SIGINT / SIGTERM (unix) or Ctrl-C (windows) watcher that
/// signals shutdown when the first signal arrives. Subsequent signals
/// are ignored; the runtime cleans up on return from `serve`.
pub fn spawn_signal_watcher(signal_tx: Arc<tokio::sync::watch::Sender<bool>>) {
    tokio::spawn(async move {
        wait_for_signal().await;
        eprintln!("chan: signal received; shutting down");
        let _ = signal_tx.send(true);
    });
}

#[cfg(unix)]
async fn wait_for_signal() {
    use tokio::signal::unix::{signal, SignalKind};
    let mut sigterm = match signal(SignalKind::terminate()) {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("failed to install SIGTERM handler: {e}");
            // Fall back to SIGINT only.
            let _ = tokio::signal::ctrl_c().await;
            return;
        }
    };
    let ctrl_c = tokio::signal::ctrl_c();
    tokio::select! {
        _ = sigterm.recv() => {}
        _ = ctrl_c => {}
    }
}

#[cfg(not(unix))]
async fn wait_for_signal() {
    let _ = tokio::signal::ctrl_c().await;
}
