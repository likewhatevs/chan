#![forbid(unsafe_code)]

use std::net::SocketAddr;

use chrono::{DateTime, Utc};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use url::Url;
use uuid::Uuid;

pub const PROTOCOL_VERSION: u16 = 1;
pub const CONTENT_TYPE: &str = "application/x-chan-devserver-control+json; version=1";
pub const MAX_FRAME_BYTES: usize = 1024 * 1024;
pub const MAX_SNAPSHOT_CHUNK_ROWS: usize = 128;
pub const MAX_SNAPSHOT_ROWS: usize = 100_000;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProxyId(String);

impl ProxyId {
    pub fn parse(raw: impl Into<String>) -> Result<Self, ValidationError> {
        let raw = raw.into();
        if raw.is_empty()
            || raw.len() > 63
            || !raw
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
            || raw.starts_with('-')
            || raw.ends_with('-')
        {
            return Err(ValidationError::ProxyId);
        }
        Ok(Self(raw))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CanonicalOrigin(String);

impl CanonicalOrigin {
    pub fn parse(raw: &str) -> Result<Self, ValidationError> {
        let url: Url = raw.parse().map_err(|_| ValidationError::Origin)?;
        if !matches!(url.scheme(), "http" | "https")
            || url.host_str().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.path() != "/"
            || url.query().is_some()
            || url.fragment().is_some()
        {
            return Err(ValidationError::Origin);
        }
        Ok(Self(url.origin().ascii_serialization()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone)]
pub struct ProxyOriginTemplate {
    raw: String,
}

impl ProxyOriginTemplate {
    pub fn parse(raw: &str) -> Result<Self, ValidationError> {
        if raw.matches("{proxy_id}").count() != 1 {
            return Err(ValidationError::OriginTemplate);
        }
        let probe = raw.replace("{proxy_id}", "proxy");
        CanonicalOrigin::parse(&probe).map_err(|_| ValidationError::OriginTemplate)?;
        Ok(Self {
            raw: raw.to_string(),
        })
    }

    pub fn expand(&self, proxy_id: &ProxyId) -> Result<CanonicalOrigin, ValidationError> {
        CanonicalOrigin::parse(&self.raw.replace("{proxy_id}", proxy_id.as_str()))
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ValidationError {
    #[error("proxy id must be one lowercase DNS label")]
    ProxyId,
    #[error("proxy URL must be a canonical http(s) origin")]
    Origin,
    #[error("proxy URL template must be a canonical origin with one {{proxy_id}} placeholder")]
    OriginTemplate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TunnelRow {
    pub registration_id: Uuid,
    pub user: String,
    pub devserver_id: String,
    pub peer_addr: Option<SocketAddr>,
    pub connected_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientFrame {
    ClientHello {
        protocol_version: u16,
        package_version: String,
        proxy_id: ProxyId,
        proxy_base_url: CanonicalOrigin,
        boot_id: Uuid,
    },
    SnapshotStart {
        base_generation: u64,
    },
    SnapshotChunk {
        rows: Vec<TunnelRow>,
    },
    SnapshotEnd {
        base_generation: u64,
    },
    TunnelUp {
        generation: u64,
        row: TunnelRow,
    },
    TunnelDown {
        generation: u64,
        registration_id: Uuid,
    },
    AdmissionRequest {
        request_id: Uuid,
        registration_id: Uuid,
        user: String,
        devserver_id: String,
    },
    AdmissionCancel {
        request_id: Uuid,
        registration_id: Uuid,
    },
    CommandResult {
        command_id: Uuid,
        killed: Vec<Uuid>,
        missing: Vec<Uuid>,
        failed: Vec<Uuid>,
    },
    Pong {
        nonce: u64,
    },
}

impl ClientFrame {
    pub fn validate(&self) -> Result<(), FrameError> {
        if let Self::SnapshotChunk { rows } = self {
            if rows.len() > MAX_SNAPSHOT_CHUNK_ROWS {
                return Err(FrameError::SnapshotChunkTooLarge(rows.len()));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdmissionDecision {
    Admit,
    AtCapacity,
    ControlWarming,
    Stale,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerFrame {
    ServerHello {
        protocol_version: u16,
        package_version: String,
        heartbeat_seconds: u64,
        dead_seconds: u64,
        grace_seconds: u64,
    },
    SnapshotAccepted {
        base_generation: u64,
    },
    FleetReady,
    AdmissionDecision {
        request_id: Uuid,
        registration_id: Uuid,
        decision: AdmissionDecision,
    },
    KillRegistrations {
        command_id: Uuid,
        registration_ids: Vec<Uuid>,
    },
    ResyncRequired {
        expected_generation: u64,
    },
    Ping {
        nonce: u64,
    },
    Shutdown {
        reason: String,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum FrameError {
    #[error("control frame length is zero")]
    ZeroLength,
    #[error("control frame is too large: {0} bytes")]
    TooLarge(usize),
    #[error("snapshot chunk exceeds {MAX_SNAPSHOT_CHUNK_ROWS} rows: {0}")]
    SnapshotChunkTooLarge(usize),
    #[error("control frame I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("control frame JSON: {0}")]
    Json(#[from] serde_json::Error),
}

pub async fn read_frame<R, T>(reader: &mut R) -> Result<T, FrameError>
where
    R: AsyncRead + Unpin,
    T: DeserializeOwned,
{
    let len = reader.read_u32().await? as usize;
    if len == 0 {
        return Err(FrameError::ZeroLength);
    }
    if len > MAX_FRAME_BYTES {
        return Err(FrameError::TooLarge(len));
    }
    let mut payload = vec![0; len];
    reader.read_exact(&mut payload).await?;
    Ok(serde_json::from_slice(&payload)?)
}

pub async fn write_frame<W, T>(writer: &mut W, frame: &T) -> Result<(), FrameError>
where
    W: AsyncWrite + Unpin,
    T: Serialize,
{
    let payload = serde_json::to_vec(frame)?;
    if payload.is_empty() {
        return Err(FrameError::ZeroLength);
    }
    if payload.len() > MAX_FRAME_BYTES {
        return Err(FrameError::TooLarge(payload.len()));
    }
    writer.write_u32(payload.len() as u32).await?;
    writer.write_all(&payload).await?;
    writer.flush().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_ids_and_origins() {
        assert_eq!(ProxyId::parse("p1").unwrap().as_str(), "p1");
        for bad in ["", "P1", "-p1", "p1-", "p_1", "a.b"] {
            assert!(ProxyId::parse(bad).is_err(), "accepted {bad:?}");
        }
        assert_eq!(
            CanonicalOrigin::parse("https://p1.example.test:443")
                .unwrap()
                .as_str(),
            "https://p1.example.test"
        );
        for bad in [
            "ftp://p1.example.test",
            "https://user@p1.example.test",
            "https://p1.example.test/path",
            "https://p1.example.test/?query=1",
        ] {
            assert!(CanonicalOrigin::parse(bad).is_err(), "accepted {bad:?}");
        }
        let template = ProxyOriginTemplate::parse("https://{proxy_id}.proxy.example.test").unwrap();
        assert_eq!(
            template
                .expand(&ProxyId::parse("p1").unwrap())
                .unwrap()
                .as_str(),
            "https://p1.proxy.example.test"
        );
        for bad in [
            "https://proxy.example.test",
            "https://{proxy_id}.{proxy_id}.example.test",
            "https://example.test/{proxy_id}",
        ] {
            assert!(ProxyOriginTemplate::parse(bad).is_err(), "accepted {bad:?}");
        }
    }

    #[tokio::test]
    async fn every_frame_round_trips() {
        let registration_id = Uuid::new_v4();
        let request_id = Uuid::new_v4();
        let command_id = Uuid::new_v4();
        let boot_id = Uuid::new_v4();
        let row = TunnelRow {
            registration_id,
            user: "alice".into(),
            devserver_id: "devserver".into(),
            peer_addr: Some("127.0.0.1:7001".parse().unwrap()),
            connected_at: Utc::now(),
        };
        let clients = vec![
            ClientFrame::ClientHello {
                protocol_version: PROTOCOL_VERSION,
                package_version: "0.73.0".into(),
                proxy_id: ProxyId::parse("p1").unwrap(),
                proxy_base_url: CanonicalOrigin::parse("https://p1.example.test").unwrap(),
                boot_id,
            },
            ClientFrame::SnapshotStart { base_generation: 7 },
            ClientFrame::SnapshotChunk {
                rows: vec![row.clone()],
            },
            ClientFrame::SnapshotEnd { base_generation: 7 },
            ClientFrame::TunnelUp {
                generation: 8,
                row: row.clone(),
            },
            ClientFrame::TunnelDown {
                generation: 9,
                registration_id,
            },
            ClientFrame::AdmissionRequest {
                request_id,
                registration_id,
                user: "alice".into(),
                devserver_id: "devserver".into(),
            },
            ClientFrame::AdmissionCancel {
                request_id,
                registration_id,
            },
            ClientFrame::CommandResult {
                command_id,
                killed: vec![registration_id],
                missing: Vec::new(),
                failed: Vec::new(),
            },
            ClientFrame::Pong { nonce: 42 },
        ];
        for frame in clients {
            let (mut client, mut server) = tokio::io::duplex(4096);
            write_frame(&mut client, &frame).await.unwrap();
            let decoded: ClientFrame = read_frame(&mut server).await.unwrap();
            assert_eq!(decoded, frame);
        }

        let servers = vec![
            ServerFrame::ServerHello {
                protocol_version: PROTOCOL_VERSION,
                package_version: "0.73.0".into(),
                heartbeat_seconds: 5,
                dead_seconds: 15,
                grace_seconds: 30,
            },
            ServerFrame::SnapshotAccepted { base_generation: 7 },
            ServerFrame::FleetReady,
            ServerFrame::AdmissionDecision {
                request_id,
                registration_id,
                decision: AdmissionDecision::Admit,
            },
            ServerFrame::KillRegistrations {
                command_id,
                registration_ids: vec![registration_id],
            },
            ServerFrame::ResyncRequired {
                expected_generation: 8,
            },
            ServerFrame::Ping { nonce: 42 },
            ServerFrame::Shutdown {
                reason: "test".into(),
            },
        ];
        for frame in servers {
            let (mut client, mut server) = tokio::io::duplex(4096);
            write_frame(&mut client, &frame).await.unwrap();
            let decoded: ServerFrame = read_frame(&mut server).await.unwrap();
            assert_eq!(decoded, frame);
        }
    }

    #[tokio::test]
    async fn rejects_bad_lengths_truncation_and_malformed_json() {
        let (mut client, mut server) = tokio::io::duplex(16);
        client.write_u32(0).await.unwrap();
        assert!(matches!(
            read_frame::<_, ServerFrame>(&mut server).await,
            Err(FrameError::ZeroLength)
        ));

        let (mut client, mut server) = tokio::io::duplex(16);
        client
            .write_u32((MAX_FRAME_BYTES + 1) as u32)
            .await
            .unwrap();
        assert!(matches!(
            read_frame::<_, ServerFrame>(&mut server).await,
            Err(FrameError::TooLarge(_))
        ));

        let (mut client, mut server) = tokio::io::duplex(16);
        client.write_u32(4).await.unwrap();
        client.write_all(b"{}").await.unwrap();
        drop(client);
        assert!(matches!(
            read_frame::<_, ServerFrame>(&mut server).await,
            Err(FrameError::Io(error)) if error.kind() == std::io::ErrorKind::UnexpectedEof
        ));

        let (mut client, mut server) = tokio::io::duplex(16);
        client.write_u32(1).await.unwrap();
        client.write_all(b"{").await.unwrap();
        assert!(matches!(
            read_frame::<_, ServerFrame>(&mut server).await,
            Err(FrameError::Json(_))
        ));
    }

    #[test]
    fn snapshot_chunks_are_bounded() {
        let row = TunnelRow {
            registration_id: Uuid::new_v4(),
            user: "alice".into(),
            devserver_id: "devserver".into(),
            peer_addr: None,
            connected_at: Utc::now(),
        };
        let frame = ClientFrame::SnapshotChunk {
            rows: vec![row; MAX_SNAPSHOT_CHUNK_ROWS + 1],
        };
        assert!(matches!(
            frame.validate(),
            Err(FrameError::SnapshotChunkTooLarge(_))
        ));
    }
}
