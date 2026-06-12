//! GET /api/health.

use std::sync::Arc;

use axum::extract::State;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

use crate::indexer::IndexerHealth;
use crate::state::AppState;

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
    /// Random id minted at tenant build. The SPA compares it across
    /// `/ws` reconnects: a changed id = the process was restarted (its
    /// PTYs and in-memory state are gone) and the window reloads
    /// itself instead of going stale.
    instance: String,
    /// Present on workspace tenants; `null` on the workspace-less
    /// terminal tenant (no indexer exists there BY DESIGN) and during
    /// the transient storage-reset swap window.
    indexer: Option<IndexerHealth>,
}

pub async fn api_health(State(state): State<Arc<AppState>>) -> Response {
    // Health means "this process answers" on EVERY tenant. Erroring on
    // a missing indexer made the standalone terminal tenant 503 each
    // time a terminal window's instance probe ran on watch-socket
    // connect — a tower-http ERROR line in the desktop log per
    // Cmd+T / Cmd+Shift+N. The indexer block is diagnostics, not a
    // liveness gate; absent simply means "no indexer here right now".
    let indexer = state.try_indexer().ok().map(|ix| ix.health_snapshot());
    Json(HealthResponse {
        status: "ok",
        instance: state.instance_id.clone(),
        indexer,
    })
    .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::indexer::{IndexerHealth, IndexerHealthStatus};

    #[test]
    fn health_answers_without_an_indexer_on_workspace_less_tenants() {
        // The standalone terminal tenant has no indexer by design; the
        // route must answer 200 with a null block, not 503 (which made
        // tower-http log an ERROR per terminal-window instance probe).
        let value = serde_json::to_value(HealthResponse {
            status: "ok",
            instance: "boot-term".to_string(),
            indexer: None,
        })
        .unwrap();
        assert_eq!(value["status"], "ok");
        assert_eq!(value["instance"], "boot-term");
        assert!(value["indexer"].is_null());
    }

    #[test]
    fn health_response_serializes_indexer_block() {
        let value = serde_json::to_value(HealthResponse {
            status: "ok",
            instance: "boot-abc123".to_string(),
            indexer: Some(IndexerHealth {
                status: IndexerHealthStatus::Settling,
                queue_depth: 2,
                last_event_at: Some(1_700_000_000),
                last_settled_at: Some(1_699_999_999),
                coalesced_rebuild: false,
            }),
        })
        .unwrap();

        assert_eq!(value["status"], "ok");
        // Wire pin: the SPA's restart-reload check reads `instance`.
        assert_eq!(value["instance"], "boot-abc123");
        assert_eq!(value["indexer"]["status"], "settling");
        assert_eq!(value["indexer"]["queue_depth"], 2);
        assert_eq!(value["indexer"]["last_event_at"], 1_700_000_000);
        assert_eq!(value["indexer"]["last_settled_at"], 1_699_999_999);
        assert_eq!(value["indexer"]["coalesced_rebuild"], false);
    }
}
