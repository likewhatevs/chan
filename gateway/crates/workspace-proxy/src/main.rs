use std::process::ExitCode;
use std::sync::Arc;

use chan_tunnel_server::{serve_tunnel_listener, Validator};
use tokio::sync::Notify;
use tracing_subscriber::EnvFilter;
use workspace_proxy::{
    config::Config,
    http,
    identity_validator::{CapturingValidator, IdentityValidator},
    registry::Registry,
    throttle_validator::ThrottlingValidator,
};

#[tokio::main]
async fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(false)
        .init();

    if let Err(e) = run().await {
        tracing::error!(error = ?e, "workspace-proxy-service exited with error");
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}

async fn run() -> anyhow::Result<()> {
    let cfg = Config::from_env()?;
    tracing::info!(
        public = %cfg.bind_addr,
        tunnel = %cfg.tunnel_bind_addr,
        apex = %cfg.apex_host,
        wildcard = %cfg.wildcard_suffix,
        identity = %cfg.identity_url,
        max_workspaces_per_user = cfg.max_workspaces_per_user,
        "starting workspace-proxy-service",
    );

    let registry = Registry::new();

    // Validator chain (outermost first):
    //   CapturingValidator   records (username, user_id) on success
    //                        so the proxy gate can resolve owner_id
    //                        without a profile round trip.
    //   ThrottlingValidator  per-token-fingerprint rate limit before
    //                        any round trip to identity-service.
    //   IdentityValidator    upstream PAT lookup.
    let identity =
        IdentityValidator::new(cfg.identity_url.clone(), cfg.identity_auth_token.clone())?;
    let throttled = ThrottlingValidator::new(identity);
    let validator: Arc<dyn Validator> =
        Arc::new(CapturingValidator::new(throttled, registry.clone()));

    let public_listener = tokio::net::TcpListener::bind(cfg.bind_addr).await?;
    let tunnel_listener = tokio::net::TcpListener::bind(cfg.tunnel_bind_addr).await?;

    // Single fan-out shutdown trigger: one OS-signal listener notifies
    // both the public axum server (graceful drain) and the tunnel
    // acceptor (cancellation). Splitting the listeners across two
    // tasks means a panic in either must abort its sibling so the
    // process actually exits and orchestration can restart it.
    let shutdown = Arc::new(Notify::new());
    {
        let s = shutdown.clone();
        tokio::spawn(async move {
            gateway_common::shutdown_signal().await;
            s.notify_waiters();
        });
    }

    let app = http::router(Arc::new(cfg.clone()), registry.clone());
    // into_make_service_with_connect_info populates the
    // ConnectInfo<SocketAddr> extension on every request so the
    // reverse proxy can read the peer IP and append it to
    // X-Forwarded-For.
    let public_shutdown = shutdown.clone();
    let mut public = tokio::spawn(async move {
        axum::serve(
            public_listener,
            app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
        )
        .with_graceful_shutdown(async move { public_shutdown.notified().await })
        .await
    });

    let tunnel_shutdown = shutdown.clone();
    let mut tunnel = {
        let validator = validator.clone();
        let tunnels = registry.tunnels();
        let max_workspaces = cfg.max_workspaces_per_user;
        tokio::spawn(async move {
            tokio::select! {
                r = serve_tunnel_listener(tunnel_listener, validator, tunnels, max_workspaces) => r,
                _ = tunnel_shutdown.notified() => {
                    tracing::info!("tunnel listener received shutdown");
                    Ok(())
                }
            }
        })
    };

    // Wait for whichever task exits first, log it, then trigger the
    // other to drain. A panic in either listener surfaces here instead
    // of being silently dropped with the sibling's JoinHandle.
    tokio::select! {
        r = &mut public => {
            match r {
                Ok(Ok(())) => tracing::info!("public listener exited cleanly"),
                Ok(Err(e)) => tracing::error!(error = ?e, "public listener errored"),
                Err(e) => tracing::error!(error = ?e, "public listener panicked"),
            }
            shutdown.notify_waiters();
            if let Err(e) = tunnel.await {
                tracing::warn!(error = ?e, "tunnel listener join error on shutdown");
            }
        }
        r = &mut tunnel => {
            match r {
                Ok(Ok(())) => tracing::info!("tunnel listener exited cleanly"),
                Ok(Err(e)) => tracing::error!(error = ?e, "tunnel listener errored"),
                Err(e) => tracing::error!(error = ?e, "tunnel listener panicked"),
            }
            shutdown.notify_waiters();
            if let Err(e) = public.await {
                tracing::warn!(error = ?e, "public listener join error on shutdown");
            }
        }
    }
    Ok(())
}
