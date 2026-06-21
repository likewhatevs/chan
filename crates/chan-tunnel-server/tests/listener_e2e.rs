//! End-to-end tests for the tunnel listener.
//!
//! Workspaces a real `chan-tunnel-client` against `serve_tunnel_listener`
//! over a localhost socket, exercising the auth gates (base scope, cap)
//! that unit tests can only exercise in pieces.
//! The client dials h2c (`http://...`); no TLS plumbing on this
//! side, since TLS is nginx's job in production.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chan_tunnel_client::{dial, ClientConfig, ClientError};
use chan_tunnel_proto::error_code;
use chan_tunnel_server::{
    serve_tunnel_listener, Registry, ServerError, Validated, Validator, TUNNEL_SCOPE,
};
use tokio::net::TcpListener;
use url::Url;
use uuid::Uuid;

/// Validator stub. One expected token, one canned `Validated`.
/// Anything else is `InvalidToken`.
struct StubValidator {
    expected_token: String,
    username: String,
    scopes: Vec<String>,
}

#[async_trait]
impl Validator for StubValidator {
    async fn validate(&self, token: &str) -> Result<Validated, ServerError> {
        if token != self.expected_token {
            return Err(ServerError::InvalidToken);
        }
        Ok(Validated {
            user_id: Uuid::nil(),
            username: self.username.clone(),
            scopes: self.scopes.clone(),
        })
    }
}

struct Harness {
    port: u16,
    registry: Arc<Registry>,
    /// Kept so the listener task lives at least as long as the test.
    /// `drop(Harness)` aborts via the JoinHandle's Drop.
    _task: tokio::task::JoinHandle<()>,
}

async fn spawn_listener(validator: Arc<dyn Validator>, max_workspaces_per_user: usize) -> Harness {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind 0");
    let port = listener.local_addr().unwrap().port();
    let registry = Registry::new();
    let registry_for_task = registry.clone();
    let _task = tokio::spawn(async move {
        let _ = serve_tunnel_listener(
            listener,
            validator,
            registry_for_task,
            max_workspaces_per_user,
        )
        .await;
    });
    Harness {
        port,
        registry,
        _task,
    }
}

fn cfg(port: u16, token: &str, workspace: &str) -> ClientConfig {
    ClientConfig {
        tunnel_url: Url::parse(&format!("http://127.0.0.1:{port}/v1/tunnel"))
            .expect("hard-coded url is valid"),
        token: token.into(),
        workspace: workspace.into(),
        client_version: "chan/test".into(),
        initial_backoff: Duration::from_millis(50),
        max_backoff: Duration::from_secs(1),
        dial_timeout: Duration::from_secs(5),
        events: None,
        proxy: None,
        max_concurrent_substreams: chan_tunnel_client::DEFAULT_MAX_CONCURRENT_SUBSTREAMS,
    }
}

/// Spin-wait briefly for the listener to insert the registration.
/// The client's `dial` returns the moment it reads HelloAck, but
/// the server side does `register_with_cap` a few statements later;
/// without a wait, tests querying the registry can race that gap.
async fn wait_registered(reg: &Registry, user: &str, workspace: &str) -> bool {
    for _ in 0..100 {
        if reg.get(user, workspace).is_some() {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    false
}

#[tokio::test]
async fn happy_path_private_workspace() {
    let validator = Arc::new(StubValidator {
        expected_token: "good".into(),
        username: "alice".into(),
        scopes: vec![TUNNEL_SCOPE.into()],
    });
    let h = spawn_listener(validator, 0).await;
    let (reg, _yconn) = dial(&cfg(h.port, "good", "notes")).await.expect("dial ok");
    assert_eq!(reg.user, "alice");
    assert_eq!(reg.workspace, "notes");
    assert_eq!(reg.prefix, "/notes");
    assert!(wait_registered(&h.registry, "alice", "notes").await);
    let workspaces = h.registry.list_workspaces_for("alice");
    assert_eq!(workspaces.len(), 1);
}

#[tokio::test]
async fn invalid_token_returns_401() {
    let validator = Arc::new(StubValidator {
        expected_token: "good".into(),
        username: "alice".into(),
        scopes: vec![TUNNEL_SCOPE.into()],
    });
    let h = spawn_listener(validator, 0).await;
    let err = dial(&cfg(h.port, "bad", "notes"))
        .await
        .map(|_| ())
        .expect_err("bad token should fail");
    let msg = err.to_string();
    // Client's dial layer translates 401 into "unauthorized (bad
    // token)" before the substream handshake even starts.
    assert!(msg.to_lowercase().contains("unauthorized"), "got: {msg}");
    assert!(h.registry.list_workspaces_for("alice").is_empty());
}

#[tokio::test]
async fn missing_base_scope_returns_403() {
    // Token authenticates but lacks `TUNNEL_SCOPE` entirely.
    let validator = Arc::new(StubValidator {
        expected_token: "good".into(),
        username: "alice".into(),
        scopes: vec![],
    });
    let h = spawn_listener(validator, 0).await;
    let err = dial(&cfg(h.port, "good", "notes"))
        .await
        .map(|_| ())
        .expect_err("missing tunnel scope should fail");
    let msg = err.to_string();
    assert!(msg.to_lowercase().contains("forbidden"), "got: {msg}");
    assert!(h.registry.list_workspaces_for("alice").is_empty());
}

#[tokio::test]
async fn per_user_cap_blocks_third_workspace() {
    let validator = Arc::new(StubValidator {
        expected_token: "good".into(),
        username: "alice".into(),
        scopes: vec![TUNNEL_SCOPE.into()],
    });
    let h = spawn_listener(validator, 2).await;
    let (_a, _y1) = dial(&cfg(h.port, "good", "d1")).await.expect("first dial");
    assert!(wait_registered(&h.registry, "alice", "d1").await);
    let (_b, _y2) = dial(&cfg(h.port, "good", "d2")).await.expect("second dial");
    assert!(wait_registered(&h.registry, "alice", "d2").await);
    let err = dial(&cfg(h.port, "good", "d3"))
        .await
        .map(|_| ())
        .expect_err("third dial should hit the cap");
    // Refusal is in pre_ack; the server emits a structured
    // HelloAck::Refused so the client surfaces code + message.
    match err {
        ClientError::RemoteRefusal {
            ref code,
            ref message,
        } => {
            assert_eq!(code, error_code::TOO_MANY_WORKSPACES);
            assert!(message.contains("alice"), "got: {message}");
        }
        other => panic!("expected RemoteRefusal, got {other:?}"),
    }
    let workspaces: Vec<_> = h
        .registry
        .list_workspaces_for("alice")
        .into_iter()
        .map(|d| d.workspace.as_ref().to_string())
        .collect();
    assert_eq!(workspaces, vec!["d1".to_string(), "d2".to_string()]);
}

#[tokio::test]
async fn reconnect_evicts_previous_registration() {
    // Same user + workspace registers twice. The second dial succeeds
    // (chan serve restart reclaiming its slot) and the first
    // registration is replaced.
    let validator = Arc::new(StubValidator {
        expected_token: "good".into(),
        username: "alice".into(),
        scopes: vec![TUNNEL_SCOPE.into()],
    });
    let h = spawn_listener(validator, 0).await;
    let (_a, _y1) = dial(&cfg(h.port, "good", "notes"))
        .await
        .expect("first dial");
    assert!(wait_registered(&h.registry, "alice", "notes").await);
    let first_at = h.registry.get("alice", "notes").unwrap().connected_at;
    // Tiny pause so the timestamps differ unambiguously.
    tokio::time::sleep(Duration::from_millis(20)).await;
    let (_b, _y2) = dial(&cfg(h.port, "good", "notes"))
        .await
        .expect("second dial");
    // Wait for the new registration to land; it has a strictly
    // later connected_at than the first one.
    for _ in 0..100 {
        if let Some(h2) = h.registry.get("alice", "notes") {
            if h2.connected_at > first_at {
                return;
            }
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("second registration did not supersede the first");
}
