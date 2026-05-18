//! Long-lived PTY session registry.
//!
//! A terminal WebSocket is only an attachment. The PTY, child process,
//! replay ring, and lifecycle policy live here so browser reloads can
//! detach and reattach without killing the shell.

use std::collections::{HashMap, VecDeque};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, Weak};
use std::time::Duration;

use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use rand::RngCore;
use serde::Serialize;
use tokio::sync::{broadcast, watch};
use tokio::task::JoinHandle;

use crate::config::TerminalConfig;
use crate::event_watcher::{AgentEvent, EventWatcherHandle};
use crate::signal::now_unix_secs;

#[cfg(target_os = "macos")]
use std::process::Command;

const BROADCAST_CAP: usize = 1024;
const ALT_SCREEN_ENTER: &[u8] = b"\x1b[?1049h";
const ALT_SCREEN_EXIT: &[u8] = b"\x1b[?1049l";
const ALT_SCREEN_TAIL_BYTES: usize = ALT_SCREEN_ENTER.len() - 1;
const REDRAW_WOBBLE_DELAY: Duration = Duration::from_millis(50);

pub const ALT_SCREEN_ATTACH_PRELUDE: &[u8] = b"\x1b[?1049h\x1b[2J\x1b[H";

#[derive(Debug, Clone)]
pub struct RegistryConfig {
    pub drive_root: PathBuf,
    pub mcp_socket_path: Option<PathBuf>,
    pub control_socket_path: Option<PathBuf>,
    pub terminal: TerminalConfig,
}

#[derive(Debug)]
pub struct Registry {
    config: RegistryConfig,
    sessions: Mutex<HashMap<String, Arc<Session>>>,
    watcher_dropped_events: Arc<AtomicU64>,
}

#[derive(Debug, Clone)]
pub struct CreateOptions {
    pub size: PtySize,
    pub tab_name: Option<String>,
    pub window_id: Option<String>,
    pub mcp_env: bool,
    pub cwd: Option<PathBuf>,
}

#[derive(Debug)]
pub enum CreateError {
    Capped,
    Spawn(anyhow::Error),
}

impl std::fmt::Display for CreateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CreateError::Capped => f.write_str("terminal session cap reached"),
            CreateError::Spawn(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for CreateError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CloseReason {
    Idle,
    Drive,
    Shutdown,
    Explicit,
    Capped,
}

impl CloseReason {
    pub fn as_str(self) -> &'static str {
        match self {
            CloseReason::Idle => "idle",
            CloseReason::Drive => "drive",
            CloseReason::Shutdown => "shutdown",
            CloseReason::Explicit => "explicit",
            CloseReason::Capped => "capped",
        }
    }
}

#[derive(Debug, Clone)]
pub enum SessionEvent {
    Output(Vec<u8>),
    Resize(PtySize),
    Exit(u32),
    Error(String),
    Closed(CloseReason),
}

#[derive(Debug)]
pub struct AttachHandle {
    id: String,
    session: Arc<Session>,
    pub rx: broadcast::Receiver<SessionEvent>,
    pub replay: Vec<Vec<u8>>,
    pub seq: u64,
    pub missed_bytes: u64,
    pub alt_screen: bool,
}

impl AttachHandle {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn send_input(&self, data: &[u8]) {
        self.session.send_input(data);
    }

    pub fn resize(&self, size: PtySize) {
        self.session.resize(size);
    }

    pub fn request_redraw(&self) {
        self.session.request_redraw();
    }

    pub fn cwd(&self) -> Option<PathBuf> {
        self.session.cwd()
    }
}

impl Drop for AttachHandle {
    fn drop(&mut self) {
        self.session.attach_count.fetch_sub(1, Ordering::Relaxed);
    }
}

impl Registry {
    pub fn new(config: RegistryConfig) -> Self {
        Self {
            config,
            sessions: Mutex::new(HashMap::new()),
            watcher_dropped_events: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn create(&self, opts: CreateOptions) -> Result<AttachHandle, CreateError> {
        let mut sessions = self.sessions.lock().expect("terminal registry poisoned");
        if sessions.len() >= self.config.terminal.session_cap {
            return Err(CreateError::Capped);
        }
        let id = self.unused_id(&sessions);
        let session =
            Session::spawn(id.clone(), self.config.clone(), opts).map_err(CreateError::Spawn)?;
        sessions.insert(id.clone(), session.clone());
        Ok(session.attach(Some(0)))
    }

    pub fn attach(&self, id: &str, since: Option<u64>) -> Option<AttachHandle> {
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

    pub fn get_or_create(
        &self,
        id: Option<&str>,
        since: Option<u64>,
        opts: CreateOptions,
    ) -> Result<AttachHandle, CreateError> {
        if let Some(id) = id {
            if let Some(handle) = self.attach(id, since) {
                return Ok(handle);
            }
        }
        self.create(opts)
    }

    pub fn close(&self, id: &str, reason: CloseReason) -> bool {
        let session = self
            .sessions
            .lock()
            .expect("terminal registry poisoned")
            .remove(id);
        if let Some(session) = session {
            session.close(reason);
            true
        } else {
            false
        }
    }

    pub fn remove(&self, id: &str) -> bool {
        self.sessions
            .lock()
            .expect("terminal registry poisoned")
            .remove(id)
            .is_some()
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
    }

    pub fn set_watcher(self: &Arc<Self>, id: &str, dir: PathBuf) -> anyhow::Result<bool> {
        let session = self
            .sessions
            .lock()
            .expect("terminal registry poisoned")
            .get(id)
            .cloned();
        let Some(session) = session else {
            return Ok(false);
        };
        let weak = Arc::downgrade(self);
        let dispatch = Arc::new(move |event: AgentEvent| {
            if let Some(registry) = Weak::upgrade(&weak) {
                registry.dispatch_agent_event(event);
            }
        });
        let watcher =
            EventWatcherHandle::start(dir, dispatch, self.watcher_dropped_events.clone())?;
        *session.watcher.lock().expect("terminal watcher poisoned") = Some(watcher);
        Ok(true)
    }

    pub fn clear_watcher(&self, id: &str) -> bool {
        let session = self
            .sessions
            .lock()
            .expect("terminal registry poisoned")
            .get(id)
            .cloned();
        if let Some(session) = session {
            session
                .watcher
                .lock()
                .expect("terminal watcher poisoned")
                .take()
                .is_some()
        } else {
            false
        }
    }

    pub fn watcher_dropped_events(&self) -> u64 {
        self.watcher_dropped_events.load(Ordering::Relaxed)
    }

    pub fn prune_idle(&self) -> usize {
        self.prune_idle_at(now_unix_secs() as i64)
    }

    pub fn prune_idle_at(&self, now: i64) -> usize {
        let idle_timeout = self.config.terminal.idle_timeout_secs as i64;
        let to_close: Vec<String> = {
            let sessions = self.sessions.lock().expect("terminal registry poisoned");
            sessions
                .iter()
                .filter_map(|(id, session)| {
                    let attached = session.attach_count.load(Ordering::Relaxed);
                    let last = session.last_activity.load(Ordering::Relaxed);
                    if attached == 0 && now.saturating_sub(last) > idle_timeout {
                        Some(id.clone())
                    } else {
                        None
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
                        self.prune_idle();
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

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.sessions
            .lock()
            .expect("terminal registry poisoned")
            .len()
    }

    fn dispatch_agent_event(&self, event: AgentEvent) {
        let Some(session) = self.find_agent_session(&event.to) else {
            self.watcher_dropped_events.fetch_add(1, Ordering::Relaxed);
            tracing::warn!(
                id = %event.id,
                from = %event.from,
                to = %event.to,
                "dropping agent event with no matching terminal session"
            );
            return;
        };
        // TODO: wire /clear, /effort, and /fast automation here once
        // @@Alex's richer control commands are cut for a later task.
        session.send_input(b"poke\n");
    }

    fn find_agent_session(&self, target: &str) -> Option<Arc<Session>> {
        let normalized_target = normalize_agent_target(target)?;
        let sessions = self.sessions.lock().expect("terminal registry poisoned");
        sessions
            .values()
            .find(|session| {
                session
                    .tab_name
                    .as_deref()
                    .and_then(normalize_agent_target)
                    .as_deref()
                    == Some(normalized_target.as_str())
            })
            .cloned()
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

#[derive(Debug)]
struct Session {
    id: String,
    tab_name: Option<String>,
    drive_root: PathBuf,
    child_pid: Option<u32>,
    command_tx: std::sync::mpsc::Sender<PtyCommand>,
    output_tx: broadcast::Sender<SessionEvent>,
    ring: Mutex<RingBuffer>,
    seq: AtomicU64,
    last_activity: AtomicI64,
    attach_count: AtomicUsize,
    winsize: Mutex<PtySize>,
    in_alt_screen: AtomicBool,
    alt_screen_tail: Mutex<Vec<u8>>,
    watcher: Mutex<Option<EventWatcherHandle>>,
    closed: AtomicBool,
}

impl Session {
    fn spawn(id: String, config: RegistryConfig, opts: CreateOptions) -> anyhow::Result<Arc<Self>> {
        let pty_system = native_pty_system();
        let pair = pty_system.openpty(opts.size)?;
        let mut cmd = CommandBuilder::new_default_prog();
        let cwd = opts.cwd.unwrap_or_else(|| config.drive_root.clone());
        cmd.cwd(cwd);
        if let Some(home) = terminal_home_dir() {
            cmd.env("HOME", &home);
            #[cfg(windows)]
            cmd.env("USERPROFILE", home);
        }
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");
        cmd.env("CLICOLOR", "1");
        cmd.env("CLICOLOR_FORCE", "1");
        cmd.env("FORCE_COLOR", "3");
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
        if let Some(window_id) = opts.window_id {
            cmd.env("CHAN_WINDOW_ID", window_id);
        }
        if let Some(socket_path) = config.control_socket_path.as_deref() {
            if let Some(socket) = socket_path.to_str() {
                cmd.env("CHAN_CONTROL_SOCKET", socket);
            }
        }
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
            drive_root: config.drive_root.clone(),
            child_pid,
            command_tx,
            output_tx,
            ring: Mutex::new(RingBuffer::new(config.terminal.ring_bytes)),
            seq: AtomicU64::new(0),
            last_activity: AtomicI64::new(now_unix_secs() as i64),
            attach_count: AtomicUsize::new(0),
            winsize: Mutex::new(opts.size),
            in_alt_screen: AtomicBool::new(false),
            alt_screen_tail: Mutex::new(Vec::new()),
            watcher: Mutex::new(None),
            closed: AtomicBool::new(false),
        });

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
                            session.broadcast(SessionEvent::Exit(status.exit_code()));
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
        AttachHandle {
            id: self.id.clone(),
            session: self,
            rx,
            replay,
            seq,
            missed_bytes,
            alt_screen,
        }
    }

    fn send_input(&self, data: &[u8]) {
        self.last_activity
            .store(now_unix_secs() as i64, Ordering::Relaxed);
        let _ = self.command_tx.send(PtyCommand::Input(data.to_vec()));
    }

    fn resize(&self, size: PtySize) {
        let _ = self.command_tx.send(PtyCommand::Resize(size));
    }

    fn request_redraw(&self) {
        let _ = self.command_tx.send(PtyCommand::Redraw);
    }

    fn cwd(&self) -> Option<PathBuf> {
        let cwd = process_cwd(self.child_pid?)?;
        path_inside_root(&cwd, &self.drive_root).then_some(cwd)
    }

    fn close(&self, reason: CloseReason) {
        if self.closed.swap(true, Ordering::Relaxed) {
            return;
        }
        self.watcher
            .lock()
            .expect("terminal watcher poisoned")
            .take();
        self.broadcast(SessionEvent::Closed(reason));
        let _ = self.command_tx.send(PtyCommand::Kill);
    }

    fn record_output(&self, bytes: &[u8]) {
        if bytes.is_empty() {
            return;
        }
        self.last_activity
            .store(now_unix_secs() as i64, Ordering::Relaxed);
        self.update_alt_screen(bytes);
        let end_seq = {
            let mut ring = self.ring.lock().expect("terminal ring poisoned");
            ring.push(bytes);
            ring.end_seq()
        };
        self.seq.store(end_seq, Ordering::Relaxed);
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
}

fn normalize_agent_target(name: &str) -> Option<String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return None;
    }
    let bare = trimmed.strip_prefix("@@").unwrap_or(trimmed).trim();
    if bare.is_empty() {
        return None;
    }
    Some(
        bare.chars()
            .filter(|c| !c.is_ascii_whitespace() && *c != '-' && *c != '_')
            .flat_map(char::to_lowercase)
            .collect(),
    )
}

fn path_inside_root(path: &Path, root: &Path) -> bool {
    let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    path == root || path.starts_with(root)
}

#[cfg(target_os = "linux")]
fn process_cwd(pid: u32) -> Option<PathBuf> {
    std::fs::read_link(format!("/proc/{pid}/cwd")).ok()
}

#[cfg(target_os = "macos")]
fn process_cwd(pid: u32) -> Option<PathBuf> {
    let output = Command::new("/usr/sbin/lsof")
        .args(["-a", "-d", "cwd", "-Fn", "-p", &pid.to_string()])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .find_map(|line| line.strip_prefix('n'))
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn process_cwd(_pid: u32) -> Option<PathBuf> {
    None
}

enum PtyCommand {
    Input(Vec<u8>),
    Resize(PtySize),
    Redraw,
    Kill,
}

fn contains_subslice(haystack: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty() && haystack.windows(needle.len()).any(|w| w == needle)
}

fn force_redraw_with_wobble<E>(
    original: PtySize,
    delay: Duration,
    mut resize: impl FnMut(PtySize) -> Result<(), E>,
) -> Result<(), E> {
    let wobble = redraw_wobble_size(original);
    resize(wobble)?;
    std::thread::sleep(delay);
    resize(original)
}

fn redraw_wobble_size(original: PtySize) -> PtySize {
    let rows = if original.rows > 1 {
        original.rows - 1
    } else {
        original.rows.saturating_add(1)
    };
    PtySize { rows, ..original }
}

#[derive(Debug)]
struct RingBuffer {
    cap: usize,
    chunks: VecDeque<(u64, Vec<u8>)>,
    start_seq: u64,
    end_seq: u64,
    len: usize,
}

impl RingBuffer {
    fn new(cap: usize) -> Self {
        Self {
            cap: cap.max(1),
            chunks: VecDeque::new(),
            start_seq: 0,
            end_seq: 0,
            len: 0,
        }
    }

    fn push(&mut self, bytes: &[u8]) {
        let start = self.end_seq;
        self.end_seq = self.end_seq.saturating_add(bytes.len() as u64);
        if bytes.len() >= self.cap {
            self.chunks.clear();
            let tail = bytes[bytes.len() - self.cap..].to_vec();
            self.start_seq = self.end_seq.saturating_sub(tail.len() as u64);
            self.len = tail.len();
            self.chunks.push_back((self.start_seq, tail));
            return;
        }
        self.len = self.len.saturating_add(bytes.len());
        self.chunks.push_back((start, bytes.to_vec()));
        while self.len > self.cap {
            if let Some((_start, chunk)) = self.chunks.pop_front() {
                self.len = self.len.saturating_sub(chunk.len());
                self.start_seq = self.start_seq.saturating_add(chunk.len() as u64);
            } else {
                self.start_seq = self.end_seq;
                self.len = 0;
                break;
            }
        }
    }

    fn end_seq(&self) -> u64 {
        self.end_seq
    }

    fn snapshot_since(&self, since: Option<u64>) -> (Vec<Vec<u8>>, u64) {
        let requested = since.unwrap_or(self.start_seq);
        let replay_start = requested.max(self.start_seq);
        let missed = self.start_seq.saturating_sub(requested);
        let mut replay = Vec::new();
        for (chunk_start, chunk) in &self.chunks {
            let chunk_end = chunk_start.saturating_add(chunk.len() as u64);
            if chunk_end <= replay_start {
                continue;
            }
            let offset = replay_start.saturating_sub(*chunk_start) as usize;
            replay.push(chunk[offset..].to_vec());
        }
        (replay, missed)
    }
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

pub(crate) fn set_mcp_env(cmd: &mut CommandBuilder, socket_path: &std::path::Path) {
    let Some(socket) = socket_path.to_str() else {
        return;
    };
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let Some(exe) = exe.to_str() else {
        return;
    };
    let argv_json = serde_json::json!([exe, "__mcp-proxy", socket]).to_string();
    let server_json = serde_json::json!({
        "name": "chan",
        "command": exe,
        "args": ["__mcp-proxy", socket],
    })
    .to_string();

    cmd.env("CHAN_MCP_SERVER_NAME", "chan");
    cmd.env("CHAN_MCP_SOCKET", socket);
    cmd.env("CHAN_MCP_COMMAND", format!("{exe} __mcp-proxy {socket}"));
    cmd.env("CHAN_MCP_COMMAND_JSON", argv_json);
    cmd.env("CHAN_MCP_SERVER_JSON", server_json);
}

fn clear_mcp_env(cmd: &mut CommandBuilder) {
    for key in [
        "CHAN_MCP_SERVER_NAME",
        "CHAN_MCP_SOCKET",
        "CHAN_MCP_COMMAND",
        "CHAN_MCP_COMMAND_JSON",
        "CHAN_MCP_SERVER_JSON",
        "CHAN_WINDOW_ID",
        "CHAN_CONTROL_SOCKET",
    ] {
        cmd.env_remove(key);
    }
}

pub(crate) fn terminal_home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(dirs::home_dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config(ring_bytes: usize, cap: usize, idle: u64) -> RegistryConfig {
        let tmp = tempfile::tempdir().unwrap();
        let drive_root = tmp.path().to_path_buf();
        std::mem::forget(tmp);
        RegistryConfig {
            drive_root,
            mcp_socket_path: None,
            control_socket_path: None,
            terminal: TerminalConfig {
                idle_timeout_secs: idle,
                session_cap: cap,
                ring_bytes,
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
            drive_root: PathBuf::from("/"),
            child_pid: None,
            command_tx,
            output_tx,
            ring: Mutex::new(RingBuffer::new(ring_bytes)),
            seq: AtomicU64::new(0),
            last_activity: AtomicI64::new(now_unix_secs() as i64),
            attach_count: AtomicUsize::new(0),
            winsize: Mutex::new(test_size()),
            in_alt_screen: AtomicBool::new(false),
            alt_screen_tail: Mutex::new(Vec::new()),
            watcher: Mutex::new(None),
            closed: AtomicBool::new(false),
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
                window_id: None,
                mcp_env: true,
                cwd: None,
            })
            .unwrap();
        let id = handle.id().to_string();
        drop(handle);
        assert_eq!(registry.len(), 1);
        assert_eq!(registry.prune_idle_at(now_unix_secs() as i64 + 11), 1);
        assert_eq!(registry.len(), 0);
        assert!(registry.attach(&id, None).is_none());
    }

    #[test]
    fn cap_exceeded_refuses_create() {
        let registry = Registry::new(test_config(1024, 1, 10));
        let _first = registry
            .create(CreateOptions {
                size: test_size(),
                tab_name: None,
                window_id: None,
                mcp_env: true,
                cwd: None,
            })
            .unwrap();
        let err = registry
            .create(CreateOptions {
                size: test_size(),
                tab_name: None,
                window_id: None,
                mcp_env: true,
                cwd: None,
            })
            .unwrap_err();
        assert!(matches!(err, CreateError::Capped));
    }

    #[tokio::test]
    async fn dispatch_agent_event_writes_poke_to_matching_tab() {
        let registry = Arc::new(Registry::new(test_config(4096, 4, 60)));
        let mut handle = registry
            .create(CreateOptions {
                size: test_size(),
                tab_name: Some("Systacean".into()),
                window_id: None,
                mcp_env: true,
                cwd: None,
            })
            .unwrap();

        registry.dispatch_agent_event(AgentEvent {
            id: "event-1".into(),
            event_type: crate::event_watcher::AgentEventType::Poke,
            from: "@@Architect".into(),
            to: "@@Systacean".into(),
            topic: None,
            questions: None,
            standing_options: None,
            scope: None,
            answers: None,
            scope_grant: None,
            note: None,
        });

        let out = collect_until(&mut handle, "poke", Duration::from_secs(5)).await;
        assert!(
            out.contains("poke"),
            "target terminal did not receive poke: {out:?}"
        );
        assert_eq!(registry.watcher_dropped_events(), 0);
        registry.close(handle.id(), CloseReason::Explicit);
    }

    #[test]
    fn dispatch_agent_event_counts_unmatched_targets() {
        let registry = Registry::new(test_config(4096, 4, 60));

        registry.dispatch_agent_event(AgentEvent {
            id: "event-1".into(),
            event_type: crate::event_watcher::AgentEventType::Poke,
            from: "@@Architect".into(),
            to: "@@Missing".into(),
            topic: None,
            questions: None,
            standing_options: None,
            scope: None,
            answers: None,
            scope_grant: None,
            note: None,
        });

        assert_eq!(registry.watcher_dropped_events(), 1);
    }

    #[test]
    fn drive_close_removes_sessions() {
        let registry = Registry::new(test_config(1024, 4, 10));
        let handle = registry
            .create(CreateOptions {
                size: test_size(),
                tab_name: None,
                window_id: None,
                mcp_env: true,
                cwd: None,
            })
            .unwrap();
        let id = handle.id().to_string();
        registry.close_all(CloseReason::Drive);
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
                window_id: None,
                mcp_env: true,
                cwd: None,
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
                window_id: None,
                mcp_env: true,
                cwd: None,
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
}
