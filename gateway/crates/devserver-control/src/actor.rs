use devserver_control_proto::{CanonicalOrigin, ProxyId, TunnelRow};
use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;

use crate::{ControllerState, KillTarget, ProxyView, StateError, TunnelView};

const QUEUE_CAPACITY: usize = 1024;

#[derive(Clone)]
pub struct ControllerHandle {
    tx: mpsc::Sender<Command>,
}

enum Command {
    BeginSession {
        proxy_id: ProxyId,
        base_url: CanonicalOrigin,
        package_version: String,
        boot_id: Uuid,
        reply: oneshot::Sender<()>,
    },
    AcceptSnapshot {
        proxy_id: ProxyId,
        base_generation: u64,
        rows: Vec<TunnelRow>,
        reply: oneshot::Sender<Result<Vec<KillTarget>, StateError>>,
    },
    Disconnect {
        proxy_id: String,
    },
    Readiness {
        reply: oneshot::Sender<bool>,
    },
    Tunnels {
        reply: oneshot::Sender<Vec<TunnelView>>,
    },
    Proxies {
        reply: oneshot::Sender<Vec<ProxyView>>,
    },
}

pub fn spawn_controller(max_devservers_per_user: usize) -> ControllerHandle {
    let (tx, mut rx) = mpsc::channel(QUEUE_CAPACITY);
    tokio::spawn(async move {
        let mut state = ControllerState::new(max_devservers_per_user);
        while let Some(command) = rx.recv().await {
            match command {
                Command::BeginSession {
                    proxy_id,
                    base_url,
                    package_version,
                    boot_id,
                    reply,
                } => {
                    state.begin_session(proxy_id, base_url, package_version, boot_id);
                    let _ = reply.send(());
                }
                Command::AcceptSnapshot {
                    proxy_id,
                    base_generation,
                    rows,
                    reply,
                } => {
                    let _ = reply.send(state.accept_snapshot(&proxy_id, base_generation, rows));
                }
                Command::Disconnect { proxy_id } => state.disconnect(&proxy_id),
                Command::Readiness { reply } => {
                    let _ = reply.send(state.is_ready());
                }
                Command::Tunnels { reply } => {
                    let _ = reply.send(state.tunnel_views());
                }
                Command::Proxies { reply } => {
                    let _ = reply.send(state.proxy_views());
                }
            }
        }
    });
    ControllerHandle { tx }
}

impl ControllerHandle {
    pub async fn begin_session(
        &self,
        proxy_id: ProxyId,
        base_url: CanonicalOrigin,
        package_version: String,
        boot_id: Uuid,
    ) -> Result<(), ActorError> {
        let (reply, wait) = oneshot::channel();
        self.tx
            .send(Command::BeginSession {
                proxy_id,
                base_url,
                package_version,
                boot_id,
                reply,
            })
            .await
            .map_err(|_| ActorError::Stopped)?;
        wait.await.map_err(|_| ActorError::Stopped)
    }

    pub async fn accept_snapshot(
        &self,
        proxy_id: ProxyId,
        base_generation: u64,
        rows: Vec<TunnelRow>,
    ) -> Result<Vec<KillTarget>, ActorError> {
        let (reply, wait) = oneshot::channel();
        self.tx
            .send(Command::AcceptSnapshot {
                proxy_id,
                base_generation,
                rows,
                reply,
            })
            .await
            .map_err(|_| ActorError::Stopped)?;
        wait.await
            .map_err(|_| ActorError::Stopped)?
            .map_err(ActorError::State)
    }

    pub async fn disconnect(&self, proxy_id: String) -> Result<(), ActorError> {
        self.tx
            .send(Command::Disconnect { proxy_id })
            .await
            .map_err(|_| ActorError::Stopped)
    }

    pub async fn is_ready(&self) -> Result<bool, ActorError> {
        self.request(|reply| Command::Readiness { reply }).await
    }

    pub async fn tunnels(&self) -> Result<Vec<TunnelView>, ActorError> {
        self.request(|reply| Command::Tunnels { reply }).await
    }

    pub async fn proxies(&self) -> Result<Vec<ProxyView>, ActorError> {
        self.request(|reply| Command::Proxies { reply }).await
    }

    async fn request<T>(
        &self,
        build: impl FnOnce(oneshot::Sender<T>) -> Command,
    ) -> Result<T, ActorError> {
        let (reply, wait) = oneshot::channel();
        self.tx
            .send(build(reply))
            .await
            .map_err(|_| ActorError::Stopped)?;
        wait.await.map_err(|_| ActorError::Stopped)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ActorError {
    #[error("controller actor stopped")]
    Stopped,
    #[error(transparent)]
    State(#[from] StateError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn actor_serializes_snapshot_visibility() {
        let actor = spawn_controller(100);
        let proxy_id = ProxyId::parse("p1").unwrap();
        actor
            .begin_session(
                proxy_id.clone(),
                CanonicalOrigin::parse("https://p1.proxy.example.test").unwrap(),
                "1".into(),
                Uuid::new_v4(),
            )
            .await
            .unwrap();
        assert!(!actor.is_ready().await.unwrap());
        actor
            .accept_snapshot(proxy_id, 0, Vec::new())
            .await
            .unwrap();
        assert!(actor.is_ready().await.unwrap());
        assert_eq!(actor.proxies().await.unwrap().len(), 1);
    }
}
