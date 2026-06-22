//! Local-workspace runtime and workspace-window helpers.
//!
//! chan-desktop opens local workspaces through the embedded chan-server
//! `WorkspaceHost`. Each running workspace is tracked in `AppState.serves`
//! with its route prefix and token-bearing URL. chan-desktop links
//! `chan-workspace` and `chan-server` directly; there is no `chan`
//! binary at runtime. Registry mutations and feature toggles run
//! in-process against the embedded host's shared `Library`, and
//! local serving never spawns `chan open`.

use std::hash::{DefaultHasher, Hash, Hasher};
use std::net::SocketAddr;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use chan_server::{WindowKind, WindowRecord};

/// Per-process monotonic counter appended to every workspace-window
/// label so the user can open more than one window for the same
/// workspace. Tauri requires unique window labels per process; the
/// prefix encodes the workspace identity and the seq disambiguates
/// instances.
static WINDOW_SEQ: AtomicU64 = AtomicU64::new(0);

fn next_window_seq() -> u64 {
    WINDOW_SEQ.fetch_add(1, Ordering::Relaxed)
}

use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent};

use crate::config::{self, WindowConfig};
use crate::AppState;

/// Tauri event emitted when any local runtime starts or stops. The
/// frontend reacts by re-fetching the workspace list.
pub const SERVES_CHANGED: &str = "serves-changed";

const MAX_WINDOWS_PER_WORKSPACE: usize = 10;

/// Window-title kind glyphs. A workspace window's title leads with one of
/// these so the OS title bar + window switcher encode the kind at a glance,
/// then the locator (path / URL). Emoji render as color glyphs in the macOS
/// title bar; named constants so swapping to a monochrome set (e.g. arrows
/// for outbound) is a one-line change each.
const ICON_LOCAL_HOME: &str = "\u{1F3E0}"; // house: local disk, under $HOME
const ICON_LOCAL_OTHER: &str = "\u{1F5A5}\u{FE0F}"; // desktop computer: local, elsewhere
const ICON_OUTBOUND: &str = "\u{1F4E4}"; // outbox tray: we dial OUT to a URL

/// Live state for one running serve. Held in `AppState.serves`
/// keyed by canonical workspace path.
pub struct ServeHandle {
    prefix: String,
    pub url: Option<String>,
}

impl ServeHandle {
    fn embedded(prefix: String, url: String) -> Self {
        Self {
            prefix,
            url: Some(url),
        }
    }
}

/// Open a local workspace through the embedded chan-server host.
pub async fn start(app: AppHandle, state: Arc<AppState>, key: String) -> Result<(), String> {
    if state.serves.lock().unwrap().contains_key(&key) {
        return Ok(());
    }
    let Some(embedded) = state.embedded.get() else {
        return Err("embedded local server is unavailable".to_string());
    };
    let url = embedded.open_workspace(&key).await?;
    let prefix = url_prefix_from_local_url(&url)?;
    {
        let mut serves = state.serves.lock().unwrap();
        if serves.contains_key(&key) {
            drop(serves);
            if let Err(e) = embedded.close_prefix(&prefix) {
                tracing::warn!(key = %key, error = %e, "closing duplicate embedded workspace failed");
            }
            return Ok(());
        }
        serves.insert(key.clone(), ServeHandle::embedded(prefix, url.clone()));
    }
    let _ = app.emit(SERVES_CHANGED, ());
    // Mint the FIRST window only when this workspace has no persisted window
    // record yet (a fresh turn-on); the watcher then opens it. On a re-on or
    // boot re-serve the records already exist, and the mount above (which fired
    // the library change signal) makes them live, so the watcher reopens them
    // at their stable window_id — restoring each window's tabs. The registry is
    // the sole window-creation authority; there is no imperative window build.
    let has_window = embedded.assemble_window_records().iter().any(|r| {
        r.kind == WindowKind::Workspace && r.workspace_path.as_deref() == Some(key.as_str())
    });
    if !has_window {
        if let Err(e) = embedded.mint_window(WindowKind::Workspace, Some(key.clone())) {
            if let Some(handle) = state.serves.lock().unwrap().remove(&key) {
                stop_handle(None, &state, &key, handle);
            }
            let _ = app.emit(SERVES_CHANGED, ());
            return Err(e);
        }
    }
    Ok(())
}

fn url_prefix_from_local_url(url: &str) -> Result<String, String> {
    let parsed = url
        .parse::<url::Url>()
        .map_err(|e| format!("parsing embedded workspace URL: {e}"))?;
    let path = parsed.path().trim_end_matches('/');
    let path = path.strip_suffix("/index.html").unwrap_or(path);
    if path.is_empty() {
        Ok(String::new())
    } else {
        Ok(path.to_string())
    }
}

/// Stop a running serve. No-op if the workspace isn't running. Removes
/// the live entry before waiting so an immediate stop -> start can
/// mount a fresh runtime instead of observing stale map state.
pub fn stop(app: Option<&AppHandle>, state: &AppState, key: &str) {
    let handle = state.serves.lock().unwrap().remove(key);
    if let Some(h) = handle {
        stop_handle(app, state, key, h);
    }
}

/// Stop every running serve. Called from the Tauri Exit hook so
/// embedded workspace state shuts down before the desktop exits.
pub fn stop_all(state: &AppState) {
    let handles: Vec<(String, ServeHandle)> = state.serves.lock().unwrap().drain().collect();
    for (key, h) in handles {
        stop_handle(None, state, &key, h);
    }
}

fn stop_handle(app: Option<&AppHandle>, state: &AppState, key: &str, handle: ServeHandle) {
    if let Some(embedded) = state.embedded.get() {
        if let Err(e) = embedded.close_prefix(&handle.prefix) {
            tracing::warn!(key = %key, error = %e, "closing embedded workspace failed");
        }
    }
    if let Some(app) = app {
        // No imperative window teardown: unmounting fired the library change
        // signal, so the watcher reconciles the now-tenant-less windows closed
        // (their token emptied → not shown) while KEEPING the persisted records,
        // so turning the workspace back on reopens them at the same window_id.
        let _ = app.emit(SERVES_CHANGED, ());
    }
}

/// Stable Tauri window-label prefix for a local workspace. Used to
/// recognise every window that belongs to the workspace when the user
/// has opened more than one (close-all on serve exit, capability
/// matching). Tauri labels must match `[a-zA-Z0-9_-]+`, and workspace
/// keys are filesystem paths, so we hash the key.
pub fn workspace_window_prefix(key: &str) -> String {
    let mut h = DefaultHasher::new();
    key.hash(&mut h);
    format!("workspace-{:016x}", h.finish())
}

/// Window title for a local-workspace webview: a kind glyph (home vs this
/// machine) then the workspace path. The path is the locator (the
/// disambiguating signal in the OS window switcher); the glyph prefix
/// makes the kind read at a glance.
fn workspace_title(key: &str) -> String {
    local_title(key, dirs::home_dir().as_deref())
}

/// Pure home-vs-elsewhere title formatting, split from `workspace_title` so it
/// is testable without depending on the process's real home dir. A path under
/// `home` gets the house glyph; anything else (or no resolvable home) gets the
/// computer glyph.
fn local_title(key: &str, home: Option<&Path>) -> String {
    let icon = match home {
        Some(home) if Path::new(key).starts_with(home) => ICON_LOCAL_HOME,
        _ => ICON_LOCAL_OTHER,
    };
    format!("{icon} {key}")
}

/// Title for a devserver (remote) webview, per spec `icon devserver / repo`:
/// the remote glyph, the devserver's display name, then the workspace's repo
/// (the path basename). `build_workspace_window` appends ` Window {N}`. A
/// terminal carries no workspace, so it reads `icon devserver Terminal`. The
/// full remote path is NOT used (it would read as a meaningless local path —
/// `workspace_title`'s home-vs-computer glyph is wrong for a remote box).
fn devserver_window_title(devserver_name: &str, record: &WindowRecord) -> String {
    match record.kind {
        WindowKind::Terminal => format!("{ICON_OUTBOUND} {devserver_name} Terminal"),
        WindowKind::Workspace => {
            let repo = record
                .workspace_path
                .as_deref()
                .and_then(|p| Path::new(p).file_name())
                .and_then(|n| n.to_str());
            match repo {
                Some(repo) => format!("{ICON_OUTBOUND} {devserver_name} / {repo}"),
                None => format!("{ICON_OUTBOUND} {devserver_name}"),
            }
        }
    }
}

/// Stable window-label prefix for an outbound URL attachment.
pub fn outbound_window_prefix(id: &str) -> String {
    let mut h = DefaultHasher::new();
    id.hash(&mut h);
    format!("outbound-{:016x}", h.finish())
}

/// Fresh, unique window label for an outbound URL webview.
pub fn new_outbound_window_label(id: &str) -> String {
    format!("{}-{}", outbound_window_prefix(id), next_window_seq())
}

/// True when a Tauri label belongs to an embedded-served SPA webview
/// (workspace / outbound / standalone terminal). All three host the chan
/// SPA and accept the `chan:command` dispatch bridge, so menu items that
/// defer to the focused window (Settings, New Terminal's toggle branch)
/// target any of them.
pub fn is_workspace_webview_label(label: &str) -> bool {
    label.starts_with("workspace-")
        || label.starts_with("outbound-")
        || label.starts_with("terminal-")
        // Watcher-opened local windows carry the composite native label
        // `local::<window_id>`; they host the same embedded SPA.
        || label.starts_with("local::")
        // Watcher-opened devserver windows carry `lib-<hex>::<window_id>` — the
        // same SPA, served by the remote devserver.
        || label.starts_with("lib-")
}

/// Open (or rebuild-in-place at the same label) a native window for a
/// library-minted local window `record`, driven by the window watcher (the
/// SOLE caller). The Tauri label is the composite native key
/// `{library_id}::{window_id}`; the loaded SPA carries `?w=<window_id>` — the
/// bare per-library session key, decoupled from the OS-window label. Local
/// tenants are always up, so the tenant URL loads directly (no connecting
/// screen). An off workspace carries an empty token and the SPA turns it on
/// before attaching (O-W2).
pub(crate) fn open_watched_local_window(
    app: &AppHandle,
    addr: SocketAddr,
    record: &WindowRecord,
) -> Result<(), String> {
    let label = crate::window_watcher::native_label(record);
    let url = format!(
        "http://{addr}{}/index.html?t={}",
        record.prefix, record.token
    );
    let (title, kind) = match record.kind {
        WindowKind::Terminal => ("Terminal".to_string(), Some("terminal")),
        WindowKind::Workspace => (
            record
                .workspace_path
                .as_deref()
                .map(workspace_title)
                .unwrap_or_else(|| "Workspace".to_string()),
            None,
        ),
    };
    build_workspace_window(
        app,
        WindowSpec {
            label: &label,
            session_id: &record.window_id,
            library_id: &record.library_id,
            title: &title,
            url: &url,
            url_hash_seed: "",
            config_key: String::new(),
            zoom_seed: 1.0,
            connecting: None,
            kind,
        },
    )
}

/// Open a watched REMOTE (devserver) window — the watcher's analog of
/// [`open_watched_local_window`], but the SPA is served by the remote devserver
/// at `host:port`, so the navigate target is the assembled tenant URL and the
/// window routes through the connecting screen (the remote may be down). The
/// native label is the composite `{library_id}::{window_id}`; `?w=` is the bare
/// `window_id` (decoupled), carried as the SPA session id. No `config_key`: the
/// library owns persistence (the layout blob is keyed by `window_id`).
pub(crate) fn open_watched_remote_window(
    app: &AppHandle,
    host: &str,
    port: u16,
    devserver_name: &str,
    record: &WindowRecord,
) -> Result<(), String> {
    let label = crate::window_watcher::native_label(record);
    let url = crate::devserver::assemble_tenant_url(host, port, &record.prefix, &record.token)?;
    let title = devserver_window_title(devserver_name, record);
    let kind = match record.kind {
        WindowKind::Terminal => Some("terminal"),
        WindowKind::Workspace => None,
    };
    build_workspace_window(
        app,
        WindowSpec {
            label: &label,
            session_id: &record.window_id,
            library_id: &record.library_id,
            title: &title,
            url: &url,
            url_hash_seed: "",
            config_key: String::new(),
            zoom_seed: 1.0,
            connecting: Some(url.as_str()),
            kind,
        },
    )
}

/// Spawn a new outbound URL webview window. The desktop does not own
/// the remote process; this only creates another webview pointed at
/// the persisted URL.
pub fn spawn_remote_workspace_window(
    app: &AppHandle,
    id: &str,
    url: &str,
) -> Result<String, String> {
    let prefix = outbound_window_prefix(id);
    let config_key = config::remote_window_key(id);
    let restore =
        match unbury_or_restore(app, &prefix, &config_key, || new_outbound_window_label(id))? {
            OpenOutcome::Unburied(label) => {
                crate::refresh_remote_windows_menu(app);
                return Ok(label);
            }
            OpenOutcome::Build(restore) => restore,
        };
    // Outbound title is the outbound glyph + the URL (the locator),
    // not the user's label (which still names the launcher row).
    let title = remote_window_title(url);
    let label = restore.label.clone();
    // Outbound = an outgoing connection to a remote we do not own. Route
    // through the connecting screen so a down remote shows a retrying
    // surface instead of a blank white webview. `url` is the display +
    // probe URL; `build_workspace_window` assembles the navigate target.
    let built = build_workspace_window(
        app,
        WindowSpec {
            label: &restore.label,
            session_id: &restore.label,
            // An outbound URL attachment is not part of any chan-library, so it
            // carries no `?lib=` (the SPA defaults it to `local`, isolating its
            // tab d&d to itself).
            library_id: "",
            title: &title,
            url,
            url_hash_seed: &restore.url_hash,
            config_key,
            zoom_seed: restore.zoom,
            connecting: Some(url),
            kind: None,
        },
    );
    // An outbound window just appeared: re-poll the remote's window
    // list so the Window menu's remote section reflects it.
    crate::refresh_remote_windows_menu(app);
    built.map(|()| label)
}

/// Mint a standalone terminal window. Like every local window it is a library
/// registry row (`local::<id>`), so it persists and restores across quit/reopen;
/// the watcher opens it (in `?kind=terminal` mode) at the ONE shared `/terminal`
/// tenant, mounted on first use. All terminal windows share that tenant — so a
/// terminal moved between windows keeps its live PTY — and it lives for the
/// process lifetime (orphaned PTYs idle-prune). Returns the new window's
/// composite native label.
pub async fn spawn_local_terminal_window(state: Arc<AppState>) -> Result<String, String> {
    let Some(embedded) = state.embedded.get() else {
        return Err("embedded local server is unavailable".to_string());
    };
    // Ensure the shared terminal tenant is mounted (records its prefix so the
    // minted record resolves to it); cached after the first mount.
    embedded.open_terminal().await?;
    // Mint the window; the watcher opens it. The registry is the sole window
    // authority, so the terminal can never be double-opened and it persists.
    let record = embedded.mint_window(WindowKind::Terminal, None)?;
    Ok(crate::window_watcher::native_label(&record))
}

/// A spawned control terminal: its terminal tenant prefix, used to scrape the
/// token the connect script prints (and, on disconnect, to reap the tenant).
/// The window is addressed by its deterministic `control_terminal_label`, so
/// the struct doesn't carry the label.
pub struct ControlTerminal {
    pub prefix: String,
}

/// Spawn a control terminal: a standalone terminal window whose PTY runs a
/// devserver's connect script. The script brings the devserver up, possibly
/// over an interactive ssh session whose prompts the user answers in the
/// window. The label is stable per devserver so the connect flow can tuck
/// the window away once connected and a later reopen finds the same window.
pub async fn spawn_control_terminal_window(
    app: AppHandle,
    state: Arc<AppState>,
    devserver_id: &str,
    script: String,
) -> Result<ControlTerminal, String> {
    let Some(embedded) = state.embedded.get() else {
        return Err("embedded local server is unavailable".to_string());
    };
    let (url, prefix) = embedded.open_terminal_with_command(script).await?;
    let label = control_terminal_label(devserver_id);
    build_workspace_window(
        &app,
        WindowSpec {
            label: &label,
            session_id: &label,
            // The control terminal runs on the local embedded library's shared
            // terminal tenant, so it belongs to the `local` library.
            library_id: "local",
            title: "Control Terminal",
            url: &url,
            url_hash_seed: "",
            config_key: String::new(),
            zoom_seed: 1.0,
            connecting: None,
            // `control` (not `terminal`) puts the SPA in the singleton
            // control sub-mode: terminal-only, but with the tab strip / pane
            // chrome hidden and Cmd+T / splits disabled so it never spawns a
            // second tab. It also tags the window kind in `cs window list`,
            // keeping it distinct from persisted standalone terminals (W10).
            kind: Some("control"),
        },
    )?;
    Ok(ControlTerminal { prefix })
}

/// Stable window label for a devserver's control terminal.
pub fn control_terminal_label(devserver_id: &str) -> String {
    format!("control-terminal-{devserver_id}")
}

/// `cs window open`: focus a live window, un-hide a buried one, or
/// best-effort reopen a closed-but-saved workspace window whose
/// workspace is still running. Errors when the id names nothing the
/// desktop can act on.
/// Resolve the id an open/hide op carries to a native window label. The
/// launcher's status-dot affordance sends a BARE library-minted `window_id`
/// (e.g. `w-1a2b`), but a watched window's native label is the composite
/// `{library_id}::{window_id}` ([`crate::window_watcher::native_label`]) — so a
/// bare id never matches `get_webview_window` directly. `cs window` callers
/// pass the full label already (composite, or a legacy `terminal-`/`workspace-`
/// scheme), so an id that is itself a live label OR already contains `::` is
/// used verbatim. Otherwise match the open native window whose label ends with
/// `::{id}` (a devserver window hides its webview alive, so the scan finds it).
/// With NO live window the id is a buried LOCAL watched window — the reconcile
/// destroyed its native window on bury, so it can't be scanned; fall back to the
/// `local::` composite for the view-driven un-bury in [`open_window_by_label`].
pub(crate) fn resolve_window_label(app: &AppHandle, id: &str) -> String {
    // A live window whose exact label IS `id` wins — covers `cs window` passing a
    // legacy `terminal-`/`workspace-` label that carries no `::`.
    if app.get_webview_window(id).is_some() {
        return id.to_string();
    }
    let open: Vec<String> = app.webview_windows().into_keys().collect();
    resolve_label_from(id, &open)
}

/// Pure resolution core (unit-testable without a live Tauri app): pick the
/// native label for `id` given the currently-open native labels. A composite or
/// legacy label (one containing `::`) is used verbatim; a bare `window_id`
/// matches the open `{library_id}::{id}` window; with none open it resolves to
/// the local composite (the only windows that bury by DESTROY — vs hide-alive —
/// are the local library's, so an un-scannable bare id is a buried local one).
fn resolve_label_from(id: &str, open_labels: &[String]) -> String {
    if id.contains("::") {
        return id.to_string();
    }
    let suffix = format!("::{id}");
    if let Some(label) = open_labels.iter().find(|l| l.ends_with(&suffix)) {
        return label.clone();
    }
    format!("local::{id}")
}

pub fn open_window_by_label(
    app: &AppHandle,
    state: &Arc<AppState>,
    label: &str,
) -> Result<(), String> {
    let label = resolve_window_label(app, label);
    let label = label.as_str();
    // A watched LOCAL window un-buries through the watcher view: its bury
    // DESTROYED the native window (the reconcile closed it), so there is no
    // webview to `show()` — `unbury_window` flips the view and the reconcile
    // reopens it at its `window_id`. This must run even when there is no live
    // webview, so it precedes the `get_webview_window` check below.
    if label.starts_with("local::") {
        crate::unbury_window(app, label);
        return Ok(());
    }
    if app.get_webview_window(label).is_some() {
        // Live (visible or hidden-alive — e.g. a devserver window): `unbury_window`
        // shows + focuses, and drops it from the buried list / Window menu if it
        // was hidden.
        crate::unbury_window(app, label);
        return Ok(());
    }
    // Not live: best-effort reopen a saved workspace window if we can
    // resolve a still-running workspace from the label's hash prefix.
    // Terminal windows are ephemeral (never saved), so only workspace
    // labels reach a useful branch here.
    if let Some((key, url)) = running_workspace_for_label(state, label) {
        let title = workspace_title(&key);
        // Reuse the EXACT saved label so the SPA's `GET /api/session?w=`
        // restores this window's panes/tabs.
        build_workspace_window(
            app,
            WindowSpec {
                label,
                session_id: label,
                // A running local workspace window belongs to the `local` library.
                library_id: "local",
                title: &title,
                url: &url,
                url_hash_seed: "",
                config_key: config::local_window_key(&key),
                zoom_seed: 1.0,
                connecting: None,
                kind: None,
            },
        )?;
        return Ok(());
    }
    Err(format!(
        "window {label} isn't open; if it's a saved workspace window, open its workspace first"
    ))
}

/// (key, launch URL) of the running local workspace whose window-label
/// family matches `label` (`workspace-<hash(key)>-<seq>`), or `None` when
/// no running workspace owns that label.
fn running_workspace_for_label(state: &Arc<AppState>, label: &str) -> Option<(String, String)> {
    let serves = state.serves.lock().unwrap();
    serves.iter().find_map(|(key, handle)| {
        let prefix = workspace_window_prefix(key);
        if label.starts_with(&format!("{prefix}-")) {
            handle.url.clone().map(|url| (key.clone(), url))
        } else {
            None
        }
    })
}

/// True when `label`'s window still has at least one live PTY shell — the
/// `cs window rm` confirmation gate. Resolves the shared terminal tenant
/// for `terminal-*` labels and the owning workspace tenant otherwise.
pub fn window_has_live_shells(state: &Arc<AppState>, label: &str) -> bool {
    let Some(embedded) = state.embedded.get() else {
        return false;
    };
    if label.starts_with("terminal-") {
        embedded.terminal_window_has_live_shells(label)
    } else if let Some((key, _)) = running_workspace_for_label(state, label) {
        embedded.workspace_window_has_live_shells(&key, label)
    } else {
        false
    }
}

/// Base window title for an outbound (we-dial-out) workspace window.
/// pub: the remote Window-menu refresh derives the same title without
/// opening a window.
pub fn remote_window_title(url: &str) -> String {
    format!("{ICON_OUTBOUND} {url}")
}

/// Reopen a REMOTE-known window (a `saved && !connected` row from the
/// remote serve's `GET /api/windows`) by building a webview with that
/// exact label: the `?w=<label>` the build appends makes the remote
/// hydrate that window's session blob, so the panes/tabs the user left
/// there come back. No LRU pop — the restore state lives remote-side.
pub fn reopen_remote_window(
    app: &AppHandle,
    label: &str,
    entry: &crate::RemoteReopen,
) -> Result<(), String> {
    build_workspace_window(
        app,
        WindowSpec {
            label,
            session_id: label,
            // The imperative remote-reopen path predates the window-watcher feed
            // that carries `library_id`; it has no record to read one from, so it
            // passes none (the SPA defaults to `local`).
            library_id: "",
            title: &entry.base_title,
            url: &entry.url,
            url_hash_seed: "",
            config_key: entry.config_key.clone(),
            zoom_seed: 1.0,
            // Outbound remotes route through the connecting screen like
            // any other outbound window (a down remote must not paint a
            // blank webview).
            connecting: entry.connecting.then_some(entry.url.as_str()),
            kind: None,
        },
    )
}

/// True when the webview is still showing the bundled connecting/retry
/// screen (`connecting.html`, the outbound pre-navigation page). Such a
/// window has no per-window session, no shells, and nothing to restore,
/// so close affordances treat it as cancel-and-really-close instead of
/// burying. The URL read is guarded like `capture_window_config`: a
/// dead webview's `url()` can panic on a nil URL, and any failure reads
/// as "not the connecting screen" (bury — the safe pre-existing path).
pub fn window_on_connecting_screen(app: &AppHandle, label: &str) -> bool {
    let Some(window) = app.get_webview_window(label) else {
        return false;
    };
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| window.url())) {
        Ok(Ok(url)) => url.path().ends_with("connecting.html"),
        _ => false,
    }
}

/// Reopen this workspace family's most recently buried window instead
/// of spawning a new one, when one exists. Every "open a window for
/// this workspace" entry point (launcher Open, Cmd/Ctrl+Shift+N's
/// spawn fallback, deep links) funnels through the spawn fns, so the
/// check lives here: a window the user put away via the close button
/// IS the window they get back — a reopens-the-last-closed-window
/// feel, with live state. `prefix` is the family
/// prefix WITHOUT the trailing dash (the spawn fns' label prefix).
/// Raise the family's most recent hidden window instead of spawning a
/// fresh one, returning that window's label when one was unburied. `None`
/// means nothing was buried (so the caller should build a new window).
fn unbury_instead_of_spawn(app: &AppHandle, prefix: &str) -> Option<String> {
    let family = format!("{prefix}-");
    let buried = app.state::<Arc<AppState>>().most_recent_buried(&family)?;
    if crate::unbury_window(app, &buried) {
        Some(buried)
    } else {
        None
    }
}

/// Label + restore state for a window about to be (re)built, popped
/// from the window-config stack or freshly minted.
struct RestoredWindow {
    label: String,
    url_hash: String,
    zoom: f64,
}

/// Shared open preamble for the local / outbound spawn fns:
/// prefer unburying the family's most recent hidden window, enforce the
/// per-family window cap, then pop a compatible WindowConfig for the
/// label + restore state (fresh label, empty hash, default zoom when
/// nothing restorable exists). `Ok(None)` means an unburied window
/// already satisfied the open and no new window should be built.
fn unbury_or_restore(
    app: &AppHandle,
    prefix: &str,
    config_key: &str,
    fresh_label: impl FnOnce() -> String,
) -> Result<OpenOutcome, String> {
    if let Some(label) = unbury_instead_of_spawn(app, prefix) {
        return Ok(OpenOutcome::Unburied(label));
    }
    ensure_window_capacity(app, prefix)?;
    let restore = pop_compatible_config(app, config_key, prefix);
    Ok(OpenOutcome::Build(RestoredWindow {
        label: restore
            .as_ref()
            .map(|c| c.window_label.clone())
            .unwrap_or_else(fresh_label),
        url_hash: restore
            .as_ref()
            .map(|c| c.url_hash.clone())
            .unwrap_or_default(),
        zoom: restore.as_ref().map(|c| c.zoom_level).unwrap_or(1.0),
    }))
}

/// Result of the open preamble: either an already-hidden window of the
/// family was raised (no build needed), or a fresh/restored window should
/// be built. Either way the caller can report the resolved window label.
enum OpenOutcome {
    Unburied(String),
    Build(RestoredWindow),
}

/// Pop the top-of-stack window config for `config_key` only if the
/// stored label is safe to reuse. Live-label entries are SKIPPED in
/// place (not popped): a buried window's entry must survive for the
/// quit-while-buried restore, and Tauri labels are unique per process
/// so reusing one would collide. The popped label must additionally
/// match the workspace's current hash prefix (defends against the
/// workspace key changing canonicalisation under us); a stale-prefix
/// entry gets dropped on the floor — we don't keep cycling through
/// stale stack entries, since the next bury pushes a fresh one anyway.
fn pop_compatible_config(
    app: &AppHandle,
    config_key: &str,
    expected_prefix: &str,
) -> Option<WindowConfig> {
    let state = app.state::<Arc<AppState>>();
    let entry =
        state.pop_window_config(config_key, |label| app.get_webview_window(label).is_some())?;
    if !entry.window_label.starts_with(expected_prefix) {
        tracing::debug!(
            label = %entry.window_label,
            prefix = %expected_prefix,
            "discarding window config with stale prefix",
        );
        return None;
    }
    Some(entry)
}

/// Inputs for one SPA webview window build: identity (label/title),
/// where to point it, what to restore, and how to load.
struct WindowSpec<'a> {
    /// Unique Tauri window label: the OS-window identity, decoupled from the
    /// SPA session key (`session_id`). For most windows the two are equal; the
    /// window watcher's composite native label (`{library_id}::{window_id}`)
    /// differs from its bare `?w=` (`window_id`).
    label: &'a str,
    /// The `?w=` per-window SPA session key appended to the loaded URL — what
    /// the SPA keys its session blob / `/ws` presence on. Equals `label`
    /// except for watcher-opened windows, which pass the bare `window_id`.
    session_id: &'a str,
    /// The owning chan-library's id, appended as `?lib=` so the SPA can scope
    /// cross-window tab drag-and-drop to the same library (`local` for the
    /// baked-in local disk library, `lib-<hex>` for a devserver). The SPA
    /// defaults a missing `?lib=` to `local`, so a window with no library
    /// identity (an outbound URL attachment) passes the empty string.
    library_id: &'a str,
    /// Base title; the builder suffixes a reused " Window N" display number.
    title: &'a str,
    /// The workspace/terminal URL the webview ultimately shows.
    url: &'a str,
    /// URL fragment from the window-config stack: applied verbatim so
    /// overlay state (file browser path, search query, graph scope)
    /// restores alongside the panes/tabs that come back from
    /// `session.json`. Empty when there's nothing to restore.
    url_hash_seed: &'a str,
    /// WindowConfig identity key (`local_window_key` or
    /// `remote_window_key`). Stamped onto the close handler so a
    /// user-initiated close pushes the window's final URL hash back
    /// into the LRU stack. Empty for terminal windows (no LRU restore).
    config_key: String,
    /// Zoom level to restore; 1.0 (the default) skips the IPC round-trip.
    zoom_seed: f64,
    /// Load strategy. `None` (local) loads `url` directly via
    /// `WebviewUrl::External`: that backend is up before the window
    /// opens. `Some(display_url)` (outbound) instead loads the bundled
    /// `connecting.html` and hands it the display URL plus the assembled
    /// navigate target through an injected `window.__CHAN_CONNECTING__`;
    /// the page probes the remote via `probe_url` and navigates on
    /// success. A direct External load of a down outbound remote paints a
    /// blank white webview (WKWebView never finishes navigating, see
    /// `capture_window_config`), which is the bug the connecting screen
    /// fixes.
    connecting: Option<&'a str>,
    /// `Some("terminal")` makes the SPA boot in terminal-only mode (no
    /// workspace fetch); `Some("control")` is the stricter singleton control
    /// sub-mode (terminal-only + hidden chrome, one PTY); `None` is full
    /// workspace mode. Also the kind `cs window list` shows.
    kind: Option<&'a str>,
}

/// Build and show a chan-style workspace webview window on the main
/// thread. Internal: call `open_watched_local_window` (the watcher path) /
/// `spawn_remote_workspace_window` from outside. Centralising the
/// key-bridge JS, the size defaults, the zoom-hotkey polyfill, and
/// the drag-drop handler off in one place means workspace UX changes
/// don't fork between the local and outbound paths.
fn build_workspace_window(app: &AppHandle, spec: WindowSpec<'_>) -> Result<(), String> {
    let WindowSpec {
        label: window_label,
        session_id,
        library_id,
        title,
        url,
        url_hash_seed,
        config_key,
        zoom_seed,
        connecting,
        kind,
    } = spec;
    let Ok(mut parsed) = url.parse::<tauri::Url>() else {
        return Err(format!("bad chan URL for {window_label}: {url}"));
    };
    // The SPA keys its per-window session (panes/tabs, `/ws` presence) on
    // `?w=`; that is the `session_id`, NOT the Tauri label (they diverge only
    // for watcher-opened windows, where the label is the composite native key).
    parsed.query_pairs_mut().append_pair("w", session_id);
    // `kind=terminal` / `kind=control` are the SPA's only signal to enter
    // terminal-only mode (no workspace fetch, terminal panes only);
    // `control` additionally selects the singleton control sub-mode.
    // Workspace/outbound windows pass `None` and the SPA stays in full
    // workspace mode.
    if let Some(kind) = kind {
        parsed.query_pairs_mut().append_pair("kind", kind);
    }
    // `lib=<library_id>` next to `?w=`/`?kind=` tells the SPA which chan-library
    // this window belongs to, so cross-window tab d&d accepts a drop only from
    // the same library. Skipped when empty (an outbound URL attachment has no
    // library identity; the SPA defaults a missing `?lib=` to `local`).
    if !library_id.is_empty() {
        parsed.query_pairs_mut().append_pair("lib", library_id);
    }
    if !url_hash_seed.is_empty() {
        parsed.set_fragment(Some(url_hash_seed));
    }
    // The connecting page receives its inputs before any page script runs
    // (same mechanism as KEY_BRIDGE_JS). `target` is the fully-assembled
    // navigate URL (remote + ?w=<label> + restored #fragment) so the SPA's
    // per-window state + restore survive the success navigation.
    let (webview_url, init_script) = match connecting {
        Some(display_url) => {
            let payload = serde_json::json!({
                "url": display_url,
                "target": parsed.as_str(),
            });
            let script = format!("window.__CHAN_CONNECTING__ = {payload};\n{KEY_BRIDGE_JS}");
            (WebviewUrl::App("connecting.html".into()), script)
        }
        None => (WebviewUrl::External(parsed), KEY_BRIDGE_JS.to_string()),
    };
    let app_owned = app.clone();
    let label_owned = window_label.to_string();
    let title_owned = title.to_string();
    // The SPA's `?w=` session id (= `WindowRecord.window_id` for a watcher
    // window), owned so the 'static close handler can query the active-transfer
    // guard by it — it diverges from the native label for watcher windows.
    let session_owned = session_id.to_string();
    // The passed kind (`terminal` / `control`) for terminal windows, else
    // "workspace" (covers local / outbound) — the kind `cs window list` shows.
    // Captured owned so the 'static main-thread closure can hold it.
    let kind_owned = kind.unwrap_or("workspace").to_string();
    let res = app.run_on_main_thread(move || {
        // Defensive: window labels are unique-per-instance now, so
        // a collision shouldn't happen. If it ever does (e.g. some
        // future code reusing a stable label), destroy the stale
        // window so `build` doesn't panic.
        if let Some(old) = app_owned.get_webview_window(&label_owned) {
            let _ = old.destroy();
        }
        // Suffix a reused, lowest-free display number so the OS Window
        // menu disambiguates windows that share a base title (two
        // windows on one workspace, several standalone terminals). The
        // number is freed on close (Ok branch handler / Err branch
        // below) so the next same-base window reuses it.
        let state = app_owned.state::<Arc<AppState>>();
        let window_number = state.assign_window_number(&label_owned, &title_owned);
        // A `cs window title` override (kept across the bury/reopen cycle)
        // wins over the auto "{base} Window {N}" scheme; otherwise use the
        // default. The resolved title is registered below once the window
        // builds, so `cs window list` shows what the title bar shows.
        let display_title = state
            .window_title_override(&label_owned)
            .unwrap_or_else(|| format!("{title_owned} Window {window_number}"));
        match WebviewWindowBuilder::new(&app_owned, &label_owned, webview_url)
            .title(display_title.clone())
            .inner_size(1200.0, 800.0)
            .min_inner_size(640.0, 400.0)
            .resizable(true)
            .initialization_script(init_script.as_str())
            // The explicit `zoom_in` / `zoom_out` / `zoom_reset`
            // IPC commands fired from KEY_BRIDGE_JS
            // are the primary path; this Tauri-level polyfill stays
            // on as a mousewheel + pinch fallback (the chord
            // overlap is harmless because KEY_BRIDGE_JS's capture-
            // phase listener calls preventDefault before the
            // polyfill's bubble-phase listener sees the keydown).
            // Requires `core:webview:allow-set-webview-zoom` on
            // workspace-* / outbound-* windows per
            // capabilities/workspace.json.
            .zoom_hotkeys_enabled(true)
            // Hand HTML5 drag-and-drop to the page — this must stay
            // disabled. With wry's native handler enabled, WebKit
            // never sees ANY drag on macOS (wry forwards to the OS
            // default only when the handler returns false, and
            // tauri-runtime-wry's handler returns true
            // unconditionally), which kills the editor/file-browser
            // drop zones AND in-page pane-to-pane tab moves. The
            // SPA's window-level drop guard owns the no-takeover
            // guarantee for stray OS file drops, and the terminal
            // path-print reads the drag pasteboard via
            // `read_dropped_paths` (dropped_paths.rs) instead of
            // native drag events.
            .disable_drag_drop_handler()
            .build()
        {
            Ok(window) => {
                // Register the OS title + kind so `cs window list` shows
                // the same title the title bar does. The `Destroyed` arm
                // below drops the entry. No-op without an embedded server
                // (there always is one in the desktop).
                if let Some(embedded) = state.embedded.get() {
                    embedded.window_titles().set(
                        &label_owned,
                        chan_server::WindowMeta {
                            title: display_title.clone(),
                            kind: Some(kind_owned.clone()),
                        },
                    );
                }
                // Restore the persisted zoom level from
                // the popped WindowConfig (if any). 1.0 is the chan-
                // desktop default; skip the IPC round-trip when there's
                // nothing to apply. Best-effort: a Tauri set_zoom error
                // here just leaves the new window at default zoom; the
                // user can re-press Cmd++/Cmd+- to recover.
                if (zoom_seed - 1.0).abs() > f64::EPSILON {
                    if let Err(e) = window.set_zoom(zoom_seed) {
                        tracing::warn!(
                            label = %label_owned,
                            error = %e,
                            "restoring window zoom level failed",
                        );
                    } else {
                        let state = app_owned.state::<Arc<AppState>>();
                        state
                            .live_window_zooms
                            .lock()
                            .unwrap()
                            .insert(label_owned.clone(), zoom_seed);
                    }
                }
                let app_for_close = app_owned.clone();
                let label_for_close = label_owned.clone();
                let key_for_close = config_key.clone();
                let session_for_close = session_owned.clone();
                window.on_window_event(move |event| match event {
                    // The OS close button BURIES an SPA window instead of
                    // destroying it: the webview hides, live terminals and
                    // layout state stay warm, and the Window menu (or
                    // Cmd/Ctrl+Shift+N) reopens it. Two exceptions really
                    // close: a standalone terminal window with NO live
                    // shells left, and a window still showing the
                    // connecting/retry screen (no session, no shells —
                    // burying it would leave an unkillable hidden retry
                    // loop).
                    // Programmatic closes (the SPA's empty-window cascade,
                    // workspace-off teardown) call `destroy()`
                    // and never reach this branch.
                    WindowEvent::CloseRequested { api, .. } => {
                        let state = app_for_close.state::<Arc<AppState>>();
                        // A5: a launcher status-dot hide routes through this same
                        // close path (so the bury handler runs) but is its own
                        // explicit hide gesture — consume its one-shot flag here so
                        // the bury below skips the teaching notice. A genuine
                        // red-button close finds no flag and shows it.
                        let silent_hide = state.take_silent_hide(&label_for_close);
                        // Active-transfer guard (BEFORE any bury/close path): a
                        // window with an in-flight upload/download must never close
                        // silently and kill the transfer. A LOCAL window reports its
                        // count through the embedded host (keyed on the `?w=` session
                        // id), and its red-dot close DESTROYS it — so the prompt
                        // offers "Cancel transfer & close" vs "Keep open". A
                        // connected-DEVSERVER window's transfer lives in the remote
                        // SPA + server, surfaced via the `active_transfer` feed bit
                        // (cached, keyed by composite label); its red-dot close only
                        // HIDES it (the transfer keeps running in the live webview),
                        // so that prompt is "Hide" vs "Keep open" — the desktop never
                        // cancels a remote transfer (the user does, from the SPA).
                        if state
                            .embedded
                            .get()
                            .map(|e| e.window_has_active_transfer(&session_for_close))
                            .unwrap_or(false)
                        {
                            api.prevent_close();
                            prompt_transfer_close(&app_for_close, &state, &label_for_close);
                            return;
                        }
                        if state.devserver_window_has_active_transfer(&label_for_close) {
                            api.prevent_close();
                            prompt_devserver_transfer_close(
                                &app_for_close,
                                &state,
                                &label_for_close,
                            );
                            return;
                        }
                        // A watcher-managed local window (`local::<id>`): the
                        // red-dot close BURIES it through the watcher view state
                        // (should_show false -> the reconcile closes the native
                        // window; the record stays, reopenable from the Window
                        // menu). Mirror into the legacy buried list so the menu
                        // lists it; the Destroyed handler keeps it there because
                        // it is in the watcher bury set.
                        if label_for_close.starts_with("local::") {
                            api.prevent_close();
                            let title = app_for_close
                                .get_webview_window(&label_for_close)
                                .and_then(|w| w.title().ok())
                                .unwrap_or_else(|| label_for_close.clone());
                            if let Some(view) = state.local_watcher_view() {
                                view.bury(&label_for_close);
                            }
                            state.bury_window(&label_for_close, &title);
                            crate::rebuild_window_menu(&app_for_close);
                            if !silent_hide {
                                show_bury_notice(&app_for_close, &title);
                            }
                            return;
                        }
                        let bury = if label_for_close.starts_with("terminal-") {
                            state
                                .embedded
                                .get()
                                .map(|e| e.terminal_window_has_live_shells(&label_for_close))
                                .unwrap_or(false)
                        } else if let Some(id) = label_for_close.strip_prefix("control-terminal-") {
                            // A control terminal closed (red button) WHILE STILL
                            // CONNECTING must NOT bury: a hidden control window
                            // leaves the connect script running and strands the
                            // launcher on "Connecting..." (the connect flow's
                            // scrape loop keeps polling a window it can still
                            // see). Destroy instead so the scrape loop sees it
                            // gone, aborts, and surveys (abandon/edit/retry).
                            // Once connected, burying is fine — the PTY is the
                            // live connection endpoint and stays warm, hidden,
                            // reopenable; only an actual close (^W / script exit)
                            // takes the connection down via request_close_window.
                            state.devservers.is_connected(id)
                        } else {
                            !window_on_connecting_screen(&app_for_close, &label_for_close)
                        };
                        if !bury {
                            // Real close; the Destroyed branch cleans up.
                            return;
                        }
                        api.prevent_close();
                        // Capture the restore snapshot NOW (webview alive,
                        // URL hash + zoom readable): burying replaces
                        // closing as the moment "the user put this window
                        // away", and the entry also covers an app quit
                        // while buried. The zoom stays in
                        // `live_window_zooms` (peek, not drain) — the
                        // window is still alive and may be unburied.
                        if !label_for_close.starts_with("terminal-") {
                            capture_window_config(
                                &app_for_close,
                                &label_for_close,
                                &key_for_close,
                                false,
                            );
                        }
                        let Some(window) = app_for_close.get_webview_window(&label_for_close)
                        else {
                            return;
                        };
                        let title = window.title().unwrap_or_else(|_| label_for_close.clone());
                        let _ = window.hide();
                        state.bury_window(&label_for_close, &title);
                        crate::rebuild_window_menu(&app_for_close);
                        if !silent_hide {
                            show_bury_notice(&app_for_close, &title);
                        }
                    }
                    // Single cleanup point for EVERY destroy path: the
                    // no-live-shells close above, the SPA cascade destroy,
                    // workspace-off / outbound-forget
                    // teardown, and app exit. Frees the display number,
                    // drops the zoom entry, and clears a stale buried
                    // registry entry if the window died while hidden.
                    WindowEvent::Destroyed => {
                        let state = app_for_close.state::<Arc<AppState>>();
                        state.release_window_number(&label_for_close);
                        // Drop the registered OS title so `cs window list`
                        // stops showing one for a window that's gone. The
                        // `cs window title` override is intentionally KEPT:
                        // a best-effort reopen reuses the same label and
                        // should restore the custom title.
                        if let Some(embedded) = state.embedded.get() {
                            embedded.window_titles().remove(&label_for_close);
                        }
                        state
                            .live_window_zooms
                            .lock()
                            .unwrap()
                            .remove(&label_for_close);
                        // A watcher-buried local window destroyed here was buried
                        // by the reconcile (the user red-dot-closed it); KEEP it
                        // in the reopen menu. Only drop it from the buried list on
                        // a real teardown/discard (not in the watcher bury set).
                        let watcher_buried = state
                            .local_watcher_view()
                            .map(|v| v.is_buried(&label_for_close))
                            .unwrap_or(false);
                        if !watcher_buried && state.remove_buried(&label_for_close) {
                            crate::rebuild_window_menu(&app_for_close);
                        }
                        // A destroyed remote-backed window may now be a
                        // reopenable `saved && !connected` row on the
                        // remote — re-poll so the menu offers it.
                        if label_for_close.starts_with("outbound-") {
                            crate::refresh_remote_windows_menu(&app_for_close);
                        }
                    }
                    _ => {}
                });
            }
            Err(e) => {
                // Build failed: hand the just-assigned number back so it
                // isn't leaked out of the live set.
                app_owned
                    .state::<Arc<AppState>>()
                    .release_window_number(&label_owned);
                tracing::warn!(label = %label_owned, error = %e, "opening workspace window failed")
            }
        }
    });
    res.map_err(|e| format!("scheduling workspace window for {window_label}: {e}"))
}

/// Informational notice shown when the OS close (red) button buries a
/// window: the dialog is the teaching surface for the hide-not-close
/// behaviour (smoke tests assert it appears). The launcher status-dot hide
/// suppresses it (its `silent_hide` flag) — that dot is its own explicit hide
/// gesture and needs no teaching. Async `.show` only — a blocking dialog on
/// the event-loop thread deadlocks.
fn show_bury_notice(app: &AppHandle, title: &str) {
    use tauri_plugin_dialog::{DialogExt, MessageDialogKind};
    app.dialog()
        .message(format!(
            "\"{title}\" was hidden, not closed. Reopen it from the Window menu."
        ))
        .title("Window Hidden")
        .kind(MessageDialogKind::Info)
        .show(|_| {});
}

/// The active-transfer close guard's prompt (mirror of the live-shells confirm).
/// The caller has ALREADY `prevent_close`d, so:
/// - "Keep open" (the safe default / Escape) leaves the window untouched and
///   VISIBLE so the user watches the transfer's bubble finish — a hold, NOT a
///   bury.
/// - "Cancel transfer & close" buries the watcher view (so the reconcile won't
///   reopen it) + keeps it in the Window menu, then DESTROYS the webview now.
///   That teardown aborts the in-flight XHR (server upload cleanup is already
///   safe — no orphan/partial); the workspace's terminal PTYs survive
///   server-side, so a later reopen reconnects them with no transfer.
///
/// Async `.show` only — a blocking dialog on the event-loop thread deadlocks
/// (see `show_bury_notice`); the callback runs on the main thread, where the
/// view/menu/destroy mutations are safe.
fn prompt_transfer_close(app: &AppHandle, state: &Arc<AppState>, label: &str) {
    use tauri_plugin_dialog::{DialogExt, MessageDialogButtons};
    let title = app
        .get_webview_window(label)
        .and_then(|w| w.title().ok())
        .unwrap_or_else(|| label.to_string());
    let app_cb = app.clone();
    let state_cb = Arc::clone(state);
    let label_cb = label.to_string();
    app.dialog()
        .message(format!(
            "\"{title}\" has a file transfer in progress. Cancel it and close the \
             window, or keep the window open until the transfer finishes?"
        ))
        .title("Transfer in progress")
        .buttons(MessageDialogButtons::OkCancelCustom(
            "Cancel transfer & close".into(),
            "Keep open".into(),
        ))
        .show(move |cancel_and_close| {
            if !cancel_and_close {
                // Keep open: `prevent_close` already kept it open + visible.
                return;
            }
            // Cancel: bury the view so the watcher reconcile won't reopen it,
            // keep it in the reopen menu, then destroy the webview now to abort
            // the in-flight XHR (the PTYs survive server-side for a later reopen).
            if let Some(view) = state_cb.local_watcher_view() {
                view.bury(&label_cb);
            }
            state_cb.bury_window(&label_cb, &title);
            crate::rebuild_window_menu(&app_cb);
            if let Some(w) = app_cb.get_webview_window(&label_cb) {
                let _ = w.destroy();
            }
        });
}

/// Active-transfer guard prompt for a CONNECTED-DEVSERVER window. The transfer
/// lives in the remote SPA the webview hosts (and on the remote server), so the
/// desktop can't cancel it cleanly — and DESTROYING the webview would just make
/// the devserver watcher reopen the window on its next feed push. So the choice
/// is hold vs hide, never "cancel": "Keep open" (default/Escape) stays visible to
/// watch it; "Hide" buries the window the normal devserver way (the webview stays
/// ALIVE and hidden, so the transfer keeps running, reopenable from the Window
/// menu). To actually cancel, the user uses the SPA's transfer bar. Async `.show`
/// only (a blocking dialog on the event-loop thread deadlocks); the callback runs
/// on the main thread, where the hide/menu mutations are safe.
fn prompt_devserver_transfer_close(app: &AppHandle, state: &Arc<AppState>, label: &str) {
    use tauri_plugin_dialog::{DialogExt, MessageDialogButtons};
    let title = app
        .get_webview_window(label)
        .and_then(|w| w.title().ok())
        .unwrap_or_else(|| label.to_string());
    let app_cb = app.clone();
    let state_cb = Arc::clone(state);
    let label_cb = label.to_string();
    app.dialog()
        .message(format!(
            "\"{title}\" has a file transfer in progress. Keep the window open to \
             watch it finish, or hide it — the transfer keeps running in the \
             background (cancel it from the transfer bar if you need to)."
        ))
        .title("Transfer in progress")
        .buttons(MessageDialogButtons::OkCancelCustom(
            "Hide window".into(),
            "Keep open".into(),
        ))
        .show(move |hide| {
            if !hide {
                // Keep open: `prevent_close` already kept it open + visible.
                return;
            }
            // Hide the normal devserver way: the webview stays alive (so the
            // transfer continues), buried into the Window menu for reopen. Mirrors
            // the `else`-branch bury in the close handler, minus the second bury
            // notice (this prompt already explained the hide).
            if let Some(w) = app_cb.get_webview_window(&label_cb) {
                let t = w.title().unwrap_or_else(|_| label_cb.clone());
                let _ = w.hide();
                state_cb.bury_window(&label_cb, &t);
                crate::rebuild_window_menu(&app_cb);
            }
        });
}

/// Snapshot the window's URL hash and push the resulting WindowConfig
/// onto the LRU stack. Called at BURY time (the hide-not-close moment;
/// webview alive, URL readable) and from any explicit capture path.
/// Best-effort: a webview that's already torn down reports no URL and
/// we skip the push. The hash is read from `WebviewWindow::url()`
/// because the webview SPA writes the latest state to `location.hash`
/// via `persistStateToHash`, and Tauri's URL reflection picks that up
/// on platforms with the WKWebView / WebView2 backends.
///
/// Also captures the live zoom level for this window
/// into `WindowConfig.zoom_level` so the next open of the same
/// workspace restores the zoom. `drain_zoom` controls whether the
/// `live_window_zooms` entry is removed (a window on its way out) or
/// peeked (a buried window stays alive and keeps zooming rights; its
/// entry is dropped by the `Destroyed` cleanup instead).
fn capture_window_config(app: &AppHandle, window_label: &str, config_key: &str, drain_zoom: bool) {
    let Some(window) = app.get_webview_window(window_label) else {
        return;
    };
    // Reading the URL hash is best-effort and must never crash the app on a
    // window close. Two nil-URL failure modes trip a panic deep in the runtime
    // (a nil/empty webview URL fails tauri-runtime-wry's `.parse().expect()` /
    // wry's `.URL().unwrap()`); that panic runs on the event-loop thread and
    // takes the WHOLE app down. The chan-side `match` below cannot catch it
    // because the panic is upstream of the returned `Result`.
    //   - Outbound windows point at a remote we do not own; when that remote is
    //     down the WKWebView never finishes navigating and reports a nil URL.
    //     The hash is chan-SPA restore state, meaningless for an outbound
    //     remote, so skip the read entirely (no url() call, no panic).
    //   - A local window whose backend died before close can hit the
    //     same nil-URL panic, so guard that read with catch_unwind (the release
    //     profile unwinds, so this is catchable) and degrade to an empty hash.
    let url_hash = if window_label.starts_with("outbound-") {
        String::new()
    } else {
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| window.url())) {
            Ok(Ok(u)) => u.fragment().unwrap_or("").to_string(),
            Ok(Err(e)) => {
                tracing::debug!(
                    label = %window_label,
                    error = %e,
                    "could not read url for closing window; pushing empty hash",
                );
                String::new()
            }
            Err(_) => {
                tracing::warn!(
                    label = %window_label,
                    "reading url for a closing window panicked (dead webview); pushing empty hash",
                );
                String::new()
            }
        }
    };
    let state = app.state::<Arc<AppState>>();
    let zoom_level = {
        let mut zooms = state.live_window_zooms.lock().unwrap();
        if drain_zoom {
            zooms.remove(window_label)
        } else {
            zooms.get(window_label).copied()
        }
        .unwrap_or(1.0)
    };
    state.push_window_config(WindowConfig {
        key: config_key.to_string(),
        window_label: window_label.to_string(),
        url_hash,
        zoom_level,
        saved_at: 0,
    });
}

/// Cap the VISIBLE windows of one workspace. Buried (hidden, not
/// closed) windows stay live as webviews but are excluded here: the
/// cap exists to stop runaway window creation, and counting windows
/// the user can't see surfaces a "close one before opening another"
/// error that points at nothing on screen. Unbury can therefore
/// raise the visible count past the cap — it shows an existing
/// webview rather than creating one, so it stays uncapped.
fn ensure_window_capacity(app: &AppHandle, prefix: &str) -> Result<(), String> {
    let state = app.state::<Arc<AppState>>();
    let buried = state.buried_windows.lock().unwrap();
    let count = app
        .webview_windows()
        .keys()
        .filter(|label| label.starts_with(prefix))
        .filter(|label| !buried.iter().any(|b| b.label == label.as_str()))
        .count();
    if count >= MAX_WINDOWS_PER_WORKSPACE {
        return Err(format!(
            "Workspace already has {MAX_WINDOWS_PER_WORKSPACE} open windows; close one before opening another."
        ));
    }
    Ok(())
}

/// Destroy every webview window opened for this outbound URL
/// attachment. Used when the user forgets the attachment row.
pub fn close_remote_workspace_windows(app: &AppHandle, id: &str) {
    close_windows_with_prefix(app, &outbound_window_prefix(id))
}

/// Destroy a window by its exact label, if it exists. Best-effort; used to
/// tear down a devserver's control terminal on disconnect. Window operations
/// run on the main thread.
pub fn close_window_by_label(app: &AppHandle, label: &str) {
    let app_owned = app.clone();
    let label = label.to_string();
    let _ = app.run_on_main_thread(move || {
        if let Some(w) = app_owned.get_webview_window(&label) {
            let _ = w.destroy();
        }
    });
}

pub(crate) fn close_windows_with_prefix(app: &AppHandle, prefix: &str) {
    let app_owned = app.clone();
    let prefix_owned = prefix.to_string();
    let _ = app.run_on_main_thread(move || {
        // Snapshot first; destroying inside the iterator would
        // mutate the underlying map mid-walk.
        let labels: Vec<String> = app_owned
            .webview_windows()
            .keys()
            .filter(|l| l.starts_with(&prefix_owned))
            .cloned()
            .collect();
        for l in labels {
            if let Some(w) = app_owned.get_webview_window(&l) {
                let _ = w.destroy();
            }
        }
    });
}

/// Native keyboard shortcuts for workspace webviews. Translates chords
/// into the host-agnostic `chan:command` window event that chan's
/// App.svelte listens for. Runs before any page script, in capture
/// phase with stopImmediatePropagation, so this script is the sole
/// authority on every chord it claims, so chan's onWindowKey doesn't
/// fire for these even if its keymap drifts.
///
/// Layout mirrors VS Code; chords that browsers reserve at OS level
/// (Cmd+W, Cmd+N, Cmd+Shift+[/], Cmd+1..9) are bound here because
/// the native webview doesn't have those reservations. chan's web
/// fallbacks (Alt+Shift, Ctrl+Alt) keep working independently.
const KEY_BRIDGE_JS: &str = r#"
(() => {
  function fire(e, name, detail) {
    e.preventDefault();
    e.stopImmediatePropagation();
    window.dispatchEvent(new CustomEvent('chan:command',
      { detail: Object.assign({ name: name }, detail || {}) }));
  }
  // Cmd+R reloads the webview, Cmd+Opt+I opens
  // DevTools. Both bypass the SPA event bus and invoke their
  // Tauri IPC commands directly so a frozen Svelte runtime or a
  // broken chord registry can't lock the dev affordances away.
  // GUARD the bridge BEFORE swallowing the event: when window.__TAURI__ is
  // absent (e.g. a devserver window where the bridge did not survive the
  // connecting -> external navigation), do NOT preventDefault — let the event
  // bubble to the SPA's own handler (Cmd+R -> location.reload()) so the chord
  // degrades to a working fallback instead of dying. Swallowing first then
  // finding no bridge killed Cmd+R/devtools/zoom outright (no IPC, no fallback).
  function invokeIpc(e, cmd) {
    const tauri = window.__TAURI__;
    if (!(tauri && tauri.core && typeof tauri.core.invoke === 'function')) {
      return;
    }
    e.preventDefault();
    e.stopImmediatePropagation();
    tauri.core.invoke(cmd).catch((err) => {
      console.error('[chan] IPC ' + cmd + ' failed:', err);
    });
  }
  // Chord policy: actions reachable through Pane Mode (Cmd+K) stay
  // unbound here (Cmd+`, Cmd+Shift+F) so the native layer claims as
  // little as possible. Direct chords exist where Pane Mode is no
  // substitute: Cmd+W (close tab; pairs with the SPA's context-aware
  // Ctrl+D), Cmd+F/G (find on page), Cmd+1..9 (jump to tab),
  // Cmd+[/Cmd+] (pane nav), Cmd+S (search), Cmd+/ and Cmd+Shift+/
  // (split right / down), Cmd+Shift+T (reopen closed), Cmd+Shift+[/]
  // (tab nav), Cmd+Shift+G (find prev), plus the context-aware spawn
  // family Cmd+T (terminal) / Cmd+O (File Browser) / Cmd+P (Team
  // Work) / Cmd+Shift+M (Graph), whose `app.files.toggle` /
  // `app.terminal.teamWork` / `app.graph.toggle` commands route
  // through the context-aware helpers in App.svelte. Universal
  // Hybrid NAV `t/o/p/v` covers the web/Win/Linux fallback path.
  function onKey(e) {
    const meta = e.metaKey || e.ctrlKey;
    if (!meta) return;
    const shift = e.shiftKey;
    const alt = e.altKey;
    const code = e.code;
    if (alt) {
      // Cmd+Opt+I (macOS) / Ctrl+Alt+I (Linux/Windows) → DevTools.
      // No other meta+alt chord today; bail out for everything else
      // so we don't shadow the webview's defaults.
      if (!shift && code === 'KeyI') {
        invokeIpc(e, 'open_devtools');
      }
      return;
    }
    // Zoom chords route regardless of shift so
    // Cmd+= (US) and Cmd+Shift+= (= Cmd++) both fire zoom_in.
    // NumpadAdd / NumpadSubtract similarly. Cmd+0 / Cmd+Numpad0
    // reset to 100 %.
    switch (code) {
      case 'Equal':
      case 'NumpadAdd':
        invokeIpc(e, 'zoom_in');
        return;
      case 'Minus':
      case 'NumpadSubtract':
        invokeIpc(e, 'zoom_out');
        return;
      case 'Digit0':
      case 'Numpad0':
        invokeIpc(e, 'zoom_reset');
        return;
    }
    if (!shift) {
      switch (code) {
        // Reload. macOS binds Cmd+R (metaKey); Linux/Windows moves to
        // Ctrl+Shift+R (shift branch below) so plain Ctrl+R reaches a
        // focused terminal's shell reverse-search. Gating on metaKey
        // here leaves Linux/macOS plain Ctrl+R untouched (no
        // preventDefault -> falls through to xterm), mirroring the
        // Cmd+W idiom below.
        case 'KeyR': if (e.metaKey) invokeIpc(e, 'reload_window'); return;
        case 'KeyT': fire(e, 'app.terminal.toggle'); return;
        case 'KeyO': fire(e, 'app.files.toggle');    return;
        case 'KeyP': fire(e, 'app.terminal.teamWork'); return;
        // Cmd+W closes the tab
        // on macOS. On Linux the platform mod is Ctrl and Ctrl+W is
        // readline delete-word inside a focused terminal, so DON'T
        // claim it - let it reach xterm. Linux closes tabs with Ctrl+D
        // (context-aware via the SPA's onCtrlDCapture, which leaves a
        // focused terminal to its EOF). Gating on metaKey (Cmd) leaves
        // Linux Ctrl+W untouched (no preventDefault -> reaches xterm).
        case 'KeyW':
          if (e.metaKey) {
            // On the connecting/retry page there are no tabs and the
            // app.tab.close dispatch is dead: Cmd+W means cancel, so
            // close the window for real (request_close_window
            // destroys, bypassing bury-on-close). The bridge claims
            // KeyW with stopImmediatePropagation BEFORE the page's own
            // listener AND before the File menu accelerator gets a
            // look-in, so the routing must happen here. Gate on the
            // CURRENT document (this init script re-runs after the
            // success navigation, where Cmd+W must stay tab-close).
            if (location.pathname.endsWith('/connecting.html')) {
              invokeIpc(e, 'request_close_window');
              return;
            }
            fire(e, 'app.tab.close');
          }
          return;
        case 'KeyS': fire(e, 'app.search.toggle');    return;
        case 'KeyF': fire(e, 'app.find.open');        return;
        case 'KeyG': fire(e, 'app.find.next');        return;
        // Cmd+I does NOT open Dashboard; it is reserved for the
        // editor's italic chord (bound in
        // Wysiwyg.svelte's CM6 keymap). Dashboard is reachable via
        // Hybrid Nav `Cmd+. i` + the Dashboard hamburger. With no
        // `KeyI` case here, Cmd+I falls through to the focused webview
        // (the editor toggles italic; otherwise inert). Cmd+Opt+I
        // (DevTools, alt branch above) and Cmd+Shift+I (broadcast,
        // shift branch below) are unaffected.
        case 'BracketLeft':  fire(e, 'app.pane.prev'); return;
        case 'BracketRight': fire(e, 'app.pane.next'); return;
        // Cmd+/ split right. Split
        // bottom is Cmd+Shift+/ (shift branch below). Cmd+\ is
        // deliberately NOT used: 1Password's system-wide Cmd+\
        // hotkey is dispatched by macOS before the key reaches this
        // webview, so chan never receives it. Web reaches splits via
        // Hybrid Nav `/` and `?`.
        case 'Slash':        fire(e, 'app.pane.splitRight'); return;
      }
      const m = code.match(/^Digit([1-9])$/);
      if (m) {
        fire(e, 'app.tab.jump', { index: Number(m[1]) - 1 });
        return;
      }
    } else {
      switch (code) {
        // Reload on Linux/Windows: Ctrl+Shift+R. Gate on !metaKey so
        // macOS Cmd+Shift+R does NOT reload (macOS reloads on Cmd+R in
        // the !shift branch above); the !metaKey form fires only for the
        // Ctrl+Shift+R that Linux/Windows users press.
        case 'KeyR': if (!e.metaKey) invokeIpc(e, 'reload_window'); return;
        // Close on Linux/Windows: Ctrl+Shift+W (plain Ctrl+W stays
        // readline delete-word inside a focused terminal, which is why
        // the !shift branch never claims it off macOS). Gate on
        // !metaKey so macOS keeps plain Cmd+W and Cmd+Shift+W stays
        // unclaimed there. Same routing as Cmd+W: cancel-close on the
        // connecting screen, tab-close everywhere else.
        case 'KeyW':
          if (!e.metaKey) {
            if (location.pathname.endsWith('/connecting.html')) {
              invokeIpc(e, 'request_close_window');
              return;
            }
            fire(e, 'app.tab.close');
          }
          return;
        case 'KeyG':         fire(e, 'app.find.prev');     return;
        case 'KeyT':         fire(e, 'app.tab.reopenClosed'); return;
        case 'KeyM':         fire(e, 'app.graph.toggle');  return;
        // Cmd+Shift+I (mac) / Ctrl+Shift+I (Linux,
        // Windows) toggles broadcast-input select-all/deselect-all for the
        // active terminal (mirrors iTerm). Ungated within the shift branch so
        // both platform mods fire; DevTools lives on the ALT chord
        // (Cmd+Opt+I / Ctrl+Alt+I, the `alt` branch above), and `fire()`
        // preventDefaults so the webview's built-in Ctrl+Shift+I DevTools
        // chord is suppressed. Web has no binding (cmd+shift+i is the browser
        // DevTools there).
        case 'KeyI': fire(e, 'app.terminal.broadcastToggle'); return;
        case 'BracketLeft':  fire(e, 'app.tab.prev');      return;
        case 'BracketRight': fire(e, 'app.tab.next');      return;
        // Cmd+Shift+/ (= Cmd+?) splits the active pane
        // bottom, pairing with Cmd+/ split-right above. Cmd+\ is
        // avoided - 1Password's global hotkey shadows it.
        case 'Slash':        fire(e, 'app.pane.splitDown');  return;
      }
    }
  }
  window.addEventListener('keydown', onKey, true);
})();
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_label_matches_a_bare_window_id_to_its_composite_native_label() {
        // The launcher's status-dot open/hide sends the BARE library-minted
        // `window_id`; the desktop must resolve it to the composite native label
        // the watcher actually opened (`{library_id}::{window_id}`).
        let open = vec!["local::w-1".to_string(), "lib-abc::w-2".to_string()];
        assert_eq!(resolve_label_from("w-1", &open), "local::w-1");
        assert_eq!(resolve_label_from("w-2", &open), "lib-abc::w-2");
    }

    #[test]
    fn resolve_label_falls_back_to_local_for_a_buried_window_with_no_native_surface() {
        // A buried LOCAL watched window was DESTROYED by the reconcile, so it is
        // not in the open set — resolve it to its `local::` composite so the
        // view-driven un-bury still reaches it.
        let open = vec!["lib-abc::w-9".to_string()];
        assert_eq!(resolve_label_from("w-1", &open), "local::w-1");
        assert_eq!(resolve_label_from("w-1", &[]), "local::w-1");
    }

    #[test]
    fn resolve_label_passes_a_full_composite_or_legacy_label_through_verbatim() {
        // `cs window open/hide` passes the full label already; a composite is used
        // verbatim even when its native window was destroyed (not in the open set),
        // so `cs window open <composite>` reaches the view-driven reopen too.
        let open = vec!["local::w-5".to_string()];
        assert_eq!(resolve_label_from("local::w-1", &open), "local::w-1");
        assert_eq!(resolve_label_from("lib-z::w-3", &[]), "lib-z::w-3");
    }

    #[test]
    fn refresh_library_transfers_replaces_only_that_librarys_slice() {
        use std::collections::HashSet;
        let mut set: HashSet<String> = ["lib-a::w1", "lib-a::w2", "lib-b::w9"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        // A fresh lib-a push: only w3 is transferring now — its old slice drops,
        // the other library is untouched.
        crate::refresh_library_transfers(&mut set, "lib-a", &["lib-a::w3".to_string()]);
        assert!(set.contains("lib-a::w3"));
        assert!(!set.contains("lib-a::w1"));
        assert!(!set.contains("lib-a::w2"));
        assert!(set.contains("lib-b::w9"));
        // A lib-a push with nothing transferring clears its slice only.
        crate::refresh_library_transfers(&mut set, "lib-a", &[]);
        assert!(!set.iter().any(|l| l.starts_with("lib-a::")));
        assert!(set.contains("lib-b::w9"));
    }

    #[test]
    fn invoke_handler_registers_reload_window_and_open_devtools() {
        // The IPC commands `reload_window` and
        // `open_devtools` MUST be in the `tauri::generate_handler!`
        // list so the SPA's tab context-menu and the
        // accelerator path can reach them. The generate_handler!
        // macro does not catch a missing handler at compile time,
        // so we pin it here against the source file. Tests live in
        // serve.rs because main.rs has no test module today; using
        // `include_str!` keeps the pin source-of-truth-correct.
        const MAIN_RS: &str = include_str!("main.rs");
        assert!(MAIN_RS.contains("reload_window,"));
        assert!(MAIN_RS.contains("open_devtools,"));
        assert!(MAIN_RS.contains("fn reload_window(window: tauri::WebviewWindow)"));
        assert!(MAIN_RS.contains("fn open_devtools(window: tauri::WebviewWindow)"));
    }

    #[test]
    fn key_bridge_wires_zoom_chords_to_ipc() {
        // Cmd+= / Cmd+- / Cmd+0 (and their
        // Numpad variants) route directly to the chan-desktop
        // zoom IPC commands. Routed BEFORE the shift branch so
        // Cmd+Shift+= (= Cmd++) also zooms in. Capture-phase
        // listener stops the keydown so Tauri's `zoom_hotkeys_enabled`
        // polyfill (still on as a mousewheel + pinch fallback)
        // doesn't double-fire.
        assert!(KEY_BRIDGE_JS.contains("invokeIpc(e, 'zoom_in')"));
        assert!(KEY_BRIDGE_JS.contains("invokeIpc(e, 'zoom_out')"));
        assert!(KEY_BRIDGE_JS.contains("invokeIpc(e, 'zoom_reset')"));
        assert!(KEY_BRIDGE_JS.contains("case 'Equal':"));
        assert!(KEY_BRIDGE_JS.contains("case 'Minus':"));
        assert!(KEY_BRIDGE_JS.contains("case 'Digit0':"));
        assert!(KEY_BRIDGE_JS.contains("case 'NumpadAdd':"));
        assert!(KEY_BRIDGE_JS.contains("case 'NumpadSubtract':"));
        assert!(KEY_BRIDGE_JS.contains("case 'Numpad0':"));
    }

    #[test]
    fn key_bridge_wires_shift_i_to_broadcast_toggle_on_both_mods() {
        // Cmd+Shift+I (mac) / Ctrl+Shift+I (Linux, Windows) toggles
        // broadcast-input select-all/deselect-all for the active terminal.
        // Ungated within the shift branch (no `metaKey` gate) so both
        // platform mods fire; DevTools stays on the ALT chord.
        assert!(KEY_BRIDGE_JS.contains("case 'KeyI': fire(e, 'app.terminal.broadcastToggle');"));
        assert!(!KEY_BRIDGE_JS.contains("if (e.metaKey) fire(e, 'app.terminal.broadcastToggle')"));
    }

    #[test]
    fn invoke_handler_registers_zoom_commands() {
        // zoom_in / zoom_out / zoom_reset must be
        // in `tauri::generate_handler!` so KEY_BRIDGE_JS's IPC
        // invocations reach a registered command. generate_handler!
        // doesn't catch missing entries at compile time; pin here.
        const MAIN_RS: &str = include_str!("main.rs");
        assert!(MAIN_RS.contains("zoom_in,"));
        assert!(MAIN_RS.contains("zoom_out,"));
        assert!(MAIN_RS.contains("zoom_reset,"));
    }

    #[test]
    fn invoke_handler_registers_outbound_attach_ipcs() {
        const MAIN_RS: &str = include_str!("main.rs");
        assert!(MAIN_RS.contains("add_outbound_workspace,"));
        assert!(MAIN_RS.contains("open_outbound_workspace,"));
        assert!(MAIN_RS.contains("remove_outbound_workspace,"));
        assert!(MAIN_RS.contains("fn add_outbound_workspace("));
        assert!(MAIN_RS.contains("fn open_outbound_workspace("));
        assert!(MAIN_RS.contains("fn remove_outbound_workspace("));
    }

    #[test]
    fn outbound_windows_load_the_connecting_page_not_the_remote() {
        // Blank-white outbound bug: a direct WebviewUrl::External(remote)
        // paints white when the remote is down. Outbound windows load the
        // bundled connecting page instead, which probes via `probe_url` and
        // navigates on success. Needles are built at runtime so this test's
        // own source text doesn't satisfy the `contains` checks (the
        // bin_status test uses the same trick).
        let serve_rs = include_str!("serve.rs");
        let app_load = format!("WebviewUrl::App({q}connecting.html", q = '"');
        let handoff = format!("__CHAN{u}CONNECTING__", u = '_');
        assert!(
            serve_rs.contains(&app_load),
            "outbound windows must load connecting.html, not the remote directly",
        );
        assert!(
            serve_rs.contains(&handoff),
            "the connecting page must receive its inputs via window.__CHAN_CONNECTING__",
        );
    }

    #[test]
    fn invoke_handler_registers_probe_url() {
        // The connecting screen's retry loop calls `probe_url` each attempt;
        // it must be in `tauri::generate_handler!` or the IPC denies and the
        // screen never detects a reachable remote. generate_handler! doesn't
        // catch a missing entry at compile time, so pin it here.
        const MAIN_RS: &str = include_str!("main.rs");
        assert!(MAIN_RS.contains("probe_url,"));
        assert!(MAIN_RS.contains("fn probe_url("));
    }

    #[test]
    fn registry_commands_run_in_process_not_via_chan_cli() {
        // chan-desktop runs without a `chan` binary: `add_workspace`
        // and `remove_workspace` route through the embedded host's
        // shared `Library` rather than spawning chan. (Optional-layer
        // enablement is not a desktop concern at all: the SPA's
        // onboarding card drives it post-boot through chan-server.)
        // Pin the in-process call shape so a future change can't
        // silently reintroduce a subprocess dependency, and assert
        // the deleted subprocess argument shapes are gone.
        const MAIN_RS: &str = include_str!("main.rs");
        assert!(
            MAIN_RS.contains("embedded.library()"),
            "registry commands must route through the embedded shared Library",
        );
        assert!(
            MAIN_RS.contains("register_workspace") && MAIN_RS.contains("unregister_workspace"),
            "add_workspace/remove_workspace must use Library register/unregister in-process",
        );
        assert!(
            !MAIN_RS.contains("read_features_via_chan_index_status"),
            "the `chan index status --json` read path must be gone",
        );
        assert!(
            !MAIN_RS.contains("\"--semantic-search\"") && !MAIN_RS.contains("\"enable-semantic\""),
            "no chan CLI feature-flag arguments may remain",
        );
    }

    #[test]
    fn bin_status_machinery_is_gone() {
        // chan-desktop has no bundled-binary preflight or gating
        // (no subprocess paths). Pin the absence so a future change can't
        // quietly re-add a `chan` binary dependency or its gating.
        const MAIN_RS: &str = include_str!("main.rs");
        assert!(
            !MAIN_RS.contains("chan_bin_status"),
            "chan_bin_status command + registration must be gone",
        );
        assert!(
            !MAIN_RS.contains("fn require_bin"),
            "require_bin gating helper must be gone",
        );
        assert!(
            !MAIN_RS.contains("struct BinStatus"),
            "BinStatus struct must be gone",
        );
        // serve.rs must no longer carry the binary resolver. Build
        // the needle at runtime so this assertion's own source text
        // doesn't satisfy the `contains` check it performs.
        let serve_rs = include_str!("serve.rs");
        let resolver_sig = format!("fn resolve{}binary", "_chan_");
        assert!(
            !serve_rs.contains(&resolver_sig),
            "binary resolution helpers must be gone from serve.rs",
        );
    }

    #[test]
    fn new_window_accelerator_uses_cmd_shift_n() {
        // The "New Window" menu item binds
        // `CmdOrCtrl+Shift+N`; plain Cmd+N belongs to
        // the SPA's New Draft handler. Pin the
        // chord so a future menu edit can't silently land on
        // plain Cmd+N and clash with the SPA chord.
        const MAIN_RS: &str = include_str!("main.rs");
        assert!(
            MAIN_RS.contains(".accelerator(\"CmdOrCtrl+Shift+N\")"),
            "main.rs must bind the New Window menu item to CmdOrCtrl+Shift+N"
        );
        assert!(
            !MAIN_RS.contains(".accelerator(\"CmdOrCtrl+N\")"),
            "main.rs must NOT bind any menu item to plain CmdOrCtrl+N (reserved for SPA New Draft)"
        );
    }

    #[test]
    fn key_bridge_wires_reload_and_devtools_ipc() {
        // Cmd+R fires the `reload_window` IPC and
        // Cmd+Opt+I fires `open_devtools`, bypassing the SPA event
        // bus so a frozen Svelte runtime can't lock the dev
        // affordances away. The accelerator path goes through
        // `invokeIpc(...)` (not the `chan:command` `fire(...)`
        // bridge), so the contract pin checks both the IPC command
        // names and the case-label they're bound from.
        assert!(KEY_BRIDGE_JS.contains("invokeIpc(e, 'reload_window')"));
        assert!(KEY_BRIDGE_JS.contains("invokeIpc(e, 'open_devtools')"));
        // Reload is per-OS: Cmd+R (metaKey, no-shift branch) on macOS and
        // Ctrl+Shift+R (!metaKey, shift branch) on Linux/Windows, so plain
        // Ctrl+R is never claimed and reaches the terminal's reverse-search.
        assert!(KEY_BRIDGE_JS.contains("case 'KeyR': if (e.metaKey) invokeIpc(e, 'reload_window')"));
        assert!(
            KEY_BRIDGE_JS.contains("case 'KeyR': if (!e.metaKey) invokeIpc(e, 'reload_window')")
        );
        assert!(KEY_BRIDGE_JS.contains("code === 'KeyI'"));
    }

    #[test]
    fn embedded_url_prefix_parser_strips_query_and_trailing_slash() {
        let prefix = url_prefix_from_local_url("http://127.0.0.1:1234/workspace-abcd/?t=token")
            .expect("prefix");
        assert_eq!(prefix, "/workspace-abcd");
    }

    #[test]
    fn embedded_url_prefix_parser_strips_index_html() {
        let prefix =
            url_prefix_from_local_url("http://127.0.0.1:1234/workspace-abcd/index.html?t=token")
                .expect("prefix");
        assert_eq!(prefix, "/workspace-abcd");
    }

    #[test]
    fn key_bridge_invokes_tauri_ipc_via_core_invoke() {
        // The `invokeIpc` helper grabs `window.__TAURI__.core.invoke`
        // (Tauri 2's invoke surface; was `window.__TAURI__.invoke`
        // in Tauri 1). Pin so a future bridge rewrite doesn't
        // silently regress to the v1 shape. The new shape returns
        // undefined from a webview without the v2 IPC surface
        // attached, which silently swallows the Cmd+R / Cmd+Opt+I
        // accelerators.
        assert!(KEY_BRIDGE_JS.contains("window.__TAURI__"));
        assert!(KEY_BRIDGE_JS.contains("tauri.core.invoke"));
    }

    #[test]
    fn key_bridge_drops_chords_covered_by_pane_mode() {
        // Chords with a Pane Mode equivalent stay out of the native
        // bridge. The direct-chord exceptions (Cmd+T terminal, Cmd+O
        // files, Cmd+Shift+M graph, Cmd+P Team Work, Cmd+S search)
        // are asserted in `key_bridge_keeps_independent_chords`; the
        // absences here catch accidental reverts of chords that
        // should go through Pane Mode only.
        assert!(!KEY_BRIDGE_JS.contains("app.file.new"));
        assert!(!KEY_BRIDGE_JS.contains("Backquote"));
    }

    #[test]
    fn key_bridge_keeps_independent_chords() {
        // Tab close + reopen + Find on page + tab nav + tab jump
        // are NOT duplicated by Pane Mode and must stay reachable
        // through the native bridge. Cmd+T / Cmd+O / Cmd+P /
        // Cmd+Shift+M are the context-aware
        // spawn chord family.
        assert!(KEY_BRIDGE_JS.contains("app.terminal.toggle"));
        assert!(KEY_BRIDGE_JS.contains("app.files.toggle"));
        assert!(KEY_BRIDGE_JS.contains("app.terminal.teamWork"));
        assert!(KEY_BRIDGE_JS.contains("app.pane.prev"));
        assert!(KEY_BRIDGE_JS.contains("app.pane.next"));
        assert!(KEY_BRIDGE_JS.contains("app.graph.toggle"));
        assert!(KEY_BRIDGE_JS.contains("app.tab.close"));
        assert!(KEY_BRIDGE_JS.contains("app.tab.reopenClosed"));
        assert!(KEY_BRIDGE_JS.contains("app.find.open"));
        assert!(KEY_BRIDGE_JS.contains("app.tab.jump"));
        assert!(KEY_BRIDGE_JS.contains("app.tab.next"));
        assert!(KEY_BRIDGE_JS.contains("app.tab.prev"));
        // Cmd+S search + Cmd+/ (right)
        // / Cmd+Shift+/ (bottom) splits route through the native bridge
        // too.
        assert!(KEY_BRIDGE_JS.contains("app.search.toggle"));
        assert!(KEY_BRIDGE_JS.contains("app.pane.splitRight"));
        assert!(KEY_BRIDGE_JS.contains("app.pane.splitDown"));
        // Cmd+I is reserved for the editor's
        // italic chord, so the native bridge must not map it to
        // Dashboard. Pin the absence so a regression that re-adds the
        // case is caught (Dashboard is Hybrid-Nav-only).
        assert!(!KEY_BRIDGE_JS.contains("app.dashboard.open"));
    }

    #[test]
    fn local_title_prefixes_home_vs_computer_glyph_then_path() {
        // The local title leads with the kind glyph (home when under the
        // user's home dir, computer otherwise) then the path verbatim
        // (the path is the disambiguating window-switcher
        // signal). `local_title` takes home explicitly so the test does not
        // depend on the process's real home dir.
        let home = Path::new("/Users/alex");
        assert_eq!(
            local_title("/Users/alex/dev/github.com/fiorix/chan", Some(home)),
            format!("{ICON_LOCAL_HOME} /Users/alex/dev/github.com/fiorix/chan"),
        );
        // Outside home -> computer glyph. Trailing slash passed through.
        assert_eq!(
            local_title("/tmp/scratch/", Some(home)),
            format!("{ICON_LOCAL_OTHER} /tmp/scratch/"),
        );
        // No resolvable home dir -> computer glyph (never mislabels as home).
        assert_eq!(
            local_title("/Users/alex/notes", None),
            format!("{ICON_LOCAL_OTHER} /Users/alex/notes"),
        );
    }

    // Workspace and outbound webviews host the SPA, which
    // routes external http(s) link clicks through tauri-plugin-opener
    // via the `plugin:opener|open_url` IPC. Without these permissions
    // the IPC denies, the SPA falls back to the clipboard-copy notify
    // branch, and "click external link" looks like a no-op to the
    // user. Pin the capability
    // shape here so a future capability-file edit can't silently drop
    // the permissions without the test catching it.
    const WORKSPACE_CAPABILITY_JSON: &str = include_str!("../capabilities/workspace.json");
    const DEFAULT_CAPABILITY_JSON: &str = include_str!("../capabilities/default.json");
    const LOCAL_DROP_CAPABILITY_JSON: &str = include_str!("../capabilities/local-drop.json");
    const APP_PERMISSIONS_TOML: &str = include_str!("../permissions/app.toml");

    fn capability_permissions(raw: &str) -> Vec<String> {
        let v: serde_json::Value = serde_json::from_str(raw).expect("capability JSON parses");
        v["permissions"]
            .as_array()
            .expect("permissions is an array")
            .iter()
            .map(|p| p.as_str().expect("permission is a string").to_string())
            .collect()
    }

    fn capability_windows(raw: &str) -> Vec<String> {
        let v: serde_json::Value = serde_json::from_str(raw).expect("capability JSON parses");
        v["windows"]
            .as_array()
            .expect("windows is an array")
            .iter()
            .map(|w| w.as_str().expect("window glob is a string").to_string())
            .collect()
    }

    fn capability_remote_urls(raw: &str) -> Vec<String> {
        let v: serde_json::Value = serde_json::from_str(raw).expect("capability JSON parses");
        v["remote"]["urls"]
            .as_array()
            .expect("remote urls is an array")
            .iter()
            .map(|u| {
                u.as_str()
                    .expect("remote URL pattern is a string")
                    .to_string()
            })
            .collect()
    }

    fn app_permission_set(id: &str) -> Vec<String> {
        let v: toml::Value = toml::from_str(APP_PERMISSIONS_TOML).expect("app permissions parse");
        v["set"]
            .as_array()
            .expect("permission sets is an array")
            .iter()
            .find(|set| set["identifier"].as_str() == Some(id))
            .unwrap_or_else(|| panic!("missing app permission set {id}"))["permissions"]
            .as_array()
            .expect("permission set entries are an array")
            .iter()
            .map(|p| p.as_str().expect("permission id is a string").to_string())
            .collect()
    }

    /// Every command a permission set grants, resolved through the
    /// `[[permission]]` blocks it references. Panics if the set names a
    /// permission identifier that has no block (also a parity failure).
    fn app_permission_set_commands(set_id: &str) -> Vec<String> {
        let v: toml::Value = toml::from_str(APP_PERMISSIONS_TOML).expect("app permissions parse");
        let blocks = v["permission"].as_array().expect("permission blocks");
        app_permission_set(set_id)
            .iter()
            .flat_map(|id| {
                let block = blocks
                    .iter()
                    .find(|p| p["identifier"].as_str() == Some(id))
                    .unwrap_or_else(|| panic!("set references missing permission {id}"));
                block["commands"]["allow"]
                    .as_array()
                    .expect("commands.allow is an array")
                    .iter()
                    .map(|c| c.as_str().expect("command is a string").to_string())
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    /// Command identifiers registered in `generate_handler![]`, module paths
    /// stripped (`auth::auth_status` -> `auth_status`), comments and cfg
    /// attributes dropped.
    fn invoke_handler_commands(main_rs: &str) -> Vec<String> {
        let marker = "generate_handler![";
        let start = main_rs.find(marker).expect("generate_handler! present") + marker.len();
        // The macro closes with `])`; a bare `]` would match a comment's `[]`
        // (e.g. "returns [] off macOS") or a cfg attribute's `)]` first.
        let len = main_rs[start..]
            .find("])")
            .expect("generate_handler! closes");
        main_rs[start..start + len]
            .lines()
            .filter_map(|l| l.split("//").next())
            .collect::<Vec<_>>()
            .join("\n")
            .split(',')
            .map(|t| t.rsplit("::").next().unwrap_or(t).trim().to_string())
            .filter(|t| !t.is_empty() && t.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'))
            .collect()
    }

    /// Every command any `[[permission]]` block grants (regardless of which
    /// set references it).
    fn all_granted_app_commands() -> Vec<String> {
        let v: toml::Value = toml::from_str(APP_PERMISSIONS_TOML).expect("app permissions parse");
        v["permission"]
            .as_array()
            .expect("permission blocks")
            .iter()
            .flat_map(|p| {
                p["commands"]["allow"]
                    .as_array()
                    .expect("commands.allow is an array")
                    .iter()
                    .map(|c| c.as_str().expect("command is a string").to_string())
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    #[test]
    fn workspace_capability_grants_opener_to_workspace_and_outbound_windows() {
        let windows = capability_windows(WORKSPACE_CAPABILITY_JSON);
        assert!(
            windows.iter().any(|w| w == "workspace-*"),
            "workspace capability must target workspace-* windows: {windows:?}",
        );
        assert!(
            windows.iter().any(|w| w == "outbound-*"),
            "workspace capability must target outbound-* windows: {windows:?}",
        );
        let perms = capability_permissions(WORKSPACE_CAPABILITY_JSON);
        assert!(
            perms.iter().any(|p| p == "workspace-window"),
            "workspace capability must include workspace-window app commands: {perms:?}",
        );
        assert!(
            perms.iter().any(|p| p == "opener:allow-open-url"),
            "workspace capability must include opener:allow-open-url: {perms:?}",
        );
    }

    #[test]
    fn workspace_capability_covers_control_terminal_windows() {
        // A control terminal's window label is `control-terminal-<id>`
        // (`control_terminal_label`), which matches NONE of workspace-* /
        // outbound-* / terminal-* — so without this glob the control window has
        // no capability and Tauri denies every IPC from it, including the
        // request_close_window that rules (b)/(c) of the control-terminal dialog
        // (Cmd+W / the not-connected close button) route through. Pin the grant
        // so a glob rename can't silently strand the control window again.
        let windows = capability_windows(WORKSPACE_CAPABILITY_JSON);
        assert!(
            windows.iter().any(|w| w == "control-terminal-*"),
            "workspace capability must target control-terminal-* windows so the connect-script \
             window can request_close_window: {windows:?}",
        );
        let label = control_terminal_label("abc123");
        assert!(
            label.starts_with("control-terminal-"),
            "control terminal label must keep the control-terminal- prefix the glob matches: {label}",
        );
    }

    #[test]
    fn workspace_capability_covers_watcher_opened_local_windows() {
        // Watcher-opened local windows carry the composite native label
        // `local::<window_id>` (`window_watcher::native_label`), which matches
        // NONE of the workspace-* / outbound-* / terminal-* globs — so without
        // `local::*` a minted window gets no capability and Tauri denies every
        // SPA IPC (the command bridge, opener, drag). Pin the grant so a glob
        // change can't silently strand minted windows.
        let windows = capability_windows(WORKSPACE_CAPABILITY_JSON);
        assert!(
            windows.iter().any(|w| w == "local::*"),
            "workspace capability must target local::* watcher-opened windows: {windows:?}",
        );
    }

    #[test]
    fn workspace_capability_covers_loopback_server_urls() {
        // Workspace windows load chan-server through loopback HTTP
        // origins. Without a remote URL match, Tauri omits the IPC
        // bridge and workspace-window app commands such as reload_window
        // or the zoom chords never reach Rust.
        let remote_urls = capability_remote_urls(WORKSPACE_CAPABILITY_JSON);
        assert!(
            remote_urls.iter().any(|u| u == "http://127.0.0.1:*"),
            "workspace capability must include 127.0.0.1 loopback: {remote_urls:?}",
        );
        assert!(
            remote_urls.iter().any(|u| u == "http://localhost:*"),
            "workspace capability must include localhost loopback: {remote_urls:?}",
        );
    }

    #[test]
    fn app_acl_allows_workspace_window_commands() {
        let workspace_set = app_permission_set("workspace-window");
        for expected in [
            "allow-reload-window",
            "allow-open-devtools",
            "allow-save-file-to-downloads",
            "allow-zoom-in",
            "allow-zoom-out",
            "allow-zoom-reset",
            // The connecting screen (outbound-* windows) probes the remote
            // through this command; without the ACL grant the IPC denies and
            // the screen never detects a reachable remote.
            "allow-probe-url",
        ] {
            assert!(
                workspace_set.iter().any(|p| p == expected),
                "workspace-window app permission set must include {expected}: {workspace_set:?}",
            );
        }
    }

    // Tauri's ACL denies any `generate_handler!` command that no granted
    // permission allows. The gate and the mock smoke both bypass the ACL (unit
    // tests call the Rust fns directly; a mocked Tauri has no ACL), so a
    // registered-but-ungranted command only fails in the real app. These two
    // tests pin the command/ACL parity so drift reds the gate instead.

    #[test]
    fn app_acl_grants_every_registered_command() {
        // Complete coverage: every command in generate_handler! must be
        // grantable somewhere the SPA can reach it. App-command grants come
        // from the two sets plus the local-drop capability (read_dropped_paths,
        // scoped to locally-served windows). Catches a command the workspace
        // SPA invokes (e.g. platform_os, read_clipboard_text) that no set
        // grants.
        const MAIN_RS: &str = include_str!("main.rs");
        let mut granted: std::collections::HashSet<String> =
            app_permission_set_commands("main-window")
                .into_iter()
                .chain(app_permission_set_commands("workspace-window"))
                .collect();
        granted.insert("read_dropped_paths".to_string());
        for command in invoke_handler_commands(MAIN_RS) {
            assert!(
                granted.contains(&command),
                "`{command}` is in generate_handler! but granted by no permission set or capability; \
                 the launcher or workspace SPA invokes it and Tauri denies it at runtime",
            );
        }
    }

    #[test]
    fn app_acl_has_no_stale_grants() {
        // Reverse parity: every command app.toml grants must still exist in
        // generate_handler!, so a removed command's grant doesn't linger.
        const MAIN_RS: &str = include_str!("main.rs");
        let registered: std::collections::HashSet<String> =
            invoke_handler_commands(MAIN_RS).into_iter().collect();
        for command in all_granted_app_commands() {
            assert!(
                registered.contains(&command),
                "permissions/app.toml grants `{command}` but it is not in generate_handler! (a stale \
                 grant; remove its permission)",
            );
        }
    }

    #[test]
    fn invoke_handler_registers_read_dropped_paths() {
        // The SPA's terminal drop handler invokes `read_dropped_paths`
        // at DOM drop time; it must be in `tauri::generate_handler!`
        // or the IPC denies and the terminal path-print silently
        // no-ops. generate_handler! doesn't catch missing entries at
        // compile time, so pin it here.
        const MAIN_RS: &str = include_str!("main.rs");
        assert!(MAIN_RS.contains("dropped_paths::read_dropped_paths,"));
        const DROPPED_PATHS_RS: &str = include_str!("dropped_paths.rs");
        assert!(DROPPED_PATHS_RS.contains("pub async fn read_dropped_paths("));
        // NSPasteboard is AppKit state: the read must run on the main
        // thread, not the IPC worker thread.
        assert!(DROPPED_PATHS_RS.contains("run_on_main_thread"));
    }

    #[test]
    fn drag_pasteboard_read_is_scoped_to_locally_served_windows() {
        // The macOS drag pasteboard is system-wide and persists after
        // the drag ends: a remote-served SPA (outbound-* windows) must
        // NOT be able to poll `read_dropped_paths` and harvest paths the
        // user drags around in other applications.
        // The grant therefore lives in its own capability targeting
        // only the locally-served window kinds...
        let windows = capability_windows(LOCAL_DROP_CAPABILITY_JSON);
        assert!(
            windows.iter().any(|w| w == "workspace-*"),
            "local-drop capability must cover workspace-* windows: {windows:?}",
        );
        assert!(
            windows.iter().any(|w| w == "terminal-*"),
            "local-drop capability must cover terminal-* windows: {windows:?}",
        );
        assert!(
            windows.iter().all(|w| w != "outbound-*" && w != "main"),
            "local-drop capability must stay off remote-served and launcher windows: {windows:?}",
        );
        let perms = capability_permissions(LOCAL_DROP_CAPABILITY_JSON);
        assert!(
            perms.iter().any(|p| p == "allow-read-dropped-paths"),
            "local-drop capability must grant allow-read-dropped-paths: {perms:?}",
        );
        // ...and must not leak in through the broad surfaces that
        // outbound-* windows DO receive.
        let workspace_perms = capability_permissions(WORKSPACE_CAPABILITY_JSON);
        assert!(
            workspace_perms
                .iter()
                .all(|p| p != "allow-read-dropped-paths"),
            "workspace capability must not carry the drag-pasteboard grant: {workspace_perms:?}",
        );
        let workspace_set = app_permission_set("workspace-window");
        assert!(
            workspace_set.iter().all(|p| p != "allow-read-dropped-paths"),
            "workspace-window permission set must not carry the drag-pasteboard grant: {workspace_set:?}",
        );
        // Belt symmetry: the launcher (default capability) is
        // locally-served and outside the harvest threat model, but it
        // has no drop surface either — pin it off so the grant can't
        // drift in through the third broad capability.
        let default_perms = capability_permissions(DEFAULT_CAPABILITY_JSON);
        assert!(
            default_perms.iter().all(|p| p != "allow-read-dropped-paths"),
            "launcher default capability must not carry the drag-pasteboard grant: {default_perms:?}",
        );
        let main_set = app_permission_set("main-window");
        assert!(
            main_set.iter().all(|p| p != "allow-read-dropped-paths"),
            "main-window permission set must not carry the drag-pasteboard grant: {main_set:?}",
        );
    }

    #[test]
    fn connecting_screen_windows_close_for_real() {
        // A window still on connecting.html must be CLOSABLE: the red
        // dot really closes (no bury - a buried connecting window is an
        // unkillable hidden retry loop), and the page itself offers
        // Cmd/Ctrl+W + Ctrl+D chords that invoke request_close_window
        // (destroy, bypassing the bury handler). concat! so the source
        // pin doesn't match this test.
        const SERVE_RS: &str = include_str!("serve.rs");
        assert!(SERVE_RS.contains(concat!(
            "!window_on_connecting",
            "_screen(&app_for_close, &label_for_close)"
        )));
        // KEY_BRIDGE_JS claims the close chord (window capture +
        // stopImmediatePropagation) before BOTH the page's listener and
        // the File-menu accelerator, so the bridge itself must route
        // KeyW to request_close_window while on connecting.html — a
        // page-level chord alone never sees the key (dead Cmd+W).
        // TWO routings: macOS plain Cmd+W (!shift branch) and the
        // Linux/Windows Ctrl+Shift+W (shift branch).
        let close_invoke = concat!("invokeIpc(e, 'request_close", "_window')");
        assert_eq!(SERVE_RS.matches(close_invoke).count(), 2);
        assert!(SERVE_RS.contains("location.pathname.endsWith('/connecting.html')"));
        const CONNECTING_JS: &str = include_str!("../../src/connecting.js");
        assert!(CONNECTING_JS.contains("request_close_window"));
        assert!(CONNECTING_JS.contains("key === 'd'"));
        assert!(CONNECTING_JS.contains("key === 'w'"));
    }

    #[test]
    fn default_capability_covers_extra_launcher_windows() {
        // The default capability covers `main-*` alongside the
        // singleton `main`: any launcher-class window must inherit
        // the same capability as `main`, or external link handling
        // and other plugin IPCs break the moment one exists.
        let windows = capability_windows(DEFAULT_CAPABILITY_JSON);
        assert!(
            windows.iter().any(|w| w == "main"),
            "default capability must still target main: {windows:?}",
        );
        assert!(
            windows.iter().any(|w| w == "main-*"),
            "default capability must target additional main-N launchers: {windows:?}",
        );
        let perms = capability_permissions(DEFAULT_CAPABILITY_JSON);
        assert!(
            perms.iter().any(|p| p == "main-window"),
            "default capability must include main-window app commands: {perms:?}",
        );
        assert!(
            perms.iter().any(|p| p == "opener:allow-open-url"),
            "default capability must include opener:allow-open-url: {perms:?}",
        );
    }
}
