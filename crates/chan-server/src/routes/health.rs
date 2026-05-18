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
    indexer: IndexerHealth,
}

pub async fn api_health(State(state): State<Arc<AppState>>) -> Response {
    Json(HealthResponse {
        status: "ok",
        indexer: state.indexer().health_snapshot(),
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
        assert_eq!(value["indexer"]["status"], "settling");
        assert_eq!(value["indexer"]["queue_depth"], 2);
        assert_eq!(value["indexer"]["last_event_at"], 1_700_000_000);
        assert_eq!(value["indexer"]["last_settled_at"], 1_699_999_999);
        assert_eq!(value["indexer"]["coalesced_rebuild"], false);
    }
}
