use std::collections::HashSet;
use std::io;
use std::sync::Arc;
use std::time::Duration;

use chan_tunnel_proto::H2Duplex;
use devserver_control_proto::{
    read_frame, write_frame, CanonicalOrigin, ClientFrame, FrameError, ProxyId,
    ProxyOriginTemplate, ServerFrame, TunnelRow, CONNECT_PATH, CONTENT_TYPE, MAX_SNAPSHOT_ROWS,
    PROTOCOL_VERSION,
};
use h2::server::SendResponse;
use http::{header, Method, Request, Response, StatusCode};
use subtle::ConstantTimeEq;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::{mpsc, watch, Semaphore};
use tokio::task::JoinSet;
use tokio::time::Instant;
use uuid::Uuid;

use crate::{ActorError, ControllerHandle, MutationStatus, ProxyControlSession};

const H2_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
const FIRST_STREAM_TIMEOUT: Duration = Duration::from_secs(10);
const HELLO_TIMEOUT: Duration = Duration::from_secs(10);
const SNAPSHOT_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_EXTRA_STREAMS: usize = 16;
const MAX_INFLIGHT_CONNECTIONS: usize = 1024;
const CLIENT_FRAME_QUEUE_CAPACITY: usize = 1024;

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

pub async fn serve_control_listener(
    listener: TcpListener,
    controller: ControllerHandle,
    proxy_token: String,
    origin_template: ProxyOriginTemplate,
    mut shutdown: watch::Receiver<bool>,
) -> io::Result<()> {
    let proxy_token: Arc<[u8]> = Arc::from(proxy_token.into_bytes());
    let inflight = Arc::new(Semaphore::new(MAX_INFLIGHT_CONNECTIONS));
    let mut connections = JoinSet::new();
    loop {
        tokio::select! {
            biased;
            changed = shutdown.changed() => {
                if changed.is_err() || *shutdown.borrow() {
                    break;
                }
            }
            joined = connections.join_next(), if !connections.is_empty() => {
                if let Some(Err(error)) = joined {
                    connections.shutdown().await;
                    return Err(io::Error::other(format!("control connection task failed: {error}")));
                }
            }
            accepted = listener.accept() => {
                let (stream, peer) = accepted?;
                let Ok(permit) = inflight.clone().try_acquire_owned() else {
                    tracing::warn!(%peer, max = MAX_INFLIGHT_CONNECTIONS, "proxy control connection cap reached");
                    continue;
                };
                let controller = controller.clone();
                let proxy_token = proxy_token.clone();
                let origin_template = origin_template.clone();
                connections.spawn(async move {
                    let _permit = permit;
                    if let Err(error) = handle_connection(
                        stream,
                        controller,
                        proxy_token,
                        origin_template,
                    ).await {
                        tracing::warn!(%peer, error = ?error, "proxy control connection closed");
                    }
                });
            }
        }
    }
    connections.shutdown().await;
    Ok(())
}

async fn handle_connection<T>(
    stream: T,
    controller: ControllerHandle,
    proxy_token: Arc<[u8]>,
    origin_template: ProxyOriginTemplate,
) -> Result<(), SessionError>
where
    T: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let mut connection = tokio::time::timeout(H2_HANDSHAKE_TIMEOUT, h2::server::handshake(stream))
        .await
        .map_err(|_| SessionError::Timeout("h2 handshake"))??;
    let accepted = tokio::time::timeout(FIRST_STREAM_TIMEOUT, connection.accept())
        .await
        .map_err(|_| SessionError::Timeout("first control stream"))?;
    let (request, mut respond) = match accepted {
        Some(result) => result?,
        None => return Ok(()),
    };

    if let Err(status) = validate_request(&request, proxy_token.as_ref()) {
        send_http_response(&mut respond, status, true)?;
        connection.graceful_shutdown();
        let _ = tokio::time::timeout(Duration::from_secs(1), async {
            while connection.accept().await.is_some() {}
        })
        .await;
        return Ok(());
    }
    let (_parts, recv) = request.into_parts();
    let send = send_http_response(&mut respond, StatusCode::OK, false)?
        .expect("non-terminal response has a body stream");

    let mut session = Box::pin(run_session(
        H2Duplex::new(send, recv),
        controller,
        origin_template,
    ));
    let mut rejected = 0usize;
    let result = loop {
        tokio::select! {
            result = &mut session => break result,
            stream = connection.accept() => {
                let Some(stream) = stream else {
                    break session.await;
                };
                if let Ok((_request, mut respond)) = stream {
                    let _ = send_http_response(&mut respond, StatusCode::CONFLICT, true);
                    rejected += 1;
                    if rejected >= MAX_EXTRA_STREAMS {
                        connection.abrupt_shutdown(h2::Reason::ENHANCE_YOUR_CALM);
                    }
                }
            }
        }
    };
    connection.graceful_shutdown();
    let _ = tokio::time::timeout(Duration::from_secs(1), async {
        while connection.accept().await.is_some() {}
    })
    .await;
    result
}

fn validate_request<B>(request: &Request<B>, proxy_token: &[u8]) -> Result<(), StatusCode> {
    if request.method() != Method::POST {
        return Err(StatusCode::METHOD_NOT_ALLOWED);
    }
    if request.uri().path() != CONNECT_PATH {
        return Err(StatusCode::NOT_FOUND);
    }
    let provided = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "));
    if !provided.is_some_and(|token| bool::from(token.as_bytes().ct_eq(proxy_token))) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let content_type = request
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok());
    if content_type != Some(CONTENT_TYPE) {
        return Err(StatusCode::UNSUPPORTED_MEDIA_TYPE);
    }
    Ok(())
}

fn send_http_response(
    respond: &mut SendResponse<bytes::Bytes>,
    status: StatusCode,
    end_stream: bool,
) -> Result<Option<h2::SendStream<bytes::Bytes>>, SessionError> {
    let response = Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, CONTENT_TYPE)
        .body(())
        .map_err(|error| SessionError::Protocol(error.to_string()))?;
    let send = respond.send_response(response, end_stream)?;
    Ok((!end_stream).then_some(send))
}

async fn run_session<S>(
    stream: S,
    controller: ControllerHandle,
    origin_template: ProxyOriginTemplate,
) -> Result<(), SessionError>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let (mut reader, mut writer) = tokio::io::split(stream);
    let hello = tokio::time::timeout(HELLO_TIMEOUT, read_frame::<_, ClientFrame>(&mut reader))
        .await
        .map_err(|_| SessionError::Timeout("ClientHello"))??;
    let ClientFrame::ClientHello {
        protocol_version,
        package_version,
        proxy_id,
        proxy_base_url,
        boot_id,
    } = hello
    else {
        send_shutdown(&mut writer, "ClientHello must be the first frame").await?;
        return Err(SessionError::Protocol(
            "first control frame was not ClientHello".into(),
        ));
    };
    if protocol_version != PROTOCOL_VERSION {
        send_shutdown(&mut writer, "unsupported control protocol version").await?;
        return Err(SessionError::Protocol(
            "unsupported control protocol version".into(),
        ));
    }
    if package_version != env!("CARGO_PKG_VERSION") {
        send_shutdown(&mut writer, "gateway package version mismatch").await?;
        return Err(SessionError::Protocol(
            "gateway package version mismatch".into(),
        ));
    }
    if let Err(message) = validate_origin(&origin_template, &proxy_id, &proxy_base_url) {
        send_shutdown(&mut writer, message).await?;
        return Err(SessionError::Protocol(message.to_string()));
    }

    let mut session = controller
        .begin_session(proxy_id.clone(), proxy_base_url, package_version, boot_id)
        .await?;
    write_frame(
        &mut writer,
        &ServerFrame::ServerHello {
            protocol_version: PROTOCOL_VERSION,
            package_version: env!("CARGO_PKG_VERSION").into(),
            heartbeat_seconds: crate::HEARTBEAT_INTERVAL.as_secs(),
            dead_seconds: crate::SESSION_DEAD_AFTER.as_secs(),
            grace_seconds: 30,
        },
    )
    .await?;

    let incarnation = session.incarnation;
    let result = run_established(reader, &mut writer, &controller, &proxy_id, &mut session).await;
    if let Err(error) = controller.disconnect(proxy_id, incarnation).await {
        if !matches!(error, ActorError::State(crate::StateError::StaleSession)) {
            tracing::warn!(error = ?error, "failed to remove closed proxy control session");
        }
    }
    result
}

fn validate_origin(
    template: &ProxyOriginTemplate,
    proxy_id: &ProxyId,
    provided: &CanonicalOrigin,
) -> Result<(), &'static str> {
    let expected = template
        .expand(proxy_id)
        .map_err(|_| "proxy base URL template expansion failed")?;
    if &expected == provided {
        Ok(())
    } else {
        Err("proxy base URL does not match its validated proxy id")
    }
}

async fn run_established<R, W>(
    mut reader: R,
    writer: &mut W,
    controller: &ControllerHandle,
    proxy_id: &ProxyId,
    session: &mut ProxyControlSession,
) -> Result<(), SessionError>
where
    R: AsyncRead + Unpin + Send + 'static,
    W: AsyncWrite + Unpin,
{
    // One task owns the framed reader for the entire established session.
    // `read_frame` is not cancellation-safe after consuming a length prefix
    // or payload fragment, so it must never be recreated by a `select!` arm.
    let (incoming_tx, mut incoming_rx) = mpsc::channel(CLIENT_FRAME_QUEUE_CAPACITY);
    let (overflow_tx, mut overflow_rx) = mpsc::channel(1);
    let reader_task = AbortOnDropTask::new(tokio::spawn(async move {
        loop {
            let frame = read_frame::<_, ClientFrame>(&mut reader).await;
            let terminal = frame.is_err();
            match incoming_tx.try_send(frame) {
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

    let mut phase = Phase::awaiting_snapshot();
    let mut overflow_open = true;
    let result = async {
        loop {
            let deadline = phase.deadline();
            tokio::select! {
                biased;
                _ = wait_deadline(deadline) => {
                    send_shutdown(writer, "snapshot deadline exceeded").await?;
                    return Err(SessionError::Timeout("initial snapshot"));
                }
                overflowed = overflow_rx.recv(), if overflow_open => {
                    match overflowed {
                        Some(()) => {
                            send_shutdown(writer, "client frame queue overflowed").await?;
                            return Err(SessionError::Protocol("client frame queue overflowed".into()));
                        }
                        None => overflow_open = false,
                    }
                }
                outgoing = session.commands.recv() => {
                    let Some(outgoing) = outgoing else {
                        return Ok(());
                    };
                    let resync = matches!(outgoing, ServerFrame::ResyncRequired { .. });
                    let shutdown = matches!(outgoing, ServerFrame::Shutdown { .. });
                    tracing::debug!(frame = ?outgoing, "sending proxy control frame");
                    write_frame(writer, &outgoing).await?;
                    if resync {
                        phase = Phase::awaiting_snapshot();
                    }
                    if shutdown {
                        writer.shutdown().await.map_err(FrameError::Io)?;
                        return Ok(());
                    }
                }
                incoming = incoming_rx.recv() => {
                    let incoming = incoming
                        .ok_or_else(|| SessionError::Protocol("client frame reader stopped".into()))??;
                    if let Err(error) = incoming.validate() {
                        send_shutdown(writer, "invalid control frame").await?;
                        return Err(error.into());
                    }
                    handle_client_frame(
                        incoming,
                        &mut phase,
                        writer,
                        controller,
                        proxy_id,
                        session.incarnation,
                    ).await?;
                }
            }
        }
    }
    .await;
    reader_task.cancel().await;
    result
}

#[allow(clippy::too_many_arguments)]
async fn handle_client_frame<W>(
    frame: ClientFrame,
    phase: &mut Phase,
    writer: &mut W,
    controller: &ControllerHandle,
    proxy_id: &ProxyId,
    incarnation: crate::SessionIncarnation,
) -> Result<(), SessionError>
where
    W: AsyncWrite + Unpin,
{
    let frame = match frame {
        ClientFrame::Pong { nonce } => {
            controller
                .pong(proxy_id.clone(), incarnation, nonce)
                .await?;
            return Ok(());
        }
        frame => frame,
    };
    match phase {
        Phase::AwaitSnapshot { deadline } => match frame {
            ClientFrame::SnapshotStart { base_generation } => {
                controller
                    .record_activity(proxy_id.clone(), incarnation)
                    .await?;
                *phase = Phase::Snapshot {
                    deadline: *deadline,
                    base_generation,
                    rows: Vec::new(),
                    registration_ids: HashSet::new(),
                };
            }
            ClientFrame::ClientHello { .. } => {
                return illegal_frame(writer, "duplicate ClientHello").await;
            }
            _ => {
                send_resync(writer, 0).await?;
                *phase = Phase::awaiting_snapshot();
            }
        },
        Phase::Snapshot {
            deadline: _,
            base_generation,
            rows,
            registration_ids,
        } => match frame {
            ClientFrame::SnapshotChunk { rows: chunk } => {
                controller
                    .record_activity(proxy_id.clone(), incarnation)
                    .await?;
                if !snapshot_rows_fit(rows.len(), chunk.len()) {
                    send_shutdown(writer, "snapshot row limit exceeded").await?;
                    return Err(SessionError::SnapshotTooLarge);
                }
                let mut chunk_ids = HashSet::with_capacity(chunk.len());
                if chunk.iter().any(|row| {
                    registration_ids.contains(&row.registration_id)
                        || !chunk_ids.insert(row.registration_id)
                }) {
                    send_resync(writer, *base_generation).await?;
                    *phase = Phase::awaiting_snapshot();
                    return Ok(());
                }
                registration_ids.extend(chunk_ids);
                rows.extend(chunk);
            }
            ClientFrame::SnapshotEnd {
                base_generation: end_generation,
            } if end_generation == *base_generation => {
                let base_generation = *base_generation;
                let rows = std::mem::take(rows);
                *phase = Phase::Active;
                controller
                    .accept_snapshot(proxy_id.clone(), incarnation, base_generation, rows)
                    .await?;
            }
            ClientFrame::ClientHello { .. } => {
                return illegal_frame(writer, "duplicate ClientHello").await;
            }
            _ => {
                let expected_generation = *base_generation;
                send_resync(writer, expected_generation).await?;
                *phase = Phase::awaiting_snapshot();
            }
        },
        Phase::Active => match frame {
            ClientFrame::TunnelUp { generation, row } => {
                let status = controller
                    .tunnel_up(proxy_id.clone(), incarnation, generation, row)
                    .await?;
                if status == MutationStatus::Resyncing {
                    *phase = Phase::awaiting_snapshot();
                }
            }
            ClientFrame::TunnelDown {
                generation,
                registration_id,
            } => {
                let status = controller
                    .tunnel_down(proxy_id.clone(), incarnation, generation, registration_id)
                    .await?;
                if status == MutationStatus::Resyncing {
                    *phase = Phase::awaiting_snapshot();
                }
            }
            ClientFrame::AdmissionRequest {
                request_id,
                registration_id,
                user,
                devserver_id,
            } => {
                controller
                    .request_admission(
                        proxy_id.clone(),
                        incarnation,
                        request_id,
                        registration_id,
                        user,
                        devserver_id,
                    )
                    .await?;
            }
            ClientFrame::AdmissionCancel {
                request_id,
                registration_id,
            } => {
                controller
                    .cancel_admission(proxy_id.clone(), incarnation, request_id, registration_id)
                    .await?;
            }
            ClientFrame::CommandResult {
                command_id,
                killed,
                missing,
                failed,
            } => {
                controller
                    .command_result(
                        proxy_id.clone(),
                        incarnation,
                        command_id,
                        killed,
                        missing,
                        failed,
                    )
                    .await?;
            }
            ClientFrame::Pong { nonce } => {
                controller
                    .pong(proxy_id.clone(), incarnation, nonce)
                    .await?;
            }
            ClientFrame::SnapshotStart { .. }
            | ClientFrame::SnapshotChunk { .. }
            | ClientFrame::SnapshotEnd { .. } => {
                controller
                    .require_resync(proxy_id.clone(), incarnation)
                    .await?;
                *phase = Phase::awaiting_snapshot();
            }
            ClientFrame::ClientHello { .. } => {
                return illegal_frame(writer, "duplicate ClientHello").await;
            }
        },
    }
    Ok(())
}

fn snapshot_rows_fit(current: usize, incoming: usize) -> bool {
    current
        .checked_add(incoming)
        .is_some_and(|total| total <= MAX_SNAPSHOT_ROWS)
}

async fn illegal_frame<W>(writer: &mut W, reason: &'static str) -> Result<(), SessionError>
where
    W: AsyncWrite + Unpin,
{
    send_shutdown(writer, reason).await?;
    Err(SessionError::Protocol(reason.into()))
}

async fn send_shutdown<W>(writer: &mut W, reason: &'static str) -> Result<(), FrameError>
where
    W: AsyncWrite + Unpin,
{
    write_frame(
        writer,
        &ServerFrame::Shutdown {
            reason: reason.into(),
        },
    )
    .await?;
    writer.shutdown().await.map_err(FrameError::Io)
}

async fn send_resync<W>(writer: &mut W, expected_generation: u64) -> Result<(), FrameError>
where
    W: AsyncWrite + Unpin,
{
    write_frame(
        writer,
        &ServerFrame::ResyncRequired {
            expected_generation,
        },
    )
    .await
}

async fn wait_deadline(deadline: Option<Instant>) {
    match deadline {
        Some(deadline) => tokio::time::sleep_until(deadline).await,
        None => std::future::pending().await,
    }
}

enum Phase {
    AwaitSnapshot {
        deadline: Instant,
    },
    Snapshot {
        deadline: Instant,
        base_generation: u64,
        rows: Vec<TunnelRow>,
        registration_ids: HashSet<Uuid>,
    },
    Active,
}

impl Phase {
    fn awaiting_snapshot() -> Self {
        Self::AwaitSnapshot {
            deadline: Instant::now() + SNAPSHOT_TIMEOUT,
        }
    }

    fn deadline(&self) -> Option<Instant> {
        match self {
            Self::AwaitSnapshot { deadline } | Self::Snapshot { deadline, .. } => Some(*deadline),
            Self::Active => None,
        }
    }
}

#[derive(Debug, thiserror::Error)]
enum SessionError {
    #[error("control session timed out during {0}")]
    Timeout(&'static str),
    #[error("control protocol error: {0}")]
    Protocol(String),
    #[error("control snapshot exceeds the row limit")]
    SnapshotTooLarge,
    #[error(transparent)]
    Frame(#[from] FrameError),
    #[error(transparent)]
    H2(#[from] h2::Error),
    #[error(transparent)]
    Actor(#[from] ActorError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use devserver_control_proto::TunnelRow;
    use uuid::Uuid;

    struct Opened {
        status: StatusCode,
        stream: Option<H2Duplex>,
        server: tokio::task::JoinHandle<Result<(), SessionError>>,
        driver: tokio::task::JoinHandle<Result<(), h2::Error>>,
    }

    impl Drop for Opened {
        fn drop(&mut self) {
            self.server.abort();
            self.driver.abort();
        }
    }

    async fn open(
        controller: ControllerHandle,
        method: Method,
        path: &str,
        token: Option<&str>,
        content_type: Option<&str>,
    ) -> Opened {
        let (client_io, server_io) = tokio::io::duplex(64 * 1024);
        let server = tokio::spawn(handle_connection(
            server_io,
            controller,
            Arc::from(b"secret".as_slice()),
            ProxyOriginTemplate::parse("https://{proxy_id}.proxy.example.test").unwrap(),
        ));
        let (mut client, connection) = h2::client::handshake(client_io).await.unwrap();
        let driver = tokio::spawn(connection);
        let mut request = Request::builder().method(method).uri(path);
        if let Some(token) = token {
            request = request.header(header::AUTHORIZATION, format!("Bearer {token}"));
        }
        if let Some(content_type) = content_type {
            request = request.header(header::CONTENT_TYPE, content_type);
        }
        let (response, send) = client
            .send_request(request.body(()).unwrap(), false)
            .unwrap();
        let response = response.await.unwrap();
        let status = response.status();
        let stream = (status == StatusCode::OK).then(|| H2Duplex::new(send, response.into_body()));
        Opened {
            status,
            stream,
            server,
            driver,
        }
    }

    async fn connected(controller: ControllerHandle) -> Opened {
        open(
            controller,
            Method::POST,
            CONNECT_PATH,
            Some("secret"),
            Some(CONTENT_TYPE),
        )
        .await
    }

    fn hello(protocol_version: u16, package_version: &str, origin: &str) -> ClientFrame {
        ClientFrame::ClientHello {
            protocol_version,
            package_version: package_version.into(),
            proxy_id: ProxyId::parse("p1").unwrap(),
            proxy_base_url: CanonicalOrigin::parse(origin).unwrap(),
            boot_id: Uuid::new_v4(),
        }
    }

    async fn handshake(stream: &mut H2Duplex) {
        write_frame(
            stream,
            &hello(
                PROTOCOL_VERSION,
                env!("CARGO_PKG_VERSION"),
                "https://p1.proxy.example.test",
            ),
        )
        .await
        .unwrap();
        assert!(matches!(
            read_frame::<_, ServerFrame>(stream).await.unwrap(),
            ServerFrame::ServerHello {
                protocol_version: PROTOCOL_VERSION,
                heartbeat_seconds: 5,
                dead_seconds: 15,
                grace_seconds: 30,
                ..
            }
        ));
    }

    #[tokio::test]
    async fn http_connect_rejects_method_path_auth_and_content_type() {
        let cases = [
            (
                Method::GET,
                CONNECT_PATH,
                Some("secret"),
                Some(CONTENT_TYPE),
                StatusCode::METHOD_NOT_ALLOWED,
            ),
            (
                Method::POST,
                "/wrong",
                Some("secret"),
                Some(CONTENT_TYPE),
                StatusCode::NOT_FOUND,
            ),
            (
                Method::POST,
                CONNECT_PATH,
                Some("wrong"),
                Some(CONTENT_TYPE),
                StatusCode::UNAUTHORIZED,
            ),
            (
                Method::POST,
                CONNECT_PATH,
                None,
                Some(CONTENT_TYPE),
                StatusCode::UNAUTHORIZED,
            ),
            (
                Method::POST,
                CONNECT_PATH,
                Some("secret"),
                Some("application/json"),
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
            ),
        ];
        for (method, path, token, content_type, expected) in cases {
            let opened = open(
                crate::spawn_controller(100),
                method,
                path,
                token,
                content_type,
            )
            .await;
            assert_eq!(opened.status, expected);
        }
    }

    #[tokio::test]
    async fn hello_rejects_control_package_and_origin_mismatches() {
        let cases = [
            hello(
                PROTOCOL_VERSION + 1,
                env!("CARGO_PKG_VERSION"),
                "https://p1.proxy.example.test",
            ),
            hello(PROTOCOL_VERSION, "0.0.0", "https://p1.proxy.example.test"),
            hello(
                PROTOCOL_VERSION,
                env!("CARGO_PKG_VERSION"),
                "https://other.proxy.example.test",
            ),
        ];
        for hello in cases {
            let mut opened = connected(crate::spawn_controller(100)).await;
            let stream = opened.stream.as_mut().unwrap();
            write_frame(stream, &hello).await.unwrap();
            assert!(matches!(
                read_frame::<_, ServerFrame>(stream).await.unwrap(),
                ServerFrame::Shutdown { .. }
            ));
        }
    }

    #[tokio::test]
    async fn snapshot_then_generation_gap_resyncs_on_the_same_stream() {
        let controller = crate::spawn_controller(100);
        let mut opened = connected(controller).await;
        let stream = opened.stream.as_mut().unwrap();
        handshake(stream).await;
        write_frame(stream, &ClientFrame::SnapshotStart { base_generation: 0 })
            .await
            .unwrap();
        write_frame(stream, &ClientFrame::SnapshotChunk { rows: Vec::new() })
            .await
            .unwrap();
        write_frame(stream, &ClientFrame::SnapshotEnd { base_generation: 0 })
            .await
            .unwrap();
        assert!(matches!(
            read_frame::<_, ServerFrame>(stream).await.unwrap(),
            ServerFrame::SnapshotAccepted { base_generation: 0 }
        ));

        write_frame(
            stream,
            &ClientFrame::TunnelUp {
                generation: 2,
                row: TunnelRow {
                    registration_id: Uuid::new_v4(),
                    user: "alice".into(),
                    devserver_id: "one".into(),
                    peer_addr: None,
                    connected_at: chrono::Utc::now(),
                },
            },
        )
        .await
        .unwrap();
        assert!(matches!(
            read_frame::<_, ServerFrame>(stream).await.unwrap(),
            ServerFrame::ResyncRequired {
                expected_generation: 1
            }
        ));

        write_frame(stream, &ClientFrame::SnapshotStart { base_generation: 0 })
            .await
            .unwrap();
        write_frame(stream, &ClientFrame::SnapshotEnd { base_generation: 0 })
            .await
            .unwrap();
        assert!(matches!(
            read_frame::<_, ServerFrame>(stream).await.unwrap(),
            ServerFrame::SnapshotAccepted { base_generation: 0 }
        ));
    }

    #[tokio::test(start_paused = true)]
    async fn split_client_frame_survives_an_interleaved_server_command() {
        let mut opened = connected(crate::spawn_controller(100)).await;
        let stream = opened.stream.as_mut().unwrap();
        handshake(stream).await;
        write_frame(stream, &ClientFrame::SnapshotStart { base_generation: 0 })
            .await
            .unwrap();

        let chunk = ClientFrame::SnapshotChunk {
            rows: vec![TunnelRow {
                registration_id: Uuid::new_v4(),
                user: "alice".repeat(256),
                devserver_id: "one".repeat(256),
                peer_addr: None,
                connected_at: chrono::Utc::now(),
            }],
        };
        let payload = serde_json::to_vec(&chunk).unwrap();
        let split = payload.len() / 2;
        stream
            .write_all(&(payload.len() as u32).to_be_bytes())
            .await
            .unwrap();
        stream.write_all(&payload[..split]).await.unwrap();
        tokio::task::yield_now().await;

        tokio::time::advance(crate::HEARTBEAT_INTERVAL).await;
        let ping = read_frame::<_, ServerFrame>(stream).await.unwrap();
        assert!(matches!(ping, ServerFrame::Ping { .. }));

        stream.write_all(&payload[split..]).await.unwrap();
        write_frame(stream, &ClientFrame::SnapshotEnd { base_generation: 0 })
            .await
            .unwrap();
        let mut accepted = false;
        for _ in 0..4 {
            match read_frame::<_, ServerFrame>(stream).await.unwrap() {
                ServerFrame::SnapshotAccepted { base_generation: 0 } => {
                    accepted = true;
                    break;
                }
                ServerFrame::Ping { nonce } => {
                    write_frame(stream, &ClientFrame::Pong { nonce })
                        .await
                        .unwrap();
                }
                frame => panic!("unexpected server frame: {frame:?}"),
            }
        }
        assert!(accepted, "split snapshot chunk was not accepted");
    }

    #[tokio::test]
    async fn out_of_order_snapshot_frame_resyncs_and_duplicate_hello_closes() {
        let mut opened = connected(crate::spawn_controller(100)).await;
        let stream = opened.stream.as_mut().unwrap();
        handshake(stream).await;
        write_frame(
            stream,
            &ClientFrame::AdmissionRequest {
                request_id: Uuid::new_v4(),
                registration_id: Uuid::new_v4(),
                user: "alice".into(),
                devserver_id: "one".into(),
            },
        )
        .await
        .unwrap();
        assert!(matches!(
            read_frame::<_, ServerFrame>(stream).await.unwrap(),
            ServerFrame::ResyncRequired {
                expected_generation: 0
            }
        ));
        let duplicate = TunnelRow {
            registration_id: Uuid::new_v4(),
            user: "alice".into(),
            devserver_id: "one".into(),
            peer_addr: None,
            connected_at: chrono::Utc::now(),
        };
        write_frame(stream, &ClientFrame::SnapshotStart { base_generation: 0 })
            .await
            .unwrap();
        write_frame(
            stream,
            &ClientFrame::SnapshotChunk {
                rows: vec![duplicate.clone(), duplicate],
            },
        )
        .await
        .unwrap();
        assert!(matches!(
            read_frame::<_, ServerFrame>(stream).await.unwrap(),
            ServerFrame::ResyncRequired {
                expected_generation: 0
            }
        ));
        write_frame(
            stream,
            &hello(
                PROTOCOL_VERSION,
                env!("CARGO_PKG_VERSION"),
                "https://p1.proxy.example.test",
            ),
        )
        .await
        .unwrap();
        assert!(matches!(
            read_frame::<_, ServerFrame>(stream).await.unwrap(),
            ServerFrame::Shutdown { .. }
        ));
    }

    #[tokio::test(start_paused = true)]
    async fn snapshot_deadline_is_absolute_while_pongs_keep_the_session_alive() {
        let mut opened = connected(crate::spawn_controller(100)).await;
        let stream = opened.stream.as_mut().unwrap();
        handshake(stream).await;
        for _ in 0..5 {
            tokio::time::advance(crate::HEARTBEAT_INTERVAL).await;
            let nonce = loop {
                if let ServerFrame::Ping { nonce } =
                    read_frame::<_, ServerFrame>(stream).await.unwrap()
                {
                    break nonce;
                }
            };
            write_frame(stream, &ClientFrame::Pong { nonce })
                .await
                .unwrap();
        }
        tokio::time::advance(crate::HEARTBEAT_INTERVAL).await;
        let mut shutdown = false;
        for _ in 0..8 {
            let frame = read_frame::<_, ServerFrame>(stream).await.unwrap();
            if matches!(frame, ServerFrame::Shutdown { .. }) {
                shutdown = true;
                break;
            }
        }
        assert!(
            shutdown,
            "snapshot timeout did not close within eight frames"
        );
    }

    #[test]
    fn cumulative_snapshot_limit_is_checked_without_overflow() {
        assert!(snapshot_rows_fit(MAX_SNAPSHOT_ROWS - 1, 1));
        assert!(!snapshot_rows_fit(MAX_SNAPSHOT_ROWS, 1));
        assert!(!snapshot_rows_fit(usize::MAX, 1));
    }
}
