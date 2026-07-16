//! Managed per-gateway account connections.
//!
//! A configured [`Gateway`](crate::config::Gateway) is connected at the
//! ACCOUNT level: the desktop discovers the gateway once, signs in once per
//! gateway account (`desktop.account` scope, PAT in the OS keyring), and
//! polls the gateway's devserver roster; the rostered devservers surface as
//! synthesized rows in the launcher list and vanish when the gateway
//! disconnects. This module owns that lifecycle: the runtime map behind
//! [`GatewayManager`], the connect flow, the roster poll, and the cascade
//! teardown.
//!
//! Failure semantics are load-bearing and test-pinned: an upstream failure
//! (network, 5xx - the roster's own 502 body is `{"error":"upstream
//! error"}`) KEEPS the last-known roster and retries, flipping the gateway
//! to `unreachable` only after [`ROSTER_UNREACHABLE_FAILURES`] consecutive
//! misses; ONLY a 401 runs the disconnect cascade and clears the stored
//! PAT. A degraded all-offline roster is never synthesized - dropping rows
//! on a flaky upstream would close every gateway window.
//!
//! Sign-in is single-flight (the pending-auth slot in [`crate::auth`] is
//! process-global latest-wins): one gateway sign-in may be in the browser
//! at a time; a second Connect surfaces a notice instead of a parallel
//! browser leg. A callback that resumes a gateway removed mid-flight is
//! dropped with a notice.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::Emitter;
use tokio_util::sync::CancellationToken;

use chan_server::GatewayStatus;

use crate::config::{self, Gateway};
use crate::devserver::{self, GatewayDiscovery};
use crate::{auth, AppState};

/// Roster poll cadence per connected gateway.
pub const ROSTER_POLL_SECS: u64 = 10;
/// Consecutive roster-poll failures before a gateway reports
/// `unreachable`. The last-known roster stays served throughout.
pub const ROSTER_UNREACHABLE_FAILURES: u32 = 3;
/// Bound on one roster round trip.
const ROSTER_HTTP_TIMEOUT_SECS: u64 = 5;

/// The launcher-notice event: corner bubbles in the launcher, each naming
/// its SOURCE (gateway / devserver / desktop), expandable to the full
/// message, dismissable.
pub const LAUNCHER_NOTICE: &str = "launcher-notice";

/// One devserver row from the gateway's roster, plus the derived
/// `shared` flag (`owner != username`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RosterDevserver {
    pub owner: String,
    pub devserver_id: String,
    pub label: String,
    pub online: bool,
    pub role: String,
    pub shared: bool,
}

/// The roster endpoint's 200 body.
#[derive(Debug, Deserialize)]
struct RosterResponse {
    username: String,
    devservers: Vec<RosterRow>,
}

#[derive(Debug, Deserialize)]
struct RosterRow {
    owner: String,
    devserver_id: String,
    #[serde(default)]
    label: String,
    online: bool,
    #[serde(default)]
    role: String,
}

/// What one roster round trip amounted to.
#[derive(Debug)]
pub enum RosterFetch {
    /// 200: a fresh roster (and its ETag for the next conditional GET).
    Fresh {
        username: String,
        rows: Vec<RosterDevserver>,
        etag: Option<String>,
    },
    /// 304: the roster is unchanged.
    NotModified,
    /// 401: the PAT is dead or under-scoped. The ONLY outcome that
    /// cascades.
    Unauthorized,
    /// Everything else (network, decode, 5xx incl. the roster's 502
    /// upstream-error body): keep the last-known roster and retry.
    Upstream(String),
}

/// Live state of one connected (or connecting) gateway.
pub struct GatewayRuntime {
    pub discovery: GatewayDiscovery,
    pub username: String,
    pub roster: Vec<RosterDevserver>,
    pub etag: Option<String>,
    pub status: GatewayStatus,
    pub last_error: Option<String>,
    pub consecutive_failures: u32,
    pub pending_signin: bool,
    /// Stamp of the sign-in wait, so only the matching timeout clears it
    /// (a re-click re-stamps; the stale task then no-ops).
    signin_stamp: u64,
    poll_cancel: Option<CancellationToken>,
}

/// The registry's projection of a runtime: the volatile GatewayEntry
/// fields.
#[derive(Debug, Clone, Default)]
pub struct GatewayRuntimeView {
    pub status: GatewayStatus,
    pub pending_signin: bool,
    pub devserver_count: usize,
    pub last_error: Option<String>,
}

/// Effect of applying one roster fetch to a runtime, for the caller to
/// act on outside the lock.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct FetchEffect {
    /// The launcher-visible state changed: fire signal_library_change.
    pub changed: bool,
    /// The PAT is dead: run the 401 cascade.
    pub cascade: bool,
    /// This fetch crossed the unreachable threshold: emit the notice once.
    pub became_unreachable: bool,
}

/// Process-wide gateway runtime map. Lives in [`AppState`] and is shared
/// with the config registry for its live-state projection.
#[derive(Default)]
pub struct GatewayManager {
    runtimes: Mutex<HashMap<String, GatewayRuntime>>,
}

impl GatewayManager {
    /// The volatile GatewayEntry fields for `gateway_id`, or `None` when
    /// the gateway has no runtime (renders as disconnected defaults).
    pub fn view(&self, gateway_id: &str) -> Option<GatewayRuntimeView> {
        let runtimes = self.runtimes.lock().unwrap();
        runtimes.get(gateway_id).map(|rt| GatewayRuntimeView {
            status: rt.status,
            pending_signin: rt.pending_signin,
            devserver_count: rt.roster.len(),
            last_error: rt.last_error.clone(),
        })
    }

    /// Whether ANY gateway sign-in is waiting on the browser. The pending
    /// slot in auth is process-global latest-wins, so a second browser leg
    /// would orphan the first: callers surface a notice instead.
    pub fn any_pending_signin(&self) -> bool {
        self.runtimes
            .lock()
            .unwrap()
            .values()
            .any(|rt| rt.pending_signin)
    }

    /// The roster snapshot of a connected gateway (synthesized-row
    /// sources). Empty when the gateway has no runtime.
    pub fn roster(&self, gateway_id: &str) -> Vec<RosterDevserver> {
        self.runtimes
            .lock()
            .unwrap()
            .get(gateway_id)
            .map(|rt| rt.roster.clone())
            .unwrap_or_default()
    }
}

/// The synthesized launcher-row id for a rostered devserver:
/// `gw:{gateway 8hex sans prefix}:{owner}:{devserver_id}`. Every segment
/// is pinned to `[A-Za-z0-9_-]` (Tauri label charset, `::`-free for the
/// watcher's label parsing); the debug assert catches a wire value that
/// breaks the invariant before it becomes a window label.
pub fn synthesized_row_id(gateway_id: &str, owner: &str, devserver_id: &str) -> String {
    let gw_hex = gateway_id.strip_prefix("gw-").unwrap_or(gateway_id);
    debug_assert!(
        [gw_hex, owner, devserver_id].iter().all(|seg| {
            !seg.is_empty()
                && seg
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        }),
        "synthesized id segment outside [A-Za-z0-9_-]: {gw_hex}/{owner}/{devserver_id}"
    );
    format!("gw:{gw_hex}:{owner}:{devserver_id}")
}

/// Difference between two roster snapshots, keyed by (owner, id).
#[derive(Debug, Default, PartialEq, Eq)]
pub struct RosterDiff {
    pub added: Vec<(String, String)>,
    pub removed: Vec<(String, String)>,
    pub flipped_online: Vec<(String, String)>,
}

impl RosterDiff {
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty() && self.flipped_online.is_empty()
    }
}

/// Pure roster diff: which rows appeared, disappeared, or flipped their
/// online bit. Label/role edits don't count as membership changes; the
/// fresh snapshot replaces the cache either way.
pub fn diff_rosters(old: &[RosterDevserver], new: &[RosterDevserver]) -> RosterDiff {
    let key = |r: &RosterDevserver| (r.owner.clone(), r.devserver_id.clone());
    let old_map: HashMap<_, _> = old.iter().map(|r| (key(r), r.online)).collect();
    let new_map: HashMap<_, _> = new.iter().map(|r| (key(r), r.online)).collect();
    let mut diff = RosterDiff::default();
    for (k, online) in &new_map {
        match old_map.get(k) {
            None => diff.added.push(k.clone()),
            Some(was) if was != online => diff.flipped_online.push(k.clone()),
            Some(_) => {}
        }
    }
    for k in old_map.keys() {
        if !new_map.contains_key(k) {
            diff.removed.push(k.clone());
        }
    }
    diff
}

/// Apply one fetch outcome to a runtime. Pure over the runtime struct so
/// the failure semantics are table-testable: upstream failures KEEP the
/// roster and count toward unreachable; only 401 cascades; success of
/// either flavor resets the failure counter.
pub fn apply_roster_fetch(rt: &mut GatewayRuntime, fetch: RosterFetch) -> FetchEffect {
    let mut effect = FetchEffect::default();
    match fetch {
        RosterFetch::Fresh {
            username,
            rows,
            etag,
        } => {
            let diff = diff_rosters(&rt.roster, &rows);
            effect.changed = !diff.is_empty() || rt.status != GatewayStatus::Connected;
            rt.username = username;
            rt.roster = rows;
            rt.etag = etag;
            rt.consecutive_failures = 0;
            rt.status = GatewayStatus::Connected;
            rt.last_error = None;
        }
        RosterFetch::NotModified => {
            effect.changed = rt.status != GatewayStatus::Connected;
            rt.consecutive_failures = 0;
            rt.status = GatewayStatus::Connected;
            rt.last_error = None;
        }
        RosterFetch::Upstream(message) => {
            rt.consecutive_failures = rt.consecutive_failures.saturating_add(1);
            rt.last_error = Some(message);
            if rt.consecutive_failures >= ROSTER_UNREACHABLE_FAILURES
                && rt.status != GatewayStatus::Unreachable
            {
                rt.status = GatewayStatus::Unreachable;
                effect.changed = true;
                effect.became_unreachable = true;
            }
        }
        RosterFetch::Unauthorized => {
            effect.cascade = true;
        }
    }
    effect
}

fn roster_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(ROSTER_HTTP_TIMEOUT_SECS))
        .build()
        .map_err(|e| format!("building roster http client: {e}"))
}

/// One roster round trip. Status codes are the contract: 200 parses a
/// fresh roster (`shared` derived from `owner != username`), 304 answers
/// a matching If-None-Match, 401 means the PAT is dead or under-scoped,
/// and EVERYTHING else - network, decode, 5xx - is an upstream failure
/// the caller retries without dropping state.
pub async fn fetch_roster(roster_url: &str, pat_secret: &str, etag: Option<&str>) -> RosterFetch {
    let client = match roster_client() {
        Ok(c) => c,
        Err(e) => return RosterFetch::Upstream(e),
    };
    let mut req = client.get(roster_url).bearer_auth(pat_secret);
    if let Some(etag) = etag {
        req = req.header(reqwest::header::IF_NONE_MATCH, etag);
    }
    let resp = match req.send().await {
        Ok(r) => r,
        Err(e) => return RosterFetch::Upstream(format!("roster request failed: {e}")),
    };
    match resp.status() {
        reqwest::StatusCode::NOT_MODIFIED => RosterFetch::NotModified,
        reqwest::StatusCode::UNAUTHORIZED => RosterFetch::Unauthorized,
        status if status.is_success() => {
            let etag = resp
                .headers()
                .get(reqwest::header::ETAG)
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string());
            match resp.json::<RosterResponse>().await {
                Ok(body) => {
                    let username = body.username;
                    let rows = body
                        .devservers
                        .into_iter()
                        .map(|r| RosterDevserver {
                            shared: r.owner != username,
                            owner: r.owner,
                            devserver_id: r.devserver_id,
                            label: r.label,
                            online: r.online,
                            role: r.role,
                        })
                        .collect();
                    RosterFetch::Fresh {
                        username,
                        rows,
                        etag,
                    }
                }
                Err(e) => RosterFetch::Upstream(format!("decoding roster: {e}")),
            }
        }
        status => RosterFetch::Upstream(format!("roster returned HTTP {status}")),
    }
}

/// Emit a launcher-notice bubble. Fire-and-forget: the launcher's notice
/// store keeps what it receives; nothing is replayed to a launcher that
/// was not yet listening.
pub fn emit_notice(
    app: &tauri::AppHandle,
    kind: &str,
    source_type: &str,
    source_id: &str,
    source_label: &str,
    title: &str,
    message: &str,
) {
    #[derive(Serialize)]
    struct Source<'a> {
        r#type: &'a str,
        id: &'a str,
        label: &'a str,
    }
    #[derive(Serialize)]
    struct Notice<'a> {
        id: String,
        kind: &'a str,
        source: Source<'a>,
        title: &'a str,
        message: &'a str,
        at: u64,
    }
    let mut buf = [0u8; 2];
    let id = match getrandom::getrandom(&mut buf) {
        Ok(()) => format!("ntc-{:02x}{:02x}", buf[0], buf[1]),
        Err(_) => format!("ntc-{:04x}", config::now_millis() as u16),
    };
    let _ = app.emit(
        LAUNCHER_NOTICE,
        &Notice {
            id,
            kind,
            source: Source {
                r#type: source_type,
                id: source_id,
                label: source_label,
            },
            title,
            message,
            at: config::now_millis(),
        },
    );
}

fn gateway_row(state: &AppState, gateway_id: &str) -> Result<Gateway, String> {
    let cfg = state
        .store
        .lock()
        .unwrap()
        .get()
        .map_err(|e| e.to_string())?;
    cfg.gateways
        .iter()
        .find(|g| g.id == gateway_id)
        .cloned()
        .ok_or_else(|| format!("no gateway {gateway_id}"))
}

fn display_label(g: &Gateway) -> String {
    if !g.label.is_empty() {
        return g.label.clone();
    }
    url::Url::parse(&g.url)
        .ok()
        .and_then(|u| u.host_str().map(|h| h.to_string()))
        .unwrap_or_else(|| g.url.clone())
}

/// Persist the connect intent (`enabled`) on a gateway row. Missing rows
/// are a no-op (removed mid-flight).
fn persist_enabled(state: &AppState, gateway_id: &str, enabled: bool) {
    let mut store = state.store.lock().unwrap();
    let Ok(mut cfg) = store.get() else { return };
    let Some(g) = cfg.gateways.iter_mut().find(|g| g.id == gateway_id) else {
        return;
    };
    if g.enabled == enabled {
        return;
    }
    g.enabled = enabled;
    if let Err(e) = store.save(&cfg) {
        tracing::warn!(gateway = %gateway_id, error = %e, "persisting gateway enabled flag failed");
    }
}

/// Connect a configured gateway. `interactive` gates the browser leg:
/// a user click may open the sign-in page; the startup autoconnect must
/// never pop a browser and instead leaves the row disconnected with a
/// sign-in-required error.
pub async fn connect_gateway(
    app: tauri::AppHandle,
    state: Arc<AppState>,
    gateway_id: String,
    interactive: bool,
) -> Result<(), String> {
    let gateway = gateway_row(&state, &gateway_id)?;
    let label = display_label(&gateway);

    // Coalesce: a connect already in flight (and not merely parked on the
    // browser) finishes on its own; a re-click while waiting on the
    // browser falls through to re-open the sign-in page (latest-wins).
    {
        let runtimes = state.gateway_manager.runtimes.lock().unwrap();
        if let Some(existing) = runtimes.get(&gateway_id) {
            if existing.status == GatewayStatus::Connecting && !existing.pending_signin {
                return Ok(());
            }
        }
    }
    persist_enabled(&state, &gateway_id, true);

    let discovery = match devserver::discover_gateway(&gateway.url).await {
        Ok(d) => d,
        Err(e) => {
            let msg = format!("gateway discovery failed: {e}");
            upsert_disconnected(&state, &gateway_id, &msg);
            emit_notice(
                &app,
                "error",
                "gateway",
                &gateway_id,
                &label,
                "Gateway unreachable",
                &msg,
            );
            crate::signal_devserver_rows_changed(&app, &state);
            return Err(msg);
        }
    };
    let Some(roster_url) = discovery.roster_url.clone() else {
        let msg =
            "this gateway is too old for account connections - upgrade the gateway".to_string();
        upsert_disconnected(&state, &gateway_id, &msg);
        emit_notice(
            &app,
            "error",
            "gateway",
            &gateway_id,
            &label,
            "Gateway too old",
            &msg,
        );
        crate::signal_devserver_rows_changed(&app, &state);
        return Err(msg);
    };

    let pat = auth::load_gateway_pat(&discovery.identity_origin)?;
    let Some(pat) = pat else {
        return signin_leg(&app, &state, &gateway_id, &label, &discovery, interactive);
    };

    upsert_connecting(&state, &gateway_id, &discovery);
    crate::signal_devserver_rows_changed(&app, &state);

    match fetch_roster(&roster_url, &pat.secret, None).await {
        RosterFetch::Unauthorized => {
            // Dead or under-scoped PAT (a desktop.connect-era credential
            // cannot read the roster): self-heal into one re-sign-in.
            auth::clear_gateway_pat(&discovery.identity_origin)?;
            signin_leg(&app, &state, &gateway_id, &label, &discovery, interactive)
        }
        fetch => {
            let effect = {
                let mut runtimes = state.gateway_manager.runtimes.lock().unwrap();
                let rt = runtimes
                    .entry(gateway_id.clone())
                    .or_insert_with(|| new_runtime(discovery.clone()));
                apply_roster_fetch(rt, fetch)
            };
            if effect.became_unreachable {
                notice_unreachable(&app, &state, &gateway_id, &label);
            }
            spawn_roster_poll(&app, &state, &gateway_id, roster_url);
            crate::signal_devserver_rows_changed(&app, &state);
            Ok(())
        }
    }
}

fn new_runtime(discovery: GatewayDiscovery) -> GatewayRuntime {
    GatewayRuntime {
        discovery,
        username: String::new(),
        roster: Vec::new(),
        etag: None,
        status: GatewayStatus::Connecting,
        last_error: None,
        consecutive_failures: 0,
        pending_signin: false,
        signin_stamp: 0,
        poll_cancel: None,
    }
}

fn upsert_connecting(state: &AppState, gateway_id: &str, discovery: &GatewayDiscovery) {
    let mut runtimes = state.gateway_manager.runtimes.lock().unwrap();
    let rt = runtimes
        .entry(gateway_id.to_string())
        .or_insert_with(|| new_runtime(discovery.clone()));
    rt.discovery = discovery.clone();
    rt.status = GatewayStatus::Connecting;
    rt.pending_signin = false;
}

/// Park a runtime as disconnected with an error. A gateway that never got
/// past discovery has no runtime to park; the registry projection then
/// falls back to disconnected defaults and the caller's notice carries
/// the error.
fn upsert_disconnected(state: &AppState, gateway_id: &str, error: &str) {
    let mut runtimes = state.gateway_manager.runtimes.lock().unwrap();
    if let Some(rt) = runtimes.get_mut(gateway_id) {
        rt.status = GatewayStatus::Disconnected;
        rt.pending_signin = false;
        rt.last_error = Some(error.to_string());
    }
}

/// The browser sign-in leg. Non-interactive callers (startup autoconnect)
/// never open a browser: the row parks disconnected with a
/// sign-in-required error instead.
fn signin_leg(
    app: &tauri::AppHandle,
    state: &Arc<AppState>,
    gateway_id: &str,
    label: &str,
    discovery: &GatewayDiscovery,
    interactive: bool,
) -> Result<(), String> {
    if !interactive {
        upsert_disconnected(state, gateway_id, "sign-in required - click Connect");
        crate::signal_devserver_rows_changed(app, state);
        return Ok(());
    }
    if state
        .gateway_manager
        .any_pending_signin_other_than(gateway_id)
    {
        let msg = "another gateway sign-in is waiting on the browser - finish it first".to_string();
        emit_notice(
            app,
            "info",
            "gateway",
            gateway_id,
            label,
            "Sign-in busy",
            &msg,
        );
        return Ok(());
    }
    auth::open_gateway_signin(
        app,
        &discovery.identity_origin,
        &discovery.desktop_authorize_url,
        gateway_id,
    )?;
    let stamp = config::now_millis();
    {
        let mut runtimes = state.gateway_manager.runtimes.lock().unwrap();
        let rt = runtimes
            .entry(gateway_id.to_string())
            .or_insert_with(|| new_runtime(discovery.clone()));
        rt.discovery = discovery.clone();
        rt.pending_signin = true;
        rt.signin_stamp = stamp;
        rt.status = GatewayStatus::Connecting;
    }
    crate::signal_devserver_rows_changed(app, state);
    // Expire the wait like the devserver rows do: only the matching stamp
    // clears, so a re-click's fresh wait survives the stale timeout.
    let app = app.clone();
    let state = Arc::clone(state);
    let gateway_id = gateway_id.to_string();
    let label = label.to_string();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(crate::GATEWAY_SIGNIN_TIMEOUT).await;
        let expired = {
            let mut runtimes = state.gateway_manager.runtimes.lock().unwrap();
            match runtimes.get_mut(&gateway_id) {
                Some(rt) if rt.pending_signin && rt.signin_stamp == stamp => {
                    rt.pending_signin = false;
                    rt.status = GatewayStatus::Disconnected;
                    rt.last_error = Some("sign-in was not completed in the browser".to_string());
                    true
                }
                _ => false,
            }
        };
        if expired {
            emit_notice(
                &app,
                "error",
                "gateway",
                &gateway_id,
                &label,
                "Sign-in timed out",
                "sign-in was not completed in the browser; click Connect to try again",
            );
            crate::signal_devserver_rows_changed(&app, &state);
        }
    });
    Ok(())
}

impl GatewayManager {
    fn any_pending_signin_other_than(&self, gateway_id: &str) -> bool {
        self.runtimes
            .lock()
            .unwrap()
            .iter()
            .any(|(id, rt)| rt.pending_signin && id != gateway_id)
    }
}

/// Resume after a sign-in callback stored the PAT: the gateway must still
/// be configured (ruling: a mid-flight removal drops the sign-in with a
/// notice), then the connect re-runs PAT-backed.
pub async fn resume_gateway_signin(
    app: tauri::AppHandle,
    state: Arc<AppState>,
    gateway_id: String,
) {
    {
        let mut runtimes = state.gateway_manager.runtimes.lock().unwrap();
        if let Some(rt) = runtimes.get_mut(&gateway_id) {
            rt.pending_signin = false;
        }
    }
    match gateway_row(&state, &gateway_id) {
        Ok(_) => {
            if let Err(e) =
                connect_gateway(app.clone(), Arc::clone(&state), gateway_id.clone(), true).await
            {
                tracing::warn!(gateway = %gateway_id, error = %e, "gateway connect after sign-in failed");
            }
        }
        Err(_) => {
            emit_notice(
                &app,
                "info",
                "desktop",
                "desktop",
                "chan-desktop",
                "Sign-in ignored",
                "the sign-in completed for a gateway that was removed meanwhile",
            );
            let mut runtimes = state.gateway_manager.runtimes.lock().unwrap();
            runtimes.remove(&gateway_id);
        }
    }
}

fn spawn_roster_poll(
    app: &tauri::AppHandle,
    state: &Arc<AppState>,
    gateway_id: &str,
    roster_url: String,
) {
    let cancel = CancellationToken::new();
    let identity_origin = {
        let mut runtimes = state.gateway_manager.runtimes.lock().unwrap();
        let Some(rt) = runtimes.get_mut(gateway_id) else {
            return;
        };
        // A reconnect replaces the poll: cancel the previous loop first.
        if let Some(old) = rt.poll_cancel.replace(cancel.clone()) {
            old.cancel();
        }
        rt.discovery.identity_origin.clone()
    };
    let app = app.clone();
    let state = Arc::clone(state);
    let gateway_id = gateway_id.to_string();
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::select! {
                _ = cancel.cancelled() => break,
                _ = tokio::time::sleep(Duration::from_secs(ROSTER_POLL_SECS)) => {}
            }
            // Re-read the PAT each tick: a re-sign-in mid-poll swaps the
            // credential without restarting the loop.
            let secret = match auth::load_gateway_pat(&identity_origin) {
                Ok(Some(pat)) => pat.secret,
                Ok(None) | Err(_) => {
                    // No credential: the next tick retries; a cascade (which
                    // clears the PAT) also cancels this loop.
                    continue;
                }
            };
            let etag = {
                let runtimes = state.gateway_manager.runtimes.lock().unwrap();
                match runtimes.get(&gateway_id) {
                    Some(rt) => rt.etag.clone(),
                    None => break,
                }
            };
            let fetch = fetch_roster(&roster_url, &secret, etag.as_deref()).await;
            let effect = {
                let mut runtimes = state.gateway_manager.runtimes.lock().unwrap();
                match runtimes.get_mut(&gateway_id) {
                    Some(rt) => apply_roster_fetch(rt, fetch),
                    None => break,
                }
            };
            if effect.cascade {
                let label = gateway_row(&state, &gateway_id)
                    .map(|g| display_label(&g))
                    .unwrap_or_else(|_| gateway_id.clone());
                cascade_disconnect(&app, &state, &gateway_id, CascadeReason::Unauthorized).await;
                emit_notice(
                    &app,
                    "error",
                    "gateway",
                    &gateway_id,
                    &label,
                    "Gateway sign-in expired",
                    "the gateway rejected the stored sign-in; click Connect to sign in again",
                );
                break;
            }
            if effect.became_unreachable {
                let label = gateway_row(&state, &gateway_id)
                    .map(|g| display_label(&g))
                    .unwrap_or_else(|_| gateway_id.clone());
                notice_unreachable(&app, &state, &gateway_id, &label);
            }
            if effect.changed {
                crate::signal_devserver_rows_changed(&app, &state);
            }
        }
    });
}

fn notice_unreachable(
    app: &tauri::AppHandle,
    state: &Arc<AppState>,
    gateway_id: &str,
    label: &str,
) {
    let detail = state
        .gateway_manager
        .view(gateway_id)
        .and_then(|v| v.last_error)
        .unwrap_or_default();
    emit_notice(
        app,
        "error",
        "gateway",
        gateway_id,
        label,
        "Gateway unreachable",
        &format!("the gateway has missed several roster checks; its devservers stay listed with their last-known state ({detail})"),
    );
}

/// Why a cascade runs, deciding PAT handling and the enabled flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CascadeReason {
    /// The user clicked Disconnect: persist enabled=false, keep the PAT.
    UserDisconnect,
    /// The row was removed: config row is already gone, keep the PAT
    /// (a re-add reconnects without a new sign-in).
    Removed,
    /// The roster answered 401: clear the PAT (it is dead), keep enabled
    /// (the next connect runs the sign-in leg).
    Unauthorized,
}

/// Tear down everything a gateway contributed: stop the poll, drop live
/// connections of its rostered devservers (idempotent per row), drop the
/// runtime (its synthesized rows vanish from the next list), then signal.
/// Serialized per gateway by the runtimes lock taking the runtime OUT
/// before any teardown runs; a second cascade for the same id finds no
/// runtime and no-ops.
pub async fn cascade_disconnect(
    app: &tauri::AppHandle,
    state: &Arc<AppState>,
    gateway_id: &str,
    reason: CascadeReason,
) {
    let runtime = {
        let mut runtimes = state.gateway_manager.runtimes.lock().unwrap();
        runtimes.remove(gateway_id)
    };
    let Some(runtime) = runtime else {
        // Nothing live; still honor the intent flag for a user disconnect.
        if reason == CascadeReason::UserDisconnect {
            persist_enabled(state, gateway_id, false);
            crate::signal_devserver_rows_changed(app, state);
        }
        return;
    };
    if let Some(cancel) = &runtime.poll_cancel {
        cancel.cancel();
    }
    for row in &runtime.roster {
        let synth_id = synthesized_row_id(gateway_id, &row.owner, &row.devserver_id);
        crate::teardown_devserver_connection(app, state, &synth_id);
    }
    match reason {
        CascadeReason::UserDisconnect => persist_enabled(state, gateway_id, false),
        CascadeReason::Removed => {}
        CascadeReason::Unauthorized => {
            if let Err(e) = auth::clear_gateway_pat(&runtime.discovery.identity_origin) {
                tracing::warn!(gateway = %gateway_id, error = %e, "clearing gateway PAT failed");
            }
        }
    }
    crate::signal_devserver_rows_changed(app, state);
}

/// Startup autoconnect: every enabled gateway connects non-interactively
/// (no browser legs at login; PAT-less rows park as sign-in required).
pub fn autoconnect_enabled_gateways(app: &tauri::AppHandle, state: &Arc<AppState>) {
    let gateways = state
        .store
        .lock()
        .unwrap()
        .get()
        .map(|cfg| cfg.gateways)
        .unwrap_or_default();
    for g in gateways.into_iter().filter(|g| g.enabled) {
        let app = app.clone();
        let state = Arc::clone(state);
        tauri::async_runtime::spawn(async move {
            if let Err(e) = connect_gateway(app, state, g.id.clone(), false).await {
                tracing::info!(gateway = %g.id, error = %e, "startup gateway connect failed");
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(owner: &str, id: &str, online: bool) -> RosterDevserver {
        RosterDevserver {
            owner: owner.to_string(),
            devserver_id: id.to_string(),
            label: String::new(),
            online,
            role: "owner".to_string(),
            shared: false,
        }
    }

    fn runtime_with(rows: Vec<RosterDevserver>) -> GatewayRuntime {
        let mut rt = new_runtime(GatewayDiscovery {
            kind: "chan-gateway".into(),
            api_version: 1,
            identity_origin: "https://id.chan.app".into(),
            desktop_authorize_url: "https://id.chan.app/desktop/authorize".into(),
            desktop_entry_url: "https://id.chan.app/desktop/v1/devserver/entry".into(),
            devserver_proxy_origin: "https://x.devserver.chan.app".into(),
            roster_url: Some("https://id.chan.app/desktop/v1/devservers".into()),
        });
        rt.roster = rows;
        rt.status = GatewayStatus::Connected;
        rt
    }

    #[test]
    fn diff_reports_adds_removes_and_online_flips() {
        let old = vec![row("alice", "a", true), row("alice", "b", false)];
        let new = vec![row("alice", "b", true), row("bob", "c", true)];
        let diff = diff_rosters(&old, &new);
        assert_eq!(diff.added, vec![("bob".to_string(), "c".to_string())]);
        assert_eq!(diff.removed, vec![("alice".to_string(), "a".to_string())]);
        assert_eq!(
            diff.flipped_online,
            vec![("alice".to_string(), "b".to_string())]
        );
        assert!(diff_rosters(&new, &new).is_empty());
    }

    #[test]
    fn upstream_failures_keep_the_roster_and_flip_unreachable_once() {
        // Ruling: a 502/network failure NEVER drops the last-known roster;
        // the gateway flips unreachable only after N consecutive misses,
        // and only once (one notice, one signal).
        let mut rt = runtime_with(vec![row("alice", "a", true)]);
        for i in 1..ROSTER_UNREACHABLE_FAILURES {
            let effect = apply_roster_fetch(&mut rt, RosterFetch::Upstream("boom".into()));
            assert_eq!(effect, FetchEffect::default(), "failure {i} is silent");
            assert_eq!(rt.status, GatewayStatus::Connected);
            assert_eq!(rt.roster.len(), 1, "roster kept on failure {i}");
        }
        let effect = apply_roster_fetch(&mut rt, RosterFetch::Upstream("boom".into()));
        assert!(effect.changed && effect.became_unreachable);
        assert_eq!(rt.status, GatewayStatus::Unreachable);
        assert_eq!(rt.roster.len(), 1, "roster kept while unreachable");
        // Further failures stay silent (already unreachable).
        let effect = apply_roster_fetch(&mut rt, RosterFetch::Upstream("boom".into()));
        assert_eq!(effect, FetchEffect::default());
    }

    #[test]
    fn success_resets_failures_and_recovers_unreachable() {
        let mut rt = runtime_with(vec![row("alice", "a", true)]);
        for _ in 0..ROSTER_UNREACHABLE_FAILURES {
            apply_roster_fetch(&mut rt, RosterFetch::Upstream("boom".into()));
        }
        assert_eq!(rt.status, GatewayStatus::Unreachable);
        let effect = apply_roster_fetch(&mut rt, RosterFetch::NotModified);
        assert!(effect.changed, "recovery signals");
        assert_eq!(rt.status, GatewayStatus::Connected);
        assert_eq!(rt.consecutive_failures, 0);
        assert_eq!(rt.last_error, None);
    }

    #[test]
    fn only_unauthorized_cascades() {
        let mut rt = runtime_with(vec![row("alice", "a", true)]);
        let effect = apply_roster_fetch(&mut rt, RosterFetch::Unauthorized);
        assert!(effect.cascade);
        assert!(!effect.changed);
        // The roster survives until the cascade takes the runtime out.
        assert_eq!(rt.roster.len(), 1);
        let effect = apply_roster_fetch(&mut rt, RosterFetch::Upstream("502".into()));
        assert!(!effect.cascade, "upstream failure never cascades");
    }

    #[test]
    fn fresh_roster_replaces_cache_and_signals_only_on_change() {
        let mut rt = runtime_with(vec![row("alice", "a", true)]);
        // Same membership + same online bits: no signal.
        let effect = apply_roster_fetch(
            &mut rt,
            RosterFetch::Fresh {
                username: "alice".into(),
                rows: vec![row("alice", "a", true)],
                etag: Some("\"e1\"".into()),
            },
        );
        assert!(!effect.changed);
        assert_eq!(rt.etag.as_deref(), Some("\"e1\""));
        // A flip signals.
        let effect = apply_roster_fetch(
            &mut rt,
            RosterFetch::Fresh {
                username: "alice".into(),
                rows: vec![row("alice", "a", false)],
                etag: None,
            },
        );
        assert!(effect.changed);
    }

    #[test]
    fn synthesized_id_shape_is_pinned() {
        assert_eq!(
            synthesized_row_id("gw-1a2b3c4d", "alice", &"d".repeat(64)),
            format!("gw:1a2b3c4d:alice:{}", "d".repeat(64))
        );
    }

    #[test]
    #[should_panic(expected = "synthesized id segment")]
    #[cfg(debug_assertions)]
    fn synthesized_id_rejects_bad_charset() {
        let _ = synthesized_row_id("gw-1a2b3c4d", "al:ice", "abc");
    }

    async fn spawn_roster_stub(
        response: axum::response::Response<axum::body::Body>,
    ) -> (String, tokio::task::JoinHandle<()>) {
        use axum::routing::get;
        let (parts, body) = response.into_parts();
        let bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap();
        let app = axum::Router::new().route(
            "/desktop/v1/devservers",
            get(move |headers: axum::http::HeaderMap| {
                let parts = parts.clone();
                let bytes = bytes.clone();
                async move {
                    // Echo 304 when the conditional matches the stub ETag.
                    if let (Some(inm), Some(etag)) = (
                        headers.get(axum::http::header::IF_NONE_MATCH),
                        parts.headers.get(axum::http::header::ETAG),
                    ) {
                        if inm == etag {
                            return axum::http::Response::builder()
                                .status(304)
                                .body(axum::body::Body::empty())
                                .unwrap();
                        }
                    }
                    let mut resp = axum::http::Response::builder().status(parts.status);
                    for (k, v) in parts.headers.iter() {
                        resp = resp.header(k, v);
                    }
                    resp.body(axum::body::Body::from(bytes)).unwrap()
                }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        (format!("http://{addr}/desktop/v1/devservers"), handle)
    }

    fn resp(status: u16, etag: Option<&str>, body: &str) -> axum::response::Response {
        let mut b = axum::http::Response::builder().status(status);
        if let Some(etag) = etag {
            b = b.header(axum::http::header::ETAG, etag);
        }
        b.body(axum::body::Body::from(body.to_string())).unwrap()
    }

    #[tokio::test]
    async fn fetch_parses_a_fresh_roster_and_derives_shared() {
        let body = r#"{"username":"alice","devservers":[
            {"owner":"alice","devserver_id":"a1","label":"laptop","online":true,"role":"owner"},
            {"owner":"bob","devserver_id":"b1","label":"","online":false,"role":"viewer"}]}"#;
        let (url, server) = spawn_roster_stub(resp(200, Some("\"e1\""), body)).await;
        match fetch_roster(&url, "pat-secret", None).await {
            RosterFetch::Fresh {
                username,
                rows,
                etag,
            } => {
                assert_eq!(username, "alice");
                assert_eq!(etag.as_deref(), Some("\"e1\""));
                assert_eq!(rows.len(), 2);
                assert!(!rows[0].shared, "own row");
                assert!(rows[1].shared, "foreign owner derives shared");
            }
            other => panic!("expected Fresh, got {other:?}"),
        }
        server.abort();
    }

    #[tokio::test]
    async fn fetch_maps_304_401_and_502_to_their_outcomes() {
        let body = r#"{"username":"alice","devservers":[]}"#;
        let (url, server) = spawn_roster_stub(resp(200, Some("\"e2\""), body)).await;
        assert!(matches!(
            fetch_roster(&url, "s", Some("\"e2\"")).await,
            RosterFetch::NotModified
        ));
        server.abort();

        let (url, server) = spawn_roster_stub(resp(401, None, r#"{"error":"unauthorized"}"#)).await;
        assert!(matches!(
            fetch_roster(&url, "s", None).await,
            RosterFetch::Unauthorized
        ));
        server.abort();

        // The roster's pinned 502 body: an upstream failure, never a
        // cascade, never a roster drop.
        let (url, server) =
            spawn_roster_stub(resp(502, None, r#"{"error":"upstream error"}"#)).await;
        match fetch_roster(&url, "s", None).await {
            RosterFetch::Upstream(msg) => assert!(msg.contains("502"), "{msg}"),
            other => panic!("expected Upstream, got {other:?}"),
        }
        server.abort();
    }
}
