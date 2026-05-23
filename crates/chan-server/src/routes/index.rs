//! systacean-7: per-drive semantic-search state + enablement.
//!
//! Four endpoints under `/api/index/semantic/`:
//!
//! * `GET /state` — model + drive preference snapshot.
//! * `POST /download` — synchronously fetch the model into the
//!   per-machine cache (hf-hub). v1 is blocking; progress-streaming
//!   is a follow-up.
//! * `POST /enable` — flip the drive's `semantic_enabled` to true.
//!   Refuses with 409 if the model isn't on disk.
//! * `POST /disable` — flip back to BM25-only.
//!
//! Whole module gated on `embeddings` — the surface is meaningless
//! without the candle stack. `lib.rs::router()` mirrors that gate
//! when wiring the routes.
//!
//! Companion CLI: `chan index download-model | enable-semantic |
//! disable-semantic | status` (see `crates/chan/src/main.rs`).
//! Settings UI lands in `fullstack-a-21` against this contract.

#![cfg(feature = "embeddings")]

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chan_drive::index::embeddings::{global_models_dir, repo_dir_name, resolve_model, Embedder};
use serde::Serialize;

use crate::error::err_from;
use crate::state::AppState;

/// Snapshot of the per-drive semantic-search state. Settings UI +
/// `chan index status` consume this. Shape is stable for `--json`
/// scripting; new fields land as additive options.
#[derive(Debug, Clone, Serialize)]
pub struct SemanticState {
    /// "bm25" or "hybrid". Derived from the drive's
    /// `semantic_enabled` flag AND whether the model is on disk —
    /// the field is "hybrid" only when BOTH are true.
    pub mode: &'static str,
    /// True when the model files are laid out at the resolver's
    /// expected path (refs/main + complete snapshot trio). False
    /// when the model hasn't been downloaded.
    pub model_present: bool,
    /// Drive-configured model id (`IndexConfig::model`).
    pub model_name: String,
    /// Resolver's expected path under `global_models_dir()`. Stable
    /// regardless of presence — useful for diagnostics ("look at X
    /// to confirm the download landed").
    pub model_path: String,
    /// Total bytes occupied by the model on disk. `None` when
    /// `model_present` is false (no files to measure).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_size_bytes: Option<u64>,
    /// Per-drive opt-in flag (`IndexConfig::semantic_enabled`).
    /// Independent from `model_present`: a drive can be opted in
    /// without a downloaded model (mode falls back to bm25 until
    /// the model lands; enable refuses to flip without the model
    /// so this state only arises if the model is deleted out from
    /// under us).
    pub semantic_enabled: bool,
}

/// Recursive size of every regular file under `dir`. Used for the
/// `model_size_bytes` reporting field; sums the snapshots + refs
/// trio plus any other on-disk artifacts hf-hub left behind.
fn dir_total_size(dir: &std::path::Path) -> u64 {
    fn walk(dir: &std::path::Path, total: &mut u64) {
        let Ok(it) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in it.flatten() {
            let Ok(ft) = entry.file_type() else {
                continue;
            };
            if ft.is_dir() {
                walk(&entry.path(), total);
            } else if ft.is_file() {
                if let Ok(meta) = entry.metadata() {
                    *total += meta.len();
                }
            }
        }
    }
    let mut total = 0;
    walk(dir, &mut total);
    total
}

fn build_state(drive: &chan_drive::Drive) -> Result<SemanticState, chan_drive::ChanError> {
    let model_name = drive.semantic_model()?;
    let semantic_enabled = drive.semantic_enabled()?;
    let expected_dir = global_models_dir().join(repo_dir_name(&model_name));
    let model_present = resolve_model(&model_name).is_ok();
    let model_size_bytes = if model_present {
        Some(dir_total_size(&expected_dir))
    } else {
        None
    };
    // Mode is "hybrid" only when both the user opted in AND the
    // model is downloaded. A flipped-on flag with no model on disk
    // still serves bm25 (defensive — `enable` refuses to set this
    // shape, but a model deleted out from under us would otherwise
    // mis-report).
    let mode = if semantic_enabled && model_present {
        "hybrid"
    } else {
        "bm25"
    };
    Ok(SemanticState {
        mode,
        model_present,
        model_name,
        model_path: expected_dir.to_string_lossy().into_owned(),
        model_size_bytes,
        semantic_enabled,
    })
}

/// `GET /api/index/semantic/state`. Read-only snapshot.
pub async fn api_semantic_state(State(state): State<Arc<AppState>>) -> Response {
    let drive = state.drive();
    match tokio::task::spawn_blocking(move || build_state(&drive)).await {
        Ok(Ok(s)) => Json(s).into_response(),
        Ok(Err(e)) => err_from(&e),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("semantic state task panicked: {e}"),
        )
            .into_response(),
    }
}

/// Structured error payload for the 409 returned by `enable` when
/// the model isn't on disk. Mirrors `EmbedError::ModelNotDownloaded`
/// fields so the SPA can render the same hint as the CLI.
#[derive(Debug, Clone, Serialize)]
struct ModelNotDownloadedBody {
    error: &'static str,
    model_id: String,
    expected_dir: String,
    download_endpoint: &'static str,
}

/// `POST /api/index/semantic/enable`. Flip the drive to Hybrid.
/// Refuses with 409 if the model isn't on disk; payload carries
/// the structured `ModelNotDownloaded` hint pointing the caller at
/// the `/download` endpoint.
pub async fn api_semantic_enable(State(state): State<Arc<AppState>>) -> Response {
    let drive = state.drive();
    match tokio::task::spawn_blocking(move || {
        let model_name = match drive.semantic_model() {
            Ok(m) => m,
            Err(e) => return err_from(&e),
        };
        if let Err(e) = resolve_model(&model_name) {
            let expected_dir = match &e {
                chan_drive::index::embeddings::EmbedError::ModelNotDownloaded {
                    expected_dir,
                    ..
                } => expected_dir.to_string_lossy().into_owned(),
                _ => global_models_dir()
                    .join(repo_dir_name(&model_name))
                    .to_string_lossy()
                    .into_owned(),
            };
            return (
                StatusCode::CONFLICT,
                Json(ModelNotDownloadedBody {
                    error: "model_not_downloaded",
                    model_id: model_name,
                    expected_dir,
                    download_endpoint: "/api/index/semantic/download",
                }),
            )
                .into_response();
        }
        if let Err(e) = drive.set_semantic_enabled(true) {
            return err_from(&e);
        }
        match build_state(&drive) {
            Ok(s) => Json(s).into_response(),
            Err(e) => err_from(&e),
        }
    })
    .await
    {
        Ok(response) => response,
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("semantic enable task panicked: {e}"),
        )
            .into_response(),
    }
}

/// `POST /api/index/semantic/disable`. Always succeeds; idempotent
/// at the `set_semantic_enabled` layer (no-op when already off).
pub async fn api_semantic_disable(State(state): State<Arc<AppState>>) -> Response {
    let drive = state.drive();
    match tokio::task::spawn_blocking(move || {
        if let Err(e) = drive.set_semantic_enabled(false) {
            return err_from(&e);
        }
        match build_state(&drive) {
            Ok(s) => Json(s).into_response(),
            Err(e) => err_from(&e),
        }
    })
    .await
    {
        Ok(response) => response,
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("semantic disable task panicked: {e}"),
        )
            .into_response(),
    }
}

/// `POST /api/index/semantic/download`. Synchronously fetches the
/// model into `global_models_dir()` via hf-hub. Returns the
/// post-download state on success.
///
/// v1 is blocking from the caller's perspective: the response
/// arrives when the download completes (or fails). Progress-event
/// streaming via the watcher channel is a follow-up — see the
/// task tail's "deferred to follow-up" note. The blocking work
/// runs on a Tokio blocking thread so it doesn't tie up the
/// async runtime.
pub async fn api_semantic_download(State(state): State<Arc<AppState>>) -> Response {
    let drive = state.drive();
    let result = tokio::task::spawn_blocking(move || {
        let model_name = match drive.semantic_model() {
            Ok(m) => m,
            Err(e) => return err_from(&e),
        };
        let cache_dir = global_models_dir();
        if let Err(e) = std::fs::create_dir_all(&cache_dir) {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("creating model cache {}: {e}", cache_dir.display()),
            )
                .into_response();
        }
        if let Err(e) = Embedder::open(&model_name, &cache_dir).map(|_| ()) {
            // EmbedError → IndexError::Embed → ChanError → `err_from`
            // funnels every model-side failure (network, hf-hub
            // checksum, candle load) through the same error
            // rendering as the rest of the search surface. The
            // bridge is feature-gated alongside this whole module.
            let chan_err: chan_drive::ChanError = chan_drive::index::IndexError::Embed(e).into();
            return err_from(&chan_err);
        }
        match build_state(&drive) {
            Ok(s) => Json(s).into_response(),
            Err(e) => err_from(&e),
        }
    })
    .await;
    match result {
        Ok(response) => response,
        Err(join_err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("download task panicked: {join_err}"),
        )
            .into_response(),
    }
}
