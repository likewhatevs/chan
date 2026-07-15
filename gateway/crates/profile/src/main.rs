use std::process::ExitCode;

use profile::{config::Config, db, http};
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
        tracing::error!(error = ?e, "profile-service exited with error");
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}

async fn run() -> anyhow::Result<()> {
    let cfg = Config::from_env()?;
    tracing::info!(bind = %cfg.bind_addr, "starting profile-service");

    let pool = db::connect(&cfg.database_url).await?;
    db::migrate(&pool).await?;

    // Devserver registry sweeper: needs the devserver-proxy admin
    // client (live-tunnel snapshot source) AND a nonzero retention.
    // Absent either, say so once and don't spawn.
    match (cfg.workspace_admin.clone(), cfg.devserver_retention) {
        (Some(admin), Some(retention)) => {
            tracing::info!(
                retention_secs = retention.as_secs(),
                "devserver registry sweeper enabled"
            );
            tokio::spawn(profile::sweeper::run(pool.clone(), admin, retention));
        }
        _ => tracing::info!(
            "devserver registry sweeper disabled (needs DEVSERVER_ADMIN_TOKEN/URL and a \
             nonzero DEVSERVER_RETENTION_MINUTES)"
        ),
    }

    let app = http::router(http::AppState {
        pool,
        auth_token: cfg.auth_token.clone(),
        admin_token: cfg.admin_token.clone(),
        workspace_admin: cfg.workspace_admin.clone(),
    });

    let listener = tokio::net::TcpListener::bind(cfg.bind_addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(gateway_common::shutdown_signal())
        .await?;
    Ok(())
}
