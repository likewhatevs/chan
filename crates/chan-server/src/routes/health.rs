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
    indexer: IndexerHealth,
    terminal_event_watcher: TerminalEventWatcherHealth,
}

#[derive(Debug, Serialize)]
struct TerminalEventWatcherHealth {
    dropped_events: u64,
}

pub async fn api_health(State(state): State<Arc<AppState>>) -> Response {
    let indexer = match state.try_indexer() {
        Ok(indexer) => indexer,
        Err(e) => return err_state(&e),
    };
    Json(HealthResponse {
        status: "ok",
        indexer: indexer.health_snapshot(),
        terminal_event_watcher: TerminalEventWatcherHealth {
            dropped_events: state.terminal_sessions.watcher_dropped_events(),
        },
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
            terminal_event_watcher: TerminalEventWatcherHealth { dropped_events: 3 },
        })
        .unwrap();

        assert_eq!(value["status"], "ok");
        assert_eq!(value["indexer"]["status"], "settling");
        assert_eq!(value["indexer"]["queue_depth"], 2);
        assert_eq!(value["indexer"]["last_event_at"], 1_700_000_000);
        assert_eq!(value["indexer"]["last_settled_at"], 1_699_999_999);
        assert_eq!(value["indexer"]["coalesced_rebuild"], false);
        assert_eq!(value["terminal_event_watcher"]["dropped_events"], 3);
    }
}
