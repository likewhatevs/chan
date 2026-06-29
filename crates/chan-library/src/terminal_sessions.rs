//! Long-lived PTY session registry.
//!
//! A terminal WebSocket is only an attachment. The PTY, child process,
//! replay ring, and lifecycle policy live here so browser reloads can
//! detach and reattach without killing the shell.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use portable_pty::{native_pty_system, PtySize};
use rand::RngCore;
use serde::Serialize;
use tokio::sync::{broadcast, watch, Notify};
use tokio::task::JoinHandle;

use crate::config::TerminalConfig;
use crate::time::{now_unix_millis, now_unix_secs};

mod bytes;
mod platform;
mod redraw;
mod ring;

use bytes::{contains_subslice, visible_activity_bytes};
#[cfg(windows)]
pub use platform::prime_windows_shell;
#[cfg(unix)]
pub use platform::user_shell;
use platform::{
    clear_mcp_env, command_builder, locale_selects_utf8, path_inside_root, process_cwd,
    reject_terminal_spawn_if_fd_pressure, set_mcp_env, terminal_home_dir,
};
#[cfg(test)]
use platform::{fd_headroom_allows, TERMINAL_SESSION_FD_ESTIMATE};
use redraw::force_redraw_with_wobble;
#[cfg(test)]
use redraw::redraw_wobble_size;
use ring::RingBuffer;

const BROADCAST_CAP: usize = 1024;

// `cs terminal write` serialization queue (the auto-deliver poke chain).
// Each session has a bounded FIFO; the drainer delivers the next message
// only when the agent is IDLE (its output has quiesced) and then awaits the
// agent's generation-START before the next, so chained pokes submit one
// after another instead of stacking into one compose. The signal is purely
// output quiescence (`last_output_at`); see cs-write-queue-design.md.
const WRITE_QUEUE_CAP: usize = 100;
/// Output-idle threshold: the agent is considered done generating when no
/// output has arrived for this long. Conservative to ride over brief
/// mid-stream gaps; tune against real agent streaming.
const WRITE_QUEUE_QUIET_MS: i64 = 800;
/// After a deliver+submit, wait at most this long for the agent's generation
/// to START before allowing the next delivery. Caps the post-submit window
/// so a message that did not trigger generation (e.g. no submit chord) does
/// not wedge the queue.
const WRITE_QUEUE_GEN_START_CAP_MS: i64 = 2000;
/// How often the drainer scans sessions for a deliverable queued write.
const WRITE_QUEUE_DRAIN_TICK: Duration = Duration::from_millis(150);

const ALT_SCREEN_ENTER: &[u8] = b"\x1b[?1049h";
const ALT_SCREEN_EXIT: &[u8] = b"\x1b[?1049l";
const ALT_SCREEN_TAIL_BYTES: usize = ALT_SCREEN_ENTER.len() - 1;
const REDRAW_WOBBLE_DELAY: Duration = Duration::from_millis(50);
pub const ALT_SCREEN_ATTACH_PRELUDE: &[u8] = b"\x1b[?1049h\x1b[2J\x1b[H";

/// DEC private modes whose loss on a fresh client's reattach breaks INPUT —
/// key encoding (DECCKM) and mouse-event delivery/encoding — because the live
/// foreground program set them once at startup and will NOT re-announce after
/// a reattach. Reattaching in alt-screen replays no scrollback, so the original
/// set sequences are gone and the fresh terminal comes up at defaults: arrows
/// stop navigating (DECCKM) and the wheel/clicks stop reaching the program
/// (mouse). We track the set currently on (scanned from PTY output by
/// [`Session::update_private_modes`]) and re-assert it in the attach prelude —
/// generalizing the single-bool alt-screen restore. Screen-rendering modes
/// (autowrap, cursor visibility) are deliberately NOT tracked: the program's
/// post-attach redraw re-establishes them. Alt-screen (1049/1047/47) is NOT
/// here either — it is handled by [`ALT_SCREEN_ATTACH_PRELUDE`].
const TRACKED_PRIVATE_MODES: &[u16] = &[
    1,    // DECCKM — application cursor keys (arrow encoding: \e[A vs \eOA)
    1000, // mouse: normal button (press/release) tracking
    1002, // mouse: button-event (drag) tracking
    1003, // mouse: any-event (motion) tracking
    1004, // focus in/out reporting
    1006, // mouse: SGR extended coordinate encoding
    2004, // bracketed paste
];
/// Upper bound on a carried partial private-mode CSI (`\e[?<params>(h|l)`) split
/// across PTY reads: a handful of `;`-joined mode numbers. Past this, a dangling
/// `\e[?…` is not a real mode toggle and is dropped rather than buffered.
const PRIVATE_MODE_TAIL_CAP: usize = 64;

#[derive(Debug, Clone)]
pub struct RegistryConfig {
    pub workspace_root: PathBuf,
    pub mcp_socket_path: Option<PathBuf>,
    pub control_socket_path: Option<PathBuf>,
    pub terminal: TerminalConfig,
}

#[derive(Debug)]
pub struct Registry {
    config: RegistryConfig,
    sessions: Mutex<HashMap<String, Arc<Session>>>,
    /// Last PTY process exit observed in this registry. This is sticky at the
    /// tenant/registry level so a control-terminal poller can still see the
    /// script exit after an attached terminal websocket removes the session.
    last_exit: Arc<Mutex<Option<TerminalExit>>>,
    /// Fires whenever the live roster changes (create / close / restart /
    /// broadcast-toggle). The roster broadcaster task awaits this and
    /// republishes a fresh snapshot onto the `/ws` bus so every window's
    /// SPA sees the same cross-window terminal set. `Notify` coalesces
    /// bursts into one wakeup (natural debounce) and stores a permit when
    /// no waiter is parked, so a change is never missed.
    roster_notify: Arc<Notify>,
    /// Command this tenant's terminals run on their PTY when an open
    /// request carries no command of its own. `None` keeps the user's
    /// default interactive shell. A single-purpose terminal tenant (a
    /// window whose PTY runs a connect script) sets it once at creation.
    default_command: Mutex<Option<String>>,
    /// Window ids (the `?w=` session-blob key) that currently have a durable
    /// saved layout blob. Maintained by the session routes: a `PUT
    /// /api/session?w=W` marks W persisted, a `DELETE` forgets it. Drives the
    /// persistence-based session lifetime (see [`Registry::prune_idle_at`]): a
    /// persisted window's detached sessions survive a client disconnect
    /// indefinitely (browser-tab semantics — reattach on reconnect), while a
    /// window with no durable blob is an orphan and its detached sessions are
    /// reaped after a grace. The durable blob store is the source of truth;
    /// this set is the in-process cache the pruner consults without touching
    /// disk. It tracks marks for THIS process's lifetime — sessions never
    /// outlive the process (PTYs die with it), so it needs no startup seed.
    persisted_windows: Mutex<HashSet<String>>,
    /// Optional hook fired when [`reap_exited`](Self::reap_exited) reaps a
    /// session that owns a window: the host installs it (on the SHARED terminal
    /// tenant only) to drop the standalone terminal's window-feed row when its
    /// PTY exits, so it does not linger. A workspace tenant
    /// leaves this unset — a pane's death must never close its workspace window.
    window_reaper: Mutex<Option<WindowReaper>>,
    /// Optional hook fired on an EXPLICIT window discard (via
    /// [`reap_window_layout`](Self::reap_window_layout)) to delete the standalone
    /// terminal window's durable layout blob from the chan-server `terminal_blob`
    /// store, which chan-library cannot reach directly. The host wires this on
    /// the persisted terminal tenant only. Deliberately NOT fired from
    /// [`reap_exited`](Self::reap_exited): a persisted terminal whose PTY exits
    /// keeps its blob so the window resurrects on reconnect; only an explicit
    /// discard drops it.
    blob_reaper: Mutex<Option<BlobReaper>>,
    /// Per-PTY-life epoch source. Each spawn (create OR restart) takes the next
    /// value and stamps it on the session, so a reattach can prove its cached
    /// scrollback belongs to the SAME incarnation: a restart reuses the session
    /// id but resets `seq` to 0, so without the bumped generation a stale client
    /// `since` cursor would desync silently (empty replay, no warning).
    generation_counter: AtomicU64,
}

/// Host-installed hook to reap a terminal WINDOW row when its session is reaped.
/// Takes the reaped session's `window_id`. See
/// [`Registry::install_window_reaper`]. A newtype so [`Registry`] keeps deriving
/// `Debug` (a bare `dyn Fn` does not implement it).
#[derive(Clone)]
pub struct WindowReaper(Arc<dyn Fn(&str) + Send + Sync>);

impl WindowReaper {
    /// Wrap a closure taking the reaped session's `window_id`.
    pub fn new(f: impl Fn(&str) + Send + Sync + 'static) -> Self {
        Self(Arc::new(f))
    }

    fn call(&self, window_id: &str) {
        (self.0)(window_id)
    }
}

impl std::fmt::Debug for WindowReaper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("WindowReaper(..)")
    }
}

/// Host-installed hook to delete a discarded terminal window's durable layout
/// blob (the chan-server `terminal_blob` store, which chan-library can't reach).
/// Takes the discarded `window_id`. See [`Registry::install_blob_reaper`]. A
/// newtype so [`Registry`] keeps deriving `Debug` (a bare `dyn Fn` does not).
#[derive(Clone)]
pub struct BlobReaper(Arc<dyn Fn(&str) + Send + Sync>);

impl BlobReaper {
    /// Wrap a closure taking the discarded window's `window_id`.
    pub fn new(f: impl Fn(&str) + Send + Sync + 'static) -> Self {
        Self(Arc::new(f))
    }

    fn call(&self, window_id: &str) {
        (self.0)(window_id)
    }
}

impl std::fmt::Debug for BlobReaper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("BlobReaper(..)")
    }
}

/// Parse the ordinal from a default `Terminal-N` name for lowest-free
/// numbering. Bare `Terminal` counts as `1` (matching the frontend
/// `nextTerminalTitle` regex `^Terminal(?:-(\d+))?$`). Any non-default name
/// (`build`, a team `lead-2`, ...) returns `None` so it never occupies a
/// numbering slot. `Terminal-0` and malformed forms (`Terminal-`,
/// `Terminal-1x`) are rejected.
fn parse_terminal_ordinal(name: &str) -> Option<u64> {
    let rest = name.strip_prefix("Terminal")?;
    if rest.is_empty() {
        return Some(1);
    }
    rest.strip_prefix('-')?
        .parse::<u64>()
        .ok()
        .filter(|&n| n >= 1)
}

/// Broadcast group default. A terminal with no explicit group belongs to
/// this group; it is never special-cased, just the value absence resolves
/// to (mirrors the SPA's `terminalTabGroup`).
pub const DEFAULT_TERMINAL_GROUP: &str = "default";

#[derive(Debug, Clone)]
pub struct CreateOptions {
    pub size: PtySize,
    pub tab_name: Option<String>,
    /// Broadcast group label. `None` resolves to `DEFAULT_TERMINAL_GROUP`.
    /// Stored per live session so `cs term list` / `term write` can
    /// resolve groups server-side, and exported as `$CHAN_TAB_GROUP`.
    pub tab_group: Option<String>,
    pub window_id: Option<String>,
    pub mcp_env: bool,
    pub cwd: Option<PathBuf>,
    pub command: Option<String>,
    pub env: BTreeMap<String, String>,
}

/// Optional per-call overrides for [`Registry::restart`], applied onto the
/// session's own `restart_options()`. `default()` (every field `None`)
/// restarts the session exactly as it was spawned.
#[derive(Debug, Default)]
pub struct RestartOverrides {
    pub tab_name: Option<String>,
    /// Outer `None` keeps the existing group; `Some(None)` sets the
    /// default group; `Some(Some(g))` sets group `g`.
    pub tab_group: Option<Option<String>>,
    pub window_id: Option<String>,
    /// The team-bootstrap orchestrator overrides command + env to flip the
    /// host's pre-existing PTY into the lead's session (e.g. host's shell ->
    /// lead's `claude` command). When `None`, restart preserves the original
    /// spawn command/env.
    pub command: Option<String>,
    pub env: Option<BTreeMap<String, String>>,
}

/// Read-only view of a live terminal session, for the control socket's
/// `cs term list`. The control socket holds a read handle to the
/// `Registry` and renders these grouped by `tab_group`.
#[derive(Debug, Clone)]
pub struct TerminalSessionSummary {
    pub session_id: String,
    pub tab_name: Option<String>,
    /// Resolved group (never empty; `DEFAULT_TERMINAL_GROUP` when unset).
    pub tab_group: String,
    /// The window that owns this session (the `?w=` key), or `None` for a session
    /// created outside a browser window. The control socket resolves it to the
    /// owning window's kind + connected state for `cs term list`.
    pub window_id: Option<String>,
    /// The SPA pane + tab this session was last attached under, threading
    /// window->pane->tab for `cs term list`. Best-effort: `None` until a browser
    /// attaches, and re-bound on split/move.
    pub pane_id: Option<String>,
    pub tab_id: Option<String>,
    pub cwd: Option<PathBuf>,
}

/// One live terminal session in the cross-window roster the SPA reads to
/// render broadcast targets + indicators across every window of a tenant.
/// Unlike [`TerminalSessionSummary`] (the `cs term list` view, grouped by
/// `tab_group` with a live `cwd`), this carries the `window_id` and the
/// per-session `broadcast` toggle and omits the (expensive) cwd lookup: the
/// roster is pushed on every change, so it stays cheap to build. Serialized
/// directly into the `/ws` `terminal_roster` frame and the
/// `GET /api/terminals/roster` seed body.
#[derive(Debug, Clone, Serialize)]
pub struct RosterEntry {
    pub id: String,
    pub tab_name: Option<String>,
    /// Resolved group (never empty; `DEFAULT_TERMINAL_GROUP` when unset),
    /// matching the SPA's `terminalTabGroup` so a group compares equal on
    /// both sides of the wire.
    pub tab_group: String,
    pub window_id: Option<String>,
    /// The session's own broadcast toggle, synced from the SPA via the
    /// `set-broadcast` WS frame. Cross-window input is only fanned to
    /// members with this on (see [`Registry::broadcast_input_cross_window`]).
    pub broadcast: bool,
}

/// Result of enqueuing a `cs terminal write` onto the matched sessions'
/// write queues. `queued` is how many sessions accepted it, `full` how many
/// were already at `WRITE_QUEUE_CAP` (the write was dropped for those), and
/// `position` the queue length after the push when EXACTLY one session
/// matched (the caller's position; `None` for a broadcast or a full single).
#[derive(Debug, Default, Clone, Copy)]
pub struct EnqueueOutcome {
    pub queued: usize,
    pub full: usize,
    pub position: Option<usize>,
}

#[derive(Debug)]
pub enum CreateError {
    Capped,
    FdPressure(FdPressure),
    Spawn(anyhow::Error),
}

impl std::fmt::Display for CreateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CreateError::Capped => f.write_str("terminal session cap reached"),
            CreateError::FdPressure(pressure) => write!(f, "{pressure}"),
            CreateError::Spawn(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for CreateError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FdPressure {
    pub open: u64,
    pub limit: u64,
    pub required: u64,
}

impl std::fmt::Display for FdPressure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "too many open files to start terminal: {}/{} open, need {} fd headroom",
            self.open, self.limit, self.required
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CloseReason {
    Idle,
    Workspace,
    Shutdown,
    Explicit,
    Capped,
}

impl CloseReason {
    pub fn as_str(self) -> &'static str {
        match self {
            CloseReason::Idle => "idle",
            CloseReason::Workspace => "workspace",
            CloseReason::Shutdown => "shutdown",
            CloseReason::Explicit => "explicit",
            CloseReason::Capped => "capped",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum TerminalExit {
    Code { code: u32 },
    Signal { signal: String },
    Unknown,
}

impl TerminalExit {
    fn from_status(status: &portable_pty::ExitStatus) -> Self {
        let rendered = status.to_string();
        if let Some(signal) = rendered.strip_prefix("Terminated by ") {
            return Self::Signal {
                signal: signal.to_string(),
            };
        }
        if rendered == "Success" || rendered.starts_with("Exited with code ") {
            return Self::Code {
                code: status.exit_code(),
            };
        }
        Self::Unknown
    }

    pub fn legacy_code(&self) -> u32 {
        match self {
            Self::Code { code } => *code,
            Self::Signal { .. } | Self::Unknown => 1,
        }
    }
}

impl std::fmt::Display for TerminalExit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Code { code } => write!(f, "status {code}"),
            Self::Signal { signal } => write!(f, "signal {signal}"),
            Self::Unknown => f.write_str("unknown status"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum SessionEvent {
    Output(Vec<u8>),
    Activity {
        bytes_since_focus: u64,
    },
    Resize(PtySize),
    Exit(TerminalExit),
    Error(String),
    Closed(CloseReason),
    /// The session was RESTARTED in place: its PTY is being replaced under the
    /// SAME session id (the roster keeps the id). Broadcast on the OLD
    /// session's channel just before it is killed, so an attached `/ws` reader
    /// re-attaches to the relaunched session instead of tearing the socket
    /// down — the SPA tab stays put and transparently shows the new shell (no
    /// `Closed`/`Exit`, so it is never dropped). Consumed server-side in the
    /// `/ws` loop; never serialized to a client frame.
    Restarted,
    /// The write queue's MESSAGE depth changed (an enqueue on either path,
    /// or a message's tail drained). The depth is the absolute message count
    /// (see `QueuedWrite::tail`), so consumers stay idempotent under
    /// duplicate events and multi-window attaches.
    QueueDepth(usize),
    /// A Rich Prompt message's LAST write reached the PTY. `depth` is the
    /// message depth of the remainder, broadcast just before the matching
    /// `QueueDepth` so a consumer resolving `id` already has the new count.
    PromptDelivered {
        id: String,
        depth: usize,
    },
}

#[derive(Debug)]
pub struct AttachHandle {
    id: String,
    session: Arc<Session>,
    pub rx: broadcast::Receiver<SessionEvent>,
    pub replay: Vec<Vec<u8>>,
    pub seq: u64,
    /// This session incarnation's epoch, sent in the attach prelude so the
    /// client can prove a cached scrollback snapshot belongs to the SAME PTY
    /// life before resuming from a `since` cursor (a restart bumps it).
    pub generation: u64,
    pub missed_bytes: u64,
    pub alt_screen: bool,
    /// Bytes re-asserting the live tracked private-mode set (DECCKM + mouse +
    /// bracketed-paste the foreground program had on) so a fresh client that
    /// came up at defaults regains them. Empty for a plain shell. Sent by the
    /// attach prelude after the alt-screen prelude, before the redraw nudge.
    pub mode_reassert: Vec<u8>,
}

impl AttachHandle {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn send_input(&self, data: &[u8]) {
        self.session.send_input(data);
    }

    /// Enqueue a Rich Prompt message onto this session's `cs terminal write`
    /// FIFO instead of writing it straight to the PTY, so bubble prompts and
    /// CLI pokes share ONE queue + one drain (the drain appends nothing;
    /// `writes` is the ordered `submit_writes` list, chord included). The
    /// whole message is all-or-nothing at the cap. Returns the message depth
    /// after the push (the message's 1-based position), or `None` when the
    /// message does not fit.
    pub fn enqueue_prompt(&self, writes: &[Vec<u8>], prompt_id: Option<String>) -> Option<usize> {
        self.session.enqueue_prompt(writes, prompt_id)
    }

    /// Current MESSAGE depth of this session's write queue (a gemini
    /// text+chord pair counts once), for the `session` frame's depth re-sync
    /// on every (re)attach.
    pub fn queue_depth(&self) -> usize {
        self.session.queue_depth()
    }

    /// Recall a still-queued Rich Prompt message by its `prompt_id`, removing
    /// every queued write that shares it. Returns `true` if it was still
    /// queued (and removed), `false` if it had already drained to the PTY.
    /// Backs the `cancel-prompt` WS frame; the depth re-sync rides the normal
    /// `QueueDepth` broadcast on a successful removal.
    pub fn cancel_prompt(&self, prompt_id: &str) -> bool {
        self.session.cancel_prompt(prompt_id)
    }

    /// The `prompt_id`s of the Rich Prompt messages still queued, in FIFO
    /// order, for the `session` frame so a reattaching SPA can re-prove a
    /// restored pending message is still queued (vs the anonymous depth).
    pub fn queued_prompt_ids(&self) -> Vec<String> {
        self.session.queued_prompt_ids()
    }

    pub fn resize(&self, size: PtySize) {
        self.session.resize(size);
    }

    pub fn set_focused(&self, focused: bool) {
        self.session.set_focused(focused);
    }

    /// Sync this session's broadcast toggle from the SPA. The caller
    /// (`terminal_ws`) follows up with `Registry::notify_roster_change`
    /// so the new state reaches other windows' rosters.
    pub fn set_broadcast(&self, on: bool) {
        self.session.set_broadcast(on);
    }

    pub fn bytes_since_focus(&self) -> u64 {
        self.session.bytes_since_focus()
    }

    pub fn request_redraw(&self) {
        self.session.request_redraw();
    }

    pub fn cwd(&self) -> Option<PathBuf> {
        self.session.cwd()
    }

    /// Like [`cwd`](Self::cwd) but runs the probe (which shells `lsof` on
    /// macOS) on the blocking pool, so an async caller never stalls the
    /// runtime on the PTY's cwd lookup. `None` if the blocking task is
    /// cancelled or the cwd can't be read.
    pub async fn cwd_blocking(&self) -> Option<PathBuf> {
        let session = Arc::clone(&self.session);
        tokio::task::spawn_blocking(move || session.cwd())
            .await
            .ok()
            .flatten()
    }
}

impl Drop for AttachHandle {
    fn drop(&mut self) {
        // On the last client detaching (count 1 -> 0), stamp the detach time so
        // the orphan-grace pruner can age the session from when it went idle,
        // not from its last output byte.
        if self.session.attach_count.fetch_sub(1, Ordering::Relaxed) == 1 {
            self.session
                .detached_at
                .store(now_unix_secs() as i64, Ordering::Relaxed);
        }
    }
}

impl Registry {
    pub fn new(config: RegistryConfig) -> Self {
        Self {
            config,
            sessions: Mutex::new(HashMap::new()),
            last_exit: Arc::new(Mutex::new(None)),
            roster_notify: Arc::new(Notify::new()),
            default_command: Mutex::new(None),
            persisted_windows: Mutex::new(HashSet::new()),
            window_reaper: Mutex::new(None),
            blob_reaper: Mutex::new(None),
            generation_counter: AtomicU64::new(0),
        }
    }

    /// Install the hook that reaps a standalone terminal's WINDOW row when its
    /// session is reaped by [`reap_exited`](Self::reap_exited) (PTY exited and
    /// no client attached). The host wires this on the SHARED terminal tenant
    /// only, so a workspace tenant's pane death never closes its workspace
    /// window. A later install replaces the prior hook.
    pub fn install_window_reaper(&self, reaper: WindowReaper) {
        *self
            .window_reaper
            .lock()
            .expect("terminal registry poisoned") = Some(reaper);
    }

    /// Install the hook that deletes a discarded window's durable terminal layout
    /// blob ([`reap_window_layout`](Self::reap_window_layout)). The host wires
    /// this on the persisted terminal tenant only (the one whose `DELETE
    /// /api/session` routes to `terminal_blob`); ephemeral/control tenants leave
    /// it unset. A later install replaces the prior hook.
    pub fn install_blob_reaper(&self, reaper: BlobReaper) {
        *self.blob_reaper.lock().expect("terminal registry poisoned") = Some(reaper);
    }

    /// Reap the durable terminal layout blob for an EXPLICITLY discarded
    /// `window_id` via the installed [`BlobReaper`]. A no-op when no hook is
    /// installed (workspace / ephemeral / control tenants). The hook is cloned
    /// out under the lock and the (blocking, file-I/O) delete runs after release,
    /// matching the lock discipline of [`session_summaries`](Self::session_summaries).
    pub fn reap_window_layout(&self, window_id: &str) {
        let reaper = self
            .blob_reaper
            .lock()
            .expect("terminal registry poisoned")
            .clone();
        if let Some(reaper) = reaper {
            reaper.call(window_id);
        }
    }

    /// Hand out the next per-tenant default terminal name: the LOWEST-FREE
    /// `Terminal-N` (`N >= 1`) not currently in use by a live session, so a
    /// number freed by a closed terminal is reused (open Terminal-1 +
    /// Terminal-2, close Terminal-2, the next open is Terminal-2 again).
    /// Backs `GET /api/terminal/next-name`. Per-tenant because it scans only
    /// THIS registry's sessions: standalone terminal windows share one
    /// registry; each workspace has its own.
    ///
    /// This only SUGGESTS a name (the session isn't registered until the WS
    /// spawn), so two near-simultaneous calls before either spawns can both
    /// see the same free slot; the frontend `uniqueTerminalName` is the final
    /// tenant-wide dedup that resolves that rare race.
    pub fn next_terminal_name(&self) -> String {
        let taken: HashSet<u64> = {
            let sessions = self.sessions.lock().expect("terminal registry poisoned");
            sessions
                .values()
                .filter(|s| !s.closed.load(Ordering::Relaxed))
                .filter_map(|s| s.tab_name.as_deref().and_then(parse_terminal_ordinal))
                .collect()
        };
        let n = (1u64..)
            .find(|n| !taken.contains(n))
            .expect("the naturals always contain a free slot");
        format!("Terminal-{n}")
    }

    /// A handle to the roster-change signal for the broadcaster task to
    /// await. Cloning the `Arc` is cheap; both the registry and the task
    /// reference the same `Notify`.
    pub fn roster_notify(&self) -> Arc<Notify> {
        self.roster_notify.clone()
    }

    /// Wake the roster broadcaster so it republishes a fresh snapshot.
    /// Called internally on every map mutation (create / close / restart)
    /// and by the terminal WS handler after a `set-broadcast` toggle (a
    /// session-field change the map does not see).
    pub fn notify_roster_change(&self) {
        self.roster_notify.notify_one();
    }

    /// The window that owns a live session, for routing a cross-window
    /// broadcast-toggle command back to the right SPA window. Outer `None`
    /// = no such live session; inner `None` = the session has no owning
    /// window (created outside a browser window, so not remote-controllable).
    pub fn session_window_id(&self, id: &str) -> Option<Option<String>> {
        let sessions = self.sessions.lock().expect("terminal registry poisoned");
        let session = sessions.get(id)?;
        if session.closed.load(Ordering::Relaxed) {
            return None;
        }
        Some(session.window_id())
    }

    /// The live session's current incarnation epoch, or `None` if there is no
    /// such live session. Used by the WS attach path to honor a client `since`
    /// cursor only when the client's cached generation still matches.
    fn session_generation(&self, id: &str) -> Option<u64> {
        let sessions = self.sessions.lock().expect("terminal registry poisoned");
        let session = sessions.get(id)?;
        if session.closed.load(Ordering::Relaxed) {
            return None;
        }
        Some(session.generation())
    }

    /// Snapshot of every live session for the cross-window roster. Mirrors
    /// [`Registry::session_summaries`] but carries `window_id` + the
    /// `broadcast` toggle and skips the per-session cwd probe (the roster
    /// is pushed on every change, so it must stay cheap).
    pub fn roster(&self) -> Vec<RosterEntry> {
        let sessions = self.sessions.lock().expect("terminal registry poisoned");
        sessions
            .values()
            .filter(|session| !session.closed.load(Ordering::Relaxed))
            .map(|session| RosterEntry {
                id: session.id.clone(),
                tab_name: session.tab_name.clone(),
                tab_group: session
                    .tab_group
                    .clone()
                    .unwrap_or_else(|| DEFAULT_TERMINAL_GROUP.to_string()),
                window_id: session.window_id(),
                broadcast: session.broadcast.load(Ordering::Relaxed),
            })
            .collect()
    }

    /// The exit state of any PTY in this registry that has exited, or `None`
    /// while they all run. For the desktop's control-terminal connect flow:
    /// the control tenant runs exactly one PTY (the connect script), so
    /// `Some(exit)` means that script exited — the token will never come, so
    /// the desktop can stop the scrape early (instead of the full timeout) and
    /// survey on a failing connect instead of stranding an empty window.
    /// The registry-level copy is sticky after a session has been removed by
    /// the websocket path.
    pub fn last_exit(&self) -> Option<TerminalExit> {
        if let Some(exit) = self
            .last_exit
            .lock()
            .expect("terminal registry poisoned")
            .clone()
        {
            return Some(exit);
        }
        let sessions = self.sessions.lock().expect("terminal registry poisoned");
        sessions
            .values()
            .find_map(|session| session.exit.lock().expect("session exit poisoned").clone())
    }

    /// Set the command this tenant's terminals run when an open request
    /// carries no command of its own. `None` restores the default shell.
    /// A single-purpose terminal tenant sets this once at creation so its
    /// window's PTY runs a given command (e.g. an interactive connect
    /// script) instead of an interactive shell.
    pub fn set_default_command(&self, command: Option<String>) {
        *self
            .default_command
            .lock()
            .expect("terminal registry poisoned") = command;
    }

    pub fn create(&self, mut opts: CreateOptions) -> Result<AttachHandle, CreateError> {
        // Clear dead-process ghosts before minting: a killed session lingers in
        // the map (its controller thread records `exit` on exit but never
        // reaps the entry), so it would hold its tab name + occupy a
        // `session_cap` slot against a re-spawn under the same name. See
        // [`reap_exited`].
        self.reap_exited();
        // Global pre-spawn gate (fd pressure — an fd_snapshot read_dir): does
        // not need the sessions lock, so run it before taking it, keeping that
        // blocking I/O off the registry lock.
        reject_terminal_spawn_if_fd_pressure()?;
        // Validate the cap + mint the id under the lock, but SPAWN (openpty +
        // fork/exec) OUTSIDE it so a create's PTY launch doesn't stall every
        // other terminal op on the registry mutex.
        let (id, announce_command) = {
            let sessions = self.sessions.lock().expect("terminal registry poisoned");
            if sessions.len() >= self.config.terminal.session_cap {
                return Err(CreateError::Capped);
            }
            // A tenant opened to run a specific command applies it to any
            // session that brings none of its own, so the window's terminal
            // runs the command; an explicit per-session command wins.
            //
            // Only a session that inherits the TENANT's default command
            // (a single-purpose / devserver CONTROL tenant) echoes the bare
            // `{command}\r\n` banner. A per-session command (a team agent
            // terminal spawned via `POST /api/terminals`, or a restart override)
            // is NOT a single-purpose tenant and gets no banner.
            let announce_command = if opts.command.is_none() {
                let default = self
                    .default_command
                    .lock()
                    .expect("terminal registry poisoned")
                    .clone();
                let from_tenant_default = default.is_some();
                opts.command = default;
                from_tenant_default
            } else {
                false
            };
            (self.unused_id(&sessions), announce_command)
        };
        let session = Session::spawn(
            id.clone(),
            self.config.clone(),
            opts,
            announce_command,
            self.generation_counter.fetch_add(1, Ordering::Relaxed),
            self.last_exit.clone(),
        )
        .map_err(CreateError::Spawn)?;
        let mut sessions = self.sessions.lock().expect("terminal registry poisoned");
        // Re-check under the re-acquired lock: a concurrent create may have
        // filled the cap (or — astronomically — taken the random id) while we
        // spawned. If so, reap the orphan PTY before dropping it (no Drop).
        if sessions.len() >= self.config.terminal.session_cap || sessions.contains_key(&id) {
            drop(sessions);
            session.close(CloseReason::Shutdown);
            return Err(CreateError::Capped);
        }
        sessions.insert(id.clone(), session.clone());
        drop(sessions);
        self.notify_roster_change();
        Ok(session.attach(Some(0)))
    }

    pub fn restart(&self, id: &str, overrides: RestartOverrides) -> Result<bool, CreateError> {
        let RestartOverrides {
            tab_name,
            tab_group,
            window_id,
            command,
            env,
        } = overrides;
        let old = self
            .sessions
            .lock()
            .expect("terminal registry poisoned")
            .get(id)
            .cloned();
        let Some(old) = old else {
            return Ok(false);
        };
        if old.closed.load(Ordering::Relaxed) {
            return Ok(false);
        }
        reject_terminal_spawn_if_fd_pressure()?;
        let mut opts = old.restart_options();
        if tab_name.is_some() {
            opts.tab_name = tab_name;
        }
        if let Some(group) = tab_group {
            opts.tab_group = group;
        }
        if window_id.is_some() {
            opts.window_id = window_id;
        }
        // Command/env override semantics: see [`RestartOverrides::command`].
        if let Some(cmd) = command {
            opts.command = Some(cmd);
        }
        if let Some(extra_env) = env {
            opts.env.extend(extra_env);
        }
        // A restart re-runs the command but does NOT re-echo the running banner:
        // the banner names a tenant's launch command (control connect), while a
        // restart override (e.g. the team-bootstrap flip from a host shell to
        // the lead's `claude`) is not a single-purpose-tenant launch.
        let session = Session::spawn(
            id.to_string(),
            self.config.clone(),
            opts,
            false,
            self.generation_counter.fetch_add(1, Ordering::Relaxed),
            self.last_exit.clone(),
        )
        .map_err(CreateError::Spawn)?;
        let mut sessions = self.sessions.lock().expect("terminal registry poisoned");
        match sessions.get(id) {
            Some(current) if Arc::ptr_eq(current, &old) => {
                sessions.insert(id.to_string(), session);
                drop(sessions);
                // Signal an in-place restart (not a close) on the old channel so
                // an attached `/ws` reader re-attaches to the relaunched session
                // under the same id instead of dropping the tab.
                old.close_for_restart();
                self.notify_roster_change();
                Ok(true)
            }
            // A concurrent op replaced or removed the session while we spawned;
            // the freshly-spawned `session` was never inserted, so reap its PTY
            // before dropping it (Session has no Drop) — else the orphan child +
            // fds leak.
            Some(_) | None => {
                session.close(CloseReason::Shutdown);
                Ok(false)
            }
        }
    }

    #[cfg(any(test, feature = "test-util"))]
    pub fn attach(&self, id: &str, since: Option<u64>) -> Option<AttachHandle> {
        self.attach_for_ws(id, since)
    }

    pub fn attach_for_ws(&self, id: &str, since: Option<u64>) -> Option<AttachHandle> {
        let session = self
            .sessions
            .lock()
            .expect("terminal registry poisoned")
            .get(id)
            .cloned()?;
        if session.closed.load(Ordering::Relaxed) {
            return None;
        }
        Some(session.attach(since))
    }

    #[cfg(test)]
    pub fn get_or_create(
        &self,
        id: Option<&str>,
        since: Option<u64>,
        opts: CreateOptions,
    ) -> Result<AttachHandle, CreateError> {
        self.get_or_create_for_ws(id, since, opts, None, None, None)
    }

    pub fn get_or_create_for_ws(
        &self,
        id: Option<&str>,
        since: Option<u64>,
        opts: CreateOptions,
        pane_id: Option<String>,
        tab_id: Option<String>,
        client_generation: Option<u64>,
    ) -> Result<AttachHandle, CreateError> {
        if let Some(id) = id {
            // Honor the client's `since` cursor for a SNAPSHOT RESUME only when
            // its cached generation still matches the live session: a restart
            // reuses the id but resets the ring/`seq` to 0, so a stale cursor
            // would otherwise replay an empty delta with missed=0 (silent
            // desync). A client that is not resuming echoes NO generation -- its
            // `since` (the SPA's `Some(0)`) is honored as sent so a ring
            // overflow still surfaces via `missed_bytes`.
            let effective_since = match client_generation {
                Some(g) if self.session_generation(id) == Some(g) => since,
                // Echoed a generation that no longer matches (e.g. a restart it
                // did not observe): the cached cursor is stale -> full replay.
                Some(_) => None,
                // Not a resume attempt: pass `since` through unchanged.
                None => since,
            };
            if let Some(handle) = self.attach_for_ws(id, effective_since) {
                // Move invariant: re-home the session to the attaching window.
                // A cross-window terminal move re-binds it here, so a later
                // `close_for_window(source)` reaps only sessions still bound to
                // the source — not the one that just moved away.
                self.rebind_session_window(id, opts.window_id.clone());
                self.bind_session_pane_tab(id, pane_id, tab_id);
                return Ok(handle);
            }
        }
        let handle = self.create(opts)?;
        self.bind_session_pane_tab(handle.id(), pane_id, tab_id);
        Ok(handle)
    }

    /// Re-home a live session to `window_id` (the attaching window). No-op for a
    /// windowless (`None`) reattach or a vanished session. See
    /// [`Session::set_window_id`].
    fn rebind_session_window(&self, id: &str, window_id: Option<String>) {
        if window_id.is_none() {
            return;
        }
        if let Some(session) = self
            .sessions
            .lock()
            .expect("terminal registry poisoned")
            .get(id)
        {
            session.set_window_id(window_id);
        }
    }

    /// Record the browser-reported pane + tab placement on a live session, for
    /// `cs term list` window->pane->tab tracing. Sent on every (re)attach; a
    /// `None` on either axis leaves the prior value (a server-spawned session
    /// that never attached has neither). Best-effort -- the ids re-bind on
    /// split/move, so the list shows the last attach's coordinates.
    fn bind_session_pane_tab(&self, id: &str, pane_id: Option<String>, tab_id: Option<String>) {
        if pane_id.is_none() && tab_id.is_none() {
            return;
        }
        if let Some(session) = self
            .sessions
            .lock()
            .expect("terminal registry poisoned")
            .get(id)
        {
            session.set_pane_id(pane_id);
            session.set_tab_id(tab_id);
        }
    }

    pub fn close(&self, id: &str, reason: CloseReason) -> bool {
        let session = self
            .sessions
            .lock()
            .expect("terminal registry poisoned")
            .remove(id);
        if let Some(session) = session {
            session.close(reason);
            self.notify_roster_change();
            true
        } else {
            false
        }
    }

    pub fn remove(&self, id: &str) -> bool {
        let removed = self
            .sessions
            .lock()
            .expect("terminal registry poisoned")
            .remove(id)
            .is_some();
        if removed {
            self.notify_roster_change();
        }
        removed
    }

    /// Record that window `window_id` has a durable saved layout blob, so its
    /// detached terminal sessions are kept alive (reattachable on reconnect)
    /// instead of orphan-reaped. Called on a `PUT /api/session?w=<window_id>`.
    /// Idempotent.
    pub fn mark_window_persisted(&self, window_id: &str) {
        self.persisted_windows
            .lock()
            .expect("terminal registry poisoned")
            .insert(window_id.to_string());
    }

    /// Whether `window_id` is marked persisted (its detached sessions are spared
    /// the orphan-grace reap). The read side of
    /// [`mark_window_persisted`](Self::mark_window_persisted).
    pub fn is_window_persisted(&self, window_id: &str) -> bool {
        self.persisted_windows
            .lock()
            .expect("terminal registry poisoned")
            .contains(window_id)
    }

    /// Close every live session owned by `window_id` (its PTYs are killed and
    /// fds released). Returns how many were closed. The window-scoped sibling
    /// of [`Registry::close`]; the discard primitive behind
    /// [`Registry::forget_window`].
    pub fn close_for_window(&self, window_id: &str, reason: CloseReason) -> usize {
        let ids: Vec<String> = {
            let sessions = self.sessions.lock().expect("terminal registry poisoned");
            sessions
                .iter()
                .filter(|(_, session)| session.window_id().as_deref() == Some(window_id))
                .map(|(id, _)| id.clone())
                .collect()
        };
        let mut closed = 0;
        for id in ids {
            if self.close(&id, reason) {
                closed += 1;
            }
        }
        closed
    }

    /// How many LIVE sessions window `window_id` owns — the read-only twin of
    /// [`close_for_window`](Self::close_for_window), for the `cs window rm`
    /// `--force` guard. Counts only sessions not yet marked closed.
    pub fn count_for_window(&self, window_id: &str) -> usize {
        let sessions = self.sessions.lock().expect("terminal registry poisoned");
        sessions
            .values()
            .filter(|session| !session.closed.load(Ordering::Relaxed))
            .filter(|session| session.window_id().as_deref() == Some(window_id))
            .count()
    }

    /// A window was DISCARDED (its layout blob was DELETEd — `^W` to empty,
    /// `^D`, `Ctrl+Shift+W`, or an empty window). Drop it from the persisted
    /// set and immediately reap its terminal sessions. This is what frees a
    /// busy detached session the idle pruner deliberately keeps alive, and so
    /// is the discard half of "discard ⇒ reap; persist ⇒ keep". Returns how
    /// many sessions were reaped. Called on a `DELETE /api/session?w=<window_id>`.
    pub fn forget_window(&self, window_id: &str) -> usize {
        self.unpersist_window(window_id);
        self.close_for_window(window_id, CloseReason::Explicit)
    }

    /// Drop `window_id` from the persisted set WITHOUT reaping its sessions.
    /// The discard half of a cross-window move-out: the source
    /// window emptied because its tab moved away, so its layout blob is deleted
    /// (it leaves `cs window list`) but the moved PTY must survive; reattach
    /// rebinds it to the target. A move-out DELETE
    /// (`?w=W&moved=1`) routes here; a real discard (`?w=W`) routes through
    /// [`forget_window`](Self::forget_window) and reaps.
    pub fn unpersist_window(&self, window_id: &str) {
        self.persisted_windows
            .lock()
            .expect("terminal registry poisoned")
            .remove(window_id);
    }

    /// Snapshot of every live session, for `cs term list`. The control
    /// socket holds a read handle to the registry and groups these by
    /// `tab_group`. `cwd` is the session's current working directory when
    /// it can be read from the child process.
    pub fn session_summaries(&self) -> Vec<TerminalSessionSummary> {
        // Snapshot the live sessions under the lock, then read each cwd AFTER
        // releasing it. `cwd()` shells `lsof` on macOS, so computing it under
        // the sessions mutex made a multi-session `cs term list` serialize N
        // lsof probes while holding the registry lock — stalling every other
        // terminal op. The snapshot keeps the lock hold to a cheap Arc clone.
        let live: Vec<Arc<Session>> = {
            let sessions = self.sessions.lock().expect("terminal registry poisoned");
            sessions
                .values()
                .filter(|session| !session.closed.load(Ordering::Relaxed))
                .cloned()
                .collect()
        };
        live.into_iter()
            .map(|session| TerminalSessionSummary {
                session_id: session.id.clone(),
                tab_name: session.tab_name.clone(),
                tab_group: session
                    .tab_group
                    .clone()
                    .unwrap_or_else(|| DEFAULT_TERMINAL_GROUP.to_string()),
                window_id: session.window_id(),
                pane_id: session.pane_id(),
                tab_id: session.tab_id(),
                cwd: session.cwd(),
            })
            .collect()
    }

    /// Write raw bytes to the PTY stdin of every live session matching the
    /// given tab name and/or group, for `cs term write`. A `None` filter
    /// matches every session on that axis; passing both narrows to the
    /// intersection. Returns how many sessions were written to. This is the
    /// natural PTY-stdin path, independent of any SPA state.
    pub fn write_input_matching(
        &self,
        tab_name: Option<&str>,
        tab_group: Option<&str>,
        data: &[u8],
    ) -> usize {
        let sessions = self.sessions.lock().expect("terminal registry poisoned");
        let mut written = 0;
        for session in sessions.values() {
            if session.closed.load(Ordering::Relaxed) {
                continue;
            }
            if let Some(name) = tab_name {
                if session.tab_name.as_deref() != Some(name) {
                    continue;
                }
            }
            if let Some(group) = tab_group {
                let resolved = session
                    .tab_group
                    .as_deref()
                    .unwrap_or(DEFAULT_TERMINAL_GROUP);
                if resolved != group {
                    continue;
                }
            }
            session.send_input(data);
            written += 1;
        }
        written
    }

    /// Fan raw input from `source_id` to every OTHER live session in the same
    /// broadcast group whose window differs from the source's. The source PTY
    /// and the source window's broadcast members are handled by the SPA (the
    /// normal `input` frame + the client-side fan, which also respects the
    /// per-member selection); this covers only the cross-window members a
    /// single standalone terminal window's SPA cannot reach, since they live
    /// in this shared registry. Group resolves like `write_input_matching`
    /// (absent = `DEFAULT_TERMINAL_GROUP`).
    pub fn broadcast_input_cross_window(&self, source_id: &str, data: &[u8]) {
        let sessions = self.sessions.lock().expect("terminal registry poisoned");
        let Some(source) = sessions.get(source_id) else {
            return;
        };
        let source_group = source
            .tab_group
            .as_deref()
            .unwrap_or(DEFAULT_TERMINAL_GROUP)
            .to_string();
        let source_window = source.window_id();
        for (id, session) in sessions.iter() {
            if id == source_id || session.closed.load(Ordering::Relaxed) {
                continue;
            }
            let group = session
                .tab_group
                .as_deref()
                .unwrap_or(DEFAULT_TERMINAL_GROUP);
            // Same group, different window: same-window members are fanned
            // client-side, so skip them here to avoid double-delivery.
            if group != source_group || session.window_id() == source_window {
                continue;
            }
            // Respect the receiver's own broadcast toggle (synced via the
            // `set-broadcast` WS frame). Without this the cross-window fan
            // would reach group members with broadcast OFF, unlike the
            // same-window fan which honors the per-member selection. A
            // member that has not opted in does not receive.
            if !session.broadcast.load(Ordering::Relaxed) {
                continue;
            }
            session.send_input(data);
        }
    }

    /// Enqueue `data` onto the write FIFO of every live session matching the
    /// given tab name and/or group, for `cs terminal write`. Same selector
    /// semantics as `write_input_matching` (a `None` axis matches all; both
    /// narrow to the intersection), but the bytes are QUEUED, not written
    /// straight to the PTY: the drainer delivers them one at a time when the
    /// agent is idle, so chained pokes submit one after another. `data`
    /// already carries the caller's submit chord (the CLI appends it). See
    /// [`EnqueueOutcome`] for the return shape.
    pub fn enqueue_write_matching(
        &self,
        tab_name: Option<&str>,
        tab_group: Option<&str>,
        data: &[u8],
    ) -> EnqueueOutcome {
        let sessions = self.sessions.lock().expect("terminal registry poisoned");
        let matched: Vec<&Arc<Session>> = sessions
            .values()
            .filter(|session| !session.closed.load(Ordering::Relaxed))
            .filter(|session| match tab_name {
                Some(name) => session.tab_name.as_deref() == Some(name),
                None => true,
            })
            .filter(|session| match tab_group {
                Some(group) => {
                    session
                        .tab_group
                        .as_deref()
                        .unwrap_or(DEFAULT_TERMINAL_GROUP)
                        == group
                }
                None => true,
            })
            .collect();
        let single = matched.len() == 1;
        let mut outcome = EnqueueOutcome::default();
        for session in matched {
            match session.enqueue_write(data) {
                Some(position) => {
                    outcome.queued += 1;
                    if single {
                        outcome.position = Some(position);
                    }
                }
                None => outcome.full += 1,
            }
        }
        outcome
    }

    /// Full replay-ring snapshots of every live session whose tab name is
    /// `tab_name`, as `(session_id, bytes)`, for `cs terminal scrollback`.
    /// The bytes are the raw PTY stream the WS attach replays (ANSI and
    /// all), so a reader sees exactly what is on screen. There is no group
    /// axis: scrollback targets one terminal, and the control socket
    /// enforces the single-match policy, so this stays a thin selector like
    /// `write_input_matching`.
    pub fn scrollback_matching(&self, tab_name: &str) -> Vec<(String, Vec<u8>)> {
        let sessions = self.sessions.lock().expect("terminal registry poisoned");
        sessions
            .values()
            .filter(|session| !session.closed.load(Ordering::Relaxed))
            .filter(|session| session.tab_name.as_deref() == Some(tab_name))
            .map(|session| (session.id.clone(), session.scrollback()))
            .collect()
    }

    /// Raw replay-ring bytes of every live session in this registry,
    /// concatenated. A standalone terminal tenant typically holds one
    /// session, so this is its full PTY output for a caller that scrapes
    /// it (e.g. reading a connect script's output to find a printed token).
    pub fn all_scrollback(&self) -> Vec<u8> {
        let sessions = self.sessions.lock().expect("terminal registry poisoned");
        let mut out = Vec::new();
        for session in sessions.values() {
            if !session.closed.load(Ordering::Relaxed) {
                out.extend_from_slice(&session.scrollback());
            }
        }
        out
    }

    /// Restart every live session matching the given tab name and/or
    /// group, for `cs terminal restart`. Same selector semantics as
    /// `write_input_matching` (a `None` axis matches all; both narrow to
    /// the intersection). Returns how many sessions were restarted.
    ///
    /// Passing `None` for every `restart()` override preserves each
    /// session's spawn command + env, so a session launched with an agent
    /// startup command relaunches that agent. This is the out-of-band
    /// server path the Team Work self-restart needs: a shell cannot
    /// restart the very shell running its own bootstrap script, but the
    /// server can. Ids are collected under the lock and restarted after it
    /// is dropped, since `restart()` re-locks the registry internally.
    pub fn restart_matching(
        &self,
        tab_name: Option<&str>,
        tab_group: Option<&str>,
    ) -> Result<usize, CreateError> {
        let ids: Vec<String> = {
            let sessions = self.sessions.lock().expect("terminal registry poisoned");
            sessions
                .values()
                .filter(|session| !session.closed.load(Ordering::Relaxed))
                .filter(|session| match tab_name {
                    Some(name) => session.tab_name.as_deref() == Some(name),
                    None => true,
                })
                .filter(|session| match tab_group {
                    Some(group) => {
                        session
                            .tab_group
                            .as_deref()
                            .unwrap_or(DEFAULT_TERMINAL_GROUP)
                            == group
                    }
                    None => true,
                })
                .map(|session| session.id.clone())
                .collect()
        };
        let mut restarted = 0;
        for id in &ids {
            if self.restart(id, RestartOverrides::default())? {
                restarted += 1;
            }
        }
        Ok(restarted)
    }

    /// Close every live session matching the given tab name and/or group, for
    /// `cs terminal close`. Same selector semantics as `restart_matching` (a
    /// `None` axis matches all; both narrow to the intersection). Closes the
    /// PTY and removes the registry entry — the explicit teardown that was
    /// missing (killing the pid out-of-band left the entry to linger and hold
    /// its tab name). Returns how many sessions were closed.
    pub fn close_matching(&self, tab_name: Option<&str>, tab_group: Option<&str>) -> usize {
        let ids: Vec<String> = {
            let sessions = self.sessions.lock().expect("terminal registry poisoned");
            sessions
                .values()
                .filter(|session| !session.closed.load(Ordering::Relaxed))
                .filter(|session| match tab_name {
                    Some(name) => session.tab_name.as_deref() == Some(name),
                    None => true,
                })
                .filter(|session| match tab_group {
                    Some(group) => {
                        session
                            .tab_group
                            .as_deref()
                            .unwrap_or(DEFAULT_TERMINAL_GROUP)
                            == group
                    }
                    None => true,
                })
                .map(|session| session.id.clone())
                .collect()
        };
        let mut closed = 0;
        for id in &ids {
            if self.close(id, CloseReason::Explicit) {
                closed += 1;
            }
        }
        closed
    }

    /// The DISTINCT window ids that own a live session matching the given
    /// tab name and/or group, for `cs terminal survey`. Same selector
    /// semantics as `write_input_matching` (a `None` axis matches all; both
    /// narrow to the intersection). A survey overlay is an SPA-window
    /// affordance, not a PTY one, so the survey transport resolves the tab
    /// selector to the window(s) hosting those tabs and pushes the overlay
    /// there. Sessions with no `window_id` (rare: a session created outside
    /// a browser window) contribute nothing. Order is unspecified; callers
    /// fan the overlay out to each.
    pub fn window_ids_matching(
        &self,
        tab_name: Option<&str>,
        tab_group: Option<&str>,
    ) -> Vec<String> {
        let sessions = self.sessions.lock().expect("terminal registry poisoned");
        let mut seen = std::collections::HashSet::new();
        let mut out = Vec::new();
        for session in sessions.values() {
            if session.closed.load(Ordering::Relaxed) {
                continue;
            }
            if let Some(name) = tab_name {
                if session.tab_name.as_deref() != Some(name) {
                    continue;
                }
            }
            if let Some(group) = tab_group {
                let resolved = session
                    .tab_group
                    .as_deref()
                    .unwrap_or(DEFAULT_TERMINAL_GROUP);
                if resolved != group {
                    continue;
                }
            }
            if let Some(window_id) = session.window_id() {
                if seen.insert(window_id.clone()) {
                    out.push(window_id);
                }
            }
        }
        out
    }

    pub fn close_all(&self, reason: CloseReason) {
        let sessions: Vec<Arc<Session>> = self
            .sessions
            .lock()
            .expect("terminal registry poisoned")
            .drain()
            .map(|(_, session)| session)
            .collect();
        for session in sessions {
            session.close(reason);
        }
        self.notify_roster_change();
    }

    /// Reap sessions whose child PROCESS has exited and that have no client
    /// attached. A dead, unviewed session is a pure ghost — no process, no
    /// viewer — so keeping it only leaks the slot and HOLDS its tab name,
    /// making a re-spawn under the same name collide and come up renamed (the
    /// `cs terminal restart` ghost-tab regression: a killed agent's entry lingered
    /// because the controller thread records `exit` on exit but never
    /// removes the entry). Distinct axis from `prune_idle_at`, which times
    /// out *live* detached sessions and deliberately keeps persisted windows:
    /// a dead process can't be reattached, only re-spawned, so a persisted
    /// window comes back fresh on reconnect rather than stranding the ghost.
    /// An attached dead session is KEPT (a client is still viewing its final
    /// output — no natural-`exit`-vanishes regression). Returns how many were
    /// reaped. Run before every [`create`](Self::create) and on the pruner tick.
    pub fn reap_exited(&self) -> usize {
        // Capture each reaped session's owning window_id alongside its id: a
        // standalone terminal window IS its session, so reaping the session must
        // also drop the window-feed row, and `close` removes the session
        // before we could read it back.
        let to_reap: Vec<(String, Option<String>)> = {
            let sessions = self.sessions.lock().expect("terminal registry poisoned");
            sessions
                .iter()
                .filter(|(_, session)| {
                    session.attach_count.load(Ordering::Relaxed) == 0
                        && session
                            .exit
                            .lock()
                            .expect("session exit poisoned")
                            .is_some()
                })
                .map(|(id, session)| (id.clone(), session.window_id()))
                .collect()
        };
        let reaper = self
            .window_reaper
            .lock()
            .expect("terminal registry poisoned")
            .clone();
        let mut reaped = 0;
        for (id, window_id) in &to_reap {
            if self.close(id, CloseReason::Explicit) {
                reaped += 1;
                // The shared terminal tenant's hook drops the window-feed row +
                // refreshes the feed. No-op on a workspace / control window
                // (the host scopes it; the row guard double-checks the kind).
                if let (Some(window_id), Some(reaper)) = (window_id, reaper.as_ref()) {
                    reaper.call(window_id);
                }
            }
        }
        reaped
    }

    pub fn prune_idle(&self) -> usize {
        self.prune_idle_at(now_unix_secs() as i64)
    }

    /// Reap sessions whose window can never come back. Persistence-driven, NOT
    /// activity-driven (a busy detached session refreshes `last_activity` on
    /// every output byte, so the old activity timer kept htop / a `for` loop
    /// immortal). The rule:
    ///
    /// - **attached** (`attach_count > 0`) — keep; a client is live on it.
    /// - **detached, window persisted** (a durable layout blob exists, tracked
    ///   in `persisted_windows`) — keep indefinitely; the window survives a
    ///   client disconnect and reattaches on reconnect (browser-tab / devserver
    ///   semantics). Discard reaps it explicitly via [`Registry::forget_window`].
    /// - **detached, window NOT persisted** (browser window that never saved a
    ///   blob — a hard client crash before any save) — orphan; reap once it has
    ///   been detached longer than the grace.
    /// - **detached, no `window_id`** (a headless `cs terminal new` from a
    ///   native terminal) — unchanged activity-idle cleanup, timed off
    ///   `last_activity`; these are intentional, not browser-window orphans.
    ///
    /// The detach/idle grace reuses `terminal.idle_timeout_secs`.
    pub fn prune_idle_at(&self, now: i64) -> usize {
        let idle_timeout = self.config.terminal.idle_timeout_secs as i64;
        let persisted = self
            .persisted_windows
            .lock()
            .expect("terminal registry poisoned")
            .clone();
        let to_close: Vec<String> = {
            let sessions = self.sessions.lock().expect("terminal registry poisoned");
            sessions
                .iter()
                .filter_map(|(id, session)| {
                    if session.attach_count.load(Ordering::Relaxed) != 0 {
                        return None; // a client is attached
                    }
                    match session.window_id() {
                        // Persisted window: kept until an explicit discard.
                        Some(window_id) if persisted.contains(&window_id) => None,
                        // Browser window with no durable blob: orphan-grace from
                        // when it last went detached.
                        Some(_) => {
                            let detached = session.detached_at.load(Ordering::Relaxed);
                            (now.saturating_sub(detached) > idle_timeout).then(|| id.clone())
                        }
                        // Headless / control terminal: legacy activity-idle.
                        None => {
                            let last = session.last_activity.load(Ordering::Relaxed);
                            (now.saturating_sub(last) > idle_timeout).then(|| id.clone())
                        }
                    }
                })
                .collect()
        };
        let n = to_close.len();
        for id in to_close {
            self.close(&id, CloseReason::Idle);
        }
        n
    }

    pub fn spawn_pruner(self: Arc<Self>, mut shutdown_rx: watch::Receiver<bool>) -> JoinHandle<()> {
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(Duration::from_secs(60));
            loop {
                tokio::select! {
                    _ = shutdown_rx.changed() => {
                        self.close_all(CloseReason::Shutdown);
                        break;
                    }
                    _ = tick.tick() => {
                        self.reap_exited();
                        self.prune_idle();
                    }
                }
            }
        })
    }

    /// One drain pass over every live session's `cs terminal write` queue.
    /// Snapshots the session Arcs under the lock, then drains each outside it
    /// (delivery touches the session's own queue + PTY, never the registry
    /// map). A no-op for sessions with an empty queue or a busy agent.
    pub fn drain_writes(&self) {
        let now = now_unix_millis();
        let sessions: Vec<Arc<Session>> = self
            .sessions
            .lock()
            .expect("terminal registry poisoned")
            .values()
            .filter(|session| !session.closed.load(Ordering::Relaxed))
            .cloned()
            .collect();
        for session in sessions {
            session.try_drain_one(now);
        }
    }

    /// The write-queue drainer: ticks every `WRITE_QUEUE_DRAIN_TICK` and
    /// delivers each session's next queued write once its agent is idle. A
    /// sibling of `spawn_pruner` (own task, shuts down on the same signal).
    pub fn spawn_drainer(
        self: Arc<Self>,
        mut shutdown_rx: watch::Receiver<bool>,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(WRITE_QUEUE_DRAIN_TICK);
            loop {
                tokio::select! {
                    _ = shutdown_rx.changed() => break,
                    _ = tick.tick() => {
                        self.drain_writes();
                    }
                }
            }
        })
    }

    fn unused_id(&self, sessions: &HashMap<String, Arc<Session>>) -> String {
        loop {
            let id = random_session_id();
            if !sessions.contains_key(&id) {
                return id;
            }
        }
    }

    #[cfg(any(test, feature = "test-util"))]
    pub fn len(&self) -> usize {
        self.sessions
            .lock()
            .expect("terminal registry poisoned")
            .len()
    }

    #[cfg(any(test, feature = "test-util"))]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Drop for Registry {
    fn drop(&mut self) {
        if let Ok(mut sessions) = self.sessions.lock() {
            for (_, session) in sessions.drain() {
                session.close(CloseReason::Shutdown);
            }
        }
    }
}

/// One entry in a session's write FIFO. A `cs terminal write` poke enqueues
/// a single untagged tail entry; a Rich Prompt submit enqueues every write
/// `submit_writes` produced (two for gemini, one otherwise) under one
/// `prompt_id`, all-or-nothing.
#[derive(Debug)]
struct QueuedWrite {
    /// Raw PTY bytes (the submit chord, if any, is already appended).
    data: Vec<u8>,
    /// Rich Prompt message id (`None` for `cs terminal write` pokes). Tagged
    /// on EVERY write of the message, not just the tail, so a future
    /// cancel-by-id is a pure retain-filter (documented v2).
    prompt_id: Option<String>,
    /// True on a message's FINAL write (every single-write message, and the
    /// gemini chord). Depth counts tails; `PromptDelivered` fires on a
    /// tagged tail's drain.
    tail: bool,
}

/// Message depth of a write queue: the count of TAIL entries. A multi-write
/// message contributes exactly one tail, so this counts messages, not raw
/// writes — a queued gemini text+chord pair reads as ONE pending message.
fn msg_depth(q: &VecDeque<QueuedWrite>) -> usize {
    q.iter().filter(|w| w.tail).count()
}

#[derive(Debug)]
struct Session {
    id: String,
    tab_name: Option<String>,
    tab_group: Option<String>,
    /// The window this session currently belongs to (the `?w=` label). Interior
    /// mutable because a reattach REBINDS it to the attaching window: a
    /// cross-window terminal move re-homes the session, so a later
    /// `close_for_window(source)` reaps only sessions STILL bound to the source
    /// Read via [`Session::window_id`].
    window_id: Mutex<Option<String>>,
    /// The SPA layout coordinates -- pane id + tab id -- this session was last
    /// attached under, reported by the browser on each (re)attach. Interior
    /// mutable and best-effort: they re-bind on split/move and stay `None` for
    /// a session that never attached from a browser (e.g. `cs terminal new`).
    /// Read via [`Session::pane_id`] / [`Session::tab_id`] for `cs term list`.
    pane_id: Mutex<Option<String>>,
    tab_id: Mutex<Option<String>>,
    /// Per-PTY-life epoch stamped at spawn (see [`Registry::generation_counter`]).
    /// A restart mints a new session under the same id with this bumped and the
    /// ring/`seq` reset, so a client compares it before trusting a `since` cursor.
    generation: u64,
    workspace_root: PathBuf,
    spawn_opts: CreateOptions,
    child_pid: Option<u32>,
    command_tx: std::sync::mpsc::Sender<PtyCommand>,
    output_tx: broadcast::Sender<SessionEvent>,
    ring: Mutex<RingBuffer>,
    seq: AtomicU64,
    last_activity: AtomicI64,
    /// Wall-clock millis of the most recent OUTPUT byte (the agent
    /// rendering / generating), distinct from `last_activity` (which also
    /// bumps on input). The `cs terminal write` queue drains only when this
    /// has been quiet for `WRITE_QUEUE_QUIET_MS` (the agent is idle).
    last_output_at: AtomicI64,
    /// FIFO of pending writes for this session — `cs terminal write` pokes
    /// and Rich Prompt messages share it — drained one entry at a time when
    /// the agent is idle. Each entry carries raw PTY bytes plus message
    /// tagging (see [`QueuedWrite`]). Bounded at `WRITE_QUEUE_CAP` raw
    /// entries; dropped on session recycle (the session, and this queue with
    /// it, is replaced on restart/close — attached clients get Closed/Exit
    /// and re-sync their queue depth from the next attach's session frame).
    write_queue: Mutex<VecDeque<QueuedWrite>>,
    /// Millis of the drainer's last delivery (0 when nothing is pending), to
    /// time the await-generation-start window after a deliver.
    last_deliver_at: AtomicI64,
    /// True between a delivery and the agent's generation-START (or the cap),
    /// so the next queued message does not fire into the same compose.
    awaiting_gen: AtomicBool,
    attach_count: AtomicUsize,
    /// Unix seconds when `attach_count` last fell to 0 (every client detached).
    /// Seeded at spawn. The orphan-grace pruner times a detached session from
    /// THIS, not `last_activity` — a busy detached session (htop, a `for` loop)
    /// keeps `last_activity` fresh forever, so timing the grace off output kept
    /// it immortal (the FD leak). Meaningless while `attach_count > 0`.
    detached_at: AtomicI64,
    winsize: Mutex<PtySize>,
    focused: AtomicBool,
    bytes_since_focus: AtomicU64,
    in_alt_screen: AtomicBool,
    alt_screen_tail: Mutex<Vec<u8>>,
    /// The [`TRACKED_PRIVATE_MODES`] the live program currently has ON, scanned
    /// from PTY output by [`Session::update_private_modes`] and re-asserted on
    /// reattach (see [`Session::attach`]) so a fresh client whose terminal came
    /// up at defaults regains the program's key/mouse modes.
    private_modes: Mutex<BTreeSet<u16>>,
    /// Carry for a private-mode CSI split across PTY reads (mirrors
    /// `alt_screen_tail`, bounded by [`PRIVATE_MODE_TAIL_CAP`]).
    private_mode_tail: Mutex<Vec<u8>>,
    /// This session's broadcast toggle, synced from the SPA via the
    /// `set-broadcast` WS frame on toggle and on (re)connect. Gates the
    /// cross-window input fan (see `broadcast_input_cross_window`) and is
    /// surfaced in the roster so other windows can render the broadcast
    /// state of members they do not host.
    broadcast: AtomicBool,
    closed: AtomicBool,
    /// The PTY's exit state, set once its child process exits (the same value
    /// broadcast as [`SessionEvent::Exit`]). `None` while the process runs.
    /// Stored — not only broadcast — so a poller (the desktop's control-script
    /// scrape) can see the script died without subscribing to the event
    /// stream. Retained on the still-mapped session after a natural exit.
    exit: Mutex<Option<TerminalExit>>,
}

impl Session {
    fn spawn(
        id: String,
        config: RegistryConfig,
        opts: CreateOptions,
        announce_command: bool,
        generation: u64,
        registry_last_exit: Arc<Mutex<Option<TerminalExit>>>,
    ) -> anyhow::Result<Arc<Self>> {
        let pty_system = native_pty_system();
        let pair = pty_system.openpty(opts.size)?;
        let mut cmd = command_builder(opts.command.as_deref());
        let cwd = opts.cwd.unwrap_or_else(|| config.workspace_root.clone());
        cmd.cwd(&cwd);
        for (key, value) in &opts.env {
            cmd.env(key, value);
        }
        if let Some(home) = terminal_home_dir() {
            cmd.env("HOME", &home);
            #[cfg(windows)]
            cmd.env("USERPROFILE", home);
        }
        // Windows: the terminal shell is Git BASH (a hard dependency). Prepend
        // Prepend the chan bin dir (`%LOCALAPPDATA%\chan\bin`) so the `chan` /
        // `cs` shims resolve. The shim dir is only ever added to the HKCU PATH
        // registry by `cs_install::ensure_on_user_path`, which never reaches
        // this already-running process's inherited env — so prepend it here,
        // independent of registry propagation. Must match `cs_install`'s
        // `shim_bin_dir` (`dirs::data_local_dir().join("chan").join("bin")`).
        // Layered over any per-session PATH override, then the inherited PATH.
        #[cfg(windows)]
        {
            let mut prepend: Vec<PathBuf> = Vec::new();
            if let Some(local) = dirs::data_local_dir() {
                prepend.push(local.join("chan").join("bin"));
            }
            if !prepend.is_empty() {
                let inherited = opts
                    .env
                    .get("PATH")
                    .cloned()
                    .or_else(|| std::env::var("PATH").ok())
                    .unwrap_or_default();
                prepend.extend(std::env::split_paths(&inherited));
                if let Ok(joined) = std::env::join_paths(prepend) {
                    cmd.env("PATH", joined);
                }
            }
        }
        // Spawn-time TERM comes from settings. The value lives in
        // `TerminalConfig::default_term`; the SPA can
        // override the default via the Settings panel, and the change
        // takes effect on newly-spawned terminals (existing PTYs keep
        // whatever TERM they were started with).
        cmd.env("TERM", config.terminal.default_term.as_str());
        cmd.env("COLORTERM", "truecolor");
        cmd.env("CLICOLOR", "1");
        cmd.env("CLICOLOR_FORCE", "1");
        cmd.env("FORCE_COLOR", "3");
        // GUI-launched servers (notably chan-desktop on macOS) frequently
        // inherit an empty locale, so `less` and `vim` fall back to the
        // POSIX/C codeset and render multibyte UTF-8 (e.g. an em dash) as raw
        // bytes. Provide a language-neutral UTF-8 default when nothing already
        // selects one, and drop any non-UTF-8 LC_ALL/LC_CTYPE so the LANG
        // default actually controls the codeset (the user's shell profile can
        // still re-export LANG). C.UTF-8 is present on macOS, every musl Linux
        // build, and glibc >= 2.35 / Debian / Ubuntu / RHEL 8+.
        if !locale_selects_utf8(&opts.env) {
            cmd.env("LANG", "C.UTF-8");
            cmd.env_remove("LC_ALL");
            cmd.env_remove("LC_CTYPE");
        }
        cmd.env("CHAN", "1");
        clear_mcp_env(&mut cmd);
        if opts.mcp_env {
            if let Some(socket_path) = config.mcp_socket_path.as_deref() {
                set_mcp_env(&mut cmd, socket_path);
            }
        }
        let tab_name = opts.tab_name;
        if let Some(tab_name) = tab_name.as_deref() {
            cmd.env("CHAN_TAB_NAME", tab_name);
        }
        // Every terminal has a well-defined group, so $CHAN_TAB_GROUP is
        // always set (default when unset) — an agent can read it
        // unconditionally to learn its broadcast group.
        let tab_group = opts.tab_group;
        cmd.env(
            "CHAN_TAB_GROUP",
            tab_group.as_deref().unwrap_or(DEFAULT_TERMINAL_GROUP),
        );
        let window_id = opts.window_id;
        if let Some(window_id) = window_id.as_deref() {
            cmd.env("CHAN_WINDOW_ID", window_id);
        }
        if let Some(socket_path) = config.control_socket_path.as_deref() {
            if let Some(socket) = socket_path.to_str() {
                cmd.env("CHAN_CONTROL_SOCKET", socket);
            }
        }
        // Served-workspace identity for the terminal and any agents it spawns.
        // No user-managed workspace name exists; the label derives from the root
        // path basename, matching how the UI labels a workspace.
        let workspace_path = config.workspace_root.to_string_lossy();
        cmd.env("CHAN_WORKSPACE_PATH", workspace_path.as_ref());
        let workspace_name = config
            .workspace_root
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| workspace_path.into_owned());
        cmd.env("CHAN_WORKSPACE_NAME", &workspace_name);
        cmd.env_remove("NO_COLOR");
        cmd.env_remove("CI");
        cmd.env_remove("CODEX_CI");

        let mut child = pair.slave.spawn_command(cmd)?;
        let child_pid = child.process_id();
        drop(pair.slave);

        let mut reader = pair.master.try_clone_reader()?;
        let mut writer = pair.master.take_writer()?;
        let mut killer = child.clone_killer();
        let (command_tx, command_rx) = std::sync::mpsc::channel::<PtyCommand>();
        let (output_tx, _) = broadcast::channel::<SessionEvent>(BROADCAST_CAP);
        let session = Arc::new(Self {
            id,
            tab_name,
            tab_group,
            window_id: Mutex::new(window_id),
            pane_id: Mutex::new(None),
            tab_id: Mutex::new(None),
            generation,
            workspace_root: config.workspace_root.clone(),
            spawn_opts: CreateOptions {
                size: opts.size,
                tab_name: None,
                tab_group: None,
                window_id: None,
                mcp_env: opts.mcp_env,
                cwd: Some(cwd),
                command: opts.command,
                env: opts.env,
            },
            child_pid,
            command_tx,
            output_tx,
            ring: Mutex::new(RingBuffer::new(config.terminal.ring_bytes)),
            seq: AtomicU64::new(0),
            last_activity: AtomicI64::new(now_unix_secs() as i64),
            // Seed output-idle at spawn time so a brand-new session is not
            // treated as instantly idle before it has rendered anything.
            last_output_at: AtomicI64::new(now_unix_millis()),
            write_queue: Mutex::new(VecDeque::new()),
            last_deliver_at: AtomicI64::new(0),
            awaiting_gen: AtomicBool::new(false),
            attach_count: AtomicUsize::new(0),
            detached_at: AtomicI64::new(now_unix_secs() as i64),
            winsize: Mutex::new(opts.size),
            focused: AtomicBool::new(false),
            bytes_since_focus: AtomicU64::new(0),
            in_alt_screen: AtomicBool::new(false),
            alt_screen_tail: Mutex::new(Vec::new()),
            private_modes: Mutex::new(BTreeSet::new()),
            private_mode_tail: Mutex::new(Vec::new()),
            broadcast: AtomicBool::new(false),
            closed: AtomicBool::new(false),
            exit: Mutex::new(None),
        });

        // A single-purpose / devserver CONTROL tenant echoes a banner naming
        // the command it is about to run, so the user sees the launch command
        // before its output. Recorded into the replay ring HERE — after the
        // session exists but BEFORE the reader thread starts — so the banner is
        // the first ring bytes (precedes the child's output) and survives
        // scrollback replay on reload. Display-only: the executed command
        // (`command_builder` above) is untouched; this never wraps or re-quotes
        // it.
        if announce_command {
            if let Some(command) = session.spawn_opts.command.as_deref() {
                session.record_output(format!("{command}\r\n").as_bytes());
            }
        }

        {
            let session = session.clone();
            std::thread::Builder::new()
                .name("chan-terminal-reader".into())
                .spawn(move || {
                    let mut buf = [0u8; 8192];
                    loop {
                        match reader.read(&mut buf) {
                            Ok(0) => break,
                            Ok(n) => session.record_output(&buf[..n]),
                            Err(e) => {
                                session.broadcast(SessionEvent::Error(format!(
                                    "terminal read failed: {e}"
                                )));
                                break;
                            }
                        }
                    }
                })?;
        }

        {
            let session = session.clone();
            std::thread::Builder::new()
                .name("chan-terminal-controller".into())
                .spawn(move || loop {
                    while let Ok(cmd) = command_rx.try_recv() {
                        match cmd {
                            PtyCommand::Input(data) => {
                                if let Err(e) = writer.write_all(&data) {
                                    session.broadcast(SessionEvent::Error(format!(
                                        "terminal write failed: {e}"
                                    )));
                                    let _ = killer.kill();
                                    return;
                                }
                                let _ = writer.flush();
                            }
                            PtyCommand::Resize(size) => {
                                if let Err(e) = pair.master.resize(size) {
                                    session.broadcast(SessionEvent::Error(format!(
                                        "terminal resize failed: {e}"
                                    )));
                                } else {
                                    *session.winsize.lock().expect("terminal winsize poisoned") =
                                        size;
                                    session.broadcast(SessionEvent::Resize(size));
                                }
                            }
                            PtyCommand::Redraw => {
                                let size =
                                    *session.winsize.lock().expect("terminal winsize poisoned");
                                let result =
                                    force_redraw_with_wobble(size, REDRAW_WOBBLE_DELAY, |size| {
                                        pair.master.resize(size)
                                    });
                                if let Err(e) = result {
                                    session.broadcast(SessionEvent::Error(format!(
                                        "terminal redraw resize failed: {e}"
                                    )));
                                } else {
                                    session.broadcast(SessionEvent::Resize(size));
                                }
                            }
                            PtyCommand::Kill => {
                                let _ = killer.kill();
                                return;
                            }
                        }
                    }

                    match child.try_wait() {
                        Ok(Some(status)) => {
                            let exit = TerminalExit::from_status(&status);
                            // Record before broadcasting so a poller that reads
                            // the registry right after the event still sees it.
                            *session.exit.lock().expect("session exit poisoned") =
                                Some(exit.clone());
                            *registry_last_exit
                                .lock()
                                .expect("terminal registry poisoned") = Some(exit.clone());
                            session.broadcast(SessionEvent::Exit(exit));
                            return;
                        }
                        Ok(None) => std::thread::sleep(Duration::from_millis(25)),
                        Err(e) => {
                            session.broadcast(SessionEvent::Error(format!(
                                "terminal wait failed: {e}"
                            )));
                            return;
                        }
                    }
                })?;
        }

        Ok(session)
    }

    fn attach(self: Arc<Self>, since: Option<u64>) -> AttachHandle {
        self.attach_count.fetch_add(1, Ordering::Relaxed);
        let rx = self.output_tx.subscribe();
        let alt_screen = self.in_alt_screen.load(Ordering::Relaxed);
        let (replay, missed_bytes) = if alt_screen {
            (Vec::new(), 0)
        } else {
            self.ring
                .lock()
                .expect("terminal ring poisoned")
                .snapshot_since(since)
        };
        let seq = self.seq.load(Ordering::Relaxed);
        let generation = self.generation;
        let mode_reassert = self.private_mode_prelude();
        AttachHandle {
            id: self.id.clone(),
            session: self,
            rx,
            replay,
            seq,
            generation,
            missed_bytes,
            alt_screen,
            mode_reassert,
        }
    }

    fn send_input(&self, data: &[u8]) {
        self.last_activity
            .store(now_unix_secs() as i64, Ordering::Relaxed);
        let _ = self.command_tx.send(PtyCommand::Input(data.to_vec()));
    }

    /// One drainer step for this session's `cs terminal write` queue. Deliver
    /// the next queued message ONLY when the agent is idle (output quiesced),
    /// and after a delivery AWAIT the agent's generation-START before the
    /// next, so chained pokes submit one after another instead of stacking
    /// into one compose. Called on each drainer tick with the current millis;
    /// a no-op when the queue is empty or the agent is still busy.
    fn try_drain_one(&self, now_ms: i64) {
        if self
            .write_queue
            .lock()
            .expect("terminal write queue poisoned")
            .is_empty()
        {
            // Nothing pending: clear the post-deliver await state so the next
            // enqueue starts clean.
            self.last_deliver_at.store(0, Ordering::Relaxed);
            self.awaiting_gen.store(false, Ordering::Relaxed);
            return;
        }
        let last_output = self.last_output_at.load(Ordering::Relaxed);
        // After a deliver, hold the next message until the agent's generation
        // has STARTED (output advanced past the delivery) or the cap elapses
        // (the message did not trigger generation), so two messages never
        // fire into one compose in the post-submit, pre-generation window.
        if self.awaiting_gen.load(Ordering::Relaxed) {
            let delivered_at = self.last_deliver_at.load(Ordering::Relaxed);
            let generation_started = last_output > delivered_at;
            let timed_out = now_ms - delivered_at >= WRITE_QUEUE_GEN_START_CAP_MS;
            if generation_started || timed_out {
                self.awaiting_gen.store(false, Ordering::Relaxed);
            } else {
                return;
            }
        }
        // Deliver only once the agent is idle (the previous turn, if any, has
        // quiesced).
        if now_ms - last_output < WRITE_QUEUE_QUIET_MS {
            return;
        }
        let next = {
            let mut q = self
                .write_queue
                .lock()
                .expect("terminal write queue poisoned");
            // Capture the remainder's message depth under the same lock so
            // the broadcast below (outside the guard) carries a count that
            // matches exactly this pop.
            q.pop_front().map(|write| (write, msg_depth(&q)))
        };
        if let Some((write, depth)) = next {
            self.send_input(&write.data);
            self.last_deliver_at.store(now_ms, Ordering::Relaxed);
            self.awaiting_gen.store(true, Ordering::Relaxed);
            // Only a TAIL drain completes a message; a gemini body drain
            // leaves its message pending until the chord lands, so it emits
            // nothing (the message depth did not change).
            if write.tail {
                if let Some(id) = write.prompt_id {
                    self.broadcast(SessionEvent::PromptDelivered { id, depth });
                }
                self.broadcast(SessionEvent::QueueDepth(depth));
            }
        }
    }

    /// Push a `cs terminal write` payload onto this session's FIFO as one
    /// untagged single-write message. Returns the RAW queue length after the
    /// push (the caller's position), or `None` when the queue is already at
    /// `WRITE_QUEUE_CAP` (the write is dropped). The return value is raw
    /// entries while the SPA's queue depth counts messages — a deliberate
    /// divergence that keeps the CLI's stdout contract byte-for-byte stable.
    fn enqueue_write(&self, data: &[u8]) -> Option<usize> {
        let (len, depth) = {
            let mut q = self
                .write_queue
                .lock()
                .expect("terminal write queue poisoned");
            if q.len() >= WRITE_QUEUE_CAP {
                return None;
            }
            q.push_back(QueuedWrite {
                data: data.to_vec(),
                prompt_id: None,
                tail: true,
            });
            (q.len(), msg_depth(&q))
        };
        // Outside the QUEUE guard. The enqueue_write_matching caller does
        // hold the REGISTRY guard here, which is fine: broadcast::send is
        // sync, takes only the channel's internal lock, and nothing it
        // wakes can re-enter the registry synchronously.
        self.broadcast(SessionEvent::QueueDepth(depth));
        Some(len)
    }

    /// Push a Rich Prompt message onto this session's FIFO. `writes` is the
    /// ordered `submit_writes` list (two entries for gemini, one otherwise),
    /// enqueued as ONE message: all-or-nothing at the cap (a partial push
    /// could deliver a body whose submit chord was silently dropped),
    /// `prompt_id` on every entry, `tail` on the last. Returns the message
    /// depth after the push — the message's 1-based queue position — or
    /// `None` when the whole message does not fit (queue unchanged).
    fn enqueue_prompt(&self, writes: &[Vec<u8>], prompt_id: Option<String>) -> Option<usize> {
        let depth = {
            let mut q = self
                .write_queue
                .lock()
                .expect("terminal write queue poisoned");
            if q.len() + writes.len() > WRITE_QUEUE_CAP {
                return None;
            }
            for (i, data) in writes.iter().enumerate() {
                q.push_back(QueuedWrite {
                    data: data.clone(),
                    prompt_id: prompt_id.clone(),
                    tail: i == writes.len() - 1,
                });
            }
            msg_depth(&q)
        };
        self.broadcast(SessionEvent::QueueDepth(depth));
        Some(depth)
    }

    /// Recall a still-queued Rich Prompt message: drop EVERY queued write
    /// sharing `prompt_id` (body + tail) atomically under the queue lock, so
    /// the multi-write all-or-nothing invariant + `msg_depth` (tail count)
    /// stay consistent — never a partial removal. Returns whether anything was
    /// removed; on a removal, re-emit `QueueDepth` so every attached socket
    /// re-syncs its badge.
    ///
    /// The in-flight message is `pop_front`'ed before delivery
    /// (`try_drain_one`), so it is NOT in `write_queue`: the retain-filter can
    /// never touch or reorder the message currently being delivered. The
    /// cancel-vs-drain race is resolved here under the lock — if the message
    /// drained the same tick, `removed` is `false` and the caller acks that so
    /// the UI does not claim to recall a message that already hit the PTY.
    fn cancel_prompt(&self, prompt_id: &str) -> bool {
        let (removed, depth) = {
            let mut q = self
                .write_queue
                .lock()
                .expect("terminal write queue poisoned");
            let before = q.len();
            q.retain(|w| w.prompt_id.as_deref() != Some(prompt_id));
            (q.len() != before, msg_depth(&q))
        };
        if removed {
            self.broadcast(SessionEvent::QueueDepth(depth));
        }
        removed
    }

    /// The `prompt_id`s of the tail-bearing messages still queued, in FIFO
    /// order — one id per Rich Prompt message. `cs terminal write` pokes carry
    /// no `prompt_id` and are skipped, so membership is exact (a restored
    /// pending id is in the list iff still queued).
    fn queued_prompt_ids(&self) -> Vec<String> {
        self.write_queue
            .lock()
            .expect("terminal write queue poisoned")
            .iter()
            .filter(|w| w.tail)
            .filter_map(|w| w.prompt_id.clone())
            .collect()
    }

    /// Current MESSAGE depth of the write queue (tail count).
    fn queue_depth(&self) -> usize {
        msg_depth(
            &self
                .write_queue
                .lock()
                .expect("terminal write queue poisoned"),
        )
    }

    /// The full replay ring, flattened, for `cs terminal scrollback`.
    /// `snapshot_since(None)` returns every chunk currently held (no
    /// `missed`, since we ask from the ring's own start), so this is the
    /// whole scrollback the ring still has, raw PTY bytes and all. Unlike
    /// `attach`, this does not special-case the alt screen: a scrollback
    /// dump wants whatever bytes the ring holds, including a live TUI draw.
    fn scrollback(&self) -> Vec<u8> {
        let (chunks, _missed) = self
            .ring
            .lock()
            .expect("terminal ring poisoned")
            .snapshot_since(None);
        chunks.concat()
    }

    fn resize(&self, size: PtySize) {
        let _ = self.command_tx.send(PtyCommand::Resize(size));
    }

    fn set_focused(&self, focused: bool) {
        self.focused.store(focused, Ordering::Relaxed);
        if focused {
            self.bytes_since_focus.store(0, Ordering::Relaxed);
            self.broadcast(SessionEvent::Activity {
                bytes_since_focus: 0,
            });
        }
    }

    fn bytes_since_focus(&self) -> u64 {
        self.bytes_since_focus.load(Ordering::Relaxed)
    }

    fn set_broadcast(&self, on: bool) {
        self.broadcast.store(on, Ordering::Relaxed);
    }

    fn request_redraw(&self) {
        let _ = self.command_tx.send(PtyCommand::Redraw);
    }

    fn cwd(&self) -> Option<PathBuf> {
        let cwd = process_cwd(self.child_pid?)?;
        path_inside_root(&cwd, &self.workspace_root).then_some(cwd)
    }

    fn restart_options(&self) -> CreateOptions {
        let mut opts = self.spawn_opts.clone();
        opts.size = *self.winsize.lock().expect("terminal winsize poisoned");
        opts.tab_name = self.tab_name.clone();
        opts.tab_group = self.tab_group.clone();
        opts.window_id = self.window_id();
        opts
    }

    fn close(&self, reason: CloseReason) {
        if self.closed.swap(true, Ordering::Relaxed) {
            return;
        }
        self.broadcast(SessionEvent::Closed(reason));
        let _ = self.command_tx.send(PtyCommand::Kill);
    }

    /// Like [`close`](Self::close) but signals an in-place RESTART instead of a
    /// teardown: broadcast [`SessionEvent::Restarted`] (not `Closed`) before
    /// killing the old PTY, so an attached `/ws` reader re-attaches to the
    /// relaunched session (same id) rather than dropping the tab. The `Kill`
    /// command returns the controller thread before its `try_wait` `Exit`
    /// branch, so no `Exit` leaks either; and the reader moves to the new
    /// channel on `Restarted`, so any racing old-PTY event goes unseen.
    fn close_for_restart(&self) {
        if self.closed.swap(true, Ordering::Relaxed) {
            return;
        }
        self.broadcast(SessionEvent::Restarted);
        let _ = self.command_tx.send(PtyCommand::Kill);
    }

    /// The window (`?w=` label) this session currently belongs to.
    fn window_id(&self) -> Option<String> {
        self.window_id
            .lock()
            .expect("terminal window_id poisoned")
            .clone()
    }

    /// This session incarnation's epoch. Stable for the PTY's life; a restart
    /// mints a new session under the same id with a higher value.
    fn generation(&self) -> u64 {
        self.generation
    }

    /// Rebind the owning window on reattach. A `None`
    /// (windowless) reattach does NOT clear an existing binding — only a real
    /// attaching window re-homes the session.
    fn set_window_id(&self, window_id: Option<String>) {
        if window_id.is_none() {
            return;
        }
        *self.window_id.lock().expect("terminal window_id poisoned") = window_id;
    }

    /// The SPA pane id this session was last attached under (`cs term list`).
    fn pane_id(&self) -> Option<String> {
        self.pane_id
            .lock()
            .expect("terminal pane_id poisoned")
            .clone()
    }

    /// The SPA tab id this session was last attached under (`cs term list`).
    fn tab_id(&self) -> Option<String> {
        self.tab_id
            .lock()
            .expect("terminal tab_id poisoned")
            .clone()
    }

    /// Rebind the pane id on reattach. Like [`Session::set_window_id`], a `None`
    /// (the id was not reported) does NOT clear an existing binding.
    fn set_pane_id(&self, pane_id: Option<String>) {
        if pane_id.is_none() {
            return;
        }
        *self.pane_id.lock().expect("terminal pane_id poisoned") = pane_id;
    }

    /// Rebind the tab id on reattach. A `None` does NOT clear the binding.
    fn set_tab_id(&self, tab_id: Option<String>) {
        if tab_id.is_none() {
            return;
        }
        *self.tab_id.lock().expect("terminal tab_id poisoned") = tab_id;
    }

    fn record_output(&self, bytes: &[u8]) {
        if bytes.is_empty() {
            return;
        }
        self.last_activity
            .store(now_unix_secs() as i64, Ordering::Relaxed);
        // Output-only timestamp for the write-queue's idle/quiescence signal
        // (the agent is rendering / generating).
        self.last_output_at
            .store(now_unix_millis(), Ordering::Relaxed);
        self.update_alt_screen(bytes);
        self.update_private_modes(bytes);
        let end_seq = {
            let mut ring = self.ring.lock().expect("terminal ring poisoned");
            ring.push(bytes);
            ring.end_seq()
        };
        self.seq.store(end_seq, Ordering::Relaxed);
        if !self.focused.load(Ordering::Relaxed) {
            // PTYs emit cursor motion, SGR, OSC title changes, BEL,
            // and CR/LF redraw noise while idle. Only user-visible
            // non-whitespace text should trip the tab activity dot.
            let visible = visible_activity_bytes(bytes);
            if visible > 0 {
                let previous = self.bytes_since_focus.fetch_add(visible, Ordering::Relaxed);
                if previous == 0 {
                    self.broadcast(SessionEvent::Activity {
                        bytes_since_focus: visible,
                    });
                }
            }
        }
        self.broadcast(SessionEvent::Output(bytes.to_vec()));
    }

    fn broadcast(&self, event: SessionEvent) {
        let _ = self.output_tx.send(event);
    }

    fn update_alt_screen(&self, bytes: &[u8]) {
        let mut tail = self
            .alt_screen_tail
            .lock()
            .expect("terminal alt-screen tail poisoned");
        let mut scan = Vec::with_capacity(tail.len() + bytes.len());
        scan.extend_from_slice(&tail);
        scan.extend_from_slice(bytes);

        let mut matched_transition = false;
        if contains_subslice(&scan, ALT_SCREEN_ENTER) {
            self.in_alt_screen.store(true, Ordering::Relaxed);
            tracing::debug!(session = %self.id, "alt_screen entered");
            matched_transition = true;
        }
        if contains_subslice(&scan, ALT_SCREEN_EXIT) {
            self.in_alt_screen.store(false, Ordering::Relaxed);
            tracing::debug!(session = %self.id, "alt_screen exited");
            matched_transition = true;
        }

        if matched_transition {
            tail.clear();
            return;
        }

        if !scan.is_empty() {
            let keep = scan.len().min(ALT_SCREEN_TAIL_BYTES);
            tail.clear();
            tail.extend_from_slice(&scan[scan.len() - keep..]);
        }
    }

    /// Track the live [`TRACKED_PRIVATE_MODES`] set by parsing DEC private-mode
    /// CSIs — `ESC [ ? <;-joined decimal params> (h|l)` — out of PTY output.
    /// `h` adds each tracked param to the set, `l` removes it; a sequence split
    /// across reads is carried in `private_mode_tail`. Non-`h`/`l` finals (a
    /// DECRQM `$p` query, a report, …) are skipped without toggling. Sequences
    /// can carry several modes at once (htop emits `\e[?1006;1000h`).
    fn update_private_modes(&self, bytes: &[u8]) {
        let mut tail = self
            .private_mode_tail
            .lock()
            .expect("terminal private-mode tail poisoned");
        let mut scan = Vec::with_capacity(tail.len() + bytes.len());
        scan.extend_from_slice(&tail);
        scan.extend_from_slice(bytes);
        tail.clear();

        let mut changes: Vec<(u16, bool)> = Vec::new();
        let n = scan.len();
        let mut i = 0;
        while i < n {
            if scan[i] != 0x1b {
                i += 1;
                continue;
            }
            // A private-mode CSI begins ESC '[' '?'. Fewer than 3 trailing bytes
            // could still grow into one on the next read — carry from the ESC.
            if n - i < 3 {
                if n - i <= PRIVATE_MODE_TAIL_CAP {
                    tail.extend_from_slice(&scan[i..]);
                }
                break;
            }
            if scan[i + 1] != b'[' || scan[i + 2] != b'?' {
                i += 1;
                continue;
            }
            // Consume `;`-joined decimal params up to the final byte.
            let mut j = i + 3;
            while j < n && (scan[j].is_ascii_digit() || scan[j] == b';') {
                j += 1;
            }
            if j == n {
                // Params not yet terminated — carry the partial (bounded).
                if n - i <= PRIVATE_MODE_TAIL_CAP {
                    tail.extend_from_slice(&scan[i..]);
                }
                break;
            }
            let final_byte = scan[j];
            if final_byte == b'h' || final_byte == b'l' {
                let on = final_byte == b'h';
                for param in scan[i + 3..j].split(|&b| b == b';') {
                    if let Ok(mode) = std::str::from_utf8(param).unwrap_or("x").parse::<u16>() {
                        if TRACKED_PRIVATE_MODES.contains(&mode) {
                            changes.push((mode, on));
                        }
                    }
                }
            }
            i = j + 1;
        }
        drop(tail);

        if changes.is_empty() {
            return;
        }
        let mut modes = self
            .private_modes
            .lock()
            .expect("terminal private modes poisoned");
        for (mode, on) in changes {
            if on {
                modes.insert(mode);
            } else {
                modes.remove(&mode);
            }
        }
    }

    /// Bytes that re-assert the live tracked private-mode set on reattach —
    /// `ESC [ ? <n> h` per mode currently on, in mode-number order. Empty when
    /// none are on (a plain shell never bloats the prelude).
    fn private_mode_prelude(&self) -> Vec<u8> {
        let modes = self
            .private_modes
            .lock()
            .expect("terminal private modes poisoned");
        let mut out = Vec::new();
        for mode in modes.iter() {
            out.extend_from_slice(format!("\x1b[?{mode}h").as_bytes());
        }
        out
    }
}

enum PtyCommand {
    Input(Vec<u8>),
    Resize(PtySize),
    Redraw,
    Kill,
}

fn random_session_id() -> String {
    let mut bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut bytes);
    let mut out = String::with_capacity(32);
    for b in bytes {
        use std::fmt::Write as _;
        let _ = write!(&mut out, "{b:02x}");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config(ring_bytes: usize, cap: usize, idle: u64) -> RegistryConfig {
        let tmp = tempfile::tempdir().unwrap();
        let workspace_root = tmp.path().to_path_buf();
        std::mem::forget(tmp);
        RegistryConfig {
            workspace_root,
            mcp_socket_path: None,
            control_socket_path: None,
            terminal: TerminalConfig {
                idle_timeout_secs: idle,
                session_cap: cap,
                ring_bytes,
                ..TerminalConfig::default()
            },
        }
    }

    fn test_size() -> PtySize {
        PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        }
    }

    fn test_session_with_ring(ring_bytes: usize) -> Arc<Session> {
        let (command_tx, _command_rx) = std::sync::mpsc::channel();
        let (output_tx, _) = broadcast::channel(BROADCAST_CAP);
        Arc::new(Session {
            id: "test-session".to_string(),
            tab_name: None,
            tab_group: None,
            window_id: Mutex::new(None),
            pane_id: Mutex::new(None),
            tab_id: Mutex::new(None),
            generation: 0,
            workspace_root: PathBuf::from("/"),
            spawn_opts: CreateOptions {
                size: test_size(),
                tab_name: None,
                tab_group: None,
                window_id: None,
                mcp_env: true,
                cwd: None,
                command: None,
                env: Default::default(),
            },
            child_pid: None,
            command_tx,
            output_tx,
            ring: Mutex::new(RingBuffer::new(ring_bytes)),
            seq: AtomicU64::new(0),
            last_activity: AtomicI64::new(now_unix_secs() as i64),
            last_output_at: AtomicI64::new(now_unix_millis()),
            write_queue: Mutex::new(VecDeque::new()),
            last_deliver_at: AtomicI64::new(0),
            awaiting_gen: AtomicBool::new(false),
            attach_count: AtomicUsize::new(0),
            detached_at: AtomicI64::new(now_unix_secs() as i64),
            winsize: Mutex::new(test_size()),
            focused: AtomicBool::new(false),
            bytes_since_focus: AtomicU64::new(0),
            in_alt_screen: AtomicBool::new(false),
            alt_screen_tail: Mutex::new(Vec::new()),
            private_modes: Mutex::new(BTreeSet::new()),
            private_mode_tail: Mutex::new(Vec::new()),
            broadcast: AtomicBool::new(false),
            closed: AtomicBool::new(false),
            exit: Mutex::new(None),
        })
    }

    async fn collect_until(session: &mut AttachHandle, needle: &str, timeout: Duration) -> String {
        let deadline = tokio::time::Instant::now() + timeout;
        let mut out = String::new();
        loop {
            if out.contains(needle) || tokio::time::Instant::now() >= deadline {
                return out;
            }
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            match tokio::time::timeout(remaining, session.rx.recv()).await {
                Ok(Ok(SessionEvent::Output(data))) => out.push_str(&String::from_utf8_lossy(&data)),
                Ok(Ok(_)) => {}
                Ok(Err(_)) | Err(_) => return out,
            }
        }
    }

    // LC_ALL is the highest-precedence locale category, so when it is present
    // in the requested map the helper never consults the (test-host-dependent)
    // process environment; these cases stay deterministic.
    #[test]
    fn locale_selects_utf8_honors_lc_all_codeset() {
        let utf8 = |v: &str| {
            let mut env = BTreeMap::new();
            env.insert("LC_ALL".to_string(), v.to_string());
            locale_selects_utf8(&env)
        };
        assert!(utf8("en_US.UTF-8"));
        assert!(utf8("C.UTF-8"));
        assert!(utf8("en_GB.utf8"));
        assert!(!utf8("C"));
        assert!(!utf8("POSIX"));
        assert!(!utf8("en_US.ISO8859-1"));
    }

    #[test]
    fn activity_counter_tracks_output_since_focus() {
        let session = test_session_with_ring(1024);

        session.record_output(b"background");
        assert_eq!(session.bytes_since_focus(), 10);

        session.set_focused(true);
        assert_eq!(session.bytes_since_focus(), 0);

        session.record_output(b"visible");
        assert_eq!(session.bytes_since_focus(), 0);

        session.set_focused(false);
        session.record_output(b"hidden");
        assert_eq!(session.bytes_since_focus(), 6);
    }

    #[test]
    fn activity_counter_ignores_ansi_and_control_only_writes() {
        let session = test_session_with_ring(1024);

        session.record_output(b"\x1b[?25l\x1b[?25h\x1b[31m\x1b[0m\r\n\t \x07");
        session.record_output(b"\x1b]0;chan\x07");
        session.record_output(b"\x1b]2;title\x1b\\");

        assert_eq!(session.bytes_since_focus(), 0);
    }

    #[test]
    fn activity_counter_counts_plain_visible_text() {
        let session = test_session_with_ring(1024);

        session.record_output(b"echo hello\n");

        assert_eq!(session.bytes_since_focus(), 9);
    }

    #[test]
    fn activity_counter_counts_visible_text_inside_ansi_writes() {
        let session = test_session_with_ring(1024);

        session.record_output(b"\x1b[32mhello\x1b[0m\r\n");

        assert_eq!(session.bytes_since_focus(), 5);
    }

    #[tokio::test]
    async fn activity_event_fires_on_first_unfocused_output_and_clears_on_focus() {
        let session = test_session_with_ring(1024);
        let mut attached = session.clone().attach(Some(0));

        session.record_output(b"one");
        let event = tokio::time::timeout(Duration::from_secs(1), attached.rx.recv())
            .await
            .expect("activity event")
            .expect("activity frame");
        assert!(matches!(
            event,
            SessionEvent::Activity {
                bytes_since_focus: 3
            }
        ));

        session.record_output(b"two");
        let event = tokio::time::timeout(Duration::from_secs(1), attached.rx.recv())
            .await
            .expect("output event")
            .expect("output frame");
        assert!(matches!(event, SessionEvent::Output(_)));

        session.set_focused(true);
        loop {
            let event = tokio::time::timeout(Duration::from_secs(1), attached.rx.recv())
                .await
                .expect("focus clear event")
                .expect("focus clear frame");
            if matches!(
                event,
                SessionEvent::Activity {
                    bytes_since_focus: 0
                }
            ) {
                break;
            }
        }
    }

    #[test]
    fn ring_overflow_reports_missed_bytes() {
        let mut ring = RingBuffer::new(5);
        ring.push(b"abc");
        ring.push(b"def");
        let (replay, missed) = ring.snapshot_since(Some(0));
        assert_eq!(missed, 3);
        assert_eq!(replay.concat(), b"def");
    }

    #[test]
    fn scrollback_flattens_the_whole_ring() {
        let session = test_session_with_ring(1024);
        session.record_output(b"hello\n");
        session.record_output(b"world\n");
        // The full ring, in order, raw bytes and all.
        assert_eq!(session.scrollback(), b"hello\nworld\n");
    }

    #[test]
    fn scrollback_matching_selects_exactly_the_named_tab() {
        let registry = Registry::new(test_config(4096, 4, 60));
        let handle = registry
            .create(CreateOptions {
                size: test_size(),
                tab_name: Some("@@Alice".into()),
                tab_group: None,
                window_id: None,
                mcp_env: true,
                cwd: None,
                command: None,
                env: Default::default(),
            })
            .unwrap();
        // One session owns the tab name; a different name matches none. The
        // count is what the control socket's single-match policy gates on.
        assert_eq!(registry.scrollback_matching("@@Alice").len(), 1);
        assert!(registry.scrollback_matching("@@Nope").is_empty());
        registry.close(handle.id(), CloseReason::Explicit);
    }

    #[test]
    fn write_queue_enqueue_bounds_at_cap() {
        let session = test_session_with_ring(1024);
        for i in 1..=WRITE_QUEUE_CAP {
            assert_eq!(session.enqueue_write(b"x"), Some(i), "position grows");
        }
        assert_eq!(session.enqueue_write(b"x"), None, "rejected at cap");
    }

    #[test]
    fn write_queue_drains_only_when_idle_and_awaits_generation() {
        let session = test_session_with_ring(1024);
        session.enqueue_write(b"one");
        session.enqueue_write(b"two");
        let qlen = |s: &Session| s.write_queue.lock().expect("queue").len();
        let base = now_unix_millis();

        // Agent busy (output just now): nothing delivered.
        session.last_output_at.store(base, Ordering::Relaxed);
        session.try_drain_one(base);
        assert_eq!(qlen(&session), 2, "busy -> hold");

        // Agent idle (output quiet > QUIET_MS): deliver one, then await the
        // next generation-start.
        let idle_now = base + WRITE_QUEUE_QUIET_MS + 10;
        session.try_drain_one(idle_now);
        assert_eq!(qlen(&session), 1, "idle -> delivered one");
        assert!(session.awaiting_gen.load(Ordering::Relaxed), "awaiting gen");

        // Still awaiting generation-start (no new output, under the cap): hold.
        session.try_drain_one(idle_now + 10);
        assert_eq!(qlen(&session), 1, "awaiting gen -> hold the second");

        // Generation started (output advanced past the deliver) then finished
        // (idle again): the second delivers.
        let gen_at = idle_now + 20;
        session.last_output_at.store(gen_at, Ordering::Relaxed);
        session.try_drain_one(gen_at + WRITE_QUEUE_QUIET_MS + 10);
        assert_eq!(qlen(&session), 0, "turn done -> second delivered");
    }

    #[test]
    fn write_queue_gen_start_cap_unwedges_a_non_generating_message() {
        // A delivered message that never triggers generation (no output
        // advance) must not wedge the queue forever: after the gen-start cap,
        // the next message delivers.
        let session = test_session_with_ring(1024);
        session.enqueue_write(b"one");
        session.enqueue_write(b"two");
        let base = now_unix_millis();
        // last output well in the past -> always "idle".
        session.last_output_at.store(base, Ordering::Relaxed);
        let t1 = base + WRITE_QUEUE_QUIET_MS + 10;
        session.try_drain_one(t1);
        assert!(session.awaiting_gen.load(Ordering::Relaxed));
        // No output ever arrives; past the cap the await clears + the second
        // delivers (idle the whole time).
        session.try_drain_one(t1 + WRITE_QUEUE_GEN_START_CAP_MS + 10);
        assert_eq!(session.write_queue.lock().expect("queue").len(), 0);
    }

    #[test]
    fn enqueue_prompt_is_all_or_nothing_at_cap() {
        // A 2-write message (gemini) near the cap must not split: the old
        // per-write path enqueued the body at 99/100 and silently dropped
        // the CR. The whole message is rejected, the queue untouched.
        let session = test_session_with_ring(1024);
        for _ in 1..WRITE_QUEUE_CAP {
            session.enqueue_write(b"x");
        }
        let pair = vec![b"hi there".to_vec(), b"\r".to_vec()];
        assert_eq!(
            session.enqueue_prompt(&pair, Some("msg-1".into())),
            None,
            "2-write message must not split into the last slot"
        );
        assert_eq!(
            session.write_queue.lock().expect("queue").len(),
            WRITE_QUEUE_CAP - 1,
            "rejected message leaves the queue unchanged"
        );
        // A single-write message still fits the remaining slot.
        let single = vec![b"poke\x1b[27;9;13~".to_vec()];
        assert_eq!(
            session.enqueue_prompt(&single, Some("msg-2".into())),
            Some(WRITE_QUEUE_CAP),
            "1-write message fits; return is the message depth"
        );
    }

    #[test]
    fn queue_depth_counts_messages_not_writes() {
        let session = test_session_with_ring(1024);
        let pair = vec![b"hi there".to_vec(), b"\r".to_vec()];
        assert_eq!(
            session.enqueue_prompt(&pair, Some("gem-1".into())),
            Some(1),
            "first message -> depth/position 1"
        );
        assert_eq!(
            session.write_queue.lock().expect("queue").len(),
            2,
            "a gemini pair is two raw entries"
        );
        assert_eq!(session.queue_depth(), 1, "but ONE message");
        // A CLI poke behind it: raw position 3 (the frozen stdout contract),
        // message depth 2 (what the SPA badge shows).
        assert_eq!(session.enqueue_write(b"poke"), Some(3));
        assert_eq!(session.queue_depth(), 2);
    }

    #[test]
    fn cancel_prompt_removes_all_writes_of_the_id_atomically_and_reemits_depth() {
        let session = test_session_with_ring(1024);
        // m1 = single write; m2 = gemini pair (body + tail, same id); then a
        // CLI poke (no id) behind them.
        session.enqueue_prompt(&[b"first".to_vec()], Some("m1".into()));
        session.enqueue_prompt(&[b"second".to_vec(), b"\r".to_vec()], Some("m2".into()));
        session.enqueue_write(b"poke");
        assert_eq!(session.queue_depth(), 3, "two prompts + one poke");

        let mut rx = session.output_tx.subscribe();
        // Cancel the gemini message: BOTH its raw writes (body + tail) go
        // together — never a partial removal.
        assert!(session.cancel_prompt("m2"), "m2 was still queued");
        match rx.try_recv() {
            Ok(SessionEvent::QueueDepth(depth)) => assert_eq!(depth, 2, "depth re-emitted"),
            other => panic!("expected QueueDepth, got {other:?}"),
        }
        assert_eq!(session.queue_depth(), 2);
        // m2's two entries are gone; m1 + the poke remain, ordering preserved.
        let q = session.write_queue.lock().expect("queue");
        assert_eq!(
            q.len(),
            2,
            "m1 (1 write) + poke (1); m2's body+tail removed"
        );
        assert_eq!(q[0].prompt_id.as_deref(), Some("m1"));
        assert_eq!(q[1].prompt_id, None, "the CLI poke stays, in order");
    }

    #[test]
    fn cancel_prompt_on_an_absent_id_reports_not_removed_and_is_silent() {
        let session = test_session_with_ring(1024);
        session.enqueue_prompt(&[b"x".to_vec()], Some("m1".into()));
        let mut rx = session.output_tx.subscribe();
        // The id already drained (or never existed): nothing to remove, and a
        // no-op cancel must not perturb depth (the cancel-vs-drain race: the
        // caller acks removed=false so the UI does not recall a drained msg).
        assert!(!session.cancel_prompt("gone"));
        assert!(
            matches!(rx.try_recv(), Err(broadcast::error::TryRecvError::Empty)),
            "a no-op cancel must not re-emit depth"
        );
        assert_eq!(session.queue_depth(), 1, "m1 untouched");
    }

    #[test]
    fn queued_prompt_ids_lists_rich_messages_in_fifo_order_skipping_pokes() {
        let session = test_session_with_ring(1024);
        session.enqueue_prompt(&[b"a".to_vec()], Some("m1".into()));
        session.enqueue_write(b"poke"); // no prompt_id -> not listed
        session.enqueue_prompt(&[b"b".to_vec(), b"\r".to_vec()], Some("m2".into())); // pair -> one id
        assert_eq!(
            session.queued_prompt_ids(),
            vec!["m1".to_string(), "m2".to_string()],
            "one id per rich message, FIFO, CLI poke skipped"
        );
        // Membership tracks cancellation: after recalling m1, only m2 remains.
        assert!(session.cancel_prompt("m1"));
        assert_eq!(session.queued_prompt_ids(), vec!["m2".to_string()]);
    }

    #[test]
    fn drain_emits_delivered_on_last_write_only() {
        let session = test_session_with_ring(1024);
        let pair = vec![b"hi there".to_vec(), b"\r".to_vec()];
        session.enqueue_prompt(&pair, Some("msg-1".into()));
        // Subscribe AFTER the enqueue so its QueueDepth stays out of frame.
        let mut rx = session.output_tx.subscribe();
        let base = now_unix_millis();
        session.last_output_at.store(base, Ordering::Relaxed);

        // Body drain: one raw entry delivered, but the message is still
        // pending (its chord is queued) -> no events.
        let t1 = base + WRITE_QUEUE_QUIET_MS + 10;
        session.try_drain_one(t1);
        assert_eq!(session.write_queue.lock().expect("queue").len(), 1);
        assert!(
            matches!(rx.try_recv(), Err(broadcast::error::TryRecvError::Empty)),
            "body drain emits nothing"
        );

        // Chord (tail) drain: no output ever arrives, so the gen-start cap
        // unwedges; PromptDelivered fires first, then QueueDepth, both 0.
        session.try_drain_one(t1 + WRITE_QUEUE_GEN_START_CAP_MS + 10);
        assert_eq!(session.write_queue.lock().expect("queue").len(), 0);
        match rx.try_recv() {
            Ok(SessionEvent::PromptDelivered { id, depth }) => {
                assert_eq!(id, "msg-1");
                assert_eq!(depth, 0);
            }
            other => panic!("expected PromptDelivered first, got {other:?}"),
        }
        match rx.try_recv() {
            Ok(SessionEvent::QueueDepth(depth)) => assert_eq!(depth, 0),
            other => panic!("expected QueueDepth after PromptDelivered, got {other:?}"),
        }
    }

    #[test]
    fn enqueue_broadcasts_queue_depth_on_both_paths() {
        let session = test_session_with_ring(1024);
        let mut rx = session.output_tx.subscribe();

        // CLI path: returns the raw position, broadcasts the message depth.
        assert_eq!(session.enqueue_write(b"poke"), Some(1));
        match rx.try_recv() {
            Ok(SessionEvent::QueueDepth(depth)) => assert_eq!(depth, 1),
            other => panic!("expected QueueDepth, got {other:?}"),
        }

        // Prompt path: return == ack position == message depth.
        let pair = vec![b"hi".to_vec(), b"\r".to_vec()];
        assert_eq!(session.enqueue_prompt(&pair, Some("m".into())), Some(2));
        match rx.try_recv() {
            Ok(SessionEvent::QueueDepth(depth)) => assert_eq!(depth, 2),
            other => panic!("expected QueueDepth, got {other:?}"),
        }
    }

    #[test]
    fn enqueue_write_matching_reports_position_for_a_single_target() {
        let registry = Registry::new(test_config(4096, 4, 60));
        let handle = registry
            .create(CreateOptions {
                size: test_size(),
                tab_name: Some("@@A".into()),
                tab_group: None,
                window_id: None,
                mcp_env: true,
                cwd: None,
                command: None,
                env: Default::default(),
            })
            .unwrap();
        // No drainer runs in this test, so positions are stable.
        let first = registry.enqueue_write_matching(Some("@@A"), None, b"x");
        assert_eq!(first.queued, 1);
        assert_eq!(first.position, Some(1));
        let second = registry.enqueue_write_matching(Some("@@A"), None, b"y");
        assert_eq!(second.position, Some(2), "FIFO position grows");
        // No match -> nothing queued, no position.
        let none = registry.enqueue_write_matching(Some("@@Nope"), None, b"z");
        assert_eq!(none.queued, 0);
        assert_eq!(none.position, None);
        registry.close(handle.id(), CloseReason::Explicit);
    }

    #[test]
    fn session_ids_are_hex_and_distinct() {
        let a = random_session_id();
        let b = random_session_id();
        assert_ne!(a, b);
        assert_eq!(a.len(), 32);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn alt_screen_active_skips_replay_until_exit() {
        let session = test_session_with_ring(1024);
        session.record_output(b"before alt\n");
        let attached = session.clone().attach(Some(0));
        assert_eq!(attached.replay.concat(), b"before alt\n");
        drop(attached);

        session.record_output(b"\x1b[?1049hdraw tui frame");
        let attached = session.clone().attach(Some(0));
        assert!(attached.replay.is_empty());
        assert_eq!(attached.missed_bytes, 0);
        drop(attached);

        session.record_output(b"\x1b[?1049lback to shell\n");
        let attached = session.attach(Some(0));
        assert!(!attached.replay.is_empty());
        assert!(String::from_utf8_lossy(&attached.replay.concat()).contains("back to shell"));
    }

    #[test]
    fn alt_screen_sniffer_matches_expected_sequences() {
        assert!(contains_subslice(b"abc\x1b[?1049hdef", b"\x1b[?1049h"));
        assert!(contains_subslice(b"abc\x1b[?1049ldef", b"\x1b[?1049l"));
        assert!(!contains_subslice(b"abc\x1b[?1048hdef", b"\x1b[?1049h"));
    }

    #[test]
    fn alt_screen_sniffer_matches_sequences_across_chunks() {
        let session = test_session_with_ring(1024);

        session.record_output(b"\x1b");
        assert!(!session.in_alt_screen.load(Ordering::Relaxed));
        session.record_output(b"[?1049h");
        assert!(session.in_alt_screen.load(Ordering::Relaxed));

        session.record_output(b"\x1b[?");
        assert!(session.in_alt_screen.load(Ordering::Relaxed));
        session.record_output(b"1049l");
        assert!(!session.in_alt_screen.load(Ordering::Relaxed));
    }

    fn modes_on(session: &Arc<Session>) -> Vec<u16> {
        session
            .private_modes
            .lock()
            .unwrap()
            .iter()
            .copied()
            .collect()
    }

    /// THE htop-after-reload regression. A live PTY running htop set DECCKM(1) +
    /// mouse(1000;1006) + alt-screen(1049) at startup; a fresh client reattaching
    /// in alt-screen replays no scrollback, so the prelude is the ONLY chance to
    /// restore those modes. Before the fix the prelude re-asserted ONLY alt-screen,
    /// so arrows (DECCKM) and wheel/clicks (mouse) died. The reattach must now
    /// re-assert the live INPUT modes — and NOT alt-screen (handled separately).
    #[test]
    fn reattach_reasserts_htop_input_modes() {
        let session = test_session_with_ring(4096);
        // The exact private-mode set real htop 3.4.1 emits at startup (captured).
        session.record_output(b"\x1b[?1049h\x1b[?7h\x1b[?1h\x1b[?25l\x1b[?1006;1000h");
        assert_eq!(modes_on(&session), vec![1, 1000, 1006]);

        let attached = session.clone().attach(Some(0));
        // BTreeSet order → 1, 1000, 1006. Alt-screen (1049) is NOT here; autowrap
        // (7) and cursor-hide (25) are untracked screen state, also absent.
        assert_eq!(attached.mode_reassert, b"\x1b[?1h\x1b[?1000h\x1b[?1006h");
        assert!(attached.alt_screen, "alt-screen still tracked separately");
    }

    #[test]
    fn private_modes_track_set_and_reset() {
        let session = test_session_with_ring(1024);
        session.record_output(b"\x1b[?1h");
        assert_eq!(modes_on(&session), vec![1]);
        session.record_output(b"\x1b[?1000h");
        assert_eq!(modes_on(&session), vec![1, 1000]);
        // Reset DECCKM (rmkx): 1 leaves the set, 1000 stays.
        session.record_output(b"\x1b[?1l");
        assert_eq!(modes_on(&session), vec![1000]);
    }

    #[test]
    fn private_modes_parse_grouped_and_split_across_reads() {
        let session = test_session_with_ring(1024);
        // Grouped params in one CSI (htop's `\e[?1006;1000h`).
        session.record_output(b"\x1b[?1006;1000h");
        assert_eq!(modes_on(&session), vec![1000, 1006]);
        // A single CSI split across two reads must still resolve.
        session.record_output(b"\x1b[?2");
        assert_eq!(modes_on(&session), vec![1000, 1006]);
        session.record_output(b"004h");
        assert_eq!(modes_on(&session), vec![1000, 1006, 2004]);
    }

    #[test]
    fn private_modes_ignore_untracked_and_queries() {
        let session = test_session_with_ring(1024);
        // Untracked screen modes (autowrap, cursor visibility) never enter the set.
        session.record_output(b"\x1b[?7h\x1b[?25l\x1b[?12l");
        assert!(modes_on(&session).is_empty());
        // A DECRQM query (`$p` final) must not toggle anything.
        session.record_output(b"\x1b[?1006;1000$p");
        assert!(modes_on(&session).is_empty());
        // Alt-screen is tracked separately, not in the re-assert set.
        session.record_output(b"\x1b[?1049h");
        assert!(modes_on(&session).is_empty());
        assert!(session.clone().attach(Some(0)).mode_reassert.is_empty());
    }

    #[test]
    fn plain_shell_has_empty_mode_reassert() {
        // A session that never set a tracked mode re-asserts nothing — a plain
        // shell must not bloat the prelude.
        let session = test_session_with_ring(1024);
        session.record_output(b"$ echo hi\r\nhi\r\n");
        assert!(session.attach(Some(0)).mode_reassert.is_empty());
    }

    #[test]
    fn redraw_wobble_pattern_resizes_then_restores() {
        let original = PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 640,
            pixel_height: 480,
        };
        let mut calls = Vec::new();
        force_redraw_with_wobble(original, Duration::ZERO, |size| {
            calls.push(size);
            Ok::<(), ()>(())
        })
        .unwrap();

        assert_eq!(
            calls,
            vec![
                PtySize {
                    rows: 23,
                    ..original
                },
                original,
            ]
        );
    }

    #[test]
    fn redraw_wobble_keeps_single_row_sessions_moving() {
        let original = PtySize {
            rows: 1,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        };

        assert_eq!(redraw_wobble_size(original).rows, 2);
    }

    #[test]
    fn prune_idle_removes_detached_sessions() {
        let registry = Registry::new(test_config(1024, 4, 10));
        let handle = registry
            .create(CreateOptions {
                size: test_size(),
                tab_name: None,
                tab_group: None,
                window_id: None,
                mcp_env: true,
                cwd: None,
                command: None,
                env: Default::default(),
            })
            .unwrap();
        let id = handle.id().to_string();
        drop(handle);
        assert_eq!(registry.len(), 1);
        assert_eq!(registry.prune_idle_at(now_unix_secs() as i64 + 11), 1);
        assert_eq!(registry.len(), 0);
        assert!(registry.attach(&id, None).is_none());
    }

    fn opts_with_window(window_id: &str) -> CreateOptions {
        CreateOptions {
            size: test_size(),
            tab_name: None,
            tab_group: None,
            window_id: Some(window_id.to_string()),
            mcp_env: true,
            cwd: None,
            command: None,
            env: Default::default(),
        }
    }

    fn opts_with_command(command: &str) -> CreateOptions {
        CreateOptions {
            size: test_size(),
            tab_name: None,
            tab_group: None,
            window_id: Some("win-exit".to_string()),
            mcp_env: false,
            cwd: None,
            command: Some(command.to_string()),
            env: Default::default(),
        }
    }

    async fn wait_for_last_exit(registry: &Registry) -> TerminalExit {
        for _ in 0..120 {
            if let Some(exit) = registry.last_exit() {
                return exit;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        panic!("terminal exit was not recorded");
    }

    #[tokio::test]
    async fn last_exit_zero_is_sticky_after_session_removal() {
        let registry = Registry::new(test_config(1024, 4, 10));
        let handle = registry.create(opts_with_command("exit 0")).unwrap();
        let id = handle.id().to_string();

        assert_eq!(
            wait_for_last_exit(&registry).await,
            TerminalExit::Code { code: 0 }
        );
        assert!(registry.remove(&id));
        assert_eq!(registry.last_exit(), Some(TerminalExit::Code { code: 0 }));
    }

    #[tokio::test]
    async fn last_exit_nonzero_is_sticky_after_session_removal() {
        let registry = Registry::new(test_config(1024, 4, 10));
        let handle = registry.create(opts_with_command("exit 7")).unwrap();
        let id = handle.id().to_string();

        assert_eq!(
            wait_for_last_exit(&registry).await,
            TerminalExit::Code { code: 7 }
        );
        assert!(registry.remove(&id));
        assert_eq!(registry.last_exit(), Some(TerminalExit::Code { code: 7 }));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn unix_signal_exit_is_recorded_as_exit_state() {
        let registry = Registry::new(test_config(1024, 4, 10));
        let _handle = registry.create(opts_with_command("kill -TERM $$")).unwrap();

        match wait_for_last_exit(&registry).await {
            TerminalExit::Signal { signal } => assert!(!signal.is_empty()),
            other => panic!("expected a signal exit, got {other:?}"),
        }
    }

    #[test]
    fn persisted_window_session_survives_prune_and_reattaches() {
        // persist ⇒ keep: a detached session whose window has a durable blob is
        // kept indefinitely (browser-tab / devserver semantics), reattachable.
        let registry = Registry::new(test_config(1024, 4, 10));
        let handle = registry.create(opts_with_window("win-keep")).unwrap();
        let id = handle.id().to_string();
        drop(handle); // every client detached
        registry.mark_window_persisted("win-keep");
        let now = now_unix_secs() as i64;
        // Far past the idle grace — a persisted window is never idle-reaped.
        assert_eq!(registry.prune_idle_at(now + 100_000), 0);
        assert_eq!(registry.len(), 1);
        assert!(registry.attach(&id, None).is_some());
    }

    #[test]
    fn busy_orphan_window_session_is_reaped_from_detach_time() {
        // The FD-leak fix: a BUSY detached session (fresh `last_activity`) whose
        // window was never persisted is still reaped, because the grace is timed
        // off the detach instant, not the last output byte.
        let registry = Registry::new(test_config(1024, 4, 10));
        let handle = registry.create(opts_with_window("win-orphan")).unwrap();
        drop(handle); // detached; window never persisted
        let now = now_unix_secs() as i64;
        {
            let sessions = registry.sessions.lock().unwrap();
            let session = sessions.values().next().unwrap();
            // Simulate a busy session: output kept arriving "just now"...
            session
                .last_activity
                .store(now + 100_000, Ordering::Relaxed);
            // ...but it has been detached since `now`.
            session.detached_at.store(now, Ordering::Relaxed);
        }
        // 11s past detach > the 10s grace ⇒ reaped despite the fresh activity.
        assert_eq!(registry.prune_idle_at(now + 11), 1);
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn forget_window_reaps_its_sessions_and_unpersists() {
        // discard ⇒ reap: a window-blob DELETE kills exactly that window's
        // sessions and drops it from the persisted set.
        let registry = Registry::new(test_config(1024, 4, 10));
        let a1 = registry.create(opts_with_window("win-a")).unwrap();
        let a2 = registry.create(opts_with_window("win-a")).unwrap();
        let b = registry.create(opts_with_window("win-b")).unwrap();
        registry.mark_window_persisted("win-a");
        drop(a1);
        drop(a2);
        drop(b);
        assert_eq!(registry.forget_window("win-a"), 2);
        assert_eq!(registry.len(), 1); // win-b untouched
        assert!(!registry.persisted_windows.lock().unwrap().contains("win-a"));
    }

    #[test]
    fn unpersist_window_drops_persistence_without_reaping() {
        // Move-out invariant: the source's `?w=W&moved=1` DELETE unpersists
        // the window but must NOT reap; attach rebinds the moved PTY to the
        // target window.
        let registry = Registry::new(test_config(1024, 4, 10));
        let _a = registry.create(opts_with_window("win-a")).unwrap();
        registry.mark_window_persisted("win-a");

        registry.unpersist_window("win-a");
        assert_eq!(registry.len(), 1, "move-out keeps the PTY alive (no reap)");
        assert!(
            !registry.persisted_windows.lock().unwrap().contains("win-a"),
            "the source window is no longer persisted"
        );
    }

    #[test]
    fn close_for_window_only_closes_the_matching_window() {
        let registry = Registry::new(test_config(1024, 4, 10));
        let _a = registry.create(opts_with_window("win-a")).unwrap();
        let _b = registry.create(opts_with_window("win-b")).unwrap();
        assert_eq!(registry.close_for_window("win-a", CloseReason::Explicit), 1);
        assert_eq!(registry.len(), 1);
        assert_eq!(
            registry.close_for_window("win-missing", CloseReason::Explicit),
            0
        );
    }

    #[test]
    fn count_for_window_counts_only_live_matching_sessions() {
        // The read-only basis for the `cs window rm` --force guard.
        let registry = Registry::new(test_config(1024, 4, 10));
        let _a1 = registry.create(opts_with_window("win-a")).unwrap();
        let _a2 = registry.create(opts_with_window("win-a")).unwrap();
        let _b = registry.create(opts_with_window("win-b")).unwrap();
        assert_eq!(registry.count_for_window("win-a"), 2);
        assert_eq!(registry.count_for_window("win-b"), 1);
        assert_eq!(registry.count_for_window("win-missing"), 0);
        // Closing a window's sessions drops it to zero.
        registry.close_for_window("win-a", CloseReason::Explicit);
        assert_eq!(registry.count_for_window("win-a"), 0);
        registry.close_all(CloseReason::Shutdown);
    }

    #[test]
    fn reap_exited_removes_a_dead_detached_session() {
        // A killed agent: its controller thread recorded `exit` on process
        // exit but the entry lingered (the ghost-tab name-holding bug). Once
        // detached (frontend gone) it is a pure ghost ⇒ reaped, freeing the name.
        let registry = Registry::new(test_config(1024, 4, 10));
        let handle = registry.create(opts_with_window("win-dead")).unwrap();
        drop(handle); // frontend gone (detached)
        {
            let sessions = registry.sessions.lock().unwrap();
            let session = sessions.values().next().unwrap();
            *session.exit.lock().unwrap() = Some(TerminalExit::Code { code: 0 });
        }
        assert_eq!(registry.reap_exited(), 1);
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn reap_exited_keeps_an_attached_dead_session() {
        // A natural `exit` while a client still views the final output: the
        // process is dead but a viewer is attached, so the pane survives until
        // the client detaches. Guards against a natural-exit-vanishes regression.
        let registry = Registry::new(test_config(1024, 4, 10));
        let _handle = registry.create(opts_with_window("win-viewed")).unwrap(); // attached
        {
            let sessions = registry.sessions.lock().unwrap();
            let session = sessions.values().next().unwrap();
            *session.exit.lock().unwrap() = Some(TerminalExit::Code { code: 0 });
        }
        assert_eq!(registry.reap_exited(), 0);
        assert_eq!(registry.len(), 1);
        registry.close_all(CloseReason::Shutdown);
    }

    #[test]
    fn reap_exited_keeps_a_live_detached_session() {
        // Detached but the process is still running (a busy background agent):
        // NOT a ghost — kept. Process-death is the reap axis, not detach.
        let registry = Registry::new(test_config(1024, 4, 10));
        let handle = registry.create(opts_with_window("win-live")).unwrap();
        drop(handle); // detached, but exit stays None (still running)
        assert_eq!(registry.reap_exited(), 0);
        assert_eq!(registry.len(), 1);
        registry.close_all(CloseReason::Shutdown);
    }

    #[test]
    fn reap_exited_fires_the_window_reaper_for_a_dead_detached_session() {
        // A standalone terminal's PTY exits while detached -> reap_exited
        // closes the session AND fires the window-reaper hook with its
        // window_id, so the host can drop the window-feed row with it.
        let registry = Registry::new(test_config(1024, 4, 10));
        let reaped: Arc<std::sync::Mutex<Vec<String>>> =
            Arc::new(std::sync::Mutex::new(Vec::new()));
        let sink = Arc::clone(&reaped);
        registry.install_window_reaper(WindowReaper::new(move |window_id: &str| {
            sink.lock().unwrap().push(window_id.to_string());
        }));
        let handle = registry.create(opts_with_window("win-term")).unwrap();
        drop(handle); // detached
        {
            let sessions = registry.sessions.lock().unwrap();
            let session = sessions.values().next().unwrap();
            *session.exit.lock().unwrap() = Some(TerminalExit::Code { code: 0 });
        }
        assert_eq!(registry.reap_exited(), 1);
        assert_eq!(*reaped.lock().unwrap(), vec!["win-term".to_string()]);
    }

    #[test]
    fn reap_exited_does_not_fire_the_window_reaper_for_an_attached_session() {
        // The guard: an attached dead terminal is KEPT (a viewer sees the final
        // output), so the window-reaper must NOT fire and the window stays.
        let registry = Registry::new(test_config(1024, 4, 10));
        let reaped: Arc<std::sync::Mutex<Vec<String>>> =
            Arc::new(std::sync::Mutex::new(Vec::new()));
        let sink = Arc::clone(&reaped);
        registry.install_window_reaper(WindowReaper::new(move |window_id: &str| {
            sink.lock().unwrap().push(window_id.to_string());
        }));
        let _handle = registry.create(opts_with_window("win-viewed")).unwrap(); // attached
        {
            let sessions = registry.sessions.lock().unwrap();
            let session = sessions.values().next().unwrap();
            *session.exit.lock().unwrap() = Some(TerminalExit::Code { code: 0 });
        }
        assert_eq!(registry.reap_exited(), 0);
        assert!(reaped.lock().unwrap().is_empty());
        registry.close_all(CloseReason::Shutdown);
    }

    #[test]
    fn reap_window_layout_fires_the_blob_reaper() {
        // The explicit-discard hook: the host calls reap_window_layout so a
        // discarded terminal window's durable layout blob is dropped too.
        let registry = Registry::new(test_config(1024, 4, 10));
        let reaped: Arc<std::sync::Mutex<Vec<String>>> =
            Arc::new(std::sync::Mutex::new(Vec::new()));
        let sink = Arc::clone(&reaped);
        registry.install_blob_reaper(BlobReaper::new(move |window_id: &str| {
            sink.lock().unwrap().push(window_id.to_string());
        }));
        registry.reap_window_layout("win-blob");
        assert_eq!(*reaped.lock().unwrap(), vec!["win-blob".to_string()]);
    }

    #[test]
    fn reap_window_layout_is_a_noop_without_a_blob_reaper() {
        // Workspace / ephemeral / control tenants install no hook; the call must
        // be a harmless no-op, never a panic.
        let registry = Registry::new(test_config(1024, 4, 10));
        registry.reap_window_layout("win-none");
    }

    #[test]
    fn session_summaries_carry_the_owning_window_id() {
        // cs term list resolves each session's owning window from this field.
        let registry = Registry::new(test_config(4096, 4, 60));
        let _h = registry.create(opts_with_window("win-sum")).unwrap();
        let summaries = registry.session_summaries();
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].window_id.as_deref(), Some("win-sum"));
        registry.close_all(CloseReason::Shutdown);
    }

    #[test]
    fn session_summaries_carry_the_attached_pane_and_tab() {
        // cs term list traces window -> pane -> tab; the pane/tab ids ride the
        // WS attach query and are recorded best-effort on the live session.
        let registry = Registry::new(test_config(4096, 4, 60));
        let handle = registry
            .get_or_create_for_ws(
                None,
                None,
                opts_with_window("win-pt"),
                Some("pane-7".to_string()),
                Some("tab-3".to_string()),
                None,
            )
            .unwrap();
        let id = handle.id().to_string();
        let summaries = registry.session_summaries();
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].pane_id.as_deref(), Some("pane-7"));
        assert_eq!(summaries[0].tab_id.as_deref(), Some("tab-3"));

        // A reattach (split/move into another pane) re-binds the pane id; a
        // `None` on either axis leaves the prior value untouched.
        drop(handle);
        let _re = registry
            .get_or_create_for_ws(
                Some(&id),
                Some(0),
                opts_with_window("win-pt"),
                Some("pane-9".to_string()),
                None,
                None,
            )
            .unwrap();
        let summaries = registry.session_summaries();
        assert_eq!(summaries[0].pane_id.as_deref(), Some("pane-9"));
        assert_eq!(summaries[0].tab_id.as_deref(), Some("tab-3"));
        registry.close_all(CloseReason::Shutdown);
    }

    #[test]
    fn close_matching_closes_by_tab_name_and_leaves_others() {
        let registry = Registry::new(test_config(1024, 4, 10));
        let _a = registry
            .create(CreateOptions {
                tab_name: Some("@@Alice".into()),
                ..opts_with_window("win-a")
            })
            .unwrap();
        let _b = registry
            .create(CreateOptions {
                tab_name: Some("@@Bob".into()),
                ..opts_with_window("win-b")
            })
            .unwrap();
        assert_eq!(registry.close_matching(Some("@@Alice"), None), 1);
        assert_eq!(registry.len(), 1);
        // A selector that matches nothing closes nothing.
        assert_eq!(registry.close_matching(Some("@@Nobody"), None), 0);
        assert_eq!(registry.len(), 1);
        registry.close_all(CloseReason::Shutdown);
    }

    #[test]
    fn restart_signals_restarted_not_closed_on_the_old_channel() {
        // Restart-reconcile contract: a restart broadcasts
        // `Restarted` (never `Closed`/`Exit`) on the OLD channel, so the /ws
        // reader re-attaches to the relaunched session under the SAME id
        // instead of dropping the tab. The id stays live afterwards.
        let registry = Registry::new(test_config(4096, 8, 60));
        let mut handle = registry.create(opts_with_window("win-restart")).unwrap();
        let id = handle.id().to_string();
        assert!(registry.restart(&id, RestartOverrides::default()).unwrap());
        let mut saw_restarted = false;
        while let Ok(event) = handle.rx.try_recv() {
            match event {
                SessionEvent::Restarted => saw_restarted = true,
                SessionEvent::Closed(_) | SessionEvent::Exit(_) => {
                    panic!("restart must not broadcast Closed/Exit on the old channel")
                }
                _ => {}
            }
        }
        assert!(
            saw_restarted,
            "restart must broadcast Restarted on the old channel"
        );
        // The id still resolves to a live, relaunched session.
        assert!(registry.attach_for_ws(&id, None).is_some());
        registry.close_all(CloseReason::Shutdown);
    }

    #[test]
    fn generation_bumps_on_restart() {
        // The per-PTY-life epoch must advance on restart: the session keeps its
        // id but the ring/`seq` reset to 0, so a reattach with a stale `since`
        // cursor would silently desync. The bumped generation is the client's
        // cache-invalidation signal (see `get_or_create_for_ws`'s gate).
        let registry = Registry::new(test_config(4096, 8, 60));
        let first = registry.create(opts_with_window("win-gen")).unwrap();
        let id = first.id().to_string();
        let gen1 = first.generation;
        drop(first);
        assert!(registry.restart(&id, RestartOverrides::default()).unwrap());
        let second = registry.attach(&id, None).unwrap();
        assert!(
            second.generation > gen1,
            "restart must mint a higher generation (was {gen1}, got {})",
            second.generation
        );
        registry.close_all(CloseReason::Shutdown);
    }

    #[tokio::test]
    async fn reattach_honors_since_only_on_matching_generation() {
        // A reattach resumes from `since` only when the client's cached
        // generation still matches the live session; a mismatch forces a full
        // replay so a stale cursor never yields a silently-truncated screen.
        let registry = Registry::new(test_config(1 << 16, 4, 60));
        let mut first = registry.create(opts_with_window("win-gate")).unwrap();
        let id = first.id().to_string();
        let gen = first.generation;
        // Drive output so the ring is non-empty and `seq` advances.
        first.send_input(b"echo chan-gate-probe\n");
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        let mut got_output = false;
        while tokio::time::Instant::now() < deadline {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            match tokio::time::timeout(remaining, first.rx.recv()).await {
                Ok(Ok(SessionEvent::Output(_))) => {
                    got_output = true;
                    break;
                }
                Ok(Ok(_)) => {}
                Ok(Err(_)) | Err(_) => break,
            }
        }
        assert!(got_output, "session produced no output to seed the ring");
        let end = registry.attach(&id, None).unwrap().seq;
        assert!(end > 0, "seq did not advance past 0");
        // Matching generation: `since` honored -> a small delta from `end`.
        let matched = registry
            .get_or_create_for_ws(
                Some(&id),
                Some(end),
                opts_with_window("win-gate"),
                None,
                None,
                Some(gen),
            )
            .unwrap();
        // Mismatched generation: `since` ignored -> full ring replay. The full
        // replay carries the pre-`end` history the delta omits, so it is
        // strictly larger -- robust to any trailing prompt output both observe.
        let mismatched = registry
            .get_or_create_for_ws(
                Some(&id),
                Some(end),
                opts_with_window("win-gate"),
                None,
                None,
                Some(gen + 1),
            )
            .unwrap();
        assert!(
            mismatched.replay.concat().len() > matched.replay.concat().len(),
            "a mismatched generation must replay more (full ring) than the matched delta"
        );
        registry.close_all(CloseReason::Shutdown);
    }

    #[test]
    fn cross_window_move_rebinds_window_and_survives_source_discard() {
        // Move invariant: a terminal dragged from window A to window B must
        // re-home to B on reattach, so A's discard (it emptied out) does NOT
        // reap the moved session — only sessions STILL bound to A.
        let registry = Registry::new(test_config(1024, 4, 10));
        // Opened in window A...
        let handle = registry.create(opts_with_window("win-a")).unwrap();
        let id = handle.id().to_string();
        drop(handle); // the move detaches it from the source

        // ...dragged to window B: B reattaches by id with window_id=B.
        let reattached = registry
            .get_or_create_for_ws(
                Some(&id),
                Some(0),
                opts_with_window("win-b"),
                None,
                None,
                None,
            )
            .expect("reattach");
        assert_eq!(
            reattached.id(),
            id,
            "reattached the SAME session (no respawn)"
        );
        drop(reattached);

        // The SOURCE window A discards. It must reap nothing — the session
        // moved to B.
        assert_eq!(
            registry.forget_window("win-a"),
            0,
            "discarding the source must not reap the moved session"
        );
        assert_eq!(registry.len(), 1, "the moved session survives");
        assert!(registry.attach(&id, None).is_some(), "moved PTY still live");

        // Discarding B (its true owner now) reaps it.
        assert_eq!(registry.forget_window("win-b"), 1);
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn cap_exceeded_refuses_create() {
        let registry = Registry::new(test_config(1024, 1, 10));
        let _first = registry
            .create(CreateOptions {
                size: test_size(),
                tab_name: None,
                tab_group: None,
                window_id: None,
                mcp_env: true,
                cwd: None,
                command: None,
                env: Default::default(),
            })
            .unwrap();
        let err = registry
            .create(CreateOptions {
                size: test_size(),
                tab_name: None,
                tab_group: None,
                window_id: None,
                mcp_env: true,
                cwd: None,
                command: None,
                env: Default::default(),
            })
            .unwrap_err();
        assert!(matches!(err, CreateError::Capped));
    }

    #[test]
    fn fd_headroom_keeps_terminal_spawns_away_from_process_limit() {
        assert!(fd_headroom_allows(100, 256, TERMINAL_SESSION_FD_ESTIMATE));
        assert!(!fd_headroom_allows(216, 256, TERMINAL_SESSION_FD_ESTIMATE));
    }

    #[test]
    fn get_or_create_without_session_id_creates_fresh_even_for_same_window_and_tab_name() {
        let registry = Registry::new(test_config(1024, 4, 10));
        let first = registry
            .create(CreateOptions {
                size: test_size(),
                tab_name: Some("B19v2".into()),
                tab_group: None,
                window_id: Some("window-a".into()),
                mcp_env: true,
                cwd: None,
                command: None,
                env: Default::default(),
            })
            .unwrap();
        let first_id = first.id().to_string();

        let second = registry
            .get_or_create(
                None,
                Some(0),
                CreateOptions {
                    size: test_size(),
                    tab_name: Some("B19v2".into()),
                    tab_group: None,
                    window_id: Some("window-a".into()),
                    mcp_env: true,
                    cwd: None,
                    command: None,
                    env: Default::default(),
                },
            )
            .unwrap();

        assert_ne!(second.id(), first_id);
        assert_eq!(registry.len(), 2);
        registry.close(&first_id, CloseReason::Explicit);
        registry.close(second.id(), CloseReason::Explicit);
    }

    #[test]
    fn get_or_create_without_session_id_does_not_match_ambiguous_window_tab_identity() {
        let registry = Registry::new(test_config(1024, 4, 10));
        let first = registry
            .create(CreateOptions {
                size: test_size(),
                tab_name: Some("dup".into()),
                tab_group: None,
                window_id: Some("window-a".into()),
                mcp_env: true,
                cwd: None,
                command: None,
                env: Default::default(),
            })
            .unwrap();
        let second = registry
            .create(CreateOptions {
                size: test_size(),
                tab_name: Some("dup".into()),
                tab_group: None,
                window_id: Some("window-a".into()),
                mcp_env: true,
                cwd: None,
                command: None,
                env: Default::default(),
            })
            .unwrap();

        let third = registry
            .get_or_create(
                None,
                Some(0),
                CreateOptions {
                    size: test_size(),
                    tab_name: Some("dup".into()),
                    tab_group: None,
                    window_id: Some("window-a".into()),
                    mcp_env: true,
                    cwd: None,
                    command: None,
                    env: Default::default(),
                },
            )
            .unwrap();

        assert_ne!(third.id(), first.id());
        assert_ne!(third.id(), second.id());
        assert_eq!(registry.len(), 3);
        registry.close(first.id(), CloseReason::Explicit);
        registry.close(second.id(), CloseReason::Explicit);
        registry.close(third.id(), CloseReason::Explicit);
    }

    #[tokio::test]
    async fn spawn_uses_configured_default_term() {
        // TERM env var on the spawned shell honors
        // `TerminalConfig::default_term`. A bare `printf "$TERM"`
        // command exits immediately so the captured tail of output
        // contains the env value we set, not interactive shell noise.
        let mut config = test_config(4096, 4, 60);
        config.terminal.default_term = "tmux-256color".into();
        let registry = Arc::new(Registry::new(config));
        let mut handle = registry
            .create(CreateOptions {
                size: test_size(),
                tab_name: None,
                tab_group: None,
                window_id: None,
                mcp_env: false,
                cwd: None,
                command: Some("printf 'TERM=<%s>\\n' \"$TERM\"".into()),
                env: Default::default(),
            })
            .unwrap();

        let out = collect_until(&mut handle, "TERM=<tmux-256color>", Duration::from_secs(5)).await;
        assert!(
            out.contains("TERM=<tmux-256color>"),
            "PTY did not echo configured TERM: {out:?}"
        );
        registry.close(handle.id(), CloseReason::Explicit);
    }

    #[tokio::test]
    async fn tenant_default_command_runs_when_session_omits_one() {
        // A tenant default command set after construction runs on a session
        // that brings none of its own, so a single-purpose terminal window's
        // PTY runs the given command instead of an interactive shell.
        let registry = Arc::new(Registry::new(test_config(4096, 4, 60)));
        registry.set_default_command(Some("printf 'DEFAULT=<ran>\\n'".into()));
        let mut handle = registry
            .create(CreateOptions {
                size: test_size(),
                tab_name: None,
                tab_group: None,
                window_id: None,
                mcp_env: false,
                cwd: None,
                command: None,
                env: Default::default(),
            })
            .unwrap();
        let out = collect_until(&mut handle, "DEFAULT=<ran>", Duration::from_secs(5)).await;
        assert!(
            out.contains("DEFAULT=<ran>"),
            "tenant default command did not run: {out:?}"
        );
        registry.close(handle.id(), CloseReason::Explicit);
    }

    #[tokio::test]
    async fn explicit_command_overrides_tenant_default() {
        // An explicit per-session command wins over the tenant default.
        let registry = Arc::new(Registry::new(test_config(4096, 4, 60)));
        registry.set_default_command(Some("printf 'PICK=<default>\\n'".into()));
        let mut handle = registry
            .create(CreateOptions {
                size: test_size(),
                tab_name: None,
                tab_group: None,
                window_id: None,
                mcp_env: false,
                cwd: None,
                command: Some("printf 'PICK=<explicit>\\n'".into()),
                env: Default::default(),
            })
            .unwrap();
        let out = collect_until(&mut handle, "PICK=<explicit>", Duration::from_secs(5)).await;
        assert!(
            out.contains("PICK=<explicit>"),
            "explicit command did not win over tenant default: {out:?}"
        );
        assert!(
            !out.contains("PICK=<default>"),
            "tenant default ran despite an explicit command: {out:?}"
        );
        registry.close(handle.id(), CloseReason::Explicit);
    }

    #[test]
    fn control_tenant_session_echoes_command_banner_first() {
        // A session that inherits the TENANT default command (the devserver
        // control / single-purpose tenant) writes the bare `{command}\r\n` as the
        // FIRST ring bytes — before the child's output and so durable across a
        // scrollback replay. The banner is the bare script + newline (no prefix)
        // so the command's own output begins on the next line.
        let registry = Registry::new(test_config(4096, 4, 60));
        registry.set_default_command(Some("printf done".into()));
        let _handle = registry.create(opts_with_window("win-ctl")).unwrap();
        let ring = registry.all_scrollback();
        assert!(
            ring.starts_with(b"printf done\r\n"),
            "command banner must be the first ring bytes: {:?}",
            String::from_utf8_lossy(&ring)
        );
        assert!(
            !ring.starts_with(b"running:"),
            "banner must carry no `running:` prefix: {:?}",
            String::from_utf8_lossy(&ring)
        );
        registry.close_all(CloseReason::Shutdown);
    }

    #[test]
    fn shared_tenant_session_has_no_command_banner() {
        // The shared interactive tenant has no default command, so its session
        // runs the user's shell and gets NO banner — the announce path never
        // fires, so the ring never leads with an injected `{command}\r\n` line
        // (a degenerate empty banner would lead with `\r\n`).
        let registry = Registry::new(test_config(4096, 4, 60));
        let _handle = registry.create(opts_with_window("win-sh")).unwrap();
        let ring = registry.all_scrollback();
        assert!(
            !ring.starts_with(b"\r\n"),
            "shared interactive terminal must have no banner: {:?}",
            String::from_utf8_lossy(&ring)
        );
        registry.close_all(CloseReason::Shutdown);
    }

    #[test]
    fn per_session_command_has_no_command_banner() {
        // A per-session command (a team agent terminal spawned via
        // `POST /api/terminals`) is NOT a single-purpose tenant — the command did
        // not come from the tenant default, so it gets NO banner: the ring must
        // not lead with the bare `printf agent\r\n` echo a control tenant would
        // inject.
        let registry = Registry::new(test_config(4096, 4, 60));
        let mut opts = opts_with_window("win-agent");
        opts.command = Some("printf agent".into());
        let _handle = registry.create(opts).unwrap();
        let ring = registry.all_scrollback();
        assert!(
            !ring.starts_with(b"printf agent\r\n"),
            "a per-session command must have no banner: {:?}",
            String::from_utf8_lossy(&ring)
        );
        registry.close_all(CloseReason::Shutdown);
    }

    #[cfg(unix)]
    #[test]
    fn user_shell_resolves_to_a_nonempty_executable() {
        // The single-sourced resolver: $SHELL → passwd → /bin/sh, each validated
        // executable. Whatever it returns must be a real, runnable shell.
        use std::os::unix::fs::PermissionsExt;
        let shell = user_shell();
        assert!(!shell.is_empty(), "resolver returned an empty shell");
        let path = std::path::Path::new(&shell);
        assert!(path.is_absolute(), "shell path should be absolute: {shell}");
        let meta = std::fs::metadata(path).expect("resolved shell exists on disk");
        assert!(
            meta.permissions().mode() & 0o111 != 0,
            "resolved shell is executable: {shell}"
        );
    }

    #[tokio::test]
    async fn all_scrollback_returns_session_output() {
        let registry = Arc::new(Registry::new(test_config(4096, 4, 60)));
        let mut handle = registry
            .create(CreateOptions {
                size: test_size(),
                tab_name: None,
                tab_group: None,
                window_id: None,
                mcp_env: false,
                cwd: None,
                command: Some("printf 'SCRAPE=<tok123>\\n'".into()),
                env: Default::default(),
            })
            .unwrap();
        let _ = collect_until(&mut handle, "SCRAPE=<tok123>", Duration::from_secs(5)).await;
        let text = String::from_utf8_lossy(&registry.all_scrollback()).into_owned();
        assert!(
            text.contains("SCRAPE=<tok123>"),
            "all_scrollback missing the session output: {text:?}"
        );
        registry.close(handle.id(), CloseReason::Explicit);
    }

    #[test]
    fn workspace_close_removes_sessions() {
        let registry = Registry::new(test_config(1024, 4, 10));
        let handle = registry
            .create(CreateOptions {
                size: test_size(),
                tab_name: None,
                tab_group: None,
                window_id: None,
                mcp_env: true,
                cwd: None,
                command: None,
                env: Default::default(),
            })
            .unwrap();
        let id = handle.id().to_string();
        registry.close_all(CloseReason::Workspace);
        assert_eq!(registry.len(), 0);
        assert!(registry.attach(&id, None).is_none());
    }

    #[tokio::test]
    async fn two_attaches_share_io() {
        let registry = Registry::new(test_config(4096, 4, 60));
        let first = registry
            .create(CreateOptions {
                size: PtySize {
                    rows: 24,
                    cols: 80,
                    pixel_width: 0,
                    pixel_height: 0,
                },
                tab_name: None,
                tab_group: None,
                window_id: None,
                mcp_env: true,
                cwd: None,
                command: None,
                env: Default::default(),
            })
            .unwrap();
        let mut second = registry.attach(first.id(), Some(first.seq)).unwrap();
        first.send_input(b"printf '\\n__SHARED__\\n'\r");
        let mut saw = false;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        while tokio::time::Instant::now() < deadline {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            match tokio::time::timeout(remaining, second.rx.recv()).await {
                Ok(Ok(SessionEvent::Output(bytes))) => {
                    if String::from_utf8_lossy(&bytes).contains("__SHARED__") {
                        saw = true;
                        break;
                    }
                }
                Ok(Ok(_)) => {}
                Ok(Err(_)) | Err(_) => break,
            }
        }
        assert!(saw, "second attach did not receive output from first input");
        registry.close(first.id(), CloseReason::Explicit);
    }

    #[tokio::test]
    async fn request_redraw_broadcasts_current_size() {
        let registry = Registry::new(test_config(4096, 4, 60));
        let first = registry
            .create(CreateOptions {
                size: test_size(),
                tab_name: None,
                tab_group: None,
                window_id: None,
                mcp_env: true,
                cwd: None,
                command: None,
                env: Default::default(),
            })
            .unwrap();
        let mut second = registry.attach(first.id(), Some(first.seq)).unwrap();
        second.request_redraw();

        let mut saw = false;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        while tokio::time::Instant::now() < deadline {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            match tokio::time::timeout(remaining, second.rx.recv()).await {
                Ok(Ok(SessionEvent::Resize(size))) => {
                    saw = size.rows == test_size().rows && size.cols == test_size().cols;
                    if saw {
                        break;
                    }
                }
                Ok(Ok(_)) => {}
                Ok(Err(_)) | Err(_) => break,
            }
        }
        assert!(saw, "redraw did not re-apply the current PTY size");
        registry.close(first.id(), CloseReason::Explicit);
    }

    /// A registry session with a controllable id / window / group /
    /// broadcast flag and NO real PTY. `send_input` still bumps
    /// `last_activity` (the PTY write fails silently because the command
    /// receiver is dropped), so a delivery is observable as a bumped
    /// `last_activity` without spawning a shell.
    fn dummy_session(
        id: &str,
        window_id: Option<&str>,
        tab_group: Option<&str>,
        broadcast: bool,
    ) -> Arc<Session> {
        // `test_session_with_ring` already drops the command/output
        // receivers, so `send_input` fails silently but still bumps
        // `last_activity` (the delivery signal). Sole owner, so unwrap to
        // set the fields the cross-window fan reads (private, same module).
        let mut s = Arc::try_unwrap(test_session_with_ring(64)).expect("sole owner");
        s.id = id.to_string();
        s.window_id = Mutex::new(window_id.map(str::to_string));
        s.tab_group = tab_group.map(str::to_string);
        // Sentinel: 0 is distinguishable from any real `now_unix_secs()`.
        s.last_activity = AtomicI64::new(0);
        s.broadcast = AtomicBool::new(broadcast);
        Arc::new(s)
    }

    fn insert_session(registry: &Registry, session: Arc<Session>) {
        registry
            .sessions
            .lock()
            .unwrap()
            .insert(session.id.clone(), session);
    }

    /// A live session carrying a `tab_name`, for exercising the lowest-free
    /// `next_terminal_name` scan (which reads `tab_name`, not `id`).
    fn named_session(id: &str, tab_name: &str) -> Arc<Session> {
        let mut s = Arc::try_unwrap(test_session_with_ring(64)).expect("sole owner");
        s.id = id.to_string();
        s.tab_name = Some(tab_name.to_string());
        Arc::new(s)
    }

    fn was_delivered(registry: &Registry, id: &str) -> bool {
        registry
            .sessions
            .lock()
            .unwrap()
            .get(id)
            .map(|s| s.last_activity.load(Ordering::Relaxed) != 0)
            .unwrap_or(false)
    }

    #[test]
    fn cross_window_fan_respects_group_window_and_broadcast_toggle() {
        let registry = Registry::new(test_config(64, 16, 600));
        // Source in window A, group G.
        insert_session(
            &registry,
            dummy_session("src", Some("winA"), Some("G"), true),
        );
        // Same group, other window, broadcast ON -> receives.
        insert_session(
            &registry,
            dummy_session("on", Some("winB"), Some("G"), true),
        );
        // Same group, other window, broadcast OFF -> skipped (the fix).
        insert_session(
            &registry,
            dummy_session("off", Some("winB"), Some("G"), false),
        );
        // Other group, other window, broadcast ON -> skipped (wrong group).
        insert_session(
            &registry,
            dummy_session("other_group", Some("winB"), Some("H"), true),
        );
        // Same group, SAME window -> skipped (fanned client-side).
        insert_session(
            &registry,
            dummy_session("same_window", Some("winA"), Some("G"), true),
        );

        registry.broadcast_input_cross_window("src", b"hi");

        assert!(
            was_delivered(&registry, "on"),
            "broadcast-on member should receive"
        );
        assert!(
            !was_delivered(&registry, "off"),
            "broadcast-off member must not receive"
        );
        assert!(
            !was_delivered(&registry, "other_group"),
            "other-group member must not receive"
        );
        assert!(
            !was_delivered(&registry, "same_window"),
            "same-window member is handled client-side, not here"
        );
        assert!(
            !was_delivered(&registry, "src"),
            "source must not echo to itself"
        );
    }

    #[test]
    fn next_terminal_name_is_per_tenant() {
        let one = Registry::new(test_config(64, 16, 600));
        let two = Registry::new(test_config(64, 16, 600));
        insert_session(&one, named_session("a", "Terminal-1"));
        // A second tenant has its own numbering (the bug a process-global
        // static caused: a second workspace window restarting past 1). It is
        // unaffected by `one`'s live terminals.
        assert_eq!(two.next_terminal_name(), "Terminal-1");
        // `one` already has Terminal-1 live -> next is 2.
        assert_eq!(one.next_terminal_name(), "Terminal-2");
    }

    #[test]
    fn next_terminal_name_reuses_the_lowest_free_slot() {
        let reg = Registry::new(test_config(64, 16, 600));
        // Empty registry starts at 1.
        assert_eq!(reg.next_terminal_name(), "Terminal-1");
        // Two live terminals -> next extends past the max.
        insert_session(&reg, named_session("a", "Terminal-1"));
        insert_session(&reg, named_session("b", "Terminal-2"));
        assert_eq!(reg.next_terminal_name(), "Terminal-3");
        // Free the middle one -> its number is REUSED (the numbering regression:
        // open 1+2, close 2, next should be 2, not 3).
        reg.sessions.lock().unwrap().remove("b");
        assert_eq!(reg.next_terminal_name(), "Terminal-2");
        // A gap below the max is filled before extending: live {1, 3} -> 2.
        insert_session(&reg, named_session("c", "Terminal-3"));
        assert_eq!(reg.next_terminal_name(), "Terminal-2");
        // Non-default names never occupy a slot; bare "Terminal" counts as 1.
        let reg2 = Registry::new(test_config(64, 16, 600));
        insert_session(&reg2, named_session("x", "build"));
        insert_session(&reg2, named_session("y", "Terminal"));
        assert_eq!(reg2.next_terminal_name(), "Terminal-2");
    }

    #[test]
    fn parse_terminal_ordinal_parses_default_names_only() {
        assert_eq!(parse_terminal_ordinal("Terminal-1"), Some(1));
        assert_eq!(parse_terminal_ordinal("Terminal-12"), Some(12));
        assert_eq!(parse_terminal_ordinal("Terminal"), Some(1));
        assert_eq!(parse_terminal_ordinal("build"), None);
        assert_eq!(parse_terminal_ordinal("lead-2"), None);
        assert_eq!(parse_terminal_ordinal("Terminal-"), None);
        assert_eq!(parse_terminal_ordinal("Terminal-1x"), None);
        assert_eq!(parse_terminal_ordinal("Terminal-0"), None);
    }

    #[test]
    fn roster_reports_window_group_and_broadcast() {
        let registry = Registry::new(test_config(64, 16, 600));
        insert_session(&registry, dummy_session("a", Some("winA"), Some("G"), true));
        insert_session(&registry, dummy_session("b", Some("winB"), None, false));

        let mut roster = registry.roster();
        roster.sort_by(|x, y| x.id.cmp(&y.id));
        assert_eq!(roster.len(), 2);

        assert_eq!(roster[0].id, "a");
        assert_eq!(roster[0].window_id.as_deref(), Some("winA"));
        assert_eq!(roster[0].tab_group, "G");
        assert!(roster[0].broadcast);

        assert_eq!(roster[1].id, "b");
        // No explicit group resolves to the default, matching the SPA.
        assert_eq!(roster[1].tab_group, DEFAULT_TERMINAL_GROUP);
        assert!(!roster[1].broadcast);
    }
}
