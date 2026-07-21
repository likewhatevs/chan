use std::process::ExitCode;

use anyhow::Context;
use devserver_control::{admin_router, serve_control_listener, spawn_controller_owned, Config};
use tokio::sync::watch;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(false)
        .init();

    if let Err(error) = run().await {
        tracing::error!(error = ?error, "devserver-control-service exited with error");
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}

async fn run() -> anyhow::Result<()> {
    let config = Config::from_env()?;
    tracing::info!(
        admin = %config.bind_addr,
        proxy = %config.proxy_bind_addr,
        max_devservers_per_user = config.max_devservers_per_user,
        "starting devserver-control-service",
    );

    let admin_listener = tokio::net::TcpListener::bind(config.bind_addr)
        .await
        .context("bind controller admin listener")?;
    let proxy_listener = tokio::net::TcpListener::bind(config.proxy_bind_addr)
        .await
        .context("bind controller proxy listener")?;
    let (controller, mut actor_task) = spawn_controller_owned(config.max_devservers_per_user);
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    let admin_app = admin_router(controller.clone(), config.admin_credentials.clone());
    let mut admin_task = {
        let shutdown = shutdown_rx.clone();
        tokio::spawn(async move {
            axum::serve(admin_listener, admin_app)
                .with_graceful_shutdown(wait_for_shutdown(shutdown))
                .await
        })
    };
    let mut proxy_task = {
        let shutdown = shutdown_rx.clone();
        let controller = controller.clone();
        tokio::spawn(async move {
            serve_control_listener(
                proxy_listener,
                controller,
                config.proxy_credentials,
                config.admission_lease_verifier,
                config.proxy_base_url_template,
                shutdown,
            )
            .await
        })
    };

    enum FirstExit {
        Signal,
        Admin(Result<std::io::Result<()>, tokio::task::JoinError>),
        Proxy(Result<std::io::Result<()>, tokio::task::JoinError>),
        Actor(Result<(), tokio::task::JoinError>),
    }

    let first = tokio::select! {
        _ = gateway_common::shutdown_signal() => FirstExit::Signal,
        result = &mut admin_task => FirstExit::Admin(result),
        result = &mut proxy_task => FirstExit::Proxy(result),
        result = &mut actor_task => FirstExit::Actor(result),
    };
    let _ = shutdown_tx.send(true);

    match first {
        FirstExit::Signal => {
            admin_task
                .await
                .context("join controller admin listener")??;
            proxy_task
                .await
                .context("join controller proxy listener")??;
            drop(controller);
            actor_task.await.context("join controller actor")?;
            Ok(())
        }
        FirstExit::Admin(result) => {
            let _ = proxy_task.await;
            drop(controller);
            let _ = actor_task.await;
            listener_exit("admin", result)
        }
        FirstExit::Proxy(result) => {
            let _ = admin_task.await;
            drop(controller);
            let _ = actor_task.await;
            listener_exit("proxy", result)
        }
        FirstExit::Actor(result) => {
            let _ = admin_task.await;
            let _ = proxy_task.await;
            match result {
                Ok(()) => anyhow::bail!("controller actor exited unexpectedly"),
                Err(error) => Err(error).context("controller actor failed"),
            }
        }
    }
}

async fn wait_for_shutdown(mut shutdown: watch::Receiver<bool>) {
    while !*shutdown.borrow() {
        if shutdown.changed().await.is_err() {
            return;
        }
    }
}

fn listener_exit(
    name: &str,
    result: Result<std::io::Result<()>, tokio::task::JoinError>,
) -> anyhow::Result<()> {
    match result {
        Ok(Ok(())) => anyhow::bail!("controller {name} listener exited unexpectedly"),
        Ok(Err(error)) => Err(error).with_context(|| format!("controller {name} listener failed")),
        Err(error) => Err(error).with_context(|| format!("controller {name} listener task failed")),
    }
}
