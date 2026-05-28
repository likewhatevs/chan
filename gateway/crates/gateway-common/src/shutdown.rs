//! Shared graceful-shutdown signal future.
//!
//! Completes on the first of SIGTERM (Unix) or Ctrl-C. Used by every
//! gateway service to gate `axum::serve(...).with_graceful_shutdown(...)`
//! and to cancel auxiliary listeners (workspace-proxy's tunnel acceptor).
//!
//! Why not just `tokio::signal::ctrl_c()`: production processes are
//! terminated with SIGTERM by orchestration (systemd, kubernetes). A
//! ctrl-c-only handler leaves in-flight requests racing the kill window.

pub async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(e) = tokio::signal::ctrl_c().await {
            tracing::warn!(error = ?e, "ctrl-c handler install failed");
            std::future::pending::<()>().await
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut s) => {
                s.recv().await;
            }
            Err(e) => {
                tracing::warn!(error = ?e, "SIGTERM handler install failed");
                std::future::pending::<()>().await
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => tracing::info!("ctrl-c received, shutting down"),
        _ = terminate => tracing::info!("SIGTERM received, shutting down"),
    }
}
