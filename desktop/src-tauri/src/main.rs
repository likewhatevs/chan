#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod auth;
mod config;
mod default_drive;
mod download;
mod embedded;
mod registry;
mod serve;
mod tunnel;
mod watcher;

use std::collections::HashMap;
#[cfg(unix)]
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use serde::Serialize;
use tauri::menu::{Menu, MenuItemBuilder, MenuItemKind, PredefinedMenuItem, WINDOW_SUBMENU_ID};
use tauri::{Emitter, Manager, RunEvent, State, WebviewUrl, WebviewWindowBuilder, WindowEvent};

use config::{Config, ConfigStore, OutboundDrive, WindowConfig, WorkspaceFeatures};
use serve::ServeHandle;
use tunnel::TunnelState;

const CHAN_BUSY_CHANGED: &str = "chan-busy";
const SYSTEM_NOTICE: &str = "system-notice";

/// Process-wide state. Shared via `Arc` because Tauri commands and
/// background runtime owners need the same state handle.
pub struct AppState {
    store: Mutex<ConfigStore>,
    /// Live embedded local drives keyed by canonical drive path.
    serves: Mutex<HashMap<String, ServeHandle>>,
    /// In-process chan-server host for normal local drives.
    /// Initialized during Tauri setup, after the async runtime is
    /// available for Tokio listener registration.
    embedded: OnceLock<embedded::EmbeddedServer>,
    /// Embedded chan-tunnel-server. Owns the tunnel listener on
    /// 127.0.0.1:7777, the shared registry, and the per-tenant
    /// loopback listeners that proxy into registered remote
    /// `chan serve` instances.
    tunnel: Arc<TunnelState>,
    /// `fullstack-b-19`: per-live-window zoom level. Tracks the
    /// current zoom for every open webview keyed by window label so
    /// `zoom_in` / `zoom_out` / `zoom_reset` can compute the next
    /// level without spawning a JS eval round-trip to read the
    /// current. Drained into `WindowConfig.zoom_level` by the close
    /// handler so the LRU restore from `-b-1` picks the level up on
    /// the next open. Missing entry reads as 1.0 (the chan-desktop
    /// default).
    pub live_window_zooms: Mutex<HashMap<String, f64>>,
}

/// Defense-in-depth local runtime teardown: `RunEvent::Exit` is the
/// primary path, but a panic unwinding through `tauri::App` can
/// bypass it. Dropping the last `Arc<AppState>` signals every
/// running local drive via `serve::stop_all`. Idempotent: stop_all
/// drains the serves map, so a normal-exit run followed by Drop is a
/// no-op on the second pass.
impl Drop for AppState {
    fn drop(&mut self) {
        serve::stop_all(self);
    }
}

impl AppState {
    /// Push a closing window's layout onto the LRU stack. Best
    /// effort: any I/O error is logged and dropped so a flaky
    /// config disk doesn't leak through the WindowEvent handler.
    pub fn push_window_config(&self, entry: WindowConfig) {
        let mut store = self.store.lock().unwrap();
        let mut cfg = match store.get() {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(error = %e, "loading config to push window state failed");
                return;
            }
        };
        config::push_window_config(&mut cfg, entry);
        if let Err(e) = store.save(&cfg) {
            tracing::warn!(error = %e, "persisting window config stack failed");
        }
    }

    /// Pop the most-recent WindowConfig matching `key`, removing
    /// it from the stack on disk. Returns `None` when no entry
    /// exists or the config file can't be read. Same best-effort
    /// posture as `push_window_config`.
    pub fn pop_window_config(&self, key: &str) -> Option<WindowConfig> {
        let mut store = self.store.lock().unwrap();
        let mut cfg = match store.get() {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(error = %e, "loading config to pop window state failed");
                return None;
            }
        };
        let popped = config::pop_window_config(&mut cfg, key)?;
        if let Err(e) = store.save(&cfg) {
            tracing::warn!(error = %e, "persisting window config stack failed");
        }
        Some(popped)
    }
}

/// Merged drive view returned to the frontend. Two flavours share
/// the wire shape so the existing renderer can iterate one list:
///
/// * `kind = "local"`: a chan-registry entry, backed by a
///   drive mounted into the embedded server. Includes the canonical
///   filesystem path and live URL.
/// * `kind = "tunneled"`: a remote `chan serve` that dialed into
///   the embedded tunnel server. No path; `url` points at the
///   per-tenant loopback listener.
/// * `kind = "outbound"`: a remote `chan serve` explicitly attached
///   by URL. No desktop-owned lifecycle; `id` points at the stored
///   attachment row.
///
/// Fields specific to tunneled rows are optional so the JSON shape
/// is a strict superset of the local row; the renderer reads `kind`
/// once and chooses which optionals to surface.
#[derive(Debug, Clone, Serialize)]
struct Workspace {
    kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    path: String,
    on: bool,
    url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    drive: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    public: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    peer_addr: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    connected_at: Option<String>,
}

#[tauri::command]
fn list_workspaces(state: State<Arc<AppState>>) -> Result<Vec<Workspace>, String> {
    let serves = state.serves.lock().unwrap();
    let entries = registry::read().map_err(err)?;

    // `on` is derived from a live serve handle, never persisted.
    // That way a desktop restart comes up with everything off
    // (matching reality: nothing is actually running yet) and
    // there is no chance of a stale on=true sticking around after
    // chan died unexpectedly.
    let mut merged: Vec<Workspace> = entries
        .into_iter()
        .map(|e| {
            let key = canonical_key(&e.root_path);
            let display_path = key.clone();
            let handle = serves.get(&key);
            let on = handle.is_some();
            let url = handle.and_then(|h| h.url.clone()).unwrap_or_default();
            Workspace {
                kind: "local",
                id: None,
                path: display_path,
                on,
                url,
                label: None,
                drive: None,
                public: None,
                peer_addr: None,
                connected_at: None,
            }
        })
        .collect();

    // Tunneled rows: one per registered (label, drive) in the
    // embedded chan-tunnel-server. URL is populated by the
    // supervisor as soon as the per-tenant listener binds; an
    // empty URL means "just registered, the listener will follow
    // on the next 500ms tick".
    for t in state.tunnel.snapshot() {
        merged.push(Workspace {
            kind: "tunneled",
            id: None,
            path: String::new(),
            on: true,
            url: t.url,
            label: Some(t.label),
            drive: Some(t.drive),
            public: Some(t.public),
            peer_addr: t.peer_addr,
            connected_at: Some(t.connected_at),
        });
    }

    let outbound_drives = state.store.lock().unwrap().get().map_err(err)?.outbound;
    for outbound in outbound_drives {
        let label = outbound_label(&outbound);
        let id = outbound.id;
        let url = outbound.url;
        merged.push(Workspace {
            kind: "outbound",
            id: Some(id),
            path: url.clone(),
            on: true,
            url,
            label,
            drive: None,
            public: None,
            peer_addr: None,
            connected_at: None,
        });
    }

    Ok(merged)
}

/// `fullstack-b-28b` slice iii: the pre-flight modal collects the
/// user's feature choices BEFORE the drive is registered + passes
/// them through to `chan add`. The chan CLI's `--semantic-search`
/// + `--reports` flags from `systacean-27` are the right
/// registration-time entry point so chan-drive's BOOT process
/// picks up the chosen state on the FIRST open (no stub +
/// re-toggle cycle).
///
/// `features` is optional for SPA-side backward compatibility +
/// for the CLI-level `add_drive` calls that don't surface the
/// pre-flight UX. Missing or default `features` opens the drive
/// lean (BM25-only, no reports).
#[tauri::command]
async fn add_drive(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    path: String,
    features: Option<WorkspaceFeatures>,
) -> Result<(), String> {
    let path = canonical_key(Path::new(&path));
    let features = features.unwrap_or_default();
    let Some(embedded) = state.embedded.get() else {
        return Err("embedded local server is unavailable".to_string());
    };
    // Route through the SINGLE embedded Library so the in-memory
    // registry the host opens drives against learns about the new
    // row immediately. A subprocess `chan add` would mutate only
    // the on-disk registry, leaving the host's boot-time snapshot
    // stale, which is the "drive not registered" bug this replaces.
    let library = embedded.library().clone();
    let path_for_block = path.clone();

    emit_chan_busy(&app, true, "add", &path);
    // register_workspace + boot run off the async executor: boot can
    // walk a large drive on first reports activation.
    let result =
        tokio::task::spawn_blocking(move || register_and_boot(&library, &path_for_block, features))
            .await;
    emit_chan_busy(&app, false, "add", &path);
    match result {
        Ok(inner) => inner?,
        Err(e) => return Err(format!("registering drive panicked: {e}")),
    }

    // `fullstack-b-28b` slice iii: mirror the chosen features into
    // the desktop cache so `get_drive_features` returns the
    // authoritative state immediately, before the user toggles
    // anything in the launcher row.
    if features != WorkspaceFeatures::default() {
        let mut store = state.store.lock().unwrap();
        let mut cfg = store.get().map_err(err)?;
        cfg.drives.entry(path.clone()).or_default().features = features;
        store.save(&cfg).map_err(err)?;
    }

    // Auto-start: opening a drive from the desktop is the user's
    // way of saying "make this drive usable now". Spinning up the
    // serve immediately is what they expect; otherwise the freshly
    // added row sits there with On=off and Launch disabled, which
    // looks broken.
    serve::start(app, Arc::clone(&state), path).await?;
    Ok(())
}

/// Register `path` with the shared embedded Library and, if any
/// optional feature was requested, open the drive once to persist
/// the flags and kick the BOOT scan. Mirrors `chan/src/main.rs`'s
/// `cmd_add`. The transient `Arc<Workspace>` is dropped before this
/// returns so the immediately-following `serve::start` can mount
/// the drive without tripping `WorkspaceAlreadyOpen` against the
/// lifetime flock. Blocking: `register_workspace` writes the registry
/// and `boot()` can run a slow initial scan, so callers invoke it
/// via `spawn_blocking`.
fn register_and_boot(
    library: &chan_workspace::Library,
    path: &str,
    features: WorkspaceFeatures,
) -> Result<(), String> {
    let root = Path::new(path);
    if !root.exists() {
        std::fs::create_dir_all(root).map_err(|e| format!("creating drive root {path}: {e}"))?;
    }
    let entry = library
        .register_workspace(root)
        .map_err(|e| format!("registering drive {path}: {e}"))?;
    if features.bge || features.reports {
        let drive = library
            .open_workspace(&entry.root_path)
            .map_err(|e| format!("opening drive {}: {e}", entry.root_path.display()))?;
        if features.bge {
            drive
                .set_semantic_enabled(true)
                .map_err(|e| format!("enabling semantic search: {e}"))?;
        }
        if features.reports {
            drive
                .set_reports_enabled(true)
                .map_err(|e| format!("enabling reports: {e}"))?;
        }
        drive
            .boot()
            .map_err(|e| format!("boot after enabling features: {e}"))?;
        // Drop the transient handle before serve::start re-opens it.
        drop(drive);
    }
    Ok(())
}

#[tauri::command]
async fn remove_workspace(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    path: String,
) -> Result<(), String> {
    let key = canonical_key(Path::new(&path));
    // Stop the serve first: this removes the runtime synchronously
    // and drops the host's Arc<Workspace>, but background indexer /
    // request tasks may briefly keep their own clone, so the
    // unregister below tolerates a short contention window.
    serve::stop(Some(&app), &state, &key);

    let Some(embedded) = state.embedded.get() else {
        return Err("embedded local server is unavailable".to_string());
    };
    let library = embedded.library().clone();
    let key_for_block = key.clone();

    emit_chan_busy(&app, true, "remove", &key);
    let result =
        tokio::task::spawn_blocking(move || unregister_with_retry(&library, &key_for_block)).await;
    emit_chan_busy(&app, false, "remove", &key);
    match result {
        Ok(inner) => inner?,
        Err(e) => return Err(format!("unregistering drive panicked: {e}")),
    }

    let mut store = state.store.lock().unwrap();
    let mut cfg = store.get().map_err(err)?;
    cfg.drives.remove(&key);
    store.save(&cfg).map_err(err)?;
    Ok(())
}

/// Drop a drive from the shared registry after its serve has been
/// stopped. `serve::stop` removes the runtime synchronously, but a
/// background indexer rebuild or an in-flight HTTP/WS handler can
/// still hold an `Arc<Workspace>` for a moment. `unregister_workspace`
/// wipes per-drive state and so needs exclusive access; until the
/// last handle drops it returns `WorkspaceAlreadyOpen` (this process)
/// or `WorkspaceLocked` (the flock). `reset_workspace` takes the flock
/// before any registry mutation, so a failed attempt leaves no
/// half-state and a retry is safe. Any other error surfaces
/// immediately. Blocking: sleeps between attempts, so callers
/// invoke it via `spawn_blocking`.
fn unregister_with_retry(library: &chan_workspace::Library, key: &str) -> Result<(), String> {
    use chan_workspace::ChanError;
    const MAX_ATTEMPTS: usize = 20;
    const BACKOFF: std::time::Duration = std::time::Duration::from_millis(150);
    let root = Path::new(key);
    for attempt in 1..=MAX_ATTEMPTS {
        match library.unregister_workspace(root) {
            // Ok(false) means it was already absent; both forms are
            // success for a Forget action.
            Ok(_) => return Ok(()),
            Err(e @ (ChanError::WorkspaceAlreadyOpen | ChanError::WorkspaceLocked)) => {
                if attempt == MAX_ATTEMPTS {
                    return Err(format!(
                        "drive {key} is still shutting down ({e}); try Forget again in a moment"
                    ));
                }
                std::thread::sleep(BACKOFF);
            }
            Err(e) => return Err(format!("unregistering drive {key}: {e}")),
        }
    }
    unreachable!("retry loop returns on the final attempt")
}

#[tauri::command]
async fn set_drive_on(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    path: String,
    on: bool,
) -> Result<(), String> {
    let key = canonical_key(Path::new(&path));
    if on {
        serve::start(app, Arc::clone(&state), key).await?;
    } else {
        serve::stop(Some(&app), &state, &key);
    }
    Ok(())
}

#[tauri::command]
fn get_config(state: State<Arc<AppState>>) -> Result<Config, String> {
    state.store.lock().unwrap().get().map_err(err)
}

/// `fullstack-b-28a` + `-b-28b` slice ii: read the persisted
/// feature toggles for a drive. Returns the default `{bge:
/// false, reports: false}` for any drive that has no desktop cache
/// entry yet — the launcher's expand panel calls this on render
/// so first-time drives show up with both toggles off as the
/// round-2-plan specifies.
///
/// Reads chan-drive's authoritative state in-process: if the drive
/// is mounted, off the live `Arc<Workspace>` the host holds; else via a
/// transient `open_workspace` against the shared registry. On any read
/// failure (drive not registered, drive busy, etc.) the IPC falls
/// back to the desktop cache so the launcher row's expand panel
/// still renders. On a successful read the desktop cache updates if
/// the state differs, picking up out-of-band changes (e.g. a flag
/// flipped from a terminal) so the launcher reflects truth on the
/// next render without a manual refresh.
#[tauri::command]
async fn get_drive_features(
    state: State<'_, Arc<AppState>>,
    path: String,
) -> Result<WorkspaceFeatures, String> {
    let key = canonical_key(Path::new(&path));
    if let Some(embedded) = state.embedded.get() {
        let library = embedded.library().clone();
        let live = embedded.live_drive(Path::new(&key));
        let key_for_block = key.clone();
        // A transient open touches the index config on disk; keep it
        // off the async executor.
        let read = tokio::task::spawn_blocking(move || {
            read_drive_features_blocking(&library, live, &key_for_block)
        })
        .await
        .unwrap_or_else(|e| Err(format!("reading drive features panicked: {e}")));
        if let Ok(features) = read {
            let mut store = state.store.lock().unwrap();
            let mut cfg = store.get().map_err(err)?;
            let entry = cfg.drives.entry(key).or_default();
            if entry.features != features {
                entry.features = features;
                // Best-effort cache update: a save failure here
                // doesn't change the value returned to the SPA; the
                // next read retries chan-drive.
                let _ = store.save(&cfg);
            }
            return Ok(features);
        }
    }
    // Fall-through: embedded host unavailable OR the read failed.
    // The desktop cache is the best available source.
    let cfg = state.store.lock().unwrap().get().map_err(err)?;
    Ok(cfg.drives.get(&key).map(|s| s.features).unwrap_or_default())
}

/// Read the authoritative feature flags for `key` from chan-drive.
/// Prefers the live mounted handle (no re-open, so the lifetime
/// flock isn't contended); falls back to a transient `open_workspace`
/// for a registered-but-stopped drive. Returns `Err` when the drive
/// isn't registered or a read fails so the caller can fall back to
/// the desktop cache. Blocking: a transient open initializes the
/// index, so callers invoke it via `spawn_blocking`.
fn read_drive_features_blocking(
    library: &chan_workspace::Library,
    live: Option<Arc<chan_workspace::Workspace>>,
    key: &str,
) -> Result<WorkspaceFeatures, String> {
    let drive = resolve_drive_for_features(library, live, key)?;
    let bge = drive
        .semantic_enabled()
        .map_err(|e| format!("reading semantic_enabled: {e}"))?;
    let reports = drive
        .reports_enabled()
        .map_err(|e| format!("reading reports_enabled: {e}"))?;
    Ok(WorkspaceFeatures { bge, reports })
}

/// Resolve the `Arc<Workspace>` a feature read/write should act on: the
/// live mounted handle when present, otherwise a transient open of
/// a registered drive. Errors when the drive isn't registered.
fn resolve_drive_for_features(
    library: &chan_workspace::Library,
    live: Option<Arc<chan_workspace::Workspace>>,
    key: &str,
) -> Result<Arc<chan_workspace::Workspace>, String> {
    if let Some(drive) = live {
        return Ok(drive);
    }
    let root = Path::new(key);
    if library.workspace_paths_for(root).is_none() {
        return Err(format!("drive {key} is not registered"));
    }
    library
        .open_workspace(root)
        .map_err(|e| format!("opening drive {key}: {e}"))
}

/// `fullstack-b-28b` slice iv: the pre-flight report displayed in
/// the drive-add modal. Carries the load-bearing facts the
/// round-2-plan §"UI surface" calls out so the user can answer
/// "is this the folder I meant?" + "what am I about to commit
/// to?" before chan-drive's BOOT starts walking. Strict superset
/// of slice iii's modal — toggles still render at the bottom of
/// the same dialog.
///
/// All counts are best-effort: the walker caps at
/// `MAX_PREFLIGHT_FILES` files + `MAX_PREFLIGHT_SECS` wall-clock
/// seconds; on cap `truncated = true` so the modal can render
/// "100,000+" instead of a misleading exact number.
///
/// `already_registered` is checked against the shared embedded
/// registry — if the canonical path is already registered, the
/// modal flags the duplicate so the user doesn't accidentally
/// re-add the same drive.
#[derive(Debug, Clone, Serialize)]
struct PreflightReport {
    /// Canonical drive path. Mirrors what `add_drive` will
    /// pass to `chan add`.
    path: String,
    /// True iff `std::fs::metadata(path).permissions().readonly()`
    /// is false. A read-only mount surfaces an explicit warning
    /// in the modal so the user knows chan can't write notes
    /// into the drive.
    writable: bool,
    /// Files visited under `path` (excluding directories +
    /// SCM-internal / build-output trees). Capped.
    file_count: usize,
    /// Markdown files visited (extensions `.md` / `.markdown`).
    /// The primary content type chan operates on; surfaced
    /// separately so the modal can read it as "drive readiness"
    /// rather than just "total file count".
    markdown_count: usize,
    /// Total byte size of files visited. Capped alongside
    /// `file_count`.
    size_bytes: u64,
    /// Counts by media class (extension-classified). Sum can be
    /// less than `file_count` since unclassified extensions
    /// (markdown, source, configs, etc.) don't increment any
    /// of these.
    image_count: usize,
    audio_count: usize,
    video_count: usize,
    /// SCM identifier if `.git` / `.hg` / `.svn` exists at root:
    /// `Some("git")` / `Some("hg")` / `Some("svn")`. `None`
    /// means no SCM was detected; the modal stays silent in that
    /// case.
    scm: Option<String>,
    /// True iff the canonical path is already registered. The modal
    /// renders a duplicate-registration warning so the user can
    /// cancel before chan errors on add.
    already_registered: bool,
    /// True when the walker hit `MAX_PREFLIGHT_FILES` or
    /// `MAX_PREFLIGHT_SECS`. Modal renders the counts with a
    /// "+" suffix so users know more files may exist.
    truncated: bool,
}

const MAX_PREFLIGHT_FILES: usize = 100_000;
const MAX_PREFLIGHT_SECS: u64 = 5;

/// `fullstack-b-28b` slice iv: walk the drive root + collect
/// the facts the pre-flight modal needs to render. Capped so a
/// monster drive doesn't pin the chan-desktop UI for minutes;
/// the modal communicates the cap to the user via the
/// `truncated` flag.
///
/// `chan_workspace::indexer` uses the same excluded-dirs set. The
/// extension-classification map is intentionally local to keep the
/// pre-flight report cheap and independent from opening the drive
/// through the embedded server.
fn walk_drive_preflight(root: &Path, filter: &chan_workspace::WalkFilter) -> WalkOutcome {
    use std::collections::VecDeque;
    use std::time::Instant;
    let start = Instant::now();
    let mut queue: VecDeque<PathBuf> = VecDeque::new();
    queue.push_back(root.to_path_buf());
    let mut out = WalkOutcome::default();
    while let Some(dir) = queue.pop_front() {
        if out.file_count >= MAX_PREFLIGHT_FILES || start.elapsed().as_secs() >= MAX_PREFLIGHT_SECS
        {
            out.truncated = true;
            break;
        }
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(meta) = entry.metadata() else {
                continue;
            };
            let name = entry.file_name();
            if meta.is_dir() {
                if should_skip_preflight_dir(&name, filter) {
                    continue;
                }
                queue.push_back(entry.path());
            } else if meta.is_file() {
                out.file_count += 1;
                out.size_bytes = out.size_bytes.saturating_add(meta.len());
                classify_preflight_extension(&name, &mut out);
                if out.file_count >= MAX_PREFLIGHT_FILES {
                    out.truncated = true;
                    break;
                }
            }
        }
    }
    out
}

#[derive(Debug, Default, PartialEq, Eq)]
struct WalkOutcome {
    file_count: usize,
    markdown_count: usize,
    size_bytes: u64,
    image_count: usize,
    audio_count: usize,
    video_count: usize,
    truncated: bool,
}

fn preflight_walk_filter() -> chan_workspace::WalkFilter {
    chan_workspace::Registry::load()
        .map(|registry| chan_workspace::WalkFilter::new(registry.index_excluded_dirs))
        .unwrap_or_else(|_| {
            chan_workspace::WalkFilter::new(
                chan_workspace::DEFAULT_INDEX_EXCLUDED_DIRS.iter().copied(),
            )
        })
}

/// Mirrors chan-drive's configured excludes so the pre-flight count
/// and bytes line up with what chan-drive will actually index.
fn should_skip_preflight_dir(name: &std::ffi::OsStr, filter: &chan_workspace::WalkFilter) -> bool {
    let Some(name) = name.to_str() else {
        return false;
    };
    name.eq_ignore_ascii_case(".chan")
        || name.eq_ignore_ascii_case(".git")
        || filter.is_excluded(name)
}

/// Extension → media-class bucket. Mirrors chan-drive's
/// classification at a smaller scope (no Markdown / source-code
/// breakouts here; only the three media classes the round-2-plan
/// pre-flight calls out). `markdown_count` is tracked separately
/// for the "drive readiness" hint.
fn classify_preflight_extension(name: &std::ffi::OsStr, out: &mut WalkOutcome) {
    let Some(ext) = Path::new(name)
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase())
    else {
        return;
    };
    match ext.as_str() {
        "md" | "markdown" => out.markdown_count += 1,
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "heic" | "heif" | "bmp" | "tiff" | "svg"
        | "ico" => out.image_count += 1,
        "mp3" | "wav" | "m4a" | "flac" | "ogg" | "opus" | "aac" => out.audio_count += 1,
        "mp4" | "mov" | "webm" | "mkv" | "avi" | "m4v" => out.video_count += 1,
        _ => {}
    }
}

/// Return the SCM kind rooted at `root` if any. Only checks the
/// root level — chan's own walk doesn't climb above the drive
/// root either, so an SCM in an ancestor dir isn't surfaced.
fn detect_drive_scm(root: &Path) -> Option<String> {
    for (kind, dir) in [("git", ".git"), ("hg", ".hg"), ("svn", ".svn")] {
        if root.join(dir).exists() {
            return Some(kind.to_string());
        }
    }
    None
}

/// `fullstack-b-28b` slice iv: assemble the pre-flight report.
/// Walks the drive + checks SCM + checks the shared embedded
/// registry for the duplicate-registration flag. Tolerates the
/// embedded host not being up yet (returns
/// `already_registered = false`) so the modal still renders
/// something useful.
#[tauri::command]
async fn compute_drive_preflight(
    state: State<'_, Arc<AppState>>,
    path: String,
) -> Result<PreflightReport, String> {
    let key = canonical_key(Path::new(&path));
    let root = PathBuf::from(&key);
    let writable = std::fs::metadata(&root)
        .map(|m| !m.permissions().readonly())
        .unwrap_or(false);
    let filter = preflight_walk_filter();
    let walk = walk_drive_preflight(&root, &filter);
    let scm = detect_drive_scm(&root);
    // Duplicate-registration check against the shared embedded
    // registry: a quick in-memory lookup, no subprocess. Defaults
    // to false when the embedded host isn't up yet.
    let already_registered = state
        .embedded
        .get()
        .map(|embedded| embedded.library().workspace_paths_for(&root).is_some())
        .unwrap_or(false);
    Ok(PreflightReport {
        path: key,
        writable,
        file_count: walk.file_count,
        markdown_count: walk.markdown_count,
        size_bytes: walk.size_bytes,
        image_count: walk.image_count,
        audio_count: walk.audio_count,
        video_count: walk.video_count,
        scm,
        already_registered,
        truncated: walk.truncated,
    })
}

/// `fullstack-b-28a` + `-b-28b-i`: write the feature toggle pair
/// for a drive. Both fields land together so a partial flip
/// doesn't leave a half-state on disk; the SPA always sends the
/// current full state on every change.
///
/// `-b-28b-i` drove the real chan-drive state via the `chan` CLI;
/// this routes in-process instead. Each changed flag is applied to
/// the same `Arc<Workspace>` the host holds when the drive is mounted
/// (so the lifetime flock isn't contended), or to a transient
/// handle for a registered-but-stopped drive. Enabling reports
/// also runs `boot()` to kick the initial scan, mirroring
/// `chan/src/main.rs`'s `cmd_reports_set`. Flags are applied in
/// order so a failure on the first leaves the second untouched. On
/// success the desktop cache updates so subsequent
/// `get_drive_features` reads return the authoritative state
/// without re-reading chan-drive. On any failure the desktop cache
/// stays untouched and the error propagates to the SPA (the
/// launcher's `bindFeaturesToggle` reverts the checkbox).
#[tauri::command]
async fn set_drive_features(
    state: State<'_, Arc<AppState>>,
    path: String,
    features: WorkspaceFeatures,
) -> Result<(), String> {
    let key = canonical_key(Path::new(&path));
    let current = {
        let cfg = state.store.lock().unwrap().get().map_err(err)?;
        cfg.drives.get(&key).map(|s| s.features).unwrap_or_default()
    };
    let Some(embedded) = state.embedded.get() else {
        return Err("embedded local server is unavailable".to_string());
    };
    let library = embedded.library().clone();
    let live = embedded.live_drive(Path::new(&key));
    let key_for_block = key.clone();
    // set_reports_enabled(false) drops report.jsonl and boot() can
    // run a scan; keep both off the async executor.
    let result = tokio::task::spawn_blocking(move || {
        apply_drive_features_blocking(&library, live, &key_for_block, current, features)
    })
    .await;
    match result {
        Ok(inner) => inner?,
        Err(e) => return Err(format!("applying drive features panicked: {e}")),
    }
    let mut store = state.store.lock().unwrap();
    let mut cfg = store.get().map_err(err)?;
    cfg.drives.entry(key).or_default().features = features;
    store.save(&cfg).map_err(err)
}

/// Apply the changed feature flags to the resolved `Arc<Workspace>`.
/// Only flags that differ from `current` are touched, so a no-op
/// re-set doesn't reinitialize anything. Enabling reports also
/// boots the initial scan so the flag flip produces visible data
/// immediately (mirrors `cmd_reports_set`). Blocking; run via
/// `spawn_blocking`.
fn apply_drive_features_blocking(
    library: &chan_workspace::Library,
    live: Option<Arc<chan_workspace::Workspace>>,
    key: &str,
    current: WorkspaceFeatures,
    desired: WorkspaceFeatures,
) -> Result<(), String> {
    if current == desired {
        return Ok(());
    }
    let drive = resolve_drive_for_features(library, live, key)?;
    if current.bge != desired.bge {
        drive
            .set_semantic_enabled(desired.bge)
            .map_err(|e| format!("setting semantic search: {e}"))?;
    }
    if current.reports != desired.reports {
        drive
            .set_reports_enabled(desired.reports)
            .map_err(|e| format!("setting reports: {e}"))?;
        if desired.reports {
            drive
                .boot()
                .map_err(|e| format!("boot after enabling reports: {e}"))?;
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
struct TunnelStatus {
    /// True while the tunnel listener is bound.
    listening: bool,
    /// Actual bound port (only populated while `listening`).
    port: Option<u16>,
    /// User's preferred port from desktop config. `0` means
    /// "let the OS assign one". UI uses this to populate the port
    /// input field.
    preferred_port: u16,
    /// Either the user's saved label or a freshly-suggested one if
    /// they've never typed anything. Suggestions avoid colliding
    /// with labels currently registered in the running tunnel:
    /// "tunnel" → "tunnel-1" → ... up to 999.
    preferred_label: String,
    /// User's saved drive name or a default ("notes"). No
    /// collision check — drive uniqueness is scoped per label, and
    /// the desktop doesn't track which labels are remotely
    /// preferred.
    preferred_drive: String,
    /// Pre-formatted `ssh -R` reverse-forward snippet. `None` when
    /// the tunnel isn't listening (no port to reference yet).
    ssh_snippet: Option<String>,
    /// Pre-formatted `chan serve` command with the bound port,
    /// canonical TUNNEL_PATH, and the user's chosen label/drive
    /// already substituted. Copy-paste ready.
    chan_serve_snippet: Option<String>,
}

/// Build the `ssh -R` and `chan serve` snippets that the listen
/// panel renders verbatim. Pre-formatting them here means JS does
/// zero templating — and the canonical URL path (with
/// `TUNNEL_PATH`) lives in exactly one place in the codebase.
fn build_snippets(port: u16, label: &str, drive: &str) -> (String, String) {
    let ssh = format!("ssh -R {port}:localhost:{port} user@remote");
    // `--no-browser` keeps chan serve from launching the remote's
    // default browser at startup (it has nothing to point at — the
    // visitor URL belongs to chan-desktop, which is what auto-opens
    // the drive webview on this side instead). `PATH` goes last so
    // the user only needs to edit one trailing argument.
    let chan = format!(
        "chan serve --tunnel-url=http://127.0.0.1:{port}{path} \
         --tunnel-token={label} --tunnel-drive={drive} --no-browser PATH",
        path = chan_tunnel_proto::TUNNEL_PATH,
    );
    (ssh, chan)
}

/// Pick a label suggestion: if the user has one saved, use it
/// verbatim. Otherwise try "tunnel"; if a remote is already
/// registered under that label, walk "tunnel-1", "tunnel-2", ...
/// until we find a free one. Falls back to `tunnel` at the end of
/// the range (uniqueness is best-effort; the registry's
/// last-writer-wins eviction is the real arbiter).
fn suggest_label(saved: &str, state: &AppState) -> String {
    if !saved.is_empty() {
        return saved.to_string();
    }
    let in_use: std::collections::HashSet<String> = state
        .tunnel
        .snapshot()
        .into_iter()
        .map(|d| d.label)
        .collect();
    let base = "tunnel";
    if !in_use.contains(base) {
        return base.to_string();
    }
    for i in 1..1000 {
        let candidate = format!("{base}-{i}");
        if !in_use.contains(&candidate) {
            return candidate;
        }
    }
    base.to_string()
}

fn suggest_drive(saved: &str) -> String {
    if saved.is_empty() {
        "notes".to_string()
    } else {
        saved.to_string()
    }
}

#[tauri::command]
fn tunnel_status(state: State<Arc<AppState>>) -> Result<TunnelStatus, String> {
    let cfg = state.store.lock().unwrap().get().map_err(err)?.tunnel;
    let preferred_label = suggest_label(&cfg.preferred_label, &state);
    let preferred_drive = suggest_drive(&cfg.preferred_drive);
    let port = state.tunnel.tunnel_port();
    let listening = state.tunnel.is_listening();
    let (ssh_snippet, chan_serve_snippet) = match (listening, port) {
        (true, Some(p)) => {
            let (s, c) = build_snippets(p, &preferred_label, &preferred_drive);
            (Some(s), Some(c))
        }
        _ => (None, None),
    };
    Ok(TunnelStatus {
        listening,
        port,
        preferred_port: cfg.preferred_port,
        preferred_label,
        preferred_drive,
        ssh_snippet,
        chan_serve_snippet,
    })
}

/// Start the tunnel listener with the user's chosen port, label,
/// and drive. Validates `label` / `drive` against the protocol's
/// charset rules so the rendered snippet matches what the wire
/// will actually accept. Persists all three for the next session.
#[tauri::command]
async fn tunnel_start(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    preferred_port: u16,
    label: String,
    drive: String,
) -> Result<u16, String> {
    let label = label.trim().to_string();
    let drive = drive.trim().to_string();
    if !chan_tunnel_proto::is_valid_username(&label) {
        return Err(format!(
            "invalid label {label:?}: ASCII alphanumerics plus '-' / '_', \
             first char alphanumeric, ≤64 chars",
        ));
    }
    if !chan_tunnel_proto::is_valid_drive_name(&drive) {
        return Err(format!(
            "invalid drive name {drive:?}: lowercase ASCII alphanumerics plus '-', \
             first and last char alphanumeric, ≤32 chars",
        ));
    }
    {
        let mut store = state.store.lock().unwrap();
        let mut cfg = store.get().map_err(err)?;
        cfg.tunnel.preferred_port = preferred_port;
        cfg.tunnel.preferred_label = label;
        cfg.tunnel.preferred_drive = drive;
        store.save(&cfg).map_err(err)?;
    }
    let tunnel = Arc::clone(&state.tunnel);
    tunnel::start_listening(app, tunnel, preferred_port).await
}

#[tauri::command]
fn tunnel_stop(app: tauri::AppHandle, state: State<Arc<AppState>>) {
    tunnel::stop_listening(&app, &state.tunnel);
}

#[tauri::command]
fn default_drive_status() -> Result<default_drive::DefaultWorkspaceStatus, String> {
    default_drive::status()
}

#[tauri::command]
fn choose_default_drive(path: String) -> Result<(), String> {
    default_drive::choose_existing(Path::new(&path)).map(|_| ())
}

#[tauri::command]
async fn create_default_drive(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let created = default_drive::create_default_drive()?;
    reconcile_default_drive(&state, &created.root)?;
    let key = canonical_key(&created.root);
    serve::start(app, Arc::clone(&state), key).await
}

#[tauri::command]
async fn factory_reset_default_drive(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let created = default_drive::factory_reset_default_drive()?;
    reconcile_default_drive(&state, &created.root)?;
    let key = canonical_key(&created.root);
    serve::start(app, Arc::clone(&state), key).await
}

/// `default_drive` registers + seeds through its own throwaway
/// `Library` handle. Mirror that registration into the embedded
/// host's in-memory `Library` so the immediately-following
/// `serve::start` opens against an up-to-date registry rather than
/// the host's stale boot-time snapshot (the same staleness class as
/// the "drive not registered" bug). `register_workspace` is idempotent
/// (touch + persist), so re-registering the row default_drive just
/// wrote is safe, and `set_default_drive_root` keeps the in-memory
/// default aligned with what default_drive persisted.
fn reconcile_default_drive(state: &AppState, root: &Path) -> Result<(), String> {
    let Some(embedded) = state.embedded.get() else {
        // No embedded host (e.g. it failed to start at boot);
        // default_drive already persisted to disk, so a later serve
        // through a fresh handle still sees the row.
        return Ok(());
    };
    let library = embedded.library();
    library
        .register_workspace(root)
        .map_err(|e| format!("reconciling default drive {}: {e}", root.display()))?;
    library
        .set_default_drive_root(Some(root.to_path_buf()))
        .map_err(|e| format!("persisting default drive root {}: {e}", root.display()))?;
    Ok(())
}

const OUTBOUND_LABEL_MAX_CHARS: usize = 120;

/// Persist an explicit outbound URL attachment and open it in a
/// drive webview. The remote server owns its own lifecycle; desktop
/// only stores enough state to show and reopen the row.
#[tauri::command]
fn add_outbound_drive(
    app: tauri::AppHandle,
    state: State<Arc<AppState>>,
    url: String,
    label: String,
) -> Result<String, String> {
    let url = normalize_outbound_url(&url)?;
    let label = normalize_outbound_label(&label)?;
    let (id, title, stored_url) = {
        let mut store = state.store.lock().unwrap();
        let mut cfg = store.get().map_err(err)?;
        let (id, title, stored_url) = match cfg.outbound.iter_mut().find(|d| d.url == url) {
            Some(existing) => {
                if !label.is_empty() {
                    existing.label = label.clone();
                }
                (
                    existing.id.clone(),
                    outbound_title(&existing.label, &existing.url),
                    existing.url.clone(),
                )
            }
            None => {
                let entry = OutboundDrive {
                    id: uuid::Uuid::new_v4().to_string(),
                    url: url.clone(),
                    label,
                    added_at: config::current_millis(),
                };
                let id = entry.id.clone();
                let title = outbound_title(&entry.label, &entry.url);
                cfg.outbound.push(entry);
                (id, title, url)
            }
        };
        store.save(&cfg).map_err(err)?;
        (id, title, stored_url)
    };
    serve::spawn_outbound_drive_window(&app, &id, &title, &stored_url)?;
    let _ = app.emit(serve::SERVES_CHANGED, ());
    Ok(id)
}

/// Open another webview for a stored outbound URL attachment.
#[tauri::command]
fn open_outbound_drive(
    app: tauri::AppHandle,
    state: State<Arc<AppState>>,
    id: String,
) -> Result<(), String> {
    let (title, url) = {
        let cfg = state.store.lock().unwrap().get().map_err(err)?;
        let outbound = cfg
            .outbound
            .iter()
            .find(|d| d.id == id)
            .ok_or_else(|| format!("no outbound drive attachment {id}"))?;
        (
            outbound_title(&outbound.label, &outbound.url),
            outbound.url.clone(),
        )
    };
    serve::spawn_outbound_drive_window(&app, &id, &title, &url)
}

/// Forget an outbound URL attachment. The remote server is not
/// stopped; only desktop config and open webviews for this
/// attachment are removed.
#[tauri::command]
fn remove_outbound_drive(
    app: tauri::AppHandle,
    state: State<Arc<AppState>>,
    id: String,
) -> Result<(), String> {
    {
        let mut store = state.store.lock().unwrap();
        let mut cfg = store.get().map_err(err)?;
        let before = cfg.outbound.len();
        cfg.outbound.retain(|d| d.id != id);
        if cfg.outbound.len() != before {
            store.save(&cfg).map_err(err)?;
        }
    }
    serve::close_outbound_drive_windows(&app, &id);
    let _ = app.emit(serve::SERVES_CHANGED, ());
    Ok(())
}

fn normalize_outbound_url(raw: &str) -> Result<String, String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err("remote URL is required".to_string());
    }
    let mut parsed =
        url::Url::parse(raw).map_err(|e| format!("invalid remote URL {raw:?}: {e}"))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err("remote URL must use http:// or https://".to_string());
    }
    if parsed.host_str().is_none() {
        return Err("remote URL must include a host".to_string());
    }
    strip_query_param(&mut parsed, "w");
    Ok(parsed.to_string())
}

fn strip_query_param(parsed: &mut url::Url, name: &str) {
    if !parsed.query_pairs().any(|(key, _)| key == name) {
        return;
    }
    let pairs: Vec<(String, String)> = parsed
        .query_pairs()
        .filter(|(key, _)| key != name)
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect();
    let mut query = parsed.query_pairs_mut();
    query.clear();
    for (key, value) in pairs {
        query.append_pair(&key, &value);
    }
}

fn normalize_outbound_label(raw: &str) -> Result<String, String> {
    let label = raw.trim().to_string();
    if label.chars().count() > OUTBOUND_LABEL_MAX_CHARS {
        return Err(format!(
            "remote label must be {OUTBOUND_LABEL_MAX_CHARS} characters or fewer",
        ));
    }
    Ok(label)
}

fn outbound_title(label: &str, url: &str) -> String {
    let label = label.trim();
    if label.is_empty() {
        url.to_string()
    } else {
        label.to_string()
    }
}

fn outbound_label(outbound: &OutboundDrive) -> Option<String> {
    let label = outbound.label.trim();
    if label.is_empty() {
        None
    } else {
        Some(label.to_string())
    }
}

/// Open an additional in-app Tauri webview for a running local
/// drive. The first window is auto-opened by the serve supervisor
/// when chan prints its URL; subsequent clicks on Launch reach
/// here and add new windows alongside it. Errors if the drive is
/// not currently running (no URL captured yet).
#[tauri::command]
fn open_local_drive(
    app: tauri::AppHandle,
    state: State<Arc<AppState>>,
    path: String,
) -> Result<(), String> {
    let key = canonical_key(Path::new(&path));
    let url = state
        .serves
        .lock()
        .unwrap()
        .get(&key)
        .and_then(|h| h.url.clone())
        .ok_or_else(|| format!("drive {key} is not running"))?;
    serve::spawn_local_drive_window(&app, &key, &url)?;
    Ok(())
}

/// Open a drive in a native window in response to a CLI handoff
/// request (`chan serve <drive>` while this desktop is running).
///
/// Mirrors the `add_drive` flow: register + boot the drive through the
/// shared embedded Library, then `serve::start` (mount + spawn the
/// first window). If the drive is ALREADY running, `serve::start`
/// returns early without spawning a window, so we raise an additional
/// window via `spawn_local_drive_window` to match the user's intent
/// ("show me this drive now").
///
/// The slow work (registry write, boot scan, mount) runs on a spawned
/// task so the callback returns promptly and the CLI doesn't block on
/// the handshake. The synchronous return therefore reports only that
/// the request was accepted, not that the window is fully up; on a
/// genuine mount failure the desktop emits a system notice (same as
/// the first-launch default-drive path) rather than blocking the CLI.
#[cfg(unix)]
fn open_workspace_from_handoff(
    app: tauri::AppHandle,
    state: Arc<AppState>,
    path: PathBuf,
) -> Result<(), String> {
    let key = canonical_key(&path);

    // Already running: raise an additional window immediately. This is
    // synchronous and gives the user the window without a mount cycle.
    let running_url = state
        .serves
        .lock()
        .unwrap()
        .get(&key)
        .and_then(|h| h.url.clone());
    if let Some(url) = running_url {
        return serve::spawn_local_drive_window(&app, &key, &url);
    }

    // Not running: register (creating the dir for a fresh path) + boot
    // through the shared Library, then mount + spawn the window. Off
    // the listener task so the CLI gets a prompt response.
    let Some(embedded) = state.embedded.get() else {
        return Err("embedded local server is unavailable".to_string());
    };
    let library = embedded.library().clone();
    let key_for_block = key.clone();
    tauri::async_runtime::spawn(async move {
        let library_for_register = library.clone();
        let key_for_register = key_for_block.clone();
        let registered = tokio::task::spawn_blocking(move || {
            register_and_boot(
                &library_for_register,
                &key_for_register,
                WorkspaceFeatures::default(),
            )
        })
        .await;
        match registered {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                emit_system_notice(
                    &app,
                    "warning",
                    format!("Could not open {key_for_block} from chan serve: {e}"),
                );
                return;
            }
            Err(e) => {
                emit_system_notice(
                    &app,
                    "warning",
                    format!("Opening {key_for_block} from chan serve panicked: {e}"),
                );
                return;
            }
        }
        if let Err(e) = serve::start(app.clone(), Arc::clone(&state), key_for_block.clone()).await {
            emit_system_notice(
                &app,
                "warning",
                format!("Could not open {key_for_block} from chan serve: {e}"),
            );
        }
    });
    Ok(())
}

/// Open an additional in-app Tauri webview for a tunneled drive.
/// Each call yields a NEW window — the first one is opened by the
/// supervisor on registration, and the Launch button calls this
/// for subsequent windows. Errors if the per-tenant listener
/// hasn't bound yet (URL not formed).
#[tauri::command]
fn open_tunneled_drive(
    app: tauri::AppHandle,
    state: State<Arc<AppState>>,
    label: String,
    drive: String,
) -> Result<(), String> {
    let url = state
        .tunnel
        .snapshot()
        .into_iter()
        .find(|d| d.label == label && d.drive == drive)
        .map(|d| d.url)
        .ok_or_else(|| format!("no tunneled drive {label}/{drive}"))?;
    if url.is_empty() {
        return Err(format!(
            "tunneled drive {label}/{drive} has no URL yet; per-tenant listener still binding",
        ));
    }
    serve::spawn_tunneled_drive_window(&app, &label, &drive, &url)?;
    Ok(())
}

/// User's home directory as a plain string, for the Drives window
/// to abbreviate paths to `~/...`. Returns an empty string when the
/// platform can't resolve it.
#[tauri::command]
fn home_dir() -> String {
    dirs::home_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_default()
}

/// Open the given folder in the OS file manager. macOS: Finder,
/// Linux: default file manager, Windows: Explorer. Used by the
/// Drives window's path cell so users can jump to the drive folder
/// from the row. Trusts the caller to pass a path the user just saw
/// in the list — paths come from `list_workspaces`, which sources from
/// the chan registry; no shell interpolation, args are passed as
/// argv to the OS open command.
#[tauri::command]
fn reveal_in_finder(path: String) -> Result<(), String> {
    let opener = if cfg!(target_os = "macos") {
        "open"
    } else if cfg!(target_os = "windows") {
        "explorer"
    } else {
        "xdg-open"
    };
    let status = std::process::Command::new(opener)
        .arg(&path)
        .status()
        .map_err(|e| format!("opening {path}: {e}"))?;
    if !status.success() {
        return Err(format!("opening {path}: {opener} exited with {status}"));
    }
    Ok(())
}

fn show_window(app: &tauri::AppHandle, label: &str) -> Result<(), String> {
    if let Some(w) = app.get_webview_window(label) {
        w.show().map_err(err)?;
        w.set_focus().map_err(err)?;
    }
    Ok(())
}

/// Reload the calling webview window. Drives the SPA's tab
/// context-menu "Reload" entry (via `fullstack-a-36`) AND the
/// `Cmd+R` accelerator wired in `KEY_BRIDGE_JS`. The accelerator
/// path bypasses the SPA event bus and invokes this command
/// directly so a SPA-side fault (frozen Svelte runtime, JS error
/// in the chord handler) doesn't lock the dev affordance away.
#[tauri::command]
fn reload_window(window: tauri::WebviewWindow) -> Result<(), String> {
    // Tauri 2's `WebviewWindow::eval` runs JS inside the webview;
    // we use it instead of the missing-in-2 `reload()` method.
    window
        .eval("window.location.reload()")
        .map_err(|e| format!("reloading window: {e}"))
}

/// Open the DevTools inspector on the calling webview. Mirrors
/// the SPA's "Open Inspector" context-menu entry from `-a-36`
/// AND the `Cmd+Opt+I` accelerator in `KEY_BRIDGE_JS`. Requires
/// the `devtools` Cargo feature on the `tauri` crate (enabled in
/// `desktop/src-tauri/Cargo.toml`) so release builds carry the
/// inspector affordance, not just debug builds. Tauri 2 removed
/// the `app.devTools` JSON config key in favour of this
/// compile-time flag.
#[tauri::command]
fn open_devtools(window: tauri::WebviewWindow) {
    window.open_devtools();
}

/// `fullstack-b-19`: browser-style zoom controls. Step size is
/// 10 % per Cmd++/Cmd+- press; the clamp range matches Tauri's own
/// `zoom_hotkeys_enabled` polyfill semantics (0.25-5.0).
const ZOOM_STEP: f64 = 0.10;
const ZOOM_MIN: f64 = 0.25;
const ZOOM_MAX: f64 = 5.0;

/// Read the current zoom level for `label` from process state,
/// defaulting to 1.0 (chan-desktop's initial zoom). Pure read; the
/// IPC handlers compute the next level locally and write back.
fn current_zoom(state: &AppState, label: &str) -> f64 {
    state
        .live_window_zooms
        .lock()
        .unwrap()
        .get(label)
        .copied()
        .unwrap_or(1.0)
}

fn apply_zoom(window: &tauri::WebviewWindow, state: &AppState, next: f64) -> Result<(), String> {
    let clamped = next.clamp(ZOOM_MIN, ZOOM_MAX);
    window
        .set_zoom(clamped)
        .map_err(|e| format!("setting webview zoom on {}: {e}", window.label()))?;
    state
        .live_window_zooms
        .lock()
        .unwrap()
        .insert(window.label().to_string(), clamped);
    Ok(())
}

/// Zoom the calling webview one step up (Cmd++ / Ctrl++).
#[tauri::command]
fn zoom_in(window: tauri::WebviewWindow, state: State<Arc<AppState>>) -> Result<(), String> {
    let current = current_zoom(&state, window.label());
    apply_zoom(&window, &state, current + ZOOM_STEP)
}

/// Zoom the calling webview one step down (Cmd+- / Ctrl+-).
#[tauri::command]
fn zoom_out(window: tauri::WebviewWindow, state: State<Arc<AppState>>) -> Result<(), String> {
    let current = current_zoom(&state, window.label());
    apply_zoom(&window, &state, current - ZOOM_STEP)
}

/// Reset the calling webview to 100 % (Cmd+0 / Ctrl+0).
#[tauri::command]
fn zoom_reset(window: tauri::WebviewWindow, state: State<Arc<AppState>>) -> Result<(), String> {
    apply_zoom(&window, &state, 1.0)
}

/// Canonical-path key used for desktop config, serve identity, and
/// the displayed path. `canonicalize` falls back to the input on
/// error so we still produce a stable key for not-yet-existing or
/// asleep paths.
fn canonical_key(p: &Path) -> String {
    p.canonicalize()
        .unwrap_or_else(|_| PathBuf::from(p))
        .display()
        .to_string()
}

fn err<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

fn emit_chan_busy(app: &tauri::AppHandle, busy: bool, op: &str, path: &str) {
    let _ = app.emit(
        CHAN_BUSY_CHANGED,
        serde_json::json!({ "busy": busy, "op": op, "path": path }),
    );
}

fn emit_system_notice(app: &tauri::AppHandle, level: &str, message: impl Into<String>) {
    let _ = app.emit(
        SYSTEM_NOTICE,
        serde_json::json!({ "level": level, "message": message.into() }),
    );
}

fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_env("CHAN_LOG")
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn,chan_desktop=info")),
        )
        .with_writer(std::io::stderr)
        .init();
}

#[cfg(unix)]
fn run_hidden_mcp_proxy_if_requested() -> Result<bool, String> {
    let mut args = std::env::args_os();
    let _program = args.next();
    if args.next().as_deref() != Some(OsStr::new("__mcp-proxy")) {
        return Ok(false);
    }
    let socket = args
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| "__mcp-proxy requires a socket path".to_string())?;
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("building MCP proxy runtime: {e}"))?;
    rt.block_on(run_mcp_proxy(socket))?;
    Ok(true)
}

#[cfg(not(unix))]
fn run_hidden_mcp_proxy_if_requested() -> Result<bool, String> {
    Ok(false)
}

#[cfg(unix)]
async fn run_mcp_proxy(socket: PathBuf) -> Result<(), String> {
    use tokio::io::{stdin, stdout};
    let stream = tokio::net::UnixStream::connect(&socket)
        .await
        .map_err(|e| format!("connecting to MCP socket {}: {e}", socket.display()))?;
    let (mut read_sock, mut write_sock) = stream.into_split();
    let mut stdin = stdin();
    let mut stdout = stdout();
    let to_socket = tokio::io::copy(&mut stdin, &mut write_sock);
    let from_socket = tokio::io::copy(&mut read_sock, &mut stdout);
    tokio::select! {
        r = to_socket => {
            r.map_err(|e| format!("piping stdin to MCP socket: {e}"))?;
        }
        r = from_socket => {
            r.map_err(|e| format!("piping MCP socket to stdout: {e}"))?;
        }
    }
    Ok(())
}

fn main() {
    match run_hidden_mcp_proxy_if_requested() {
        Ok(true) => return,
        Ok(false) => {}
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
    init_tracing();
    let default_drive_boot = match default_drive::ensure_fresh_default_drive() {
        Ok(created) => created,
        Err(e) => {
            tracing::warn!(error = %e, "first-launch default drive setup failed");
            None
        }
    };
    let store = ConfigStore::new().expect("failed to init config store");
    let state = Arc::new(AppState {
        store: Mutex::new(store),
        serves: Mutex::new(HashMap::new()),
        embedded: OnceLock::new(),
        tunnel: TunnelState::new(),
        live_window_zooms: Mutex::new(HashMap::new()),
    });
    let state_for_exit = Arc::clone(&state);
    let state_for_setup = Arc::clone(&state);

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(state)
        .setup(move |app| {
            install_app_menu(app.handle())?;

            match tauri::async_runtime::block_on(embedded::EmbeddedServer::start()) {
                Ok(server) => {
                    if state_for_setup.embedded.set(server).is_err() {
                        tracing::warn!("embedded local server initialized more than once");
                    }
                }
                Err(e) => {
                    tracing::warn!(error = %e, "embedded local server disabled");
                }
            }

            // Deep-link callbacks from the system browser
            // (`chan://auth/callback#...`). Cold-start URLs and
            // runtime URLs both flow through the same handler so the
            // sign-in completes whether the user clicked "Open with
            // chan-desktop" before or after the app was running.
            use tauri_plugin_deep_link::DeepLinkExt;
            let app_for_links = app.handle().clone();
            app.deep_link().on_open_url(move |event| {
                for url in event.urls() {
                    auth::handle_callback(&app_for_links, url.as_str());
                }
            });
            if let Ok(Some(urls)) = app.deep_link().get_current() {
                for url in urls {
                    auth::handle_callback(app.handle(), url.as_str());
                }
            }

            // Closing the main window via the red traffic light or
            // Cmd+W should hide it, not destroy it: hidden serve
            // children can still keep the process alive, and
            // reopening via Dock click or the Window > Drives menu
            // item should be instant. Without this, a closed main
            // window cannot be brought back without quitting and
            // relaunching.
            if let Some(main) = app.get_webview_window("main") {
                let main_for_event = main.clone();
                main.on_window_event(move |event| {
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = main_for_event.hide();
                    }
                });
                let _ = main.show();
                let _ = main.set_focus();
            }

            // Registry watcher. Leaked: we want it alive for the
            // process lifetime and the inner Watcher type is
            // unnameable through `manage`.
            match watcher::spawn(app.handle().clone(), &registry::path()) {
                Ok(d) => {
                    Box::leak(Box::new(d));
                }
                Err(e) => {
                    tracing::warn!(error = %e, "registry watcher disabled");
                    emit_system_notice(
                        app.handle(),
                        "warning",
                        "Auto-refresh disabled; close and reopen the window after running chan add.",
                    );
                }
            }

            // Tunnel listener is OFF until the user explicitly
            // clicks "Attach" in the Drives window. We just
            // construct the empty TunnelState during boot; binding
            // 127.0.0.1 happens on the IPC `tunnel_start` call.
            let _ = state_for_setup.tunnel.clone();

            // macOS CLI-to-desktop handoff listener (ratified Option
            // B). Binds the well-known per-user UDS so a `chan serve
            // <drive>` in a terminal hands the drive to this desktop
            // window instead of failing on the per-drive flock. Leaked
            // for the process lifetime (the registry watcher above uses
            // the same Box::leak pattern; the handle's Drop unlinks the
            // socket but we want it live until exit, and RunEvent::Exit
            // tears the process down anyway). A bind failure is
            // non-fatal: the CLI just falls back to its own server.
            #[cfg(unix)]
            if let Some(sock) = chan_server::handoff::well_known_socket_path() {
                let app_for_handoff = app.handle().clone();
                let state_for_handoff = Arc::clone(&state_for_setup);
                // `start_listener` binds a tokio `UnixListener` and
                // `tokio::spawn`s the accept loop, so it MUST run inside
                // a tokio runtime context. The Tauri `setup` closure runs
                // on the main thread OUTSIDE any runtime, so calling it
                // directly panics ("there is no reactor running"), which
                // aborts the whole desktop on launch. Enter the Tauri-
                // managed runtime via `block_on` (the same runtime the
                // embedded server above and every `async_runtime::spawn`
                // below use) so the bind + the spawned accept loop attach
                // to it and survive after this returns.
                let listener = tauri::async_runtime::block_on(async {
                    chan_server::handoff::start_listener(sock, move |path| {
                        open_workspace_from_handoff(
                            app_for_handoff.clone(),
                            Arc::clone(&state_for_handoff),
                            path,
                        )
                    })
                });
                match listener {
                    Ok(handle) => {
                        Box::leak(Box::new(handle));
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "CLI-to-desktop handoff listener disabled");
                    }
                }
            }

            if let Some(created) = default_drive_boot.clone() {
                let app_for_default = app.handle().clone();
                let state_for_default = Arc::clone(&state_for_setup);
                tauri::async_runtime::spawn(async move {
                    let key = canonical_key(&created.root);
                    if let Err(e) =
                        serve::start(app_for_default.clone(), state_for_default, key).await
                    {
                        tracing::warn!(
                            root = %created.root.display(),
                            error = %e,
                            "starting first-launch default drive failed",
                        );
                        emit_system_notice(
                            &app_for_default,
                            "warning",
                            format!(
                                "Created the default Chan drive at {}, but opening it failed: {e}",
                                created.root.display(),
                            ),
                        );
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_workspaces,
            add_drive,
            remove_workspace,
            set_drive_on,
            get_drive_features,
            set_drive_features,
            compute_drive_preflight,
            get_config,
            home_dir,
            reveal_in_finder,
            reload_window,
            open_devtools,
            download::save_file_to_downloads,
            zoom_in,
            zoom_out,
            zoom_reset,
            tunnel_status,
            tunnel_start,
            tunnel_stop,
            default_drive_status,
            choose_default_drive,
            create_default_drive,
            factory_reset_default_drive,
            open_local_drive,
            open_tunneled_drive,
            add_outbound_drive,
            open_outbound_drive,
            remove_outbound_drive,
            auth::auth_status,
            auth::open_signin,
            auth::signout,
        ])
        .build(tauri::generate_context!())
        .expect("error building tauri application");

    app.run(move |_app, event| {
        match event {
            RunEvent::Exit => {
                // Best-effort: unmount every embedded local drive
                // before the desktop runtime exits.
                serve::stop_all(&state_for_exit);
                // Cancel the tunnel listener (if active) and every
                // per-tenant listener. Tasks exit when their cancel
                // token fires; the process is on its way out, so we
                // don't await them.
                tunnel::shutdown(&state_for_exit.tunnel);
            }
            // macOS: Dock click or `open -a` while the process is
            // still alive. If no windows are visible (main has been
            // hidden / closed and the user has no drive windows
            // open), bring the main window back.
            #[cfg(target_os = "macos")]
            RunEvent::Reopen {
                has_visible_windows: false,
                ..
            } => {
                let _ = show_window(_app, "main");
            }
            _ => {}
        }
    });
}

/// Inject window-navigation items into the default Tauri menu.
/// Tauri's `Menu::default` produces the standard macOS menubar
/// (app / File / Edit / View / Window / Help) but its Window
/// submenu only has Minimize / Zoom / Close — a closed main
/// window has no menu path back. We prepend Drives, Settings,
/// and Logs items to that submenu so each app window is
/// reachable by name.
///
/// Settings has Cmd+, but no chan-desktop-owned UI behind it:
/// chan owns the Settings concept per-drive. The handler dispatches
/// `app.settings.toggle` into the focused drive webview, where
/// chan's `runCommand` opens its settings overlay. Cmd+, with the
/// Drives window focused is a no-op.
fn install_app_menu(app: &tauri::AppHandle) -> tauri::Result<()> {
    let menu = Menu::default(app)?;

    // Drives keeps no accelerator: Cmd+1..9 is reserved for
    // jump-to-tab in drive windows (handled by the per-drive key
    // bridge script in serve.rs). The menu entry still surfaces the
    // window by name.
    let drive_manager = MenuItemBuilder::with_id("win-main", "Drives").build(app)?;
    // `fullstack-83`: Cmd+N spawns a fresh launcher window. The
    // existing "main" window stays untouched (singleton label);
    // additional launchers land on `main-<N>` so each carries its
    // own state independently. Convention for future chan-desktop
    // shortcuts: declare a MenuItemBuilder here with the
    // `CmdOrCtrl+<key>` accelerator, prepend into the Window
    // submenu below, and add a matching `on_menu_event` branch.
    // `fullstack-b-27`: moved from `CmdOrCtrl+N` to
    // `CmdOrCtrl+Shift+N` so the SPA's New Draft handler (per
    // `fullstack-a-66`) can claim plain Cmd+N without the menu
    // accelerator intercepting first. Menu label stays
    // "New Window"; only the chord moves.
    let new_window = MenuItemBuilder::with_id("app-new-window", "New Window")
        .accelerator("CmdOrCtrl+Shift+N")
        .build(app)?;
    let settings = MenuItemBuilder::with_id("chan-settings", "Settings…")
        .accelerator("CmdOrCtrl+,")
        .build(app)?;

    if let Some(window_submenu) = menu
        .get(WINDOW_SUBMENU_ID)
        .and_then(|k| k.as_submenu().cloned())
    {
        let sep = PredefinedMenuItem::separator(app)?;
        window_submenu.prepend_items(&[&drive_manager, &new_window, &settings, &sep])?;
        // Strip the default "Close Window" item so Cmd+W reaches the
        // drive webview's key bridge (which dispatches `app.tab.close`
        // to chan). The trade-off: non-drive windows (main, console)
        // lose their Cmd+W shortcut — closing them is still possible
        // via the red traffic light. Match by text since muda assigns
        // predefined items an opaque generated id.
        if let Ok(items) = window_submenu.items() {
            for item in items {
                if let MenuItemKind::Predefined(p) = &item {
                    if let Ok(text) = p.text() {
                        if text.to_lowercase().contains("close") {
                            let _ = window_submenu.remove(&item);
                        }
                    }
                }
            }
        }
    }

    app.set_menu(menu)?;
    app.on_menu_event(|app, event| match event.id().as_ref() {
        "win-main" => {
            let _ = show_window(app, "main");
        }
        "app-new-window" => {
            if let Err(e) = open_new_launcher_window(app) {
                tracing::warn!(error = %e, "open new launcher window failed");
            }
        }
        "chan-settings" => {
            dispatch_to_focused_drive(app, "app.settings.toggle");
        }
        _ => {}
    });
    Ok(())
}

/// `fullstack-83`: spawn a fresh launcher (drive-picker) window via
/// `WebviewWindowBuilder`. The label is picked from the next free
/// `main-N` slot so each launcher carries its own per-window state
/// (mirrors the `drive-N` / `tunnel-N` convention). New windows use
/// the same `index.html` entry as the singleton `main`, so the
/// SPA's `boot()` path runs and the user lands on the drive
/// picker — never inheriting any existing launcher's runtime
/// state.
fn open_new_launcher_window(app: &tauri::AppHandle) -> Result<(), String> {
    let label = next_launcher_label(app);
    if app.get_webview_window(&label).is_some() {
        // Defensive: the slot picker scans existing windows so a
        // collision shouldn't happen. If it ever does, surface a
        // clear error rather than panicking on `build`.
        return Err(format!("launcher label {label} already exists"));
    }
    WebviewWindowBuilder::new(app, &label, WebviewUrl::App("index.html".into()))
        .title("Chan Desktop")
        .inner_size(960.0, 600.0)
        .min_inner_size(720.0, 400.0)
        .resizable(true)
        .build()
        .map_err(|e| format!("building launcher window {label}: {e}"))?;
    Ok(())
}

/// Pick the next free `main-N` label. Launchers spawn from the
/// File → New Window menu item; the singleton `main` from
/// tauri.conf.json keeps its bare label so existing
/// `show_window(app, "main")` callers and the `Drives` menu
/// entry keep working.
fn next_launcher_label(app: &tauri::AppHandle) -> String {
    let existing: std::collections::HashSet<String> = app
        .webview_windows()
        .into_keys()
        .filter(|l| l == "main" || l.starts_with("main-"))
        .collect();
    for n in 2u32..u32::MAX {
        let candidate = format!("main-{n}");
        if !existing.contains(&candidate) {
            return candidate;
        }
    }
    // Practically unreachable; falls back to a UUID-ish suffix so
    // the menu action still does *something* if a hostile loop
    // saturates the integer range.
    format!(
        "main-{:x}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    )
}

/// Eval a `chan:command` dispatch on the currently-focused drive
/// webview. Used by menu items that should defer to chan's per-drive
/// behavior (Settings). No-op when the focused window isn't a drive,
/// matching the "each window owns its own settings" model.
fn dispatch_to_focused_drive(app: &tauri::AppHandle, command: &str) {
    let Some(w) = app
        .webview_windows()
        .into_values()
        .find(|w| serve::is_drive_webview_label(w.label()) && w.is_focused().unwrap_or(false))
    else {
        return;
    };
    let js = format!(
        "window.dispatchEvent(new CustomEvent('chan:command', {{detail: {{name: {}}}}}));",
        serde_json::to_string(command).unwrap_or_else(|_| "\"\"".into())
    );
    let _ = w.eval(&js);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn desktop_binary_accepts_hidden_mcp_proxy_command() {
        const MAIN_RS: &str = include_str!("main.rs");
        assert!(MAIN_RS.contains("\"__mcp-proxy\""));
        assert!(MAIN_RS.contains("run_hidden_mcp_proxy_if_requested"));
        assert!(MAIN_RS.contains("run_mcp_proxy(socket)"));
    }

    #[test]
    fn normalize_outbound_url_accepts_http_and_strips_window_param() {
        let url = normalize_outbound_url(" http://127.0.0.1:4000/drive/?t=abc&w=old#files ")
            .expect("valid url");
        assert_eq!(url, "http://127.0.0.1:4000/drive/?t=abc#files");
    }

    #[test]
    fn normalize_outbound_url_rejects_non_http() {
        let err = normalize_outbound_url("file:///tmp/foo").expect_err("rejected");
        assert!(err.contains("http:// or https://"));
    }

    #[test]
    fn normalize_outbound_label_trims_and_caps() {
        assert_eq!(
            normalize_outbound_label("  Remote notes  ").expect("label"),
            "Remote notes",
        );
        let too_long = "x".repeat(OUTBOUND_LABEL_MAX_CHARS + 1);
        assert!(normalize_outbound_label(&too_long).is_err());
    }

    /// `fullstack-b-28b` slice iv: extension classifier maps the
    /// expected file types into the three media buckets + the
    /// markdown counter. Pin the mapping so a future drift between
    /// chan-drive's classification + chan-desktop's pre-flight
    /// doesn't silently mis-count files.
    #[test]
    fn classify_preflight_extension_maps_known_buckets() {
        let mut out = WalkOutcome::default();
        let cases = [
            ("notes.md", "markdown"),
            ("README.markdown", "markdown"),
            ("photo.jpg", "image"),
            ("ICON.PNG", "image"),
            ("scan.heic", "image"),
            ("tune.mp3", "audio"),
            ("loop.OGG", "audio"),
            ("clip.mp4", "video"),
            ("clip.MOV", "video"),
            ("README", "skip"),
            ("script.sh", "skip"),
            ("config.toml", "skip"),
        ];
        let mut expected_md = 0;
        let mut expected_img = 0;
        let mut expected_audio = 0;
        let mut expected_video = 0;
        for (name, kind) in cases {
            classify_preflight_extension(std::ffi::OsStr::new(name), &mut out);
            match kind {
                "markdown" => expected_md += 1,
                "image" => expected_img += 1,
                "audio" => expected_audio += 1,
                "video" => expected_video += 1,
                _ => {}
            }
        }
        assert_eq!(out.markdown_count, expected_md);
        assert_eq!(out.image_count, expected_img);
        assert_eq!(out.audio_count, expected_audio);
        assert_eq!(out.video_count, expected_video);
    }

    #[test]
    fn should_skip_preflight_dir_matches_chan_workspace_defaults() {
        let filter = chan_workspace::WalkFilter::new(
            chan_workspace::DEFAULT_INDEX_EXCLUDED_DIRS.iter().copied(),
        );
        for skip in [".chan"]
            .into_iter()
            .chain(chan_workspace::DEFAULT_INDEX_EXCLUDED_DIRS.iter().copied())
        {
            assert!(
                should_skip_preflight_dir(std::ffi::OsStr::new(skip), &filter),
                "{skip} must be skipped",
            );
        }
        assert!(should_skip_preflight_dir(
            std::ffi::OsStr::new("NODE_MODULES"),
            &filter
        ));
        let empty = chan_workspace::WalkFilter::default();
        assert!(should_skip_preflight_dir(
            std::ffi::OsStr::new(".git"),
            &empty
        ));
        assert!(!should_skip_preflight_dir(
            std::ffi::OsStr::new("node_modules"),
            &empty
        ));
        for keep in ["notes", "drafts", "src", "assets", ".github", "docs"] {
            assert!(
                !should_skip_preflight_dir(std::ffi::OsStr::new(keep), &filter),
                "{keep} must NOT be skipped",
            );
        }
    }

    #[test]
    fn detect_drive_scm_finds_git_hg_svn_at_root() {
        let tmp = TempDir::new().unwrap();
        // No SCM yet — None.
        assert_eq!(detect_drive_scm(tmp.path()), None);
        // git → "git"
        fs::create_dir_all(tmp.path().join(".git")).unwrap();
        assert_eq!(detect_drive_scm(tmp.path()).as_deref(), Some("git"));
    }

    #[test]
    fn walk_drive_preflight_counts_files_skips_excluded_dirs() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        fs::write(root.join("notes.md"), b"hello").unwrap();
        fs::write(root.join("photo.jpg"), b"xxx").unwrap();
        // Hidden in node_modules — must not be counted.
        fs::create_dir_all(root.join("node_modules")).unwrap();
        fs::write(root.join("node_modules/package.json"), b"{}").unwrap();
        // Hidden in .git — same.
        fs::create_dir_all(root.join(".git")).unwrap();
        fs::write(root.join(".git/HEAD"), b"ref: refs/heads/main").unwrap();
        // Nested user content — must be counted.
        fs::create_dir_all(root.join("notes")).unwrap();
        fs::write(root.join("notes/deep.md"), b"deep").unwrap();

        let filter = chan_workspace::WalkFilter::new(
            chan_workspace::DEFAULT_INDEX_EXCLUDED_DIRS.iter().copied(),
        );
        let out = walk_drive_preflight(root, &filter);
        assert_eq!(out.file_count, 3, "must skip node_modules + .git");
        assert_eq!(out.markdown_count, 2);
        assert_eq!(out.image_count, 1);
        assert!(!out.truncated);
        assert!(out.size_bytes > 0);
    }
}
