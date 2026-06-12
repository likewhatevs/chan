//! GET /api/health.

use std::sync::Arc;

use axum::extract::State;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

use crate::indexer::IndexerHealth;
use crate::{error::err_state, state::AppState};

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
    /// Random id minted at tenant build. The SPA compares it across
    /// `/ws` reconnects: a changed id = the process was restarted (its
    /// PTYs and in-memory state are gone) and the window reloads
    /// itself instead of going stale.
    instance: String,
    indexer: IndexerHealth,
}

pub async fn api_health(State(state): State<Arc<AppState>>) -> Response {
    let indexer = match state.try_indexer() {
        Ok(indexer) => indexer,
        Err(e) => return err_state(&e),
    };
    Json(HealthResponse {
        status: "ok",
        instance: state.instance_id.clone(),
        indexer: indexer.health_snapshot(),
    })
    .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::indexer::{IndexerHealth, IndexerHealthStatus};

    #[test]
    fn health_response_serializes_indexer_block() {
        let value = serde_json::to_value(HealthResponse {
            status: "ok",
            instance: "boot-abc123".to_string(),
            indexer: IndexerHealth {
                status: IndexerHealthStatus::Settling,
                queue_depth: 2,
                last_event_at: Some(1_700_000_000),
                last_settled_at: Some(1_699_999_999),
                coalesced_rebuild: false,
            },
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
