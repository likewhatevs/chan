//! `GET /api/preflight` + `POST /api/preflight/decision`: first-boot
//! workspace readiness, rendered by the SPA on a locked OverlayShell
//! (contracts §2). chan-server owns the readiness flow so local and
//! remote (tunnel) workspaces get the identical experience; the desktop
//! shell only picks a path and launches `chan serve`.
//!
//! The snapshot is DERIVED from live state on every poll, so there is no
//! first-boot flag to persist or reset:
//!
//!   - `index` step: the background indexer's `IndexStatus`. A fresh,
//!     large workspace reads `running` (with `current`/`total`) while
//!     its initial build runs and flips to `done` when the index
//!     settles; an already-indexed workspace reads `done` at once. This
//!     is the readiness gate that keeps the editor from opening onto a
//!     half-built index on a big new workspace.
//!   - `model` step (embeddings builds only): when the workspace has
//!     semantic search enabled but the embedding model is not on disk,
//!     the user must choose -- download it or fall back to keyword
//!     search. Derived from the workspace's semantic config, so a "skip"
//!     decision sticks via the existing `semantic_enabled` flag rather
//!     than needing new state.
//!
//! `locked` is simply `phase != ready`; the OverlayShell hides its close
//! button + ignores ESC while it is true and dismisses on `ready`.

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};

use super::cs_link::{self, CsLink};
use crate::error::{err, err_state};
use crate::indexer::IndexStatus;
use crate::state::AppState;

#[derive(Debug, Serialize)]
struct PreflightSnapshot {
    phase: Phase,
    /// True until `phase == ready`. The single signal the OverlayShell
    /// keys its lock on.
    locked: bool,
    steps: Vec<PreflightStep>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<PreflightError>,
    /// The `cs` terminal-alias offer, present only when `cs` is missing
    /// from `$PATH`. NON-BLOCKING: it never feeds `phase` / `locked`, so a
    /// missing alias never holds the boot overlay; the SPA renders it as a
    /// dismissible card once the workspace is ready. `build_snapshot`
    /// leaves it `None`; the route handlers attach it behind the owner
    /// gate.
    #[serde(skip_serializing_if = "Option::is_none")]
    cs_link: Option<CsLink>,
    /// Post-open workspace facts for the SPA onboarding surface (P2,
    /// open-then-configure). Cleanly SEPARATED from the lock gate: it carries
    /// no readiness signal and never feeds `phase` / `locked`. `build_snapshot`
    /// leaves it `None`; the route handlers attach it only once the workspace
    /// is `Ready`, which is exactly when the onboarding card consumes it.
    #[serde(skip_serializing_if = "Option::is_none")]
    summary: Option<Summary>,
}

/// Workspace facts the onboarding card renders to confirm "this is the folder
/// I meant" and to offer the optional layers. Derived entirely from data that
/// is already free post-open (the index stats, the workspace's own config
/// flags, and a read-only VCS-marker probe at the sandboxed root), so it adds
/// no walk and no new server plumbing.
#[derive(Debug, Clone, Serialize)]
struct Summary {
    /// BM25-indexed chunk count from `index_stats`. A coarse "there is content
    /// here" signal for the confirmation, not a file count.
    indexed_docs: u64,
    /// Detected source-control kind at the workspace root ("git" / "hg" /
    /// "svn"), or `None`. Helps the user confirm the folder is what they meant.
    #[serde(skip_serializing_if = "Option::is_none")]
    scm: Option<String>,
    /// Current optional-layer state, so the card renders the enable prompts
    /// against the truth rather than assuming both are off.
    semantic_enabled: bool,
    reports_enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum Phase {
    Running,
    NeedsDecision,
    Ready,
    Failed,
}

#[derive(Debug, Serialize)]
struct PreflightStep {
    id: &'static str,
    label: &'static str,
    state: StepState,
    /// Progress counters for a `running` step (the index build's
    /// file position). The OverlayShell's progress bar reads these as
    /// the single source of truth, so the locked shell does not also
    /// have to wire `/ws` progress frames (contracts §2 Q3).
    #[serde(skip_serializing_if = "Option::is_none")]
    current: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    total: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    decision: Option<Decision>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum StepState {
    Pending,
    Running,
    Done,
    NeedsDecision,
    Failed,
}

#[derive(Debug, Serialize)]
struct Decision {
    prompt: &'static str,
    choices: Vec<DecisionChoice>,
}

#[derive(Debug, Serialize)]
struct DecisionChoice {
    id: &'static str,
    label: &'static str,
}

#[derive(Debug, Serialize)]
struct PreflightError {
    step: &'static str,
    message: String,
}

/// Map the indexer's status onto the `index` readiness step.
///
/// The boot overlay is a FIRST-boot gate (see the module docs): it exists
/// to keep the editor from opening onto a half-built index on a brand-new
/// workspace, so the ONLY state that may lock it is one where nothing is
/// searchable yet. `indexed_docs` is the live BM25 doc count.
///
///   - `Building` + `indexed_docs == 0`: the cold initial build, nothing
///     committed yet, so the index is unsearchable -> Running (locked).
///     This is the sole locking state.
///   - `Building` + `indexed_docs > 0`: a mid-session full rebuild (e.g. a
///     VCS burst over the coalesce threshold) runs over the already-
///     committed index, which stays searchable -> Done. Re-locking a
///     booted session here is the RELOAD-HANG bug class.
///   - `Reindexing`: one incremental watcher re-index, always over a built
///     index -> Done. Mapping this to Running was the reported Cmd+R hang:
///     a session/layout write triggers a watcher reindex, and a reload
///     caught mid-reindex hard-locked the whole UI until it settled.
///   - `Idle` -> Done; `Error` -> Failed so the shell can surface it.
fn index_step(status: &IndexStatus, indexed_docs: u64) -> PreflightStep {
    let base = PreflightStep {
        id: "index",
        label: "Build search index",
        state: StepState::Pending,
        current: None,
        total: None,
        decision: None,
    };
    match status {
        IndexStatus::Building { current, total, .. } if indexed_docs == 0 => PreflightStep {
            state: StepState::Running,
            current: Some(*current),
            total: Some(*total),
            ..base
        },
        // A warm Building (rebuild over an existing index) and an
        // incremental Reindexing both run over a searchable index, so they
        // map to Done exactly like Idle: a reindex must never re-lock a
        // booted session.
        IndexStatus::Building { .. }
        | IndexStatus::Reindexing { .. }
        | IndexStatus::Idle { .. } => PreflightStep {
            state: StepState::Done,
            ..base
        },
        IndexStatus::Error { .. } => PreflightStep {
            state: StepState::Failed,
            ..base
        },
    }
}

fn index_error(status: &IndexStatus) -> Option<PreflightError> {
    match status {
        IndexStatus::Error { message } => Some(PreflightError {
            step: "index",
            message: message.clone(),
        }),
        _ => None,
    }
}

/// The `model` step, embeddings builds only. Present only when the
/// workspace has semantic search enabled: a BM25 workspace has nothing
/// to decide. When enabled + the model is on disk the step is `done`;
/// when enabled + the model is missing it is `needs_decision`
/// (download the model vs. fall back to keyword search). A plain BM25
/// workspace returns `None` (no step).
#[cfg(feature = "embeddings")]
fn model_step(workspace: &chan_workspace::Workspace) -> Option<PreflightStep> {
    if !workspace.semantic_enabled().unwrap_or(false) {
        return None;
    }
    let model = workspace.semantic_model().unwrap_or_default();
    let present = chan_workspace::index::embeddings::resolve_model(&model).is_ok();
    let base = PreflightStep {
        id: "model",
        label: "Embedding model",
        state: StepState::Pending,
        current: None,
        total: None,
        decision: None,
    };
    Some(if present {
        PreflightStep {
            state: StepState::Done,
            ..base
        }
    } else {
        PreflightStep {
            state: StepState::NeedsDecision,
            decision: Some(Decision {
                prompt: "Download the embedding model for semantic search, or use keyword search?",
                choices: vec![
                    DecisionChoice {
                        id: "download",
                        label: "Download model",
                    },
                    DecisionChoice {
                        id: "skip",
                        label: "Use keyword search",
                    },
                ],
            }),
            ..base
        }
    })
}

#[cfg(not(feature = "embeddings"))]
fn model_step(_workspace: &chan_workspace::Workspace) -> Option<PreflightStep> {
    None
}

fn build_snapshot(
    workspace: &chan_workspace::Workspace,
    status: &IndexStatus,
) -> PreflightSnapshot {
    // Live BM25 doc count: the "is anything searchable yet" signal that
    // decides whether a `Building` status is a cold first build (lock) or a
    // warm rebuild over an existing index (don't lock). A stats read error
    // means we cannot prove the index is populated, so fall back to 0
    // (treat as cold) and keep the overlay locked rather than risk opening
    // onto an unbuilt index.
    let indexed_docs = workspace.index_stats().map(|s| s.indexed_docs).unwrap_or(0);
    let mut steps = vec![index_step(status, indexed_docs)];
    if let Some(step) = model_step(workspace) {
        steps.push(step);
    }

    // Phase precedence: a failure dominates, then a pending decision,
    // then "all done" is ready, otherwise still running.
    let phase = if steps.iter().any(|s| s.state == StepState::Failed) {
        Phase::Failed
    } else if steps.iter().any(|s| s.state == StepState::NeedsDecision) {
        Phase::NeedsDecision
    } else if steps.iter().all(|s| s.state == StepState::Done) {
        Phase::Ready
    } else {
        Phase::Running
    };

    PreflightSnapshot {
        phase,
        locked: phase != Phase::Ready,
        error: index_error(status),
        steps,
        // Attached by the route handlers; neither gates the boot overlay.
        cs_link: None,
        summary: None,
    }
}

/// The `cs` offer is owner-only: a tunneled or publicly-served workspace
/// must not let a visitor mutate the host's PATH. The local-serve default
/// leaves both flags false, so the offer shows on a plain `chan serve`.
fn cs_link_allowed(state: &AppState) -> bool {
    !state.settings_disabled && !state.tunnel_public
}

/// Source-control kind at the workspace root, mirroring chan's own walk: a
/// read-only existence probe on the well-known VCS marker dir, no climb above
/// the root. This is a metadata check at the sandboxed root, not a content
/// read or write, so it stays clear of the workspace write boundary.
fn detect_scm(root: &std::path::Path) -> Option<String> {
    for (kind, dir) in [("git", ".git"), ("hg", ".hg"), ("svn", ".svn")] {
        if root.join(dir).exists() {
            return Some(kind.to_string());
        }
    }
    None
}

/// Assemble the onboarding `summary` from data that is already free post-open.
/// Every read degrades to a benign default on error so a transient stats /
/// config read never turns an otherwise-ready snapshot into a failure.
fn workspace_summary(workspace: &chan_workspace::Workspace) -> Summary {
    let indexed_docs = workspace.index_stats().map(|s| s.indexed_docs).unwrap_or(0);
    Summary {
        indexed_docs,
        scm: detect_scm(workspace.root()),
        semantic_enabled: workspace.semantic_enabled().unwrap_or(false),
        reports_enabled: workspace.reports_enabled().unwrap_or(false),
    }
}

pub async fn api_preflight(State(state): State<Arc<AppState>>) -> Response {
    let workspace = match state.try_workspace() {
        Ok(w) => w,
        Err(e) => return err_state(&e),
    };
    let indexer = match state.try_indexer() {
        Ok(i) => i,
        Err(e) => return err_state(&e),
    };
    let allow_cs = cs_link_allowed(&state);
    // Semantic reads hit sqlite + the model resolver touches the
    // filesystem, and the cs detection scans $PATH; do the whole
    // derivation on the blocking pool.
    match tokio::task::spawn_blocking(move || {
        let status = indexer.snapshot();
        let mut snapshot = build_snapshot(&workspace, &status);
        snapshot.cs_link = cs_link::detect(allow_cs);
        // The onboarding summary describes an OPEN workspace, so attach it only
        // once ready (also keeps the per-poll work off the cold-build path).
        if snapshot.phase == Phase::Ready {
            snapshot.summary = Some(workspace_summary(&workspace));
        }
        snapshot
    })
    .await
    {
        Ok(snapshot) => Json(snapshot).into_response(),
        Err(e) => err(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("preflight task panicked: {e}"),
        ),
    }
}

#[derive(Debug, Deserialize)]
pub struct DecisionBody {
    step: String,
    /// Only the embeddings build's `model` step reads this; on a
    /// no-embeddings build there are no decisions, so the field is
    /// deserialized but unused.
    #[cfg_attr(not(feature = "embeddings"), allow(dead_code))]
    choice: String,
}

// `state` is consumed only by the embeddings `model` decision arm; on a
// no-embeddings build there are no decisions, so the binding is unused.
#[cfg_attr(not(feature = "embeddings"), allow(unused_variables))]
pub async fn api_preflight_decision(
    State(state): State<Arc<AppState>>,
    Json(body): Json<DecisionBody>,
) -> Response {
    match body.step.as_str() {
        #[cfg(feature = "embeddings")]
        "model" => model_decision(&state, &body.choice).await,
        other => err(
            StatusCode::BAD_REQUEST,
            format!("no pending pre-flight decision for step {other:?}"),
        ),
    }
}

/// Apply a `model` step decision, then return the fresh snapshot so the
/// shell re-renders without a second poll. `download` fetches the model
/// (and the workspace's semantic flag stays on); `skip` flips the
/// workspace back to keyword-only, which makes the model step drop out
/// of the snapshot entirely.
#[cfg(feature = "embeddings")]
async fn model_decision(state: &Arc<AppState>, choice: &str) -> Response {
    let workspace = match state.try_workspace() {
        Ok(w) => w,
        Err(e) => return err_state(&e),
    };
    let indexer = match state.try_indexer() {
        Ok(i) => i,
        Err(e) => return err_state(&e),
    };
    let choice = choice.to_owned();
    let allow_cs = cs_link_allowed(state);
    // The blocking closure carries its error as `Box<Response>` so the
    // `Result` Err variant stays pointer-sized (an axum `Response` is
    // large; clippy::result_large_err otherwise fires under -D warnings).
    match tokio::task::spawn_blocking(move || -> Result<PreflightSnapshot, Box<Response>> {
        use chan_workspace::index::embeddings::{global_models_dir, Embedder};
        match choice.as_str() {
            "download" => {
                let model = workspace
                    .semantic_model()
                    .map_err(|e| Box::new(crate::error::err_from(&e)))?;
                let cache_dir = global_models_dir();
                if let Err(e) = std::fs::create_dir_all(&cache_dir) {
                    return Err(Box::new(err(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("creating model cache {}: {e}", cache_dir.display()),
                    )));
                }
                if let Err(e) = Embedder::open(&model, &cache_dir).map(|_| ()) {
                    let chan_err: chan_workspace::ChanError =
                        chan_workspace::index::IndexError::Embed(e).into();
                    return Err(Box::new(crate::error::err_from(&chan_err)));
                }
            }
            "skip" => {
                workspace
                    .set_semantic_enabled(false)
                    .map_err(|e| Box::new(crate::error::err_from(&e)))?;
            }
            other => {
                return Err(Box::new(err(
                    StatusCode::BAD_REQUEST,
                    format!("unknown choice {other:?} for pre-flight step \"model\""),
                )));
            }
        }
        let status = indexer.snapshot();
        let mut snapshot = build_snapshot(&workspace, &status);
        snapshot.cs_link = cs_link::detect(allow_cs);
        if snapshot.phase == Phase::Ready {
            snapshot.summary = Some(workspace_summary(&workspace));
        }
        Ok(snapshot)
    })
    .await
    {
        Ok(Ok(snapshot)) => Json(snapshot).into_response(),
        Ok(Err(response)) => *response,
        Err(e) => err(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("preflight decision task panicked: {e}"),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn workspace() -> (TempDir, TempDir, Arc<chan_workspace::Workspace>) {
        let cfg = TempDir::new().unwrap();
        let root = TempDir::new().unwrap();
        let lib = chan_workspace::Library::open_at(cfg.path().join("config.toml")).unwrap();
        lib.register_workspace(root.path()).unwrap();
        let ws = lib.open_workspace(root.path()).unwrap();
        (cfg, root, ws)
    }

    fn idle() -> IndexStatus {
        IndexStatus::Idle {
            indexed_docs: 0,
            indexed_vectors: 0,
            model: "m".into(),
            embedding: None,
        }
    }

    #[test]
    fn detect_scm_finds_git_then_none() {
        let dir = TempDir::new().unwrap();
        assert_eq!(detect_scm(dir.path()), None);
        std::fs::create_dir(dir.path().join(".git")).unwrap();
        assert_eq!(detect_scm(dir.path()).as_deref(), Some("git"));
    }

    #[test]
    fn summary_reflects_a_fresh_bm25_workspace() {
        let (_c, _r, ws) = workspace();
        let s = workspace_summary(&ws);
        // Fresh workspace defaults: BM25-only, reports off, no VCS in the
        // tempdir root.
        assert!(!s.semantic_enabled);
        assert!(!s.reports_enabled);
        assert_eq!(s.scm, None);
    }

    #[test]
    fn build_snapshot_leaves_summary_for_the_handler() {
        // The summary is an onboarding concern attached by the route handler
        // once ready, never by the gate logic. build_snapshot must leave it
        // None so the phase/locked derivation stays free of it.
        let (_c, _r, ws) = workspace();
        let snap = build_snapshot(&ws, &idle());
        assert!(snap.summary.is_none());
    }

    #[test]
    fn building_index_locks_until_settled() {
        let (_c, _r, ws) = workspace();
        let snap = build_snapshot(
            &ws,
            &IndexStatus::Building {
                current: 3,
                total: 10,
                file: "a.md".into(),
            },
        );
        assert_eq!(snap.phase, Phase::Running);
        assert!(snap.locked);
        let index = snap.steps.iter().find(|s| s.id == "index").unwrap();
        assert_eq!(index.state, StepState::Running);
        assert_eq!(index.current, Some(3));
        assert_eq!(index.total, Some(10));
    }

    #[test]
    fn reindexing_never_locks() {
        // RELOAD-HANG regression: an incremental watcher reindex maps to a
        // ready (unlocked) step regardless of doc count. Mapping it to
        // Running was what hard-locked the boot overlay on Cmd+R while a
        // session/layout write was being reindexed.
        let file = || "note-490.md".to_string();
        assert_eq!(
            index_step(&IndexStatus::Reindexing { file: file() }, 0).state,
            StepState::Done
        );
        assert_eq!(
            index_step(&IndexStatus::Reindexing { file: file() }, 1200).state,
            StepState::Done
        );
    }

    #[test]
    fn cold_build_locks_but_warm_rebuild_does_not() {
        let building = || IndexStatus::Building {
            current: 3,
            total: 10,
            file: "a.md".into(),
        };
        // Cold initial build: nothing committed yet -> locked, with the
        // progress counters the overlay's bar reads.
        let cold = index_step(&building(), 0);
        assert_eq!(cold.state, StepState::Running);
        assert_eq!(cold.current, Some(3));
        assert_eq!(cold.total, Some(10));
        // Warm rebuild over an existing index (mid-session full rebuild,
        // e.g. a VCS burst): the prior index stays searchable, so it must
        // not re-lock a booted session.
        assert_eq!(index_step(&building(), 42).state, StepState::Done);
    }

    #[test]
    fn reindexing_keeps_preflight_unlocked() {
        let (_c, _r, ws) = workspace();
        // End-to-end through build_snapshot: a fresh BM25 workspace whose
        // status reads Reindexing must report phase Ready / locked:false.
        let snap = build_snapshot(
            &ws,
            &IndexStatus::Reindexing {
                file: "n.md".into(),
            },
        );
        assert_eq!(snap.phase, Phase::Ready);
        assert!(
            !snap.locked,
            "an incremental reindex must not lock the boot overlay"
        );
        let index = snap.steps.iter().find(|s| s.id == "index").unwrap();
        assert_eq!(index.state, StepState::Done);
    }

    #[test]
    fn idle_bm25_workspace_is_ready_unlocked() {
        let (_c, _r, ws) = workspace();
        // A fresh workspace defaults to BM25 (semantic off), so there is
        // no model step and an idle index means ready at once.
        let snap = build_snapshot(&ws, &idle());
        assert_eq!(snap.phase, Phase::Ready);
        assert!(!snap.locked);
        assert!(
            snap.steps.iter().all(|s| s.id != "model"),
            "a BM25 workspace must not show a model step"
        );
        assert!(snap.error.is_none());
    }

    #[test]
    fn index_error_fails_with_message() {
        let (_c, _r, ws) = workspace();
        let snap = build_snapshot(
            &ws,
            &IndexStatus::Error {
                message: "boom".into(),
            },
        );
        assert_eq!(snap.phase, Phase::Failed);
        assert!(snap.locked);
        let error = snap.error.as_ref().expect("failed phase carries an error");
        assert_eq!(error.step, "index");
        assert_eq!(error.message, "boom");
    }

    #[cfg(feature = "embeddings")]
    #[test]
    fn semantic_enabled_surfaces_a_model_step() {
        let (_c, _r, ws) = workspace();
        // Enable semantic directly: the HTTP enable route guards on
        // model presence, but the raw setter does not, which is exactly
        // the "enabled but model maybe-missing" state the model step
        // exists for.
        ws.set_semantic_enabled(true).unwrap();
        let snap = build_snapshot(&ws, &idle());
        let model = snap
            .steps
            .iter()
            .find(|s| s.id == "model")
            .expect("semantic-enabled workspace must show a model step");
        // The test environment's global model cache decides presence;
        // assert the phase is consistent with whichever state results.
        match model.state {
            StepState::NeedsDecision => {
                assert_eq!(snap.phase, Phase::NeedsDecision);
                assert!(snap.locked);
                assert!(model.decision.is_some(), "a decision step carries choices");
            }
            StepState::Done => {
                assert_eq!(snap.phase, Phase::Ready);
                assert!(!snap.locked);
            }
            other => panic!("unexpected model step state: {other:?}"),
        }
    }

    #[cfg(feature = "embeddings")]
    #[test]
    fn skip_decision_drops_the_model_step() {
        let (_c, _r, ws) = workspace();
        ws.set_semantic_enabled(true).unwrap();
        // The "skip" decision flips semantic back off; the model step
        // then drops out entirely and an idle index is ready.
        ws.set_semantic_enabled(false).unwrap();
        let snap = build_snapshot(&ws, &idle());
        assert!(snap.steps.iter().all(|s| s.id != "model"));
        assert_eq!(snap.phase, Phase::Ready);
    }
}
