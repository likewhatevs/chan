use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use axum::http::{header, Method, Request, StatusCode};
use chan_tunnel_proto::H2Duplex;
use chan_tunnel_server::{
    RegistrationAdmission, RegistrationPermit, RegistryEvent, ServerError, TunnelInfo,
};
use devserver_control_proto::{
    read_frame, write_frame, AdmissionDecision, ClientFrame, ServerFrame, TunnelRow, CONNECT_PATH,
    CONTENT_TYPE, MAX_SNAPSHOT_CHUNK_ROWS, PROTOCOL_VERSION,
};
use rand::Rng;
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot, watch};
use tokio::time::Instant;
use uuid::Uuid;

use crate::{registry::Registry, Config};

const REQUEST_QUEUE_CAPACITY: usize = 1024;
const SERVER_FRAME_QUEUE_CAPACITY: usize = 1024;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const SNAPSHOT_TIMEOUT: Duration = Duration::from_secs(30);
const CONTROLLER_DEAD_AFTER: Duration = Duration::from_secs(15);
const GRACE_PERIOD: Duration = Duration::from_secs(30);
const BACKOFF_MIN: Duration = Duration::from_millis(500);
const BACKOFF_MAX: Duration = Duration::from_secs(10);

pub struct ControlRuntime {
    pub admission: Arc<dyn RegistrationAdmission>,
    pub readiness: watch::Receiver<bool>,
    pub task: tokio::task::JoinHandle<anyhow::Result<()>>,
}

struct AbortOnDropTask(Option<tokio::task::JoinHandle<()>>);

impl AbortOnDropTask {
    fn new(task: tokio::task::JoinHandle<()>) -> Self {
        Self(Some(task))
    }

    async fn cancel(mut self) {
        if let Some(task) = self.0.take() {
            task.abort();
            let _ = task.await;
        }
    }
}

impl Drop for AbortOnDropTask {
    fn drop(&mut self) {
        if let Some(task) = &self.0 {
            task.abort();
        }
    }
}

pub fn spawn_control_supervisor(
    config: Arc<Config>,
    registry: Registry,
    shutdown: watch::Receiver<bool>,
) -> ControlRuntime {
    let (requests_tx, requests_rx) = mpsc::channel(REQUEST_QUEUE_CAPACITY);
    let (readiness_tx, readiness) = watch::channel(false);
    let admission_epoch = Arc::new(AtomicU64::new(1));
    let admission: Arc<dyn RegistrationAdmission> = Arc::new(ControlAdmission {
        requests: requests_tx,
        readiness: readiness.clone(),
        admission_epoch: admission_epoch.clone(),
    });
    let task = tokio::spawn(async move {
        supervise(
            config,
            registry,
            requests_rx,
            readiness_tx,
            admission_epoch,
            shutdown,
        )
        .await
    });
    ControlRuntime {
        admission,
        readiness,
        task,
    }
}

#[derive(Clone)]
struct ControlAdmission {
    requests: mpsc::Sender<LocalRequest>,
    readiness: watch::Receiver<bool>,
    admission_epoch: Arc<AtomicU64>,
}

#[async_trait]
impl RegistrationAdmission for ControlAdmission {
    async fn admit(
        &self,
        _hello: &chan_tunnel_proto::Hello,
        validated: &chan_tunnel_server::Validated,
    ) -> Result<RegistrationPermit, ServerError> {
        if !*self.readiness.borrow() {
            return Err(ServerError::ControlUnavailable);
        }
        let admission_epoch = self.admission_epoch.load(Ordering::Acquire);
        if !*self.readiness.borrow() {
            return Err(ServerError::ControlUnavailable);
        }
        let request_id = Uuid::new_v4();
        let registration_id = Uuid::new_v4();
        let (reply, wait) = oneshot::channel();
        self.requests
            .send(LocalRequest::Admit {
                request_id,
                registration_id,
                user: validated.username.clone(),
                devserver_id: validated.devserver_id.clone(),
                admission_epoch,
                reply,
            })
            .await
            .map_err(|_| ServerError::ControlUnavailable)?;
        let mut cancel = CancelOnDrop {
            requests: self.requests.clone(),
            request_id,
            registration_id,
            armed: true,
        };
        let result = wait.await.map_err(|_| ServerError::ControlUnavailable)?;
        if !*self.readiness.borrow()
            || self.admission_epoch.load(Ordering::Acquire) != admission_epoch
        {
            return Err(ServerError::ControlUnavailable);
        }
        cancel.armed = false;
        result
    }

    fn permit_is_current(&self, permit: RegistrationPermit) -> bool {
        *self.readiness.borrow()
            && self.admission_epoch.load(Ordering::Acquire) == permit.admission_epoch
    }

    async fn cancel(&self, permit: RegistrationPermit) {
        let _ = self
            .requests
            .send(LocalRequest::Cancel {
                request_id: permit.request_id,
                registration_id: permit.registration_id,
            })
            .await;
    }
}

struct CancelOnDrop {
    requests: mpsc::Sender<LocalRequest>,
    request_id: Uuid,
    registration_id: Uuid,
    armed: bool,
}

impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        if self.armed {
            let _ = self.requests.try_send(LocalRequest::Cancel {
                request_id: self.request_id,
                registration_id: self.registration_id,
            });
        }
    }
}

enum LocalRequest {
    Admit {
        request_id: Uuid,
        registration_id: Uuid,
        user: String,
        devserver_id: String,
        admission_epoch: u64,
        reply: oneshot::Sender<Result<RegistrationPermit, ServerError>>,
    },
    Cancel {
        request_id: Uuid,
        registration_id: Uuid,
    },
}

struct PendingAdmission {
    registration_id: Uuid,
    user: String,
    admission_epoch: u64,
    reply: oneshot::Sender<Result<RegistrationPermit, ServerError>>,
}

enum LifecycleEvent {
    SnapshotAccepted,
}

async fn supervise(
    config: Arc<Config>,
    registry: Registry,
    mut requests: mpsc::Receiver<LocalRequest>,
    readiness: watch::Sender<bool>,
    admission_epoch: Arc<AtomicU64>,
    mut shutdown: watch::Receiver<bool>,
) -> anyhow::Result<()> {
    let boot_id = Uuid::new_v4();
    let mut backoff = BACKOFF_MIN;
    let mut grace_deadline = None;
    let mut grace_armed = false;

    loop {
        reject_queued(&mut requests);
        let (lifecycle_tx, mut lifecycle_rx) = mpsc::channel(1);
        let mut attempt = Box::pin(run_connection(
            config.as_ref(),
            boot_id,
            &registry,
            &mut requests,
            &readiness,
            lifecycle_tx,
        ));
        let outcome = loop {
            tokio::select! {
                biased;
                changed = shutdown.changed() => {
                    let _ = readiness.send(false);
                    if changed.is_err() || *shutdown.borrow() {
                        return Ok(());
                    }
                }
                event = lifecycle_rx.recv() => {
                    if matches!(event, Some(LifecycleEvent::SnapshotAccepted)) {
                        grace_deadline = None;
                        grace_armed = true;
                        backoff = BACKOFF_MIN;
                        tracing::info!(proxy_id = config.proxy_id.as_str(), %boot_id, "controller snapshot accepted");
                    }
                }
                _ = wait_deadline(grace_deadline) => {
                    let killed = registry.evict_all_for_control_loss();
                    grace_deadline = None;
                    tracing::warn!(proxy_id = config.proxy_id.as_str(), killed, "controller reconnect grace expired");
                }
                outcome = &mut attempt => break outcome,
            }
        };
        drop(attempt);

        admission_epoch.fetch_add(1, Ordering::AcqRel);
        let _ = readiness.send(false);
        reject_queued(&mut requests);
        if grace_armed {
            grace_armed = false;
            grace_deadline = Some(Instant::now() + GRACE_PERIOD);
        }
        match outcome {
            Err(AttemptError::Permanent(message)) => anyhow::bail!(message),
            Err(AttemptError::Retry(message)) => {
                tracing::warn!(proxy_id = config.proxy_id.as_str(), error = %message, "controller connection lost");
            }
            Ok(()) => {
                tracing::warn!(
                    proxy_id = config.proxy_id.as_str(),
                    "controller stream ended"
                );
            }
        }

        let delay = jitter(backoff);
        let sleep = tokio::time::sleep(delay);
        tokio::pin!(sleep);
        loop {
            tokio::select! {
                biased;
                changed = shutdown.changed() => {
                    if changed.is_err() || *shutdown.borrow() {
                        return Ok(());
                    }
                }
                _ = wait_deadline(grace_deadline) => {
                    let killed = registry.evict_all_for_control_loss();
                    grace_deadline = None;
                    tracing::warn!(proxy_id = config.proxy_id.as_str(), killed, "controller reconnect grace expired");
                }
                request = requests.recv() => {
                    match request {
                        Some(request) => reject_request(request),
                        None => return Err(anyhow::anyhow!("control admission channel closed")),
                    }
                }
                _ = &mut sleep => break,
            }
        }
        backoff = (backoff * 2).min(BACKOFF_MAX);
    }
}

async fn run_connection(
    config: &Config,
    boot_id: Uuid,
    registry: &Registry,
    requests: &mut mpsc::Receiver<LocalRequest>,
    readiness: &watch::Sender<bool>,
    lifecycle: mpsc::Sender<LifecycleEvent>,
) -> Result<(), AttemptError> {
    let host = config
        .control_url
        .host_str()
        .ok_or_else(|| AttemptError::Permanent("controller URL has no host".into()))?;
    let port = config
        .control_url
        .port_or_known_default()
        .ok_or_else(|| AttemptError::Permanent("controller URL has no port".into()))?;
    let tcp = tokio::time::timeout(CONNECT_TIMEOUT, TcpStream::connect((host, port)))
        .await
        .map_err(|_| AttemptError::Retry("controller TCP connect timed out".into()))?
        .map_err(|error| AttemptError::Retry(format!("controller TCP connect: {error}")))?;
    let _ = tcp.set_nodelay(true);
    let (mut sender, mut connection) =
        tokio::time::timeout(CONNECT_TIMEOUT, h2::client::handshake(tcp))
            .await
            .map_err(|_| AttemptError::Retry("controller h2 handshake timed out".into()))?
            .map_err(|error| AttemptError::Retry(format!("controller h2 handshake: {error}")))?;

    let mut connect_url = config.control_url.clone();
    connect_url.set_path(CONNECT_PATH);
    let request = Request::builder()
        .method(Method::POST)
        .uri(connect_url.as_str())
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", config.proxy_token),
        )
        .header(header::CONTENT_TYPE, CONTENT_TYPE)
        .body(())
        .map_err(|error| AttemptError::Permanent(format!("build controller request: {error}")))?;
    let (response, send) = sender
        .send_request(request, false)
        .map_err(|error| AttemptError::Retry(format!("send controller request: {error}")))?;
    let response = tokio::select! {
        result = &mut connection => {
            return Err(AttemptError::Retry(format!("controller h2 connection: {:?}", result.err())));
        }
        result = tokio::time::timeout(CONNECT_TIMEOUT, response) => {
            result
                .map_err(|_| AttemptError::Retry("controller response timed out".into()))?
                .map_err(|error| AttemptError::Retry(format!("controller response: {error}")))?
        }
    };
    if response.status() != StatusCode::OK {
        let message = format!("controller rejected proxy with HTTP {}", response.status());
        return if response.status().is_client_error() {
            Err(AttemptError::Permanent(message))
        } else {
            Err(AttemptError::Retry(message))
        };
    }
    if response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        != Some(CONTENT_TYPE)
    {
        return Err(AttemptError::Permanent(
            "controller returned the wrong content type".into(),
        ));
    }

    let stream = run_stream(
        H2Duplex::new(send, response.into_body()),
        config,
        boot_id,
        registry,
        requests,
        readiness,
        lifecycle,
    );
    tokio::pin!(stream);
    tokio::select! {
        result = stream => result,
        result = &mut connection => {
            Err(AttemptError::Retry(format!("controller h2 connection ended: {:?}", result.err())))
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn run_stream<S>(
    stream: S,
    config: &Config,
    boot_id: Uuid,
    registry: &Registry,
    requests: &mut mpsc::Receiver<LocalRequest>,
    readiness: &watch::Sender<bool>,
    lifecycle: mpsc::Sender<LifecycleEvent>,
) -> Result<(), AttemptError>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    let (mut reader, mut writer) = tokio::io::split(stream);
    tokio::time::timeout(
        CONNECT_TIMEOUT,
        write_control(
            &mut writer,
            &ClientFrame::ClientHello {
                protocol_version: PROTOCOL_VERSION,
                package_version: env!("CARGO_PKG_VERSION").into(),
                proxy_id: config.proxy_id.clone(),
                proxy_base_url: config.proxy_base_url.clone(),
                boot_id,
            },
        ),
    )
    .await
    .map_err(|_| AttemptError::Retry("controller ClientHello write timed out".into()))??;
    let hello = tokio::time::timeout(CONNECT_TIMEOUT, read_control(&mut reader))
        .await
        .map_err(|_| AttemptError::Retry("controller ServerHello read timed out".into()))??;
    match hello {
        ServerFrame::ServerHello {
            protocol_version,
            package_version,
            heartbeat_seconds,
            dead_seconds,
            grace_seconds,
        } if protocol_version == PROTOCOL_VERSION
            && package_version == env!("CARGO_PKG_VERSION")
            && heartbeat_seconds == 5
            && dead_seconds == 15
            && grace_seconds == 30 => {}
        ServerFrame::Shutdown { reason } => return Err(AttemptError::Permanent(reason)),
        _ => {
            return Err(AttemptError::Permanent(
                "controller returned an invalid ServerHello".into(),
            ))
        }
    }

    let (base_generation, snapshot, mut events) = registry.tunnels().snapshot_and_subscribe();
    tracing::info!(
        proxy_id = config.proxy_id.as_str(),
        %boot_id,
        base_generation,
        rows = snapshot.len(),
        "publishing controller registry snapshot"
    );
    let publish_snapshot = async {
        write_control(&mut writer, &ClientFrame::SnapshotStart { base_generation }).await?;
        for chunk in snapshot.chunks(MAX_SNAPSHOT_CHUNK_ROWS) {
            write_control(
                &mut writer,
                &ClientFrame::SnapshotChunk {
                    rows: chunk.iter().map(tunnel_row).collect(),
                },
            )
            .await?;
        }
        write_control(&mut writer, &ClientFrame::SnapshotEnd { base_generation }).await
    };
    tokio::time::timeout(SNAPSHOT_TIMEOUT, publish_snapshot)
        .await
        .map_err(|_| AttemptError::Retry("controller snapshot write timed out".into()))??;

    // Keep exactly one framed read alive for the lifetime of the active
    // session. Recreating `read_frame` inside `select!` would cancel a
    // partially consumed length or JSON payload whenever a local registry
    // event won the race, corrupting the next frame boundary.
    let (frames_tx, mut frames) = mpsc::channel(SERVER_FRAME_QUEUE_CAPACITY);
    let (overflow_tx, mut overflow) = mpsc::channel(1);
    let reader_task = AbortOnDropTask::new(tokio::spawn(async move {
        loop {
            let frame = read_control(&mut reader).await;
            let terminal = frame.is_err();
            match frames_tx.try_send(frame) {
                Ok(()) if !terminal => {}
                Ok(()) => break,
                Err(mpsc::error::TrySendError::Full(_)) => {
                    let _ = overflow_tx.try_send(());
                    break;
                }
                Err(mpsc::error::TrySendError::Closed(_)) => break,
            }
        }
    }));

    let mut fleet_ready = false;
    let mut snapshot_accepted = false;
    let mut controller_deadline = Instant::now() + CONTROLLER_DEAD_AFTER;
    let mut pending: HashMap<Uuid, PendingAdmission> = HashMap::new();
    let mut overflow_open = true;
    let result = async {
        loop {
            tokio::select! {
                biased;
                _ = tokio::time::sleep_until(controller_deadline) => {
                    break Err(AttemptError::Retry("controller heartbeat deadline expired".into()));
                }
                overflowed = overflow.recv(), if overflow_open => {
                    match overflowed {
                        Some(()) => break Err(AttemptError::Retry("controller frame queue overflowed".into())),
                        None => overflow_open = false,
                    }
                }
                frame = frames.recv() => {
                    let frame = frame
                        .ok_or_else(|| AttemptError::Retry("controller frame reader stopped".into()))??;
                    controller_deadline = Instant::now() + CONTROLLER_DEAD_AFTER;
                    match frame {
                    ServerFrame::SnapshotAccepted { base_generation: accepted }
                        if !snapshot_accepted && accepted == base_generation => {
                            snapshot_accepted = true;
                            lifecycle.send(LifecycleEvent::SnapshotAccepted).await
                                .map_err(|_| AttemptError::Retry("control supervisor stopped".into()))?;
                        }
                    ServerFrame::FleetReady if snapshot_accepted && !fleet_ready => {
                        fleet_ready = true;
                        let _ = readiness.send(true);
                        tracing::info!(
                            proxy_id = config.proxy_id.as_str(),
                            %boot_id,
                            "proxy control session reached FleetReady"
                        );
                    }
                    ServerFrame::AdmissionDecision {
                        request_id,
                        registration_id,
                        decision,
                    } if fleet_ready => {
                        let Some(pending_admission) = pending.remove(&request_id) else {
                            continue;
                        };
                        if pending_admission.registration_id != registration_id {
                            break Err(AttemptError::Permanent("controller admission id mismatch".into()));
                        }
                        tracing::debug!(
                            proxy_id = config.proxy_id.as_str(),
                            %request_id,
                            %registration_id,
                            ?decision,
                            "controller admission decision received"
                        );
                        let reply = match decision {
                            AdmissionDecision::Admit => Ok(RegistrationPermit {
                                request_id,
                                registration_id,
                                admission_epoch: pending_admission.admission_epoch,
                            }),
                            AdmissionDecision::AtCapacity => Err(ServerError::AdmissionAtCapacity { user: pending_admission.user }),
                            AdmissionDecision::ControlWarming | AdmissionDecision::Stale => Err(ServerError::ControlUnavailable),
                        };
                        let _ = pending_admission.reply.send(reply);
                    }
                    ServerFrame::KillRegistrations { command_id, registration_ids }
                        if snapshot_accepted => {
                        let mut killed = Vec::new();
                        let mut missing = Vec::new();
                        for registration_id in registration_ids {
                            if registry.tunnels().evict_registration(registration_id) {
                                killed.push(registration_id);
                            } else {
                                missing.push(registration_id);
                            }
                        }
                        tracing::info!(
                            proxy_id = config.proxy_id.as_str(),
                            %command_id,
                            killed = killed.len(),
                            missing = missing.len(),
                            "controller kill command completed"
                        );
                        write_active(&mut writer, &ClientFrame::CommandResult {
                            command_id,
                            killed,
                            missing,
                            failed: Vec::new(),
                        }, controller_deadline).await?;
                    }
                    ServerFrame::ResyncRequired { expected_generation } => {
                        tracing::warn!(
                            proxy_id = config.proxy_id.as_str(),
                            expected_generation,
                            "controller requested registry resync"
                        );
                        break Err(AttemptError::Retry("controller requested a fresh snapshot".into()));
                    }
                    ServerFrame::Ping { nonce } => {
                        tracing::debug!(nonce, "controller heartbeat received");
                        write_active(
                            &mut writer,
                            &ClientFrame::Pong { nonce },
                            controller_deadline,
                        )
                        .await?;
                    }
                    ServerFrame::Shutdown { reason } => break Err(AttemptError::Permanent(reason)),
                    _ => break Err(AttemptError::Permanent("controller sent a frame illegal in the current state".into())),
                    }
                }
                event = events.recv() => {
                    match event {
                        Ok(RegistryEvent::TunnelUp { generation, row }) => {
                            tracing::debug!(
                                proxy_id = config.proxy_id.as_str(),
                                generation,
                                registration_id = %row.registration_id,
                                "publishing tunnel up"
                            );
                            write_active(
                                &mut writer,
                                &ClientFrame::TunnelUp { generation, row: tunnel_row(&row) },
                                controller_deadline,
                            )
                            .await?;
                        }
                        Ok(RegistryEvent::TunnelDown { generation, registration_id }) => {
                            tracing::debug!(
                                proxy_id = config.proxy_id.as_str(),
                                generation,
                                %registration_id,
                                "publishing tunnel down"
                            );
                            write_active(
                                &mut writer,
                                &ClientFrame::TunnelDown { generation, registration_id },
                                controller_deadline,
                            )
                            .await?;
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                            break Err(AttemptError::Retry("registry event receiver lagged".into()));
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                            break Err(AttemptError::Permanent("registry event stream closed".into()));
                        }
                    }
                }
                request = requests.recv() => {
                    let Some(request) = request else {
                        break Err(AttemptError::Permanent("control admission channel closed".into()));
                    };
                    match request {
                        LocalRequest::Admit {
                            request_id,
                            registration_id,
                            user,
                            devserver_id,
                            admission_epoch,
                            reply,
                        } => {
                            if !fleet_ready {
                                let _ = reply.send(Err(ServerError::ControlUnavailable));
                                continue;
                            }
                            write_active(
                                &mut writer,
                                &ClientFrame::AdmissionRequest {
                                    request_id,
                                    registration_id,
                                    user: user.clone(),
                                    devserver_id,
                                },
                                controller_deadline,
                            )
                            .await?;
                            pending.insert(request_id, PendingAdmission {
                                registration_id,
                                user,
                                admission_epoch,
                                reply,
                            });
                        }
                        LocalRequest::Cancel { request_id, registration_id } => {
                            pending.remove(&request_id);
                            write_active(
                                &mut writer,
                                &ClientFrame::AdmissionCancel { request_id, registration_id },
                                controller_deadline,
                            )
                            .await?;
                        }
                    }
                }
            }
        }
    }
    .await;
    for (_, pending) in pending {
        let _ = pending.reply.send(Err(ServerError::ControlUnavailable));
    }
    reader_task.cancel().await;
    result
}

fn tunnel_row(info: &TunnelInfo) -> TunnelRow {
    TunnelRow {
        registration_id: info.registration_id,
        user: info.user.as_ref().to_string(),
        devserver_id: info.workspace.as_ref().to_string(),
        peer_addr: info.peer_addr,
        connected_at: info.connected_at,
    }
}

async fn write_control<S>(stream: &mut S, frame: &ClientFrame) -> Result<(), AttemptError>
where
    S: tokio::io::AsyncWrite + Unpin,
{
    write_frame(stream, frame)
        .await
        .map_err(|error| AttemptError::Retry(format!("write controller frame: {error}")))
}

async fn write_active<S>(
    stream: &mut S,
    frame: &ClientFrame,
    controller_deadline: Instant,
) -> Result<(), AttemptError>
where
    S: tokio::io::AsyncWrite + Unpin,
{
    tokio::select! {
        biased;
        _ = tokio::time::sleep_until(controller_deadline) => {
            Err(AttemptError::Retry("controller heartbeat deadline expired during write".into()))
        }
        result = write_control(stream, frame) => result,
    }
}

async fn read_control<S>(stream: &mut S) -> Result<ServerFrame, AttemptError>
where
    S: tokio::io::AsyncRead + Unpin,
{
    read_frame(stream)
        .await
        .map_err(|error| AttemptError::Retry(format!("read controller frame: {error}")))
}

fn reject_queued(requests: &mut mpsc::Receiver<LocalRequest>) {
    while let Ok(request) = requests.try_recv() {
        reject_request(request);
    }
}

fn reject_request(request: LocalRequest) {
    if let LocalRequest::Admit { reply, .. } = request {
        let _ = reply.send(Err(ServerError::ControlUnavailable));
    }
}

fn jitter(base: Duration) -> Duration {
    let percent = rand::thread_rng().gen_range(80_u32..=120);
    base.mul_f64(f64::from(percent) / 100.0)
}

async fn wait_deadline(deadline: Option<Instant>) {
    match deadline {
        Some(deadline) => tokio::time::sleep_until(deadline).await,
        None => std::future::pending().await,
    }
}

#[derive(Debug, thiserror::Error)]
enum AttemptError {
    #[error("{0}")]
    Retry(String),
    #[error("{0}")]
    Permanent(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use chan_tunnel_proto::{Hello, ProtocolVersion};
    use chan_tunnel_server::Validated;
    use devserver_control::{
        serve_control_listener, spawn_controller_owned, ControllerHandle, ProxyStatus,
    };
    use devserver_control_proto::ProxyOriginTemplate;
    use tokio::io::duplex;
    use tokio::net::TcpListener;

    fn test_config(control_addr: std::net::SocketAddr) -> Arc<Config> {
        Arc::new(Config {
            bind_addr: "127.0.0.1:0".parse().unwrap(),
            tunnel_bind_addr: "127.0.0.1:0".parse().unwrap(),
            apex_host: "devserver.chan.app".into(),
            wildcard_suffix: ".p1.devserver.chan.app".into(),
            identity_url: "http://127.0.0.1:7000/".parse().unwrap(),
            identity_auth_token: "identity-token".into(),
            dashboard_url: "https://id.chan.app/workspaces".into(),
            workspace_gate_secret: "gate-secret".into(),
            control_url: format!("http://{control_addr}/").parse().unwrap(),
            proxy_token: "proxy-token".into(),
            proxy_id: devserver_control_proto::ProxyId::parse("p1").unwrap(),
            proxy_base_url: devserver_control_proto::CanonicalOrigin::parse(
                "https://p1.devserver.chan.app",
            )
            .unwrap(),
            max_response_bytes: None,
            max_request_bytes: None,
            request_timeout: None,
            ws_idle_timeout: crate::config::DEFAULT_WS_IDLE_TIMEOUT,
            forwarded_proto: "https".into(),
        })
    }

    async fn settle() {
        for _ in 0..100 {
            tokio::task::yield_now().await;
        }
    }

    async fn assert_snapshot_active(controller: &ControllerHandle) {
        let mut proxies = controller.watch_proxies();
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                if proxies
                    .borrow()
                    .first()
                    .is_some_and(|proxy| proxy.status == ProxyStatus::Active)
                {
                    return;
                }
                proxies.changed().await.unwrap();
            }
        })
        .await
        .expect("controller never accepted the proxy snapshot");
    }

    #[tokio::test]
    async fn real_controller_accepts_the_supervisor_snapshot_over_h2() {
        let _ = tracing_subscriber::fmt()
            .with_test_writer()
            .with_env_filter(tracing_subscriber::EnvFilter::new(
                "devserver_proxy::control=debug,devserver_control=debug,h2=off",
            ))
            .try_init();
        let (controller, actor_task) = spawn_controller_owned(100);
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let control_addr = listener.local_addr().unwrap();
        let template = ProxyOriginTemplate::parse("https://{proxy_id}.devserver.chan.app").unwrap();
        let (listener_shutdown, listener_shutdown_rx) = watch::channel(false);
        let listener_task = tokio::spawn(serve_control_listener(
            listener,
            controller.clone(),
            "proxy-token".into(),
            template.clone(),
            listener_shutdown_rx,
        ));

        let registry = Registry::new();
        let (proxy_shutdown, proxy_shutdown_rx) = watch::channel(false);
        let ControlRuntime {
            admission: _admission,
            readiness,
            task: mut supervisor_task,
        } = spawn_control_supervisor(test_config(control_addr), registry, proxy_shutdown_rx);

        settle().await;
        tokio::select! {
            result = &mut supervisor_task => {
                panic!("control supervisor exited before snapshot: {result:?}");
            }
            () = assert_snapshot_active(&controller) => {}
        }
        assert!(!*readiness.borrow());

        proxy_shutdown.send(true).unwrap();
        listener_shutdown.send(true).unwrap();
        supervisor_task.await.unwrap().unwrap();
        listener_task.await.unwrap().unwrap();
        drop(controller);
        actor_task.await.unwrap();
    }

    #[tokio::test]
    async fn stream_correlates_admission_and_heartbeats_after_fleet_ready() {
        let config = test_config("127.0.0.1:1".parse().unwrap());
        let registry = Registry::new();
        let (requests_tx, mut requests_rx) = mpsc::channel(8);
        let (readiness_tx, readiness) = watch::channel(false);
        let (lifecycle_tx, mut lifecycle_rx) = mpsc::channel(1);
        let admission = ControlAdmission {
            requests: requests_tx,
            readiness: readiness.clone(),
            admission_epoch: Arc::new(AtomicU64::new(1)),
        };
        let boot_id = Uuid::new_v4();
        let (proxy, mut controller) = duplex(64 * 1024);

        let server = tokio::spawn(async move {
            let hello: ClientFrame = read_frame(&mut controller).await.unwrap();
            assert!(matches!(
                hello,
                ClientFrame::ClientHello {
                    protocol_version: PROTOCOL_VERSION,
                    ..
                }
            ));
            write_frame(
                &mut controller,
                &ServerFrame::ServerHello {
                    protocol_version: PROTOCOL_VERSION,
                    package_version: env!("CARGO_PKG_VERSION").into(),
                    heartbeat_seconds: 5,
                    dead_seconds: 15,
                    grace_seconds: 30,
                },
            )
            .await
            .unwrap();

            assert!(matches!(
                read_frame::<_, ClientFrame>(&mut controller).await.unwrap(),
                ClientFrame::SnapshotStart { base_generation: 0 }
            ));
            assert!(matches!(
                read_frame::<_, ClientFrame>(&mut controller).await.unwrap(),
                ClientFrame::SnapshotEnd { base_generation: 0 }
            ));
            write_frame(
                &mut controller,
                &ServerFrame::SnapshotAccepted { base_generation: 0 },
            )
            .await
            .unwrap();
            write_frame(&mut controller, &ServerFrame::FleetReady)
                .await
                .unwrap();

            let request = read_frame::<_, ClientFrame>(&mut controller).await.unwrap();
            let ClientFrame::AdmissionRequest {
                request_id,
                registration_id,
                user,
                devserver_id,
            } = request
            else {
                panic!("expected admission request, got {request:?}");
            };
            assert_eq!(user, "alice");
            assert_eq!(devserver_id, "devserver-a");
            write_frame(
                &mut controller,
                &ServerFrame::AdmissionDecision {
                    request_id,
                    registration_id,
                    decision: AdmissionDecision::Admit,
                },
            )
            .await
            .unwrap();
            assert!(matches!(
                read_frame::<_, ClientFrame>(&mut controller).await.unwrap(),
                ClientFrame::AdmissionCancel {
                    request_id: canceled_request,
                    registration_id: canceled_registration,
                } if canceled_request == request_id && canceled_registration == registration_id
            ));

            let request = read_frame::<_, ClientFrame>(&mut controller).await.unwrap();
            let ClientFrame::AdmissionRequest {
                request_id,
                registration_id,
                ..
            } = request
            else {
                panic!("expected second admission request, got {request:?}");
            };
            write_frame(
                &mut controller,
                &ServerFrame::AdmissionDecision {
                    request_id,
                    registration_id,
                    decision: AdmissionDecision::AtCapacity,
                },
            )
            .await
            .unwrap();

            write_frame(&mut controller, &ServerFrame::Ping { nonce: 42 })
                .await
                .unwrap();
            assert!(matches!(
                read_frame::<_, ClientFrame>(&mut controller).await.unwrap(),
                ClientFrame::Pong { nonce: 42 }
            ));
            write_frame(&mut controller, &ServerFrame::FleetReady)
                .await
                .unwrap();
        });

        let client = tokio::spawn(async move {
            run_stream(
                proxy,
                config.as_ref(),
                boot_id,
                &registry,
                &mut requests_rx,
                &readiness_tx,
                lifecycle_tx,
            )
            .await
        });

        let mut readiness_wait = readiness.clone();
        while !*readiness_wait.borrow() {
            readiness_wait.changed().await.unwrap();
        }
        assert!(matches!(
            lifecycle_rx.recv().await,
            Some(LifecycleEvent::SnapshotAccepted)
        ));

        let hello = Hello {
            protocol: ProtocolVersion::V1,
            client_version: "test".into(),
            workspace: "devserver".into(),
            name: None,
        };
        let validated = Validated {
            user_id: Uuid::new_v4(),
            username: "alice".into(),
            devserver_id: "devserver-a".into(),
            scopes: vec!["tunnel".into()],
            gateway_assertion_key: None,
        };
        let permit = admission.admit(&hello, &validated).await.unwrap();
        admission.cancel(permit).await;
        let error = admission.admit(&hello, &validated).await.unwrap_err();
        assert!(matches!(
            error,
            ServerError::AdmissionAtCapacity { user } if user == "alice"
        ));

        server.await.unwrap();
        let error = client.await.unwrap().unwrap_err();
        assert!(matches!(
            error,
            AttemptError::Permanent(reason)
                if reason == "controller sent a frame illegal in the current state"
        ));
    }

    #[tokio::test(start_paused = true)]
    async fn stream_expires_when_an_active_controller_goes_silent() {
        let config = test_config("127.0.0.1:1".parse().unwrap());
        let registry = Registry::new();
        let (_requests_tx, mut requests_rx) = mpsc::channel(1);
        let (readiness_tx, readiness) = watch::channel(false);
        let (lifecycle_tx, _lifecycle_rx) = mpsc::channel(1);
        let boot_id = Uuid::new_v4();
        let (proxy, mut controller) = duplex(64 * 1024);

        let server = tokio::spawn(async move {
            let _: ClientFrame = read_frame(&mut controller).await.unwrap();
            write_frame(
                &mut controller,
                &ServerFrame::ServerHello {
                    protocol_version: PROTOCOL_VERSION,
                    package_version: env!("CARGO_PKG_VERSION").into(),
                    heartbeat_seconds: 5,
                    dead_seconds: 15,
                    grace_seconds: 30,
                },
            )
            .await
            .unwrap();
            assert!(matches!(
                read_frame::<_, ClientFrame>(&mut controller).await.unwrap(),
                ClientFrame::SnapshotStart { base_generation: 0 }
            ));
            assert!(matches!(
                read_frame::<_, ClientFrame>(&mut controller).await.unwrap(),
                ClientFrame::SnapshotEnd { base_generation: 0 }
            ));
            write_frame(
                &mut controller,
                &ServerFrame::SnapshotAccepted { base_generation: 0 },
            )
            .await
            .unwrap();
            write_frame(&mut controller, &ServerFrame::FleetReady)
                .await
                .unwrap();
            std::future::pending::<()>().await;
        });

        let client = tokio::spawn(async move {
            run_stream(
                proxy,
                config.as_ref(),
                boot_id,
                &registry,
                &mut requests_rx,
                &readiness_tx,
                lifecycle_tx,
            )
            .await
        });

        let mut readiness_wait = readiness.clone();
        while !*readiness_wait.borrow() {
            readiness_wait.changed().await.unwrap();
        }
        let error = client.await.unwrap().unwrap_err();
        assert!(matches!(
            error,
            AttemptError::Retry(reason)
                if reason == "controller heartbeat deadline expired"
        ));
        server.abort();
    }

    #[tokio::test(start_paused = true)]
    async fn stream_bounds_the_server_hello_wait() {
        let config = test_config("127.0.0.1:1".parse().unwrap());
        let registry = Registry::new();
        let (_requests_tx, mut requests_rx) = mpsc::channel(1);
        let (readiness_tx, _readiness) = watch::channel(false);
        let (lifecycle_tx, _lifecycle_rx) = mpsc::channel(1);
        let (proxy, mut controller) = duplex(64 * 1024);
        let server = tokio::spawn(async move {
            let _: ClientFrame = read_frame(&mut controller).await.unwrap();
            std::future::pending::<()>().await;
        });

        let error = run_stream(
            proxy,
            config.as_ref(),
            Uuid::new_v4(),
            &registry,
            &mut requests_rx,
            &readiness_tx,
            lifecycle_tx,
        )
        .await
        .unwrap_err();
        assert!(matches!(
            error,
            AttemptError::Retry(reason)
                if reason == "controller ServerHello read timed out"
        ));
        server.abort();
    }

    #[tokio::test]
    async fn admission_rejects_immediately_while_unready() {
        let (requests, _requests_rx) = mpsc::channel(1);
        let (_readiness_tx, readiness) = watch::channel(false);
        let admission = ControlAdmission {
            requests,
            readiness,
            admission_epoch: Arc::new(AtomicU64::new(1)),
        };
        let error = admission
            .admit(
                &Hello {
                    protocol: ProtocolVersion::V1,
                    client_version: "test".into(),
                    workspace: "devserver".into(),
                    name: None,
                },
                &Validated {
                    user_id: Uuid::new_v4(),
                    username: "alice".into(),
                    devserver_id: "devserver-a".into(),
                    scopes: vec!["tunnel".into()],
                    gateway_assertion_key: None,
                },
            )
            .await
            .unwrap_err();
        assert!(matches!(error, ServerError::ControlUnavailable));
    }

    #[test]
    fn admission_permit_is_fenced_by_readiness_and_epoch() {
        let (requests, _requests_rx) = mpsc::channel(1);
        let (readiness_tx, readiness) = watch::channel(true);
        let admission_epoch = Arc::new(AtomicU64::new(7));
        let admission = ControlAdmission {
            requests,
            readiness,
            admission_epoch: admission_epoch.clone(),
        };
        let permit = RegistrationPermit {
            request_id: Uuid::new_v4(),
            registration_id: Uuid::new_v4(),
            admission_epoch: 7,
        };

        assert!(admission.permit_is_current(permit));
        admission_epoch.fetch_add(1, Ordering::AcqRel);
        assert!(!admission.permit_is_current(permit));
        readiness_tx.send(false).unwrap();
        assert!(!admission.permit_is_current(RegistrationPermit {
            admission_epoch: 8,
            ..permit
        }));
    }
}
