//! systacean-35: `GET /api/mentions?q=<prefix>&limit=<int>`.
//!
//! Returns prefix-matched mention handles from the per-drive
//! graph DB. Unblocks `-a-70`'s editor mention completion gap
//! (the editor previously queried only the contact list; this
//! exposes the broader corpus of `@@<Name>` references across
//! all indexed markdown).
//!
//! Source: `chan_drive::GraphView::mentions()` — runs a single
//! SQL aggregation over the graph's mention edges (parallel to
//! `tags()`). Returns names sorted by count desc + label asc;
//! this route filters by case-insensitive prefix + caps at the
//! query's `limit` (default 10, mirroring `/api/contacts`).

use std::sync::Arc;

use axum::extract::{Query, State};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::error::err_from;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct MentionsQuery {
    /// Case-insensitive prefix to match against mention labels.
    /// Empty / omitted matches all mentions (capped at `limit`).
    #[serde(default)]
    pub q: String,
    /// Cap on returned entries. Defaults to 10. Clamped to
    /// `1..=200` server-side so a runaway client can't pull the
    /// whole corpus in one call.
    #[serde(default)]
    pub limit: Option<u32>,
}

#[derive(Serialize)]
pub struct MentionItem {
    /// Mention label WITHOUT the `@@` sigil (e.g. `"Architect"`,
    /// `"Alex"`). The SPA reads this directly to populate the
    /// editor's mention-completion dropdown.
    pub label: String,
}

const DEFAULT_LIMIT: u32 = 10;
const MAX_LIMIT: u32 = 200;

/// `GET /api/mentions` — return prefix-matched mention labels.
pub async fn api_get_mentions(
    State(state): State<Arc<AppState>>,
    Query(params): Query<MentionsQuery>,
) -> Response {
    let drive = state.drive().clone();
    let q = params.q.clone();
    let limit = params.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT) as usize;
    let result =
        tokio::task::spawn_blocking(move || -> Result<Vec<MentionItem>, chan_drive::ChanError> {
            let graph = drive.graph()?;
            let mentions = graph.mentions()?;
            let prefix = q.to_lowercase();
            let filtered: Vec<MentionItem> = mentions
                .into_iter()
                .filter(|m| {
                    if prefix.is_empty() {
                        true
                    } else {
                        m.name.to_lowercase().starts_with(&prefix)
                    }
                })
                .take(limit)
                .map(|m| MentionItem {
                    // Compose with the `@@` sigil so the SPA can
                    // splice the result straight into the editor
                    // buffer without re-prepending. The bare name
                    // is one strip away if a consumer wants it.
                    label: format!("@@{}", m.name),
                })
                .collect();
            Ok(filtered)
        })
        .await;
    match result {
        Ok(Ok(items)) => Json(items).into_response(),
        Ok(Err(e)) => err_from(&e),
        Err(join) => err_from(&chan_drive::ChanError::Io(join.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limit_clamps_to_bounds() {
        // systacean-35: client-supplied limit gets clamped to
        // `1..=200`. A `0` would otherwise return an empty
        // (useless) result; a giant N would let one request
        // pull the entire mention corpus.
        let q = MentionsQuery {
            q: String::new(),
            limit: Some(0),
        };
        // Match the clamp expression in `api_get_mentions`.
        let clamped = q.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT) as usize;
        assert_eq!(clamped, 1);

        let q_big = MentionsQuery {
            q: String::new(),
            limit: Some(10_000),
        };
        let clamped_big = q_big.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT) as usize;
        assert_eq!(clamped_big, MAX_LIMIT as usize);

        let q_default = MentionsQuery {
            q: String::new(),
            limit: None,
        };
        let clamped_default = q_default.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT) as usize;
        assert_eq!(clamped_default, DEFAULT_LIMIT as usize);
    }
}
