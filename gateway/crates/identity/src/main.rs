use std::process::ExitCode;
use std::sync::Arc;

use identity::{api_tokens::ApiTokenService, config::Config, http, token_throttle::TokenThrottle};
use sqlx::postgres::PgPoolOptions;
use tower_sessions_sqlx_store::PostgresStore;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(false)
        .init();

    if let Err(e) = run().await {
        tracing::error!(error = ?e, "identity-service exited with error");
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}

async fn run() -> anyhow::Result<()> {
    let cfg = Config::from_env()?;
    tracing::info!(bind = %cfg.bind_addr, base = %cfg.base_url, "starting identity-service");

    // Keep the cap low so a shared Postgres (dev VM, lab cluster)
    // doesn't run out of non-superuser slots when several services
    // are running side by side.
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .acquire_timeout(std::time::Duration::from_secs(5))
        .connect(&cfg.database_url)
        .await?;

    sqlx::migrate!("../../migrations").run(&pool).await?;

    let store = PostgresStore::new(pool.clone());
    store.migrate().await?;

    let api_tokens = ApiTokenService::new(pool);
    let token_throttle = TokenThrottle::new();

    let app = http::router(Arc::new(cfg.clone()), store, api_tokens, token_throttle);
    let listener = tokio::net::TcpListener::bind(cfg.bind_addr).await?;
    // ConnectInfo populates the peer-address extension some axum
    // layers expect. Nothing in identity-service requires it today
    // (the validate path throttles per token fingerprint, not per
    // IP), but the extension is cheap to provide.
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(gateway_common::shutdown_signal())
    .await?;
    Ok(())
}
