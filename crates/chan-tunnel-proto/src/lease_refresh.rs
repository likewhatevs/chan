use serde::{Deserialize, Serialize};

/// One-shot request on a client-opened yamux stream. The PAT remains owned
/// by the live devserver client and is discarded by the proxy after the
/// identity validation call returns.
#[derive(Serialize, Deserialize)]
pub struct LeaseRefreshRequest {
    pub token: String,
}

impl std::fmt::Debug for LeaseRefreshRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LeaseRefreshRequest")
            .field("token", &"[REDACTED]")
            .finish()
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LeaseRefreshResponse {
    Refreshed,
    Refused { message: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_debug_never_exposes_the_pat() {
        let request = LeaseRefreshRequest {
            token: "chan_pat_highly-sensitive".into(),
        };
        let debug = format!("{request:?}");
        assert!(debug.contains("[REDACTED]"));
        assert!(!debug.contains("chan_pat"));
        assert!(!debug.contains("highly-sensitive"));
    }
}
