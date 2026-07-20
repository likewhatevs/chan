use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use devserver_control_proto::{CanonicalOrigin, ProxyId, ServerFrame, TunnelRow};
use tokio::sync::{mpsc, oneshot, watch};
use tokio::time::{Instant, MissedTickBehavior};
use uuid::Uuid;

use crate::{
    ControllerState, Effect, ProxyView, SessionIncarnation, SessionKey, StateError, TunnelView,
};

const ACTOR_QUEUE_CAPACITY: usize = 1024;
const SESSION_QUEUE_CAPACITY: usize = 1024;
const TICK_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Clone)]
pub struct ControllerHandle {
    tx: mpsc::Sender<Command>,
    readiness_watch: watch::Receiver<bool>,
    tunnel_watch: watch::Receiver<Arc<Vec<TunnelView>>>,
    proxy_watch: watch::Receiver<Arc<Vec<ProxyView>>>,
}

pub struct ProxyControlSession {
    pub incarnation: SessionIncarnation,
    pub commands: mpsc::Receiver<ServerFrame>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MutationStatus {
    Applied,
    Resyncing,
}

enum Command {
    BeginSession {
        proxy_id: ProxyId,
        base_url: CanonicalOrigin,
        package_version: String,
        boot_id: Uuid,
        command_tx: mpsc::Sender<ServerFrame>,
        reply: oneshot::Sender<SessionIncarnation>,
    },
    AcceptSnapshot {
        proxy_id: ProxyId,
        incarnation: SessionIncarnation,
        base_generation: u64,
        rows: Vec<TunnelRow>,
        reply: StateReply,
    },
    TunnelUp {
        proxy_id: ProxyId,
        incarnation: SessionIncarnation,
        generation: u64,
        row: TunnelRow,
        reply: StateReply,
    },
    TunnelDown {
        proxy_id: ProxyId,
        incarnation: SessionIncarnation,
        generation: u64,
        registration_id: Uuid,
        reply: StateReply,
    },
    RequestAdmission {
        proxy_id: ProxyId,
        incarnation: SessionIncarnation,
        request_id: Uuid,
        registration_id: Uuid,
        user: String,
        devserver_id: String,
        reply: StateReply,
    },
    CancelAdmission {
        proxy_id: ProxyId,
        incarnation: SessionIncarnation,
        request_id: Uuid,
        registration_id: Uuid,
        reply: StateReply,
    },
    Pong {
        proxy_id: ProxyId,
        incarnation: SessionIncarnation,
        nonce: u64,
        reply: StateReply,
    },
    RecordActivity {
        proxy_id: ProxyId,
        incarnation: SessionIncarnation,
        reply: StateReply,
    },
    ReportResult {
        proxy_id: ProxyId,
        incarnation: SessionIncarnation,
        command_id: Uuid,
        killed: Vec<Uuid>,
        missing: Vec<Uuid>,
        failed: Vec<Uuid>,
        reply: StateReply,
    },
    Disconnect {
        proxy_id: ProxyId,
        incarnation: SessionIncarnation,
        reply: StateReply,
    },
    RequireResync {
        proxy_id: ProxyId,
        incarnation: SessionIncarnation,
        reply: StateReply,
    },
    Readiness {
        reply: oneshot::Sender<bool>,
    },
    Tunnels {
        reply: oneshot::Sender<Result<Vec<TunnelView>, StateError>>,
    },
    Proxies {
        reply: oneshot::Sender<Result<Vec<ProxyView>, StateError>>,
    },
}

type StateReply = oneshot::Sender<Result<MutationStatus, StateError>>;

pub fn spawn_controller(max_devservers_per_user: usize) -> ControllerHandle {
    spawn_controller_owned(max_devservers_per_user).0
}

pub fn spawn_controller_owned(
    max_devservers_per_user: usize,
) -> (ControllerHandle, tokio::task::JoinHandle<()>) {
    let (tx, mut rx) = mpsc::channel(ACTOR_QUEUE_CAPACITY);
    let (readiness_watch_tx, readiness_watch) = watch::channel(false);
    let (tunnel_watch_tx, tunnel_watch) = watch::channel(Arc::new(Vec::new()));
    let (proxy_watch_tx, proxy_watch) = watch::channel(Arc::new(Vec::new()));

    let task = tokio::spawn(async move {
        let mut state = ControllerState::new(max_devservers_per_user);
        let mut sessions = HashMap::new();
        let mut ticker = tokio::time::interval(TICK_INTERVAL);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

        loop {
            let effects = tokio::select! {
                _ = ticker.tick() => state.tick(Instant::now(), Utc::now()),
                command = rx.recv() => {
                    let Some(command) = command else {
                        break;
                    };
                    handle_command(command, &mut state, &mut sessions)
                }
            };
            apply_effects(&mut state, &mut sessions, effects);
            publish_watches(
                &state,
                &readiness_watch_tx,
                &tunnel_watch_tx,
                &proxy_watch_tx,
            );
        }
    });

    (
        ControllerHandle {
            tx,
            readiness_watch,
            tunnel_watch,
            proxy_watch,
        },
        task,
    )
}

fn handle_command(
    command: Command,
    state: &mut ControllerState,
    sessions: &mut HashMap<SessionKey, mpsc::Sender<ServerFrame>>,
) -> Vec<Effect> {
    let now = Instant::now();
    let wall_now = Utc::now();
    match command {
        Command::BeginSession {
            proxy_id,
            base_url,
            package_version,
            boot_id,
            command_tx,
            reply,
        } => {
            let proxy_id_text = proxy_id.as_str().to_string();
            let (incarnation, effects) =
                state.begin_session(proxy_id, base_url, package_version, boot_id, now, wall_now);
            sessions.insert(
                SessionKey {
                    proxy_id: proxy_id_text,
                    incarnation,
                },
                command_tx,
            );
            let _ = reply.send(incarnation);
            effects
        }
        Command::AcceptSnapshot {
            proxy_id,
            incarnation,
            base_generation,
            rows,
            reply,
        } => finish(
            reply,
            state.accept_snapshot(&proxy_id, incarnation, base_generation, rows, now, wall_now),
        ),
        Command::TunnelUp {
            proxy_id,
            incarnation,
            generation,
            row,
            reply,
        } => finish(
            reply,
            state.tunnel_up(&proxy_id, incarnation, generation, row, now, wall_now),
        ),
        Command::TunnelDown {
            proxy_id,
            incarnation,
            generation,
            registration_id,
            reply,
        } => finish(
            reply,
            state.tunnel_down(
                &proxy_id,
                incarnation,
                generation,
                registration_id,
                now,
                wall_now,
            ),
        ),
        Command::RequestAdmission {
            proxy_id,
            incarnation,
            request_id,
            registration_id,
            user,
            devserver_id,
            reply,
        } => finish(
            reply,
            state.request_admission(
                &proxy_id,
                incarnation,
                request_id,
                registration_id,
                user,
                devserver_id,
                now,
                wall_now,
            ),
        ),
        Command::CancelAdmission {
            proxy_id,
            incarnation,
            request_id,
            registration_id,
            reply,
        } => finish_unit(
            reply,
            state.cancel_admission(
                &proxy_id,
                incarnation,
                request_id,
                registration_id,
                now,
                wall_now,
            ),
        ),
        Command::Pong {
            proxy_id,
            incarnation,
            nonce,
            reply,
        } => finish_unit(
            reply,
            state.pong(&proxy_id, incarnation, nonce, now, wall_now),
        ),
        Command::RecordActivity {
            proxy_id,
            incarnation,
            reply,
        } => finish_unit(
            reply,
            state.record_activity(&proxy_id, incarnation, now, wall_now),
        ),
        Command::ReportResult {
            proxy_id,
            incarnation,
            command_id,
            killed,
            missing,
            failed,
            reply,
        } => finish(
            reply,
            state.command_result(
                &proxy_id,
                incarnation,
                command_id,
                killed,
                missing,
                failed,
                now,
                wall_now,
            ),
        ),
        Command::Disconnect {
            proxy_id,
            incarnation,
            reply,
        } => {
            sessions.remove(&SessionKey {
                proxy_id: proxy_id.as_str().to_string(),
                incarnation,
            });
            finish(reply, state.disconnect(&proxy_id, incarnation))
        }
        Command::RequireResync {
            proxy_id,
            incarnation,
            reply,
        } => finish(reply, state.require_resync(&proxy_id, incarnation)),
        Command::Readiness { reply } => {
            let _ = reply.send(state.is_ready());
            Vec::new()
        }
        Command::Tunnels { reply } => {
            let _ = reply.send(state.read_tunnels());
            Vec::new()
        }
        Command::Proxies { reply } => {
            let _ = reply.send(state.read_proxies());
            Vec::new()
        }
    }
}

fn finish(reply: StateReply, result: Result<Vec<Effect>, StateError>) -> Vec<Effect> {
    match result {
        Ok(effects) => {
            let status = if effects.iter().any(|effect| {
                matches!(
                    effect,
                    Effect::Send {
                        frame: ServerFrame::ResyncRequired { .. },
                        ..
                    }
                )
            }) {
                MutationStatus::Resyncing
            } else {
                MutationStatus::Applied
            };
            let _ = reply.send(Ok(status));
            effects
        }
        Err(error) => {
            let _ = reply.send(Err(error));
            Vec::new()
        }
    }
}

fn finish_unit(reply: StateReply, result: Result<(), StateError>) -> Vec<Effect> {
    let _ = reply.send(result.map(|()| MutationStatus::Applied));
    Vec::new()
}

fn apply_effects(
    state: &mut ControllerState,
    sessions: &mut HashMap<SessionKey, mpsc::Sender<ServerFrame>>,
    effects: Vec<Effect>,
) {
    let mut effects: VecDeque<_> = effects.into();
    let mut failed = HashSet::new();
    while !effects.is_empty() || !failed.is_empty() {
        while let Some(effect) = effects.pop_front() {
            match effect {
                Effect::Send { session, frame } => {
                    let send_failed = match sessions.get(&session) {
                        Some(sender) => sender.try_send(frame).is_err(),
                        None => true,
                    };
                    if send_failed {
                        failed.insert(session);
                    }
                }
                Effect::Retire { session, reason } => {
                    if let Some(sender) = sessions.remove(&session) {
                        let _ = sender.try_send(ServerFrame::Shutdown { reason });
                    }
                }
            }
        }

        for session in failed.drain() {
            sessions.remove(&session);
            let proxy_id =
                ProxyId::parse(&session.proxy_id).expect("session proxy id was validated");
            if let Ok(more) = state.disconnect(&proxy_id, session.incarnation) {
                effects.extend(more);
            }
        }
    }
}

fn publish_watches(
    state: &ControllerState,
    readiness_watch: &watch::Sender<bool>,
    tunnel_watch: &watch::Sender<Arc<Vec<TunnelView>>>,
    proxy_watch: &watch::Sender<Arc<Vec<ProxyView>>>,
) {
    readiness_watch.send_if_modified(|ready| {
        let next = state.is_ready();
        if *ready == next {
            false
        } else {
            *ready = next;
            true
        }
    });
    publish(tunnel_watch, Arc::new(state.tunnel_views()));
    publish(proxy_watch, Arc::new(state.proxy_views()));
}

fn publish<T: PartialEq>(sender: &watch::Sender<Arc<Vec<T>>>, next: Arc<Vec<T>>) {
    sender.send_if_modified(|current| {
        if current.as_ref() == next.as_ref() {
            false
        } else {
            *current = next;
            true
        }
    });
}

impl ControllerHandle {
    pub async fn begin_session(
        &self,
        proxy_id: ProxyId,
        base_url: CanonicalOrigin,
        package_version: String,
        boot_id: Uuid,
    ) -> Result<ProxyControlSession, ActorError> {
        let (command_tx, commands) = mpsc::channel(SESSION_QUEUE_CAPACITY);
        let incarnation = self
            .request(|reply| Command::BeginSession {
                proxy_id,
                base_url,
                package_version,
                boot_id,
                command_tx,
                reply,
            })
            .await?;
        Ok(ProxyControlSession {
            incarnation,
            commands,
        })
    }

    pub async fn accept_snapshot(
        &self,
        proxy_id: ProxyId,
        incarnation: SessionIncarnation,
        base_generation: u64,
        rows: Vec<TunnelRow>,
    ) -> Result<MutationStatus, ActorError> {
        self.state_request(|reply| Command::AcceptSnapshot {
            proxy_id,
            incarnation,
            base_generation,
            rows,
            reply,
        })
        .await
    }

    pub async fn tunnel_up(
        &self,
        proxy_id: ProxyId,
        incarnation: SessionIncarnation,
        generation: u64,
        row: TunnelRow,
    ) -> Result<MutationStatus, ActorError> {
        self.state_request(|reply| Command::TunnelUp {
            proxy_id,
            incarnation,
            generation,
            row,
            reply,
        })
        .await
    }

    pub async fn tunnel_down(
        &self,
        proxy_id: ProxyId,
        incarnation: SessionIncarnation,
        generation: u64,
        registration_id: Uuid,
    ) -> Result<MutationStatus, ActorError> {
        self.state_request(|reply| Command::TunnelDown {
            proxy_id,
            incarnation,
            generation,
            registration_id,
            reply,
        })
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn request_admission(
        &self,
        proxy_id: ProxyId,
        incarnation: SessionIncarnation,
        request_id: Uuid,
        registration_id: Uuid,
        user: String,
        devserver_id: String,
    ) -> Result<MutationStatus, ActorError> {
        self.state_request(|reply| Command::RequestAdmission {
            proxy_id,
            incarnation,
            request_id,
            registration_id,
            user,
            devserver_id,
            reply,
        })
        .await
    }

    pub async fn cancel_admission(
        &self,
        proxy_id: ProxyId,
        incarnation: SessionIncarnation,
        request_id: Uuid,
        registration_id: Uuid,
    ) -> Result<MutationStatus, ActorError> {
        self.state_request(|reply| Command::CancelAdmission {
            proxy_id,
            incarnation,
            request_id,
            registration_id,
            reply,
        })
        .await
    }

    pub async fn pong(
        &self,
        proxy_id: ProxyId,
        incarnation: SessionIncarnation,
        nonce: u64,
    ) -> Result<MutationStatus, ActorError> {
        self.state_request(|reply| Command::Pong {
            proxy_id,
            incarnation,
            nonce,
            reply,
        })
        .await
    }

    pub async fn record_activity(
        &self,
        proxy_id: ProxyId,
        incarnation: SessionIncarnation,
    ) -> Result<MutationStatus, ActorError> {
        self.state_request(|reply| Command::RecordActivity {
            proxy_id,
            incarnation,
            reply,
        })
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn command_result(
        &self,
        proxy_id: ProxyId,
        incarnation: SessionIncarnation,
        command_id: Uuid,
        killed: Vec<Uuid>,
        missing: Vec<Uuid>,
        failed: Vec<Uuid>,
    ) -> Result<MutationStatus, ActorError> {
        self.state_request(|reply| Command::ReportResult {
            proxy_id,
            incarnation,
            command_id,
            killed,
            missing,
            failed,
            reply,
        })
        .await
    }

    pub async fn disconnect(
        &self,
        proxy_id: ProxyId,
        incarnation: SessionIncarnation,
    ) -> Result<MutationStatus, ActorError> {
        self.state_request(|reply| Command::Disconnect {
            proxy_id,
            incarnation,
            reply,
        })
        .await
    }

    pub async fn require_resync(
        &self,
        proxy_id: ProxyId,
        incarnation: SessionIncarnation,
    ) -> Result<MutationStatus, ActorError> {
        self.state_request(|reply| Command::RequireResync {
            proxy_id,
            incarnation,
            reply,
        })
        .await
    }

    pub async fn is_ready(&self) -> Result<bool, ActorError> {
        self.request(|reply| Command::Readiness { reply }).await
    }

    pub async fn tunnels(&self) -> Result<Vec<TunnelView>, ActorError> {
        self.request(|reply| Command::Tunnels { reply })
            .await?
            .map_err(ActorError::State)
    }

    pub async fn proxies(&self) -> Result<Vec<ProxyView>, ActorError> {
        self.request(|reply| Command::Proxies { reply })
            .await?
            .map_err(ActorError::State)
    }

    pub fn watch_tunnels(&self) -> watch::Receiver<Arc<Vec<TunnelView>>> {
        self.tunnel_watch.clone()
    }

    pub fn watch_readiness(&self) -> watch::Receiver<bool> {
        self.readiness_watch.clone()
    }

    pub fn watch_proxies(&self) -> watch::Receiver<Arc<Vec<ProxyView>>> {
        self.proxy_watch.clone()
    }

    async fn state_request(
        &self,
        build: impl FnOnce(StateReply) -> Command,
    ) -> Result<MutationStatus, ActorError> {
        self.request(build).await?.map_err(ActorError::State)
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
    use crate::SESSION_DEAD_AFTER;

    fn proxy() -> ProxyId {
        ProxyId::parse("p1").unwrap()
    }

    async fn keep_alive_until_ready(actor: &ControllerHandle, session: &mut ProxyControlSession) {
        for _ in 0..6 {
            tokio::time::advance(crate::HEARTBEAT_INTERVAL).await;
            let nonce = loop {
                if let ServerFrame::Ping { nonce } = session.commands.recv().await.unwrap() {
                    break nonce;
                }
            };
            actor
                .pong(proxy(), session.incarnation, nonce)
                .await
                .unwrap();
        }
        tokio::task::yield_now().await;
    }

    #[tokio::test(start_paused = true)]
    async fn actor_holds_reads_until_convergence_and_expires_silent_sessions() {
        let actor = spawn_controller(100);
        let mut proxy_watch = actor.watch_proxies();
        let mut session = actor
            .begin_session(
                proxy(),
                CanonicalOrigin::parse("https://p1.proxy.example.test").unwrap(),
                env!("CARGO_PKG_VERSION").into(),
                Uuid::new_v4(),
            )
            .await
            .unwrap();
        actor
            .accept_snapshot(proxy(), session.incarnation, 0, Vec::new())
            .await
            .unwrap();
        assert!(matches!(
            actor.proxies().await,
            Err(ActorError::State(StateError::NotReady))
        ));

        keep_alive_until_ready(&actor, &mut session).await;
        assert!(actor.is_ready().await.unwrap());
        assert_eq!(actor.proxies().await.unwrap().len(), 1);
        while let Some(frame) = session.commands.recv().await {
            if frame == ServerFrame::FleetReady {
                break;
            }
        }

        tokio::time::advance(SESSION_DEAD_AFTER).await;
        tokio::task::yield_now().await;
        assert!(!actor.is_ready().await.unwrap());
        assert!(matches!(
            actor.tunnels().await,
            Err(ActorError::State(StateError::NotReady))
        ));
        proxy_watch.changed().await.unwrap();
        assert!(proxy_watch.borrow().is_empty());
    }

    #[tokio::test(start_paused = true)]
    async fn actor_publishes_complete_watch_snapshots() {
        let actor = spawn_controller(100);
        let mut proxies = actor.watch_proxies();
        let mut tunnels = actor.watch_tunnels();
        let mut session = actor
            .begin_session(
                proxy(),
                CanonicalOrigin::parse("https://p1.proxy.example.test").unwrap(),
                env!("CARGO_PKG_VERSION").into(),
                Uuid::new_v4(),
            )
            .await
            .unwrap();
        proxies.changed().await.unwrap();
        assert_eq!(proxies.borrow().len(), 1);
        actor
            .accept_snapshot(proxy(), session.incarnation, 0, Vec::new())
            .await
            .unwrap();
        keep_alive_until_ready(&actor, &mut session).await;

        let registration_id = Uuid::new_v4();
        actor
            .request_admission(
                proxy(),
                session.incarnation,
                Uuid::new_v4(),
                registration_id,
                "alice".into(),
                "one".into(),
            )
            .await
            .unwrap();
        actor
            .tunnel_up(
                proxy(),
                session.incarnation,
                1,
                TunnelRow {
                    registration_id,
                    user: "alice".into(),
                    devserver_id: "one".into(),
                    peer_addr: None,
                    connected_at: Utc::now(),
                },
            )
            .await
            .unwrap();
        tunnels.changed().await.unwrap();
        assert_eq!(tunnels.borrow().len(), 1);
        assert_eq!(tunnels.borrow()[0].proxy_id, "p1");
    }

    #[test]
    fn full_session_command_queue_disconnects_instead_of_dropping_state() {
        let now = Instant::now();
        let mut state = ControllerState::new(100);
        let proxy_id = proxy();
        let (incarnation, _) = state.begin_session(
            proxy_id.clone(),
            CanonicalOrigin::parse("https://p1.proxy.example.test").unwrap(),
            env!("CARGO_PKG_VERSION").into(),
            Uuid::new_v4(),
            now,
            Utc::now(),
        );
        let key = SessionKey {
            proxy_id: proxy_id.as_str().to_string(),
            incarnation,
        };
        let (sender, _receiver) = mpsc::channel(1);
        sender.try_send(ServerFrame::Ping { nonce: 1 }).unwrap();
        let mut sessions = HashMap::from([(key.clone(), sender)]);
        apply_effects(
            &mut state,
            &mut sessions,
            vec![Effect::Send {
                session: key,
                frame: ServerFrame::Ping { nonce: 2 },
            }],
        );
        assert!(sessions.is_empty());
        assert!(state.proxy_views().is_empty());
    }
}
