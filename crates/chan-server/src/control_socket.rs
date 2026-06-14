//! First-party control socket for local `chan` CLI helpers.
//!
//! MCP stays scoped to workspace tools for external agents. This socket is
//! for UI commands from chan-spawned terminals, such as `cs open`,
//! where the command must target one frontend window in the already
//! running server process.

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock, RwLock};

#[cfg(unix)]
use chan_workspace::{TeamConfig, Workspace};
#[cfg(unix)]
use portable_pty::PtySize;
#[cfg(unix)]
use serde::Serialize;
#[cfg(unix)]
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
#[cfg(unix)]
use tokio::net::UnixListener;
use tokio::sync::broadcast;
#[cfg(unix)]
use tokio::task::JoinHandle;

#[cfg(unix)]
use crate::desktop_window_ops::DesktopWindowOp;
use crate::state::WorkspaceCell;
#[cfg(unix)]
use crate::terminal_sessions::CreateOptions;
use crate::terminal_sessions::Registry as TerminalRegistry;

/// Settable handle to the terminal registry. The registry is built after
/// the control socket starts (it needs the control socket path for
/// `$CHAN_CONTROL_SOCKET`), so the caller passes an empty cell here and
/// fills it once the registry exists. Category-2 requests
/// (`cs term write` / `term list`) read it.
pub type TerminalRegistryCell = Arc<OnceLock<Arc<TerminalRegistry>>>;

// The control-socket wire contract (request + response) is shared with
// the `cs` client through chan-shell, so a tag/field rename moves in
// lockstep instead of silently breaking one side. The server only touches
// these types on unix (the listener is unix-only).
#[cfg(unix)]
pub use chan_shell::{ControlRequest, ControlResponse};
// The survey types are part of the same shared wire module; the handler
// pushes a SurveySpec to the SPA and formats the SurveyReply for the CLI.
// TeamOp tags the `cs terminal team` op (new | load).
#[cfg(unix)]
use chan_shell::{submit_writes, PaneOp, SubmitAgent, SurveyReply, SurveySpec, TeamOp};

#[cfg(unix)]
#[derive(Debug, Serialize)]
#[serde(tag = "command", rename_all = "snake_case")]
// The shared `Open` prefix is the wire contract: serde renames each
// variant to its `open_*` command string that the SPA's
// `handleWindowCommand` matches on. Renaming to drop the prefix would
// rename the wire command and break the SPA.
#[allow(clippy::enum_variant_names)]
enum WindowCommand {
    OpenFile {
        path: String,
    },
    OpenBrowser {
        path: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        select: Option<String>,
        #[serde(skip_serializing_if = "is_false")]
        enter: bool,
    },
    OpenGraph {
        #[serde(skip_serializing_if = "Option::is_none")]
        path: Option<String>,
        is_dir: bool,
    },
    OpenTermNew {
        #[serde(skip_serializing_if = "Option::is_none")]
        cwd: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tab_name: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tab_group: Option<String>,
    },
    OpenDashboard {
        #[serde(skip_serializing_if = "Option::is_none")]
        carousel_index: Option<u32>,
        #[serde(skip_serializing_if = "is_false")]
        carousel_off: bool,
    },
    // Raise the `cs terminal survey` overlay. The SurveySpec nests under
    // `survey` (it is camelCase, unlike the snake_case sibling fields, so
    // nesting keeps the two conventions from mixing in one object). The SPA
    // reads `frame.survey` and POSTs a SurveyReply to the reply route.
    OpenSurvey {
        survey: SurveySpec,
        // The terminal this survey targets, so the SPA can anchor a
        // per-terminal survey instead of one window-wide modal. `cs terminal
        // survey --tab-name=X` -> Some(X); a `--tab-group` broadcast (or no
        // specific tab) -> None, where the SPA keeps its window-wide fallback.
        // camelCase `tabName` on the wire, pinned with serde(rename) so a green
        // compile can't hide a wire mismatch.
        #[serde(rename = "tabName", skip_serializing_if = "Option::is_none")]
        tab_name: Option<String>,
    },
    // `cs pane` layout query: the server asks the window for its current
    // tab/pane layout. The SPA reads its `layout` and POSTs the snapshot to
    // `POST /api/window/reply` echoing `request_id`, which fires the parked
    // window-bus oneshot. Not an `open_*` command (it queries, it does not
    // open a tab), hence the non-`Open` name and the `pane_query` wire tag.
    PaneQuery {
        request_id: String,
    },
    // `cs pane <exec>`: the server asks the window to APPLY a layout mutation
    // (focus / split / resize / close). The op nests under `op` (it is
    // internally tagged on `kind`, so the SPA reads `frame.op.kind`); the SPA
    // applies it and POSTs the result echoing `request_id`.
    PaneExec {
        request_id: String,
        op: PaneOp,
    },
    // A CLI `cs terminal team new|load` spawned a team server-side; tell
    // the window that owns it to SURFACE the agents by opening a terminal tab
    // per member ATTACHED to the already-spawned `session_id` (not a fresh
    // session), grouped under `group`. Fire-and-forget (no reply / no window
    // bus), like the `open_*` commands.
    TeamSpawned {
        group: String,
        members: Vec<SpawnedMember>,
    },
}

/// One spawned team member in a [`WindowCommand::TeamSpawned`]: the tab name
/// (the member handle) and the live `session_id` the SPA attaches its new
/// terminal tab to.
#[cfg(unix)]
#[derive(Debug, Clone, Serialize)]
struct SpawnedMember {
    tab_name: String,
    session_id: String,
}

#[cfg(unix)]
fn is_false(value: &bool) -> bool {
    !*value
}

#[cfg(unix)]
#[derive(Debug, Serialize)]
struct WindowCommandFrame {
    #[serde(rename = "type")]
    frame_type: &'static str,
    window_id: String,
    #[serde(flatten)]
    command: WindowCommand,
}

#[cfg(unix)]
pub struct ControlHandle {
    socket_path: PathBuf,
    accept_loop: Option<JoinHandle<()>>,
}

#[cfg(not(unix))]
pub struct ControlHandle {
    socket_path: PathBuf,
}

impl ControlHandle {
    pub fn socket_path(&self) -> &Path {
        &self.socket_path
    }
}

#[cfg(unix)]
impl Drop for ControlHandle {
    fn drop(&mut self) {
        if let Some(h) = self.accept_loop.take() {
            h.abort();
        }
        let _ = std::fs::remove_file(&self.socket_path);
    }
}

pub fn pick_socket_path() -> PathBuf {
    crate::mcp_bridge::pick_named_socket_path("control")
}

/// Which kind of tenant this control socket fronts. Workspace commands
/// (`cs open/graph/dashboard/search`, team ops) need an actual
/// workspace behind the cell; on a standalone terminal tenant the cell
/// is None BY DESIGN, and the error must say so instead of the
/// transient-sounding "workspace cell unavailable".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlTenant {
    /// A workspace mount (`chan serve`, a desktop workspace tenant):
    /// the cell is None only transiently (storage reset window).
    Workspace,
    /// chan-desktop's workspace-less `/terminal` tenant: terminal /
    /// pane / window commands work, workspace commands never will.
    TerminalOnly,
}

/// The workspace-only refusal for standalone terminals. A const so the
/// CLI-facing wording is pinned by a test and greppable.
pub const TERMINAL_ONLY_NEEDS_WORKSPACE: &str =
    "this command needs a workspace; this is a standalone terminal session — run it from a terminal inside a workspace window";

/// `cs terminal new --path X` on a standalone terminal window: there is no
/// workspace root to resolve the path against, so reject it clearly rather
/// than silently dropping the requested cwd. `cs terminal new` with no path
/// works (opens a terminal in the window by pure window routing).
pub const TERM_NEW_PATH_NEEDS_WORKSPACE: &str =
    "cannot resolve --path on a standalone terminal window (no workspace root); run it from a terminal inside a workspace window, or drop --path to open a terminal here";

/// Shared server resources a control-socket connection needs, plus
/// the tenant gate. One value per `start`; cloned per connection
/// (every field is a cheap handle).
#[derive(Clone)]
pub struct ControlSocketCtx {
    pub workspace_cell: Arc<RwLock<Option<WorkspaceCell>>>,
    pub events_tx: broadcast::Sender<String>,
    pub self_writes: Arc<crate::self_writes::SelfWrites>,
    pub terminal_registry: TerminalRegistryCell,
    pub survey_bus: Arc<crate::survey::SurveyBus>,
    pub window_bus: Arc<crate::window_bus::WindowBus>,
    pub window_presence: Arc<crate::window_presence::WindowPresence>,
    /// Desktop integration: the window-ops channel (`None` standalone)
    /// and the shared title map. `cs window list` reads the titles; the
    /// lifecycle verbs send ops down the channel.
    pub desktop: crate::desktop_window_ops::DesktopBridge,
    pub tenant: ControlTenant,
}

#[cfg(unix)]
pub fn start(socket_path: PathBuf, ctx: ControlSocketCtx) -> std::io::Result<ControlHandle> {
    let _ = std::fs::remove_file(&socket_path);
    let listener = UnixListener::bind(&socket_path)?;

    let accept_loop = tokio::spawn(async move {
        loop {
            let (stream, _) = match listener.accept().await {
                Ok(pair) => pair,
                Err(e) => {
                    tracing::warn!("control socket accept: {e}");
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    continue;
                }
            };
            let ctx = ctx.clone();
            tokio::spawn(async move {
                let (read, mut write) = stream.into_split();
                let mut reader = BufReader::new(read);
                let mut line = String::new();
                let response = match reader.read_line(&mut line).await {
                    Ok(0) => ControlResponse::Error {
                        message: "empty control request".into(),
                    },
                    Ok(_) => match serde_json::from_str::<ControlRequest>(&line) {
                        Ok(req) => handle_request(req, &ctx).await,
                        Err(e) => ControlResponse::Error {
                            message: format!("invalid control request: {e}"),
                        },
                    },
                    Err(e) => ControlResponse::Error {
                        message: format!("read control request: {e}"),
                    },
                };
                if let Ok(mut out) = serde_json::to_vec(&response) {
                    out.push(b'\n');
                    let _ = write.write_all(&out).await;
                }
            });
        }
    });

    Ok(ControlHandle {
        socket_path,
        accept_loop: Some(accept_loop),
    })
}

#[cfg(not(unix))]
pub fn start(_socket_path: PathBuf, _ctx: ControlSocketCtx) -> std::io::Result<ControlHandle> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "control socket requires unix-domain sockets",
    ))
}

// Async because of the one blocking variant (`TermSurvey`); every other
// arm returns synchronously without awaiting.
#[cfg(unix)]
async fn handle_request(req: ControlRequest, ctx: &ControlSocketCtx) -> ControlResponse {
    let ControlSocketCtx {
        workspace_cell,
        events_tx,
        self_writes,
        terminal_registry,
        survey_bus,
        window_bus,
        window_presence,
        desktop,
        tenant,
    } = ctx;
    // The registry is a set-once cell that may be filled after the
    // socket starts; resolve it per request, exactly as before.
    let terminal_registry = terminal_registry.get();
    let tenant = *tenant;
    match req {
        ControlRequest::OpenPath { window_id, path } => {
            if let Err(message) = require_window_id(&window_id) {
                return ControlResponse::Error { message };
            }
            let workspace = match workspace_from_cell(workspace_cell, tenant) {
                Ok(workspace) => workspace,
                Err(message) => return ControlResponse::Error { message },
            };
            into_response(open_path(
                &workspace,
                self_writes,
                &window_id,
                &path,
                events_tx,
            ))
        }
        ControlRequest::OpenGraph { window_id, path } => {
            if let Err(message) = require_window_id(&window_id) {
                return ControlResponse::Error { message };
            }
            let workspace = match workspace_from_cell(workspace_cell, tenant) {
                Ok(workspace) => workspace,
                Err(message) => return ControlResponse::Error { message },
            };
            into_response(open_graph(
                &workspace,
                &window_id,
                path.as_deref(),
                events_tx,
            ))
        }
        ControlRequest::OpenTermNew {
            window_id,
            path,
            tab_name,
            tab_group,
        } => {
            if let Err(message) = require_window_id(&window_id) {
                return ControlResponse::Error { message };
            }
            // Opening a terminal is window routing, not a workspace operation:
            // the only workspace use is resolving an optional --path cwd. So a
            // standalone terminal tenant CAN open a terminal (no cwd to
            // resolve); it just can't resolve a --path against a workspace it
            // doesn't have. This mirrors `WindowList`'s tenant branch.
            match tenant {
                ControlTenant::TerminalOnly => {
                    if path.is_some() {
                        return ControlResponse::Error {
                            message: TERM_NEW_PATH_NEEDS_WORKSPACE.into(),
                        };
                    }
                    into_response(open_term_new_standalone(
                        &window_id, tab_name, tab_group, events_tx,
                    ))
                }
                ControlTenant::Workspace => {
                    let workspace = match workspace_from_cell(workspace_cell, tenant) {
                        Ok(workspace) => workspace,
                        Err(message) => return ControlResponse::Error { message },
                    };
                    into_response(open_term_new(
                        &workspace,
                        &window_id,
                        path.as_deref(),
                        tab_name,
                        tab_group,
                        events_tx,
                    ))
                }
            }
        }
        ControlRequest::OpenDashboard {
            window_id,
            carousel_index,
            carousel_off,
        } => {
            if let Err(message) = require_window_id(&window_id) {
                return ControlResponse::Error { message };
            }
            into_response(open_dashboard(
                &window_id,
                carousel_index,
                carousel_off,
                events_tx,
            ))
        }
        ControlRequest::TermWrite {
            tab_name,
            tab_group,
            data,
        } => {
            let Some(registry) = terminal_registry else {
                return ControlResponse::Error {
                    message: "terminal registry unavailable".into(),
                };
            };
            into_response(term_write(
                registry,
                tab_name.as_deref(),
                tab_group.as_deref(),
                &data,
            ))
        }
        ControlRequest::TermList => {
            let Some(registry) = terminal_registry else {
                return ControlResponse::Error {
                    message: "terminal registry unavailable".into(),
                };
            };
            into_response(term_list(registry))
        }
        ControlRequest::TermRestart {
            tab_name,
            tab_group,
        } => {
            let Some(registry) = terminal_registry else {
                return ControlResponse::Error {
                    message: "terminal registry unavailable".into(),
                };
            };
            into_response(term_restart(
                registry,
                tab_name.as_deref(),
                tab_group.as_deref(),
            ))
        }
        ControlRequest::TermScrollback { tab_name } => {
            let Some(registry) = terminal_registry else {
                return ControlResponse::Error {
                    message: "terminal registry unavailable".into(),
                };
            };
            into_response(term_scrollback(registry, &tab_name))
        }
        ControlRequest::WindowList => {
            let connected = window_presence.connected_ids();
            let saved = match tenant {
                ControlTenant::Workspace => {
                    let workspace = match workspace_from_cell(workspace_cell, tenant) {
                        Ok(workspace) => workspace,
                        Err(message) => return ControlResponse::Error { message },
                    };
                    match tokio::task::spawn_blocking(move || workspace.list_sessions()).await {
                        Ok(Ok(keys)) => keys,
                        Ok(Err(e)) => {
                            return ControlResponse::Error {
                                message: format!("listing saved windows: {e}"),
                            }
                        }
                        Err(e) => {
                            return ControlResponse::Error {
                                message: format!("list windows task panicked: {e}"),
                            }
                        }
                    }
                }
                // A standalone terminal tenant's saved blobs describe
                // windows whose PTYs died with them — nothing reopenable.
                // Live presence is the honest list there.
                ControlTenant::TerminalOnly => Vec::new(),
            };
            let rows = crate::routes::windows::join_windows_with_titles(
                saved,
                connected,
                &desktop.window_titles,
            );
            into_response(
                serde_json::to_string(&rows).map_err(|e| format!("encoding window list: {e}")),
            )
        }
        ControlRequest::Search { query, limit } => {
            let workspace = match workspace_from_cell(workspace_cell, tenant) {
                Ok(workspace) => workspace,
                Err(message) => return ControlResponse::Error { message },
            };
            into_response(search_workspace(&workspace, &query, limit))
        }
        ControlRequest::TermSurvey {
            tab_name,
            tab_group,
            spec,
        } => {
            handle_survey(
                spec,
                tab_name.as_deref(),
                tab_group.as_deref(),
                events_tx,
                survey_bus,
                terminal_registry,
            )
            .await
        }
        ControlRequest::TerminalTeam {
            dir,
            op,
            config_toml,
            script,
            window_id,
        } => {
            handle_team(
                TeamRequest {
                    dir,
                    op,
                    config_toml,
                    script,
                    window_id,
                },
                ctx,
            )
            .await
        }
        ControlRequest::PaneQuery {
            window_id,
            tab_name,
        } => {
            handle_pane_query(
                window_id,
                tab_name,
                events_tx,
                window_bus,
                terminal_registry,
            )
            .await
        }
        ControlRequest::PaneExec {
            window_id,
            tab_name,
            op,
        } => {
            handle_pane_exec(
                window_id,
                tab_name,
                op,
                events_tx,
                window_bus,
                terminal_registry,
            )
            .await
        }
        ControlRequest::WindowNew => handle_window_new(desktop, workspace_cell, tenant).await,
        ControlRequest::WindowOpen { id } => into_response(
            desktop
                .dispatch(|reply| DesktopWindowOp::Open {
                    id: id.clone(),
                    reply,
                })
                .await
                .map(|()| format!("opened window {id}")),
        ),
        ControlRequest::WindowClose { id, force } => {
            handle_window_close(desktop, workspace_cell, tenant, id, force).await
        }
        ControlRequest::WindowHide { id } => into_response(
            desktop
                .dispatch(|reply| DesktopWindowOp::Hide {
                    id: id.clone(),
                    reply,
                })
                .await
                .map(|()| format!("hid window {id}")),
        ),
        ControlRequest::WindowTitle { id, title } => {
            let confirm = if title.is_empty() {
                format!("reset title for window {id}")
            } else {
                format!("set title for window {id}")
            };
            into_response(
                desktop
                    .dispatch(|reply| DesktopWindowOp::Title {
                        id: id.clone(),
                        title,
                        reply,
                    })
                    .await
                    .map(|()| confirm),
            )
        }
    }
}

/// `handle_team`'s dispatch payload: the `ControlRequest::TerminalTeam`
/// variant's fields, bundled at the dispatch site. The wire enum itself
/// stays flat (serde shape frozen).
#[cfg(unix)]
struct TeamRequest {
    dir: String,
    op: TeamOp,
    config_toml: Option<String>,
    script: bool,
    /// The caller's window ($CHAN_WINDOW_ID), when present: every spawned
    /// agent session binds to it so the agents carry $CHAN_WINDOW_ID and the
    /// window-targeting `cs` commands work from inside an agent. The same
    /// window receives the `TeamSpawned` surfacing push.
    window_id: Option<String>,
}

/// The `cs terminal team new|load` path. `new` parses the supplied
/// config.toml text, stamps `created_at` when omitted, validates, then
/// either emits the paste-and-run bootstrap script (`--script`) or writes
/// `config.toml` + the regenerated `bootstrap.md` + the dir tree through
/// the Workspace sandbox AND brings the team up server-side via the
/// terminal registry (lead first). `load` reads + validates
/// `{dir}/config.toml`, then emits the script (`--script`) or a one-line
/// summary. All the config logic lives in `routes::team_config` so the CLI
/// path and the HTTP route share one source of truth (the bootstrap is
/// never regenerated client-side).
///
/// async because the non-`--script` `new` spawns the team then blocks a
/// boot grace before poking each agent's identity prompt (the same
/// sequence the `--script` form runs inline with `sleep 3`).
#[cfg(unix)]
async fn handle_team(req: TeamRequest, ctx: &ControlSocketCtx) -> ControlResponse {
    use crate::routes::team_config::{
        ensure_created_at, generate_bootstrap_script, read_team_config, validate_team_config,
        write_team_config,
    };

    let TeamRequest {
        dir,
        op,
        config_toml,
        script,
        window_id,
    } = req;
    // The registry is a set-once cell that may be filled after the socket
    // starts; resolve it per request, exactly as handle_request's dispatch
    // arm used to do on this handler's behalf.
    let terminal_registry = ctx.terminal_registry.get();
    let workspace_cell = &ctx.workspace_cell;
    let tenant = ctx.tenant;
    let events_tx = &ctx.events_tx;

    // Mirror the route's dir guard so a bad dir is a clean message, not a
    // sandbox error deeper in the write path.
    let dir = dir.trim();
    if dir.is_empty() {
        return ControlResponse::Error {
            message: "team directory is required".into(),
        };
    }
    if dir.starts_with('/') {
        return ControlResponse::Error {
            message: format!("team directory must be workspace-relative, not absolute: {dir}"),
        };
    }

    // The workspace is resolved lazily: `new --script` is a pure generator
    // (no filesystem I/O), so only the write (`new`) and read (`load`)
    // paths touch the cell.
    match op {
        TeamOp::New => {
            let Some(toml_text) = config_toml else {
                return ControlResponse::Error {
                    message: "cs terminal team new needs a config (--config <file> or --stdin)"
                        .into(),
                };
            };
            let mut config: TeamConfig = match toml::from_str(&toml_text) {
                Ok(config) => config,
                Err(e) => {
                    return ControlResponse::Error {
                        message: format!("invalid team config TOML: {e}"),
                    }
                }
            };
            ensure_created_at(&mut config);
            if let Err(message) = validate_team_config(&config) {
                return ControlResponse::Error { message };
            }
            if script {
                return ControlResponse::Ok {
                    message: generate_bootstrap_script(dir, &config),
                };
            }
            let workspace = match workspace_from_cell(workspace_cell, tenant) {
                Ok(workspace) => workspace,
                Err(message) => return ControlResponse::Error { message },
            };
            if let Err(message) = write_team_config(&workspace, dir, &config) {
                return ControlResponse::Error { message };
            }
            // The config + bootstrap.md + tree are on disk. Without a
            // terminal registry (a server with terminals disabled) there is
            // nothing to spawn into, so report the write and stop.
            let Some(registry) = terminal_registry else {
                return ControlResponse::Ok {
                    message: format!(
                        "team {:?} written to {dir} ({} member(s)); no terminal registry to spawn into",
                        config.team_name,
                        config.members.len()
                    ),
                };
            };
            spawn_and_poke_team(registry, dir, &config, window_id.as_deref(), events_tx).await
        }
        TeamOp::Load => {
            let workspace = match workspace_from_cell(workspace_cell, tenant) {
                Ok(workspace) => workspace,
                Err(message) => return ControlResponse::Error { message },
            };
            let config = match read_team_config(&workspace, dir) {
                Ok(config) => config,
                Err(message) => return ControlResponse::Error { message },
            };
            if script {
                return ControlResponse::Ok {
                    message: generate_bootstrap_script(dir, &config),
                };
            }
            // Load now brings the saved team UP, not just summarizes it: read
            // + validate `{dir}/config.toml`, then spawn lead-first and
            // identity-poke each agent, exactly like `new` does after its
            // write (it shares spawn_and_poke_team, so a freshly-written and a
            // reloaded team come up identically). Without a terminal registry
            // there is nothing to spawn into, so fall back to a read-only
            // summary.
            let Some(registry) = terminal_registry else {
                let lead = config
                    .members
                    .iter()
                    .find(|m| m.is_lead)
                    .map(|m| m.handle.as_str())
                    .unwrap_or("?");
                return ControlResponse::Ok {
                    message: format!(
                        "team {:?} at {dir}: {} member(s), lead {lead}; no terminal registry to spawn into",
                        config.team_name,
                        config.members.len()
                    ),
                };
            };
            spawn_and_poke_team(registry, dir, &config, window_id.as_deref(), events_tx).await
        }
    }
}

/// Boot grace between spawning the team's agents and poking their compose
/// boxes. Matches the `--script` form's inline `sleep 3`: a freshly-spawned
/// agent needs a moment before its compose box accepts input, else the
/// identity poke lands mid-startup and is lost. This is the one magic number
/// in the spawn path; live smoke runs validated it.
#[cfg(unix)]
const TEAM_SPAWN_POKE_GRACE: std::time::Duration = std::time::Duration::from_secs(3);

/// Gap between the body write and the submit-chord write of a multi-write
/// poke (gemini: the prompt, then the bare CR as a distinct keypress). The
/// gap lets gemini render + settle on the body before the CR arrives, so the
/// CR is read as Enter rather than coalesced into the body's read. The
/// queue-based poke paths (cs / Rich Prompt) get this separation for free
/// from the drainer's idle-gating; this direct-write spawn path needs an
/// explicit gap.
#[cfg(unix)]
const SUBMIT_SPLIT_GAP: std::time::Duration = std::time::Duration::from_millis(400);

/// What a server-side team spawn produced: the resolved group, the handles
/// that came up, the ones that failed (with the spawn error), and the
/// per-agent identity pokes to deliver after the boot grace.
#[cfg(unix)]
struct TeamSpawn {
    group: String,
    spawned: Vec<String>,
    failed: Vec<(String, String)>,
    /// `(handle, writes)` for each AGENT member. `writes` is the ordered list
    /// of PTY writes that deliver the identity prompt + submit it: one element
    /// for most agents (prompt + chord), but TWO for gemini (the prompt, then
    /// the bare submit chord as a distinct write, since gemini coalesces a
    /// bulk text+CR). The delivery loop writes them with a gap between.
    pokes: Vec<(String, Vec<String>)>,
    /// Each spawned member's tab name + live `session_id`, for the
    /// SPA-surfacing push (`WindowCommand::TeamSpawned`).
    members: Vec<SpawnedMember>,
}

/// Resolve the team's terminal group against the LIVE registry, appending
/// `-N` until unique so a new team never joins an existing group. Mirrors
/// the SPA's `resolveTeamGroup` (teamOrchestrator.svelte.ts): it reads the
/// same resolved-group set `cs terminal list` shows.
#[cfg(unix)]
fn resolve_team_group(registry: &TerminalRegistry, base: &str) -> String {
    let live: std::collections::HashSet<String> = registry
        .session_summaries()
        .into_iter()
        .map(|s| s.tab_group)
        .collect();
    if !live.contains(base) {
        return base.to_string();
    }
    for n in 2..1000 {
        let candidate = format!("{base}-{n}");
        if !live.contains(&candidate) {
            return candidate;
        }
    }
    base.to_string()
}

/// Bring the team up via the terminal registry: resolve the group, spawn
/// lead-first (full command + env + tab-name + group), and compute the
/// per-agent identity pokes. The poke payload is built with the SAME
/// `identity_prompt` + `apply_submit_chord` the `--script` form emits, so
/// the direct `new` reproduces the script's bytes. Shell members (no agent)
/// spawn but get no poke. A member whose command fails to start is recorded
/// in `failed` and does not abort the rest of the team (mirrors
/// runTeamBootstrap's per-worker try/catch). This step is synchronous (the
/// boot-grace wait + poke delivery happen in `spawn_and_poke_team`).
#[cfg(unix)]
fn spawn_team(
    registry: &TerminalRegistry,
    dir: &str,
    config: &TeamConfig,
    window_id: Option<&str>,
) -> TeamSpawn {
    use crate::routes::team_config::{identity_prompt, lead_first_order, team_base_group};

    let group = resolve_team_group(registry, team_base_group(config));
    let mut spawned = Vec::new();
    let mut failed = Vec::new();
    let mut pokes = Vec::new();
    let mut members = Vec::new();
    for m in lead_first_order(config) {
        // A blank command runs the member's default login shell (a shell
        // member); a named command (claude/codex/...) is the PTY program.
        let command = {
            let trimmed = m.command.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        };
        let opts = CreateOptions {
            // No client is attached at spawn time; a real attach resizes via
            // the WS Resize frame. 80x24 is the standard default.
            size: PtySize {
                cols: 80,
                rows: 24,
                pixel_width: 0,
                pixel_height: 0,
            },
            tab_name: Some(m.handle.clone()),
            tab_group: Some(group.clone()),
            // Bind every agent to the caller's window when there is one, so
            // the spawned session exports $CHAN_WINDOW_ID (a window-targeting
            // `cs` command run inside the agent resolves a window) and
            // `window_ids_matching` finds the agent's window. None when the
            // caller is windowless (a native terminal), unchanged from before.
            window_id: window_id.map(str::to_string),
            // MCP env starts OFF; a team opts in via its config's
            // `mcp_env` toggle (team setup dialog / `cs terminal team
            // new|load`). Off by default keeps codex (which wants a
            // file-based MCP config) from failing on a stray descriptor.
            mcp_env: config.mcp_env,
            cwd: None,
            command,
            env: m.env.clone(),
        };
        match registry.create(opts) {
            Ok(handle) => {
                // Capture the session id for the surfacing push, then drop
                // the attach handle: the session stays in the registry map,
                // and the boot-grace poke re-resolves it by tab-name + group.
                spawned.push(m.handle.clone());
                members.push(SpawnedMember {
                    tab_name: m.handle.clone(),
                    session_id: handle.id().to_string(),
                });
                // Derive the submit agent from the command (+ CHAN_AGENT env
                // override), the single source of truth shared with the SPA;
                // a shell member derives None and gets no identity poke.
                if let Some(agent) =
                    SubmitAgent::derive(&m.command, m.env.get("CHAN_AGENT").map(String::as_str))
                {
                    let writes = submit_writes(identity_prompt(config, dir, m), Some(agent));
                    pokes.push((m.handle.clone(), writes));
                }
            }
            Err(e) => failed.push((m.handle.clone(), e.to_string())),
        }
    }
    TeamSpawn {
        group,
        spawned,
        failed,
        pokes,
        members,
    }
}

/// The non-`--script` `new` spawn path: bring the team up, wait the boot
/// grace, then deliver each agent's identity poke. Blocks for the grace so
/// the CLI returns only once the pokes are delivered (the same inline
/// ordering the `--script` form runs: spawn -> sleep 3 -> poke). Returns a
/// summary, or an error when nothing came up.
#[cfg(unix)]
async fn spawn_and_poke_team(
    registry: &Arc<TerminalRegistry>,
    dir: &str,
    config: &TeamConfig,
    window_id: Option<&str>,
    events_tx: &broadcast::Sender<String>,
) -> ControlResponse {
    let spawn = spawn_team(registry, dir, config, window_id);

    // Surface the spawned team in the window that owns it (each agent
    // is bound to `window_id`). The same window opens a terminal tab per
    // member ATTACHED to the live session, so a CLI `cs terminal team new`
    // shows up in the running SPA instead of only on the SPA's next attach.
    // A windowless spawn has no window to surface into. Sent before the boot
    // grace so the tabs appear while the agents start; the poke reaches the
    // PTYs independently of any SPA attach.
    if let Some(window_id) = window_id {
        if !spawn.members.is_empty() {
            let _ = send_window_command(
                window_id,
                WindowCommand::TeamSpawned {
                    group: spawn.group.clone(),
                    members: spawn.members.clone(),
                },
                events_tx,
            );
        }
    }

    // Let the agents come up before poking their compose boxes, then deliver
    // each agent its identity prompt + submit chord. A shell member has no
    // compose box, so it has no poke entry; an all-shell team (or a fully
    // failed spawn) skips the wait entirely.
    if !spawn.spawned.is_empty() && !spawn.pokes.is_empty() {
        tokio::time::sleep(TEAM_SPAWN_POKE_GRACE).await;
        for (handle, writes) in &spawn.pokes {
            // Most agents have a single write; gemini has two (prompt, then
            // the bare CR), which must arrive as distinct keypresses, so we
            // gap between writes.
            for (i, write) in writes.iter().enumerate() {
                if i > 0 {
                    tokio::time::sleep(SUBMIT_SPLIT_GAP).await;
                }
                registry.write_input_matching(Some(handle), Some(&spawn.group), write.as_bytes());
            }
        }
    }

    team_spawn_summary(&config.team_name, &spawn)
}

/// Render the CLI-facing response for a completed `spawn_team`: an error when
/// nothing came up, else a one-line summary of the spawned + poked + failed
/// counts. Pure (no I/O) so the wording is unit-tested without the boot-grace
/// wait.
#[cfg(unix)]
fn team_spawn_summary(team_name: &str, spawn: &TeamSpawn) -> ControlResponse {
    if spawn.spawned.is_empty() {
        return ControlResponse::Error {
            message: format!(
                "team {team_name:?}: no member could be spawned: {}",
                fmt_spawn_failures(&spawn.failed)
            ),
        };
    }
    let mut message = format!(
        "team {team_name:?} spawned in group {:?}: {} member(s) up, poked {} agent(s)",
        spawn.group,
        spawn.spawned.len(),
        spawn.pokes.len(),
    );
    if !spawn.failed.is_empty() {
        message.push_str(&format!(
            "; {} failed: {}",
            spawn.failed.len(),
            fmt_spawn_failures(&spawn.failed)
        ));
    }
    ControlResponse::Ok { message }
}

#[cfg(unix)]
fn fmt_spawn_failures(failed: &[(String, String)]) -> String {
    failed
        .iter()
        .map(|(handle, err)| format!("{handle} ({err})"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The blocking `cs terminal survey` path: resolve the tab selector to the
/// owning SPA window(s), mint a survey id, push the `open_survey` overlay to
/// each, park a oneshot, and AWAIT the SPA's reply (delivered by C's
/// `POST /api/survey/reply` -> `SurveyBus::complete_survey`). The returned
/// message is what the CLI prints to stdout: the chosen option label, or the
/// followup-file path the UI created on `[F]`.
#[cfg(unix)]
async fn handle_survey(
    mut spec: SurveySpec,
    tab_name: Option<&str>,
    tab_group: Option<&str>,
    events_tx: &broadcast::Sender<String>,
    survey_bus: &Arc<crate::survey::SurveyBus>,
    terminal_registry: Option<&Arc<TerminalRegistry>>,
) -> ControlResponse {
    if tab_name.is_none() && tab_group.is_none() {
        return ControlResponse::Error {
            message: "survey needs a tab name and/or group selector".into(),
        };
    }
    // Mirror the CLI-side cap so a malformed direct request is rejected too.
    if spec.options.is_empty() || spec.options.len() > 4 {
        return ControlResponse::Error {
            message: format!("survey needs 1..=4 options (got {})", spec.options.len()),
        };
    }
    let Some(registry) = terminal_registry else {
        return ControlResponse::Error {
            message: "terminal registry unavailable".into(),
        };
    };
    let windows = registry.window_ids_matching(tab_name, tab_group);
    if windows.is_empty() {
        return ControlResponse::Error {
            message: "no live terminal session matched".into(),
        };
    }
    // Park the oneshot (and stamp its id onto the spec) BEFORE pushing the
    // overlay, so a fast reply cannot arrive before the survey is
    // registered.
    let (survey_id, rx) = survey_bus.register();
    spec.survey_id = survey_id.clone();
    // Fan the overlay out to every owning window. First reply wins; later
    // ones find the id already removed and no-op. A send failure is fatal
    // (the SPA will never see the overlay), so cancel and report it.
    for window_id in &windows {
        if let Err(message) = send_window_command(
            window_id,
            WindowCommand::OpenSurvey {
                survey: spec.clone(),
                tab_name: tab_name.map(str::to_string),
            },
            events_tx,
        ) {
            survey_bus.cancel(&survey_id);
            return ControlResponse::Error { message };
        }
    }
    // Block until C's reply route fires the oneshot. A receive error means
    // the sender was dropped without a reply (server shutdown); the entry is
    // gone, but cancel defensively in case register/await ever diverge.
    match rx.await {
        Ok(reply) => ControlResponse::Ok {
            message: format_survey_reply(&reply),
        },
        Err(_) => {
            survey_bus.cancel(&survey_id);
            ControlResponse::Error {
                message: "survey cancelled before a reply".into(),
            }
        }
    }
}

/// The stdout line the CLI prints for a completed survey. Each variant prints
/// a distinct line so the asking agent can tell an answer from a deferral from
/// a dismissal: the chosen option label; the `new follow up file created: ...`
/// path on `[F]` with team context (or a bare-deferral line without); or the
/// dismissed line (Part C).
#[cfg(unix)]
fn format_survey_reply(reply: &SurveyReply) -> String {
    match reply {
        SurveyReply::Option { option_label, .. } => option_label.clone(),
        SurveyReply::Followup {
            followup_path: Some(path),
            ..
        } => {
            format!("new follow up file created: {path}")
        }
        SurveyReply::Followup {
            followup_path: None,
            ..
        } => "host deferred; no follow up file created".to_string(),
        SurveyReply::Dismissed { .. } => "survey dismissed; no answer".to_string(),
    }
}

/// How long `cs pane` waits for the SPA to answer before giving up. The SPA
/// replies in milliseconds (no user interaction, unlike a survey), so this
/// only fires when the target window is not actually connected (e.g. the
/// browser tab was closed while the terminal lived on), keeping `cs pane`
/// from blocking forever in that case.
#[cfg(unix)]
const PANE_REPLY_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

/// Resolve the target SPA window for a `cs pane` command. Prefer the explicit
/// `window_id` ($CHAN_WINDOW_ID); otherwise resolve `tab_name` (`--tab-name`)
/// to the single live window owning that tab via `window_ids_matching`, so
/// the command works from a context with no $CHAN_WINDOW_ID (an unbound
/// agent, a native terminal). Errors when neither is given, when a tab
/// selector matches no window, or when it is ambiguous.
#[cfg(unix)]
fn resolve_pane_window(
    window_id: Option<String>,
    tab_name: Option<&str>,
    terminal_registry: Option<&Arc<TerminalRegistry>>,
) -> Result<String, String> {
    if let Some(window_id) = window_id {
        let trimmed = window_id.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_string());
        }
    }
    let Some(tab_name) = tab_name.map(str::trim).filter(|t| !t.is_empty()) else {
        return Err(
            "cs pane needs a target: run inside a chan terminal ($CHAN_WINDOW_ID) or pass --tab-name"
                .into(),
        );
    };
    let Some(registry) = terminal_registry else {
        return Err("terminal registry unavailable".into());
    };
    let mut windows = registry.window_ids_matching(Some(tab_name), None);
    match windows.len() {
        0 => Err(format!("no live window owns a tab named {tab_name:?}")),
        1 => Ok(windows.pop().expect("one window")),
        n => Err(format!(
            "{n} live windows own a tab named {tab_name:?}; --tab-name must resolve to one"
        )),
    }
}

/// The shared `cs pane` round-trip: mint a request id, build the
/// window_command from it (a query or an exec), push it to `window_id`, park
/// a oneshot on the window bus, and AWAIT the SPA's reply (delivered by `POST
/// /api/window/reply` -> `WindowBus::complete`). The returned message is the
/// reply payload JSON the CLI formats. Mirrors `handle_survey`'s register ->
/// push -> await shape, with a timeout since no user is in the loop.
#[cfg(unix)]
async fn pane_round_trip<F>(
    window_id: &str,
    make_command: F,
    events_tx: &broadcast::Sender<String>,
    window_bus: &Arc<crate::window_bus::WindowBus>,
) -> ControlResponse
where
    F: FnOnce(String) -> WindowCommand,
{
    // Park the oneshot BEFORE pushing the command so a fast reply cannot
    // arrive before the request is registered.
    let (request_id, rx) = window_bus.register();
    if let Err(message) =
        send_window_command(window_id, make_command(request_id.clone()), events_tx)
    {
        window_bus.cancel(&request_id);
        return ControlResponse::Error { message };
    }
    // Block until the reply route fires the oneshot, or the timeout elapses
    // (the window never answered). Cancel the parked entry on either failure
    // so it does not leak.
    match tokio::time::timeout(PANE_REPLY_TIMEOUT, rx).await {
        Ok(Ok(payload)) => match serde_json::to_string(&payload) {
            Ok(json) => ControlResponse::Ok { message: json },
            Err(e) => ControlResponse::Error {
                message: format!("encode pane reply: {e}"),
            },
        },
        Ok(Err(_)) => {
            window_bus.cancel(&request_id);
            ControlResponse::Error {
                message: "pane request cancelled before a reply".into(),
            }
        }
        Err(_elapsed) => {
            window_bus.cancel(&request_id);
            ControlResponse::Error {
                message: "no reply from the window (is it open in a browser?)".into(),
            }
        }
    }
}

/// `cs pane` (layout query): resolve the target window, then round-trip a
/// `pane_query`. The reply payload is the layout snapshot the CLI formats.
#[cfg(unix)]
async fn handle_pane_query(
    window_id: Option<String>,
    tab_name: Option<String>,
    events_tx: &broadcast::Sender<String>,
    window_bus: &Arc<crate::window_bus::WindowBus>,
    terminal_registry: Option<&Arc<TerminalRegistry>>,
) -> ControlResponse {
    let target = match resolve_pane_window(window_id, tab_name.as_deref(), terminal_registry) {
        Ok(target) => target,
        Err(message) => return ControlResponse::Error { message },
    };
    pane_round_trip(
        &target,
        |request_id| WindowCommand::PaneQuery { request_id },
        events_tx,
        window_bus,
    )
    .await
}

/// `cs pane <exec>` (focus / split / resize / close): resolve the target
/// window, then round-trip a `pane_exec` carrying the op. The reply payload
/// is the exec result the CLI formats.
#[cfg(unix)]
async fn handle_pane_exec(
    window_id: Option<String>,
    tab_name: Option<String>,
    op: PaneOp,
    events_tx: &broadcast::Sender<String>,
    window_bus: &Arc<crate::window_bus::WindowBus>,
    terminal_registry: Option<&Arc<TerminalRegistry>>,
) -> ControlResponse {
    let target = match resolve_pane_window(window_id, tab_name.as_deref(), terminal_registry) {
        Ok(target) => target,
        Err(message) => return ControlResponse::Error { message },
    };
    pane_round_trip(
        &target,
        move |request_id| WindowCommand::PaneExec { request_id, op },
        events_tx,
        window_bus,
    )
    .await
}

#[cfg(unix)]
fn require_window_id(window_id: &str) -> Result<(), String> {
    if window_id.trim().is_empty() {
        Err("window_id is required".into())
    } else {
        Ok(())
    }
}

#[cfg(unix)]
fn into_response(result: Result<String, String>) -> ControlResponse {
    match result {
        Ok(message) => ControlResponse::Ok { message },
        Err(message) => ControlResponse::Error { message },
    }
}

/// `cs window new`: ask the desktop to spawn a window whose kind is
/// derived from the calling tenant — a terminal tenant spawns a terminal
/// window, a workspace tenant spawns another window of that workspace.
/// Replies with the new window id. Refuses ([`crate::NO_DESKTOP`]) when
/// no desktop is attached.
#[cfg(unix)]
async fn handle_window_new(
    desktop: &crate::desktop_window_ops::DesktopBridge,
    workspace_cell: &Arc<RwLock<Option<WorkspaceCell>>>,
    tenant: ControlTenant,
) -> ControlResponse {
    use crate::desktop_window_ops::{DesktopWindowOp, NewWindowKind};
    let kind = match tenant {
        ControlTenant::TerminalOnly => NewWindowKind::Terminal,
        ControlTenant::Workspace => {
            let workspace = match workspace_from_cell(workspace_cell, tenant) {
                Ok(workspace) => workspace,
                Err(message) => return ControlResponse::Error { message },
            };
            NewWindowKind::Workspace {
                key: workspace.root().to_string_lossy().into_owned(),
            }
        }
    };
    // The reply is the new window id, which is exactly the message the
    // CLI prints.
    into_response(
        desktop
            .dispatch(|reply| DesktopWindowOp::New { kind, reply })
            .await,
    )
}

/// `cs window rm`: destroy the window (the desktop prompts first when it
/// has live terminals and `force` is unset, blocking this request until
/// the user answers), then drop its saved layout so it can't reappear as
/// a reopenable `saved` row. A row with neither a live window nor a saved
/// blob is an unknown id and errors.
#[cfg(unix)]
async fn handle_window_close(
    desktop: &crate::desktop_window_ops::DesktopBridge,
    workspace_cell: &Arc<RwLock<Option<WorkspaceCell>>>,
    tenant: ControlTenant,
    id: String,
    force: bool,
) -> ControlResponse {
    use crate::desktop_window_ops::DesktopWindowOp;
    let destroyed = match desktop
        .dispatch(|reply| DesktopWindowOp::Close {
            id: id.clone(),
            force,
            reply,
        })
        .await
    {
        Ok(destroyed) => destroyed,
        Err(message) => return ControlResponse::Error { message },
    };
    // Best-effort, current tenant only: a terminal tenant has no on-disk
    // blob (its sessions are ephemeral), and an id belonging to another
    // workspace can't be reached from here (its blob may persist — a
    // known limitation). Check existence first so the reply is honest
    // about whether a layout was actually removed.
    let had_blob = if tenant == ControlTenant::Workspace {
        match workspace_from_cell(workspace_cell, tenant) {
            Ok(workspace) => {
                let key = id.clone();
                tokio::task::spawn_blocking(move || {
                    let existed = workspace
                        .get_session(&key)
                        .map(|blob| blob.is_some())
                        .unwrap_or(false);
                    if existed {
                        let _ = workspace.delete_session(&key);
                    }
                    existed
                })
                .await
                .unwrap_or(false)
            }
            Err(_) => false,
        }
    } else {
        false
    };
    match (destroyed, had_blob) {
        (true, true) => ControlResponse::Ok {
            message: format!("removed window {id} (destroyed; saved layout deleted)"),
        },
        (true, false) => ControlResponse::Ok {
            message: format!("removed window {id} (destroyed)"),
        },
        (false, true) => ControlResponse::Ok {
            message: format!("deleted saved layout for {id} (no live window)"),
        },
        (false, false) => ControlResponse::Error {
            message: format!("no window or saved layout for {id}"),
        },
    }
}

/// `cs search`: run the same content search the UI does (`Workspace::search`,
/// the `/api/search/content` path) and return the results as JSON on the
/// connection, like `term list`. One row per file (best-ranked hit),
/// score-descending. The CLI side formats this JSON: markdown by default,
/// compact `--json`, indented `--json --pretty`.
#[cfg(unix)]
fn search_workspace(
    workspace: &Workspace,
    query: &str,
    limit: Option<u32>,
) -> Result<String, String> {
    let limit = limit.filter(|n| *n > 0).unwrap_or(20);
    // Widen the candidate fetch like the route does so the per-file
    // collapse still fills `limit` rows when a file owns several chunks.
    let opts = chan_workspace::SearchOpts {
        limit: limit.saturating_mul(8).min(limit.max(200)),
        ..Default::default()
    };
    let results = workspace
        .search(query, &opts)
        .map_err(|e| format!("search: {e}"))?;
    let mut hits: Vec<serde_json::Value> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    for hit in results.hits {
        if !seen.insert(hit.path.clone()) {
            continue;
        }
        hits.push(serde_json::json!({
            "path": hit.path,
            "heading": hit.heading,
            "start_line": hit.start_line,
            "snippet": hit.snippet,
            "score": hit.score,
        }));
        if hits.len() >= limit as usize {
            break;
        }
    }
    let payload = serde_json::json!({
        "ready": results.ready,
        "mode": results.mode,
        "query": query,
        "hits": hits,
    });
    serde_json::to_string(&payload).map_err(|e| format!("serialize: {e}"))
}

#[cfg(unix)]
fn workspace_from_cell(
    workspace_cell: &Arc<RwLock<Option<WorkspaceCell>>>,
    tenant: ControlTenant,
) -> Result<Arc<Workspace>, String> {
    let cell = workspace_cell
        .read()
        .map_err(|_| "workspace cell lock poisoned".to_string())?;
    let cell = cell.as_ref().ok_or_else(|| match tenant {
        // A workspace tenant's cell is only empty transiently (the
        // storage-reset swap window); a terminal tenant's is empty by
        // design and the caller should hear that, not a flake.
        ControlTenant::Workspace => "workspace cell unavailable".to_string(),
        ControlTenant::TerminalOnly => TERMINAL_ONLY_NEEDS_WORKSPACE.to_string(),
    })?;
    Ok(cell.workspace.clone())
}

#[cfg(unix)]
fn send_window_command(
    window_id: &str,
    command: WindowCommand,
    events_tx: &broadcast::Sender<String>,
) -> Result<(), String> {
    let frame = WindowCommandFrame {
        frame_type: "window_command",
        window_id: window_id.to_string(),
        command,
    };
    let raw = serde_json::to_string(&frame).map_err(|e| format!("encode window command: {e}"))?;
    // Window commands fan out over the /ws event broadcast; the SPA window
    // owning `window_id` acts on its frame. `events_tx` is subscribed ONLY by
    // /ws connections, so a zero receiver count means NO window is connected:
    // the command would vanish silently. Surface that as an error rather than
    // a misleading "queued" so the caller knows nothing will happen (the most
    // common cause is running a window-scoped `cs` command outside a chan
    // terminal, where $CHAN_WINDOW_ID is unset and no window is open).
    if events_tx.send(raw).is_err() {
        return Err(
            "no chan window is connected to receive this; open the workspace in a window, \
             or run from inside a chan terminal so $CHAN_WINDOW_ID targets one"
                .into(),
        );
    }
    Ok(())
}

/// Resolve an optional requested path to a workspace-relative path plus
/// whether it is a directory. `None` / the workspace root resolve to
/// `(None, _)`, which the SPA treats as "no specific target".
#[cfg(unix)]
fn resolve_optional_rel(
    workspace: &Workspace,
    requested: Option<&Path>,
) -> Result<Option<(String, bool)>, String> {
    let Some(requested) = requested else {
        return Ok(None);
    };
    let rel = abs_to_workspace_rel(workspace.root(), requested)?;
    if rel.is_empty() {
        return Ok(None);
    }
    let is_dir = workspace
        .stat(&rel)
        .map(|stat| stat.is_dir)
        .unwrap_or(false);
    Ok(Some((rel, is_dir)))
}

/// Category 1: open the documentation graph in the originating window,
/// optionally focused on a file or directory.
#[cfg(unix)]
fn open_graph(
    workspace: &Workspace,
    window_id: &str,
    requested: Option<&Path>,
    events_tx: &broadcast::Sender<String>,
) -> Result<String, String> {
    let resolved = resolve_optional_rel(workspace, requested)?;
    let (path, is_dir) = match &resolved {
        Some((rel, is_dir)) => (Some(rel.clone()), *is_dir),
        None => (None, false),
    };
    send_window_command(
        window_id,
        WindowCommand::OpenGraph {
            path: path.clone(),
            is_dir,
        },
        events_tx,
    )?;
    Ok(match path {
        Some(rel) => format!("graph request queued for {rel}"),
        None => "graph request queued".into(),
    })
}

/// Category 1: open a new terminal tab in the originating window. A
/// requested file resolves to its parent directory as the cwd.
#[cfg(unix)]
fn open_term_new(
    workspace: &Workspace,
    window_id: &str,
    requested: Option<&Path>,
    tab_name: Option<String>,
    tab_group: Option<String>,
    events_tx: &broadcast::Sender<String>,
) -> Result<String, String> {
    let cwd = match resolve_optional_rel(workspace, requested)? {
        Some((rel, true)) => Some(rel),
        Some((rel, false)) => {
            let parent = parent_rel(&rel);
            (!parent.is_empty()).then_some(parent)
        }
        None => None,
    };
    send_window_command(
        window_id,
        WindowCommand::OpenTermNew {
            cwd: cwd.clone(),
            tab_name,
            tab_group,
        },
        events_tx,
    )?;
    Ok(match cwd {
        Some(rel) => format!("terminal request queued for {rel}"),
        None => "terminal request queued".into(),
    })
}

/// Category 1, workspace-less: open a new terminal tab in a standalone
/// terminal window. There is no workspace to resolve a cwd against, so the
/// command carries no cwd — pure window routing, the same shape as
/// `open_dashboard`. The caller has already rejected any `--path`.
#[cfg(unix)]
fn open_term_new_standalone(
    window_id: &str,
    tab_name: Option<String>,
    tab_group: Option<String>,
    events_tx: &broadcast::Sender<String>,
) -> Result<String, String> {
    send_window_command(
        window_id,
        WindowCommand::OpenTermNew {
            cwd: None,
            tab_name,
            tab_group,
        },
        events_tx,
    )?;
    Ok("terminal request queued".into())
}

/// Category 1: open a Dashboard tab in the originating window.
#[cfg(unix)]
fn open_dashboard(
    window_id: &str,
    carousel_index: Option<u32>,
    carousel_off: bool,
    events_tx: &broadcast::Sender<String>,
) -> Result<String, String> {
    send_window_command(
        window_id,
        WindowCommand::OpenDashboard {
            carousel_index,
            carousel_off,
        },
        events_tx,
    )?;
    Ok("dashboard request queued".into())
}

/// Category 2: ENQUEUE bytes onto the matching live sessions' write queues.
/// At least one selector is required so a missing filter cannot fan out to
/// every terminal by accident. The bytes are not written to the PTY here:
/// the per-session drainer delivers each queued write when its agent is idle
/// (the serialization the Rich Prompt / poke-chain workflow needs), so
/// chained `cs terminal write`s submit one after another. `data` already
/// carries the caller's submit chord (the CLI's `--submit`).
#[cfg(unix)]
fn term_write(
    registry: &TerminalRegistry,
    tab_name: Option<&str>,
    tab_group: Option<&str>,
    data: &str,
) -> Result<String, String> {
    if tab_name.is_none() && tab_group.is_none() {
        return Err("term write needs a tab name and/or group selector".into());
    }
    let outcome = registry.enqueue_write_matching(tab_name, tab_group, data.as_bytes());
    if outcome.queued == 0 {
        return if outcome.full > 0 {
            Err(format!(
                "matched session(s) at the {WRITE_QUEUE_CAP_MSG}-write queue cap; nothing queued"
            ))
        } else {
            Err("no live terminal session matched".into())
        };
    }
    let mut message = match outcome.position {
        Some(position) => format!("queued at position {position}"),
        None => format!("queued to {} terminal session(s)", outcome.queued),
    };
    if outcome.full > 0 {
        message.push_str(&format!("; {} at queue cap (dropped)", outcome.full));
    }
    Ok(message)
}

/// The queue cap, surfaced in the "queue full" message. Kept in sync with
/// `terminal_sessions::WRITE_QUEUE_CAP` (private there); a literal here
/// avoids widening that module's surface just for an error string.
#[cfg(unix)]
const WRITE_QUEUE_CAP_MSG: usize = 100;

/// Category 2: restart the matching live PTY sessions, preserving each
/// session's spawn command + env (so an agent relaunches). At least one
/// selector is required, mirroring `term_write`. This is the out-of-band
/// server path the Team Work self-restart needs: the bootstrap script
/// runs `cs terminal restart` against its own tab, and the server
/// respawns that session because a shell cannot restart itself.
#[cfg(unix)]
fn term_restart(
    registry: &TerminalRegistry,
    tab_name: Option<&str>,
    tab_group: Option<&str>,
) -> Result<String, String> {
    if tab_name.is_none() && tab_group.is_none() {
        return Err("term restart needs a tab name and/or group selector".into());
    }
    let restarted = registry
        .restart_matching(tab_name, tab_group)
        .map_err(|e| format!("restart failed: {e}"))?;
    if restarted == 0 {
        return Err("no live terminal session matched".into());
    }
    Ok(format!("restarted {restarted} terminal session(s)"))
}

/// Category 2: dump the full replay ring of the single live session whose
/// tab name is `tab_name`, for `cs terminal scrollback`. Requires exactly
/// one match: zero is "no session", more than one is ambiguous (scrollback
/// reads one terminal's history, so there is no group fan-out). The bytes
/// are the raw PTY stream (the same a WS attach replays), UTF-8 decoded
/// lossily for the text transport.
#[cfg(unix)]
fn term_scrollback(registry: &TerminalRegistry, tab_name: &str) -> Result<String, String> {
    let tab_name = tab_name.trim();
    if tab_name.is_empty() {
        return Err("scrollback needs a tab name".into());
    }
    let mut matches = registry.scrollback_matching(tab_name);
    match matches.len() {
        0 => Err("no live terminal session matched".into()),
        1 => {
            let (_id, bytes) = matches.pop().expect("one match");
            Ok(String::from_utf8_lossy(&bytes).into_owned())
        }
        n => Err(format!(
            "{n} live sessions match tab name {tab_name:?}; scrollback needs a single match"
        )),
    }
}

/// Category 2: list live terminal sessions as JSON, grouped by group.
#[cfg(unix)]
fn term_list(registry: &TerminalRegistry) -> Result<String, String> {
    use std::collections::BTreeMap;

    let mut groups: BTreeMap<String, Vec<serde_json::Value>> = BTreeMap::new();
    for summary in registry.session_summaries() {
        let entry = serde_json::json!({
            "name": summary.tab_name,
            "session_id": summary.session_id,
            "cwd": summary.cwd.map(|p| p.to_string_lossy().into_owned()),
        });
        groups.entry(summary.tab_group).or_default().push(entry);
    }
    let payload = serde_json::json!({ "groups": groups });
    serde_json::to_string(&payload).map_err(|e| format!("encode terminal list: {e}"))
}

#[cfg(unix)]
fn open_path(
    workspace: &Workspace,
    self_writes: &crate::self_writes::SelfWrites,
    window_id: &str,
    requested: &Path,
    events_tx: &broadcast::Sender<String>,
) -> Result<String, String> {
    let rel = abs_to_workspace_rel(workspace.root(), requested)?;
    if rel.is_empty() {
        let frame = WindowCommandFrame {
            frame_type: "window_command",
            window_id: window_id.to_string(),
            command: WindowCommand::OpenBrowser {
                path: String::new(),
                select: None,
                enter: true,
            },
        };
        let raw =
            serde_json::to_string(&frame).map_err(|e| format!("encode window command: {e}"))?;
        let _ = events_tx.send(raw);
        return Ok("open request queued for /".into());
    }
    let stat = workspace.stat(&rel).ok();
    let command = if let Some(stat) = stat {
        if stat.is_dir {
            WindowCommand::OpenBrowser {
                path: rel.clone(),
                select: None,
                enter: true,
            }
        } else if rel.ends_with(".md") {
            WindowCommand::OpenFile { path: rel.clone() }
        } else {
            let parent = parent_rel(&rel);
            WindowCommand::OpenBrowser {
                path: parent,
                select: Some(rel.clone()),
                enter: false,
            }
        }
    } else if rel.ends_with(".md") {
        // Note before the write so the watcher's Created event is in the
        // suppression set before it can fire (see files.rs::api_write_file).
        self_writes.note(&rel);
        workspace
            .write_text(&rel, "")
            .map_err(|e| format!("create {rel}: {e}"))?;
        WindowCommand::OpenFile { path: rel.clone() }
    } else {
        return Err("file does not exist; cs open creates `.md` files only".into());
    };

    let frame = WindowCommandFrame {
        frame_type: "window_command",
        window_id: window_id.to_string(),
        command,
    };
    let raw = serde_json::to_string(&frame).map_err(|e| format!("encode window command: {e}"))?;
    let _ = events_tx.send(raw);
    Ok(format!("open request queued for {rel}"))
}

#[cfg(unix)]
fn abs_to_workspace_rel(root: &Path, requested: &Path) -> Result<String, String> {
    if !requested.is_absolute() {
        return Err("control path must be absolute".into());
    }
    let root_canon = root
        .canonicalize()
        .map_err(|e| format!("canonicalize workspace root: {e}"))?;
    let existing_or_parent = if requested.exists() {
        requested
    } else {
        requested
            .parent()
            .ok_or_else(|| "path has no parent".to_string())?
    };
    let canon = existing_or_parent
        .canonicalize()
        .map_err(|e| format!("canonicalize path: {e}"))?;
    if !canon.starts_with(&root_canon) {
        return Err("path escapes workspace root".into());
    }
    let candidate = if requested.exists() {
        canon
    } else {
        canon.join(
            requested
                .file_name()
                .ok_or_else(|| "path has no file name".to_string())?,
        )
    };
    let rel = candidate
        .strip_prefix(&root_canon)
        .map_err(|_| "path escapes workspace root".to_string())?;
    Ok(path_to_posix(rel))
}

#[cfg(unix)]
fn path_to_posix(path: &Path) -> String {
    path.components()
        .filter_map(|c| match c {
            std::path::Component::Normal(s) => Some(s.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(unix)]
fn parent_rel(rel: &str) -> String {
    rel.rsplit_once('/')
        .map(|(parent, _)| parent.to_string())
        .unwrap_or_default()
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use serde_json::Value;

    /// Fresh ControlSocketCtx around the given workspace cell: empty
    /// terminal-registry cell, fresh buses, fresh presence map.
    fn test_ctx(
        workspace_cell: Arc<RwLock<Option<WorkspaceCell>>>,
        tenant: ControlTenant,
    ) -> ControlSocketCtx {
        let (events_tx, _) = broadcast::channel(1);
        ControlSocketCtx {
            workspace_cell,
            events_tx,
            self_writes: Arc::new(crate::self_writes::SelfWrites::new()),
            terminal_registry: Arc::new(std::sync::OnceLock::new()),
            survey_bus: Arc::new(crate::survey::SurveyBus::new()),
            window_bus: Arc::new(crate::window_bus::WindowBus::new()),
            window_presence: Arc::new(crate::window_presence::WindowPresence::new()),
            // No desktop attached in unit tests: lifecycle ops refuse and
            // the title map stays empty.
            desktop: crate::desktop_window_ops::DesktopBridge::default(),
            tenant,
        }
    }

    #[test]
    fn parent_rel_returns_empty_for_root_file() {
        assert_eq!(parent_rel("a.png"), "");
        assert_eq!(parent_rel("notes/a.png"), "notes");
    }

    #[tokio::test]
    async fn handle_request_reports_poisoned_workspace_cell() {
        let workspace_cell: Arc<RwLock<Option<WorkspaceCell>>> = Arc::new(RwLock::new(None));
        let poisoned = workspace_cell.clone();
        let _ = std::thread::spawn(move || {
            let _guard = poisoned.write().expect("poison setup");
            panic!("poison workspace cell");
        })
        .join();
        let ctx = test_ctx(workspace_cell.clone(), ControlTenant::Workspace);

        let response = handle_request(
            ControlRequest::OpenPath {
                window_id: "window-a".to_string(),
                path: PathBuf::from("/tmp/note.md"),
            },
            &ctx,
        )
        .await;

        match response {
            ControlResponse::Error { message } => {
                assert_eq!(message, "workspace cell lock poisoned");
            }
            ControlResponse::Ok { message } => panic!("unexpected ok response: {message}"),
        }
    }

    #[tokio::test]
    async fn workspace_commands_refuse_clearly_on_a_terminal_tenant() {
        // A standalone terminal tenant has no workspace BY DESIGN; the
        // refusal must say that (pinned wording) instead of the
        // transient-sounding "workspace cell unavailable".
        let workspace_cell: Arc<RwLock<Option<WorkspaceCell>>> = Arc::new(RwLock::new(None));
        let ctx = test_ctx(workspace_cell, ControlTenant::TerminalOnly);

        let response = handle_request(
            ControlRequest::Search {
                query: "anything".into(),
                limit: None,
            },
            &ctx,
        )
        .await;

        match response {
            ControlResponse::Error { message } => {
                assert_eq!(message, TERMINAL_ONLY_NEEDS_WORKSPACE);
                assert!(message.contains("standalone terminal"));
            }
            ControlResponse::Ok { message } => panic!("unexpected ok response: {message}"),
        }
    }

    #[tokio::test]
    async fn window_list_on_a_terminal_tenant_reports_presence() {
        // `cs window list` must WORK on a terminal tenant (presence-only
        // rows), not refuse like the workspace commands.
        let workspace_cell: Arc<RwLock<Option<WorkspaceCell>>> = Arc::new(RwLock::new(None));
        let ctx = test_ctx(workspace_cell, ControlTenant::TerminalOnly);
        let _guard = ctx.window_presence.connect("terminal-win-0");

        let response = handle_request(ControlRequest::WindowList, &ctx).await;

        match response {
            ControlResponse::Ok { message } => {
                let rows: Value = serde_json::from_str(&message).expect("rows JSON");
                assert_eq!(
                    rows,
                    serde_json::json!([
                        {"id": "terminal-win-0", "connected": true, "saved": false}
                    ]),
                );
            }
            ControlResponse::Error { message } => panic!("unexpected error: {message}"),
        }
    }

    #[tokio::test]
    async fn open_term_new_on_a_terminal_tenant_opens_without_workspace() {
        // `cs terminal new` (no path) must WORK on a standalone terminal
        // window: it's window routing, not a workspace op. A live /ws
        // subscriber stands in for the connected SPA window so
        // send_window_command succeeds.
        let workspace_cell: Arc<RwLock<Option<WorkspaceCell>>> = Arc::new(RwLock::new(None));
        let ctx = test_ctx(workspace_cell, ControlTenant::TerminalOnly);
        let mut rx = ctx.events_tx.subscribe();

        let response = handle_request(
            ControlRequest::OpenTermNew {
                window_id: "terminal-win-0".into(),
                path: None,
                tab_name: None,
                tab_group: None,
            },
            &ctx,
        )
        .await;

        match response {
            ControlResponse::Ok { message } => assert_eq!(message, "terminal request queued"),
            ControlResponse::Error { message } => panic!("unexpected error: {message}"),
        }
        // The window command was broadcast to the (stand-in) connected window.
        let frame = rx.try_recv().expect("window command broadcast");
        assert!(frame.contains("open_term_new"), "frame: {frame}");
        assert!(frame.contains("terminal-win-0"), "frame: {frame}");
    }

    #[tokio::test]
    async fn open_term_new_with_path_on_a_terminal_tenant_rejects() {
        // `cs terminal new --path X` can't resolve against a workspace root
        // that doesn't exist here; reject with the pinned message rather than
        // silently dropping the requested cwd.
        let workspace_cell: Arc<RwLock<Option<WorkspaceCell>>> = Arc::new(RwLock::new(None));
        let ctx = test_ctx(workspace_cell, ControlTenant::TerminalOnly);

        let response = handle_request(
            ControlRequest::OpenTermNew {
                window_id: "terminal-win-0".into(),
                path: Some(std::path::PathBuf::from("notes")),
                tab_name: None,
                tab_group: None,
            },
            &ctx,
        )
        .await;

        match response {
            ControlResponse::Error { message } => {
                assert_eq!(message, TERM_NEW_PATH_NEEDS_WORKSPACE)
            }
            ControlResponse::Ok { message } => panic!("unexpected ok: {message}"),
        }
    }

    #[test]
    fn open_path_creates_markdown_and_broadcasts_window_command() {
        let cfg = tempfile::tempdir().expect("config dir");
        let root = tempfile::tempdir().expect("workspace root");
        std::fs::create_dir_all(root.path().join("notes")).expect("notes dir");
        let lib =
            chan_workspace::Library::open_at(cfg.path().join("config.toml")).expect("library");
        lib.register_workspace(root.path())
            .expect("register workspace");
        let workspace = lib.open_workspace(root.path()).expect("open workspace");
        let self_writes = crate::self_writes::SelfWrites::new();
        let (tx, mut rx) = broadcast::channel(4);

        let message = open_path(
            &workspace,
            &self_writes,
            "window-a",
            &root.path().join("notes/new.md"),
            &tx,
        )
        .expect("open path");

        assert!(message.contains("notes/new.md"));
        assert!(workspace.exists("notes/new.md"));
        let frame: Value = serde_json::from_str(&rx.try_recv().expect("window command"))
            .expect("window command json");
        assert_eq!(frame["type"], "window_command");
        assert_eq!(frame["window_id"], "window-a");
        assert_eq!(frame["command"], "open_file");
        assert_eq!(frame["path"], "notes/new.md");
    }

    #[test]
    fn open_path_enters_existing_directory() {
        let cfg = tempfile::tempdir().expect("config dir");
        let root = tempfile::tempdir().expect("workspace root");
        std::fs::create_dir_all(root.path().join("notes/sub")).expect("sub dir");
        let lib =
            chan_workspace::Library::open_at(cfg.path().join("config.toml")).expect("library");
        lib.register_workspace(root.path())
            .expect("register workspace");
        let workspace = lib.open_workspace(root.path()).expect("open workspace");
        let self_writes = crate::self_writes::SelfWrites::new();
        let (tx, mut rx) = broadcast::channel(4);

        let message = open_path(
            &workspace,
            &self_writes,
            "window-a",
            &root.path().join("notes/sub"),
            &tx,
        )
        .expect("open path");

        assert!(message.contains("notes/sub"));
        let frame: Value = serde_json::from_str(&rx.try_recv().expect("window command"))
            .expect("window command json");
        assert_eq!(frame["type"], "window_command");
        assert_eq!(frame["window_id"], "window-a");
        assert_eq!(frame["command"], "open_browser");
        assert_eq!(frame["path"], "notes/sub");
        assert_eq!(frame["select"], Value::Null);
        assert_eq!(frame["enter"], true);
    }

    fn test_workspace() -> (tempfile::TempDir, tempfile::TempDir, Arc<Workspace>) {
        let cfg = tempfile::tempdir().expect("config dir");
        let root = tempfile::tempdir().expect("workspace root");
        let lib =
            chan_workspace::Library::open_at(cfg.path().join("config.toml")).expect("library");
        lib.register_workspace(root.path())
            .expect("register workspace");
        let workspace = lib.open_workspace(root.path()).expect("open workspace");
        (cfg, root, workspace)
    }

    fn empty_registry() -> (tempfile::TempDir, TerminalRegistry) {
        use crate::config::TerminalConfig;
        use crate::terminal_sessions::RegistryConfig;
        let root = tempfile::tempdir().expect("workspace root");
        let registry = TerminalRegistry::new(RegistryConfig {
            workspace_root: root.path().to_path_buf(),
            mcp_socket_path: None,
            control_socket_path: None,
            terminal: TerminalConfig::default(),
        });
        (root, registry)
    }

    #[test]
    fn open_graph_broadcasts_window_command_for_a_directory() {
        let (_cfg, root, workspace) = test_workspace();
        std::fs::create_dir_all(root.path().join("notes/sub")).expect("sub dir");
        let (tx, mut rx) = broadcast::channel(4);

        let message = open_graph(
            &workspace,
            "window-a",
            Some(&root.path().join("notes/sub")),
            &tx,
        )
        .expect("open graph");

        assert!(message.contains("notes/sub"));
        let frame: Value = serde_json::from_str(&rx.try_recv().expect("window command"))
            .expect("window command json");
        assert_eq!(frame["command"], "open_graph");
        assert_eq!(frame["path"], "notes/sub");
        assert_eq!(frame["is_dir"], true);
    }

    #[test]
    fn open_graph_without_a_path_targets_the_whole_graph() {
        let (_cfg, _root, workspace) = test_workspace();
        let (tx, mut rx) = broadcast::channel(4);

        open_graph(&workspace, "window-a", None, &tx).expect("open graph");

        let frame: Value = serde_json::from_str(&rx.try_recv().expect("window command"))
            .expect("window command json");
        assert_eq!(frame["command"], "open_graph");
        assert_eq!(frame["path"], Value::Null);
        assert_eq!(frame["is_dir"], false);
    }

    #[test]
    fn open_term_new_uses_the_parent_directory_for_a_file() {
        let (_cfg, root, workspace) = test_workspace();
        std::fs::create_dir_all(root.path().join("notes")).expect("notes dir");
        std::fs::write(root.path().join("notes/today.md"), "x").expect("write file");
        let (tx, mut rx) = broadcast::channel(4);

        open_term_new(
            &workspace,
            "window-a",
            Some(&root.path().join("notes/today.md")),
            Some("build".into()),
            Some("foobar".into()),
            &tx,
        )
        .expect("open term new");

        let frame: Value = serde_json::from_str(&rx.try_recv().expect("window command"))
            .expect("window command json");
        assert_eq!(frame["command"], "open_term_new");
        assert_eq!(frame["cwd"], "notes");
        assert_eq!(frame["tab_name"], "build");
        assert_eq!(frame["tab_group"], "foobar");
    }

    #[test]
    fn open_dashboard_carries_the_carousel_index() {
        let (tx, mut rx) = broadcast::channel(4);

        open_dashboard("window-a", Some(2), false, &tx).expect("open dashboard");

        let frame: Value = serde_json::from_str(&rx.try_recv().expect("window command"))
            .expect("window command json");
        assert_eq!(frame["command"], "open_dashboard");
        assert_eq!(frame["carousel_index"], 2);
        // carousel_off omitted from the wire when false (is_false skip).
        assert_eq!(frame["carousel_off"], Value::Null);
    }

    #[test]
    fn open_dashboard_carries_carousel_off_when_set() {
        let (tx, mut rx) = broadcast::channel(4);

        open_dashboard("window-a", None, true, &tx).expect("open dashboard");

        let frame: Value = serde_json::from_str(&rx.try_recv().expect("window command"))
            .expect("window command json");
        assert_eq!(frame["command"], "open_dashboard");
        assert_eq!(frame["carousel_off"], true);
    }

    #[test]
    fn term_write_requires_a_selector() {
        let (_root, registry) = empty_registry();
        let err = term_write(&registry, None, None, "ls").expect_err("no selector");
        assert!(err.contains("selector"), "got: {err}");
    }

    #[test]
    fn term_write_reports_no_match_on_an_empty_registry() {
        let (_root, registry) = empty_registry();
        let err = term_write(&registry, Some("nope"), None, "ls").expect_err("no match");
        assert!(err.contains("no live terminal session"), "got: {err}");
    }

    #[test]
    fn term_scrollback_requires_a_tab_name() {
        let (_root, registry) = empty_registry();
        let err = term_scrollback(&registry, "   ").expect_err("blank name");
        assert!(err.contains("needs a tab name"), "got: {err}");
    }

    #[test]
    fn term_scrollback_reports_no_match_on_an_empty_registry() {
        let (_root, registry) = empty_registry();
        let err = term_scrollback(&registry, "@@Nope").expect_err("no match");
        assert!(err.contains("no live terminal session"), "got: {err}");
    }

    #[test]
    fn resolve_pane_window_prefers_window_id_then_resolves_tab_name() {
        let (_root, registry) = empty_registry();
        let registry = Arc::new(registry);
        // An explicit window_id wins outright (no registry lookup).
        assert_eq!(
            resolve_pane_window(Some("win-x".into()), None, Some(&registry)).unwrap(),
            "win-x"
        );
        // Neither a window_id nor a tab_name -> a clear "need a target" error.
        assert!(resolve_pane_window(None, None, Some(&registry)).is_err());
        // A tab selector that matches no live window -> error.
        assert!(resolve_pane_window(None, Some("@@Nope"), Some(&registry)).is_err());
        // A session bound to a window, owning a tab: --tab-name resolves it.
        let _h = registry
            .create(CreateOptions {
                size: PtySize {
                    cols: 80,
                    rows: 24,
                    pixel_width: 0,
                    pixel_height: 0,
                },
                tab_name: Some("@@Alice".into()),
                tab_group: None,
                window_id: Some("win-b".into()),
                mcp_env: true,
                cwd: None,
                command: None,
                env: Default::default(),
            })
            .expect("spawn session");
        assert_eq!(
            resolve_pane_window(None, Some("@@Alice"), Some(&registry)).unwrap(),
            "win-b"
        );
    }

    #[test]
    fn term_list_has_no_groups_without_sessions() {
        let (_root, registry) = empty_registry();
        let json = term_list(&registry).expect("term list");
        let value: Value = serde_json::from_str(&json).expect("json");
        assert_eq!(value["groups"], serde_json::json!({}));
    }

    #[test]
    fn term_restart_requires_a_selector() {
        let (_root, registry) = empty_registry();
        let err = term_restart(&registry, None, None).expect_err("no selector");
        assert!(err.contains("selector"), "got: {err}");
    }

    #[test]
    fn term_restart_reports_no_match_on_an_empty_registry() {
        let (_root, registry) = empty_registry();
        let err = term_restart(&registry, Some("nope"), None).expect_err("no match");
        assert!(err.contains("no live terminal session"), "got: {err}");
    }

    // A valid two-member team config TOML for the handle_team tests.
    const SAMPLE_TEAM_TOML: &str = r#"
team_name = "alpha"
host_name = "Neo"
host_handle = "@@Neo"
tab_group = "alpha"
created_at = "2026-05-29T00:00:00Z"

[[members]]
handle = "@@Lead"
command = "claude"
is_lead = true
agent = "claude"

[[members]]
handle = "@@Alice"
command = "codex"
is_lead = false
agent = "codex"
"#;

    fn empty_cell() -> Arc<RwLock<Option<WorkspaceCell>>> {
        Arc::new(RwLock::new(None))
    }

    /// A throwaway events sender for `handle_team` / `spawn_and_poke_team` in
    /// tests. The `TeamSpawned` push goes here; with no receiver the
    /// broadcast `send` is a harmless no-op, so the tests do not assert on it.
    fn test_events() -> broadcast::Sender<String> {
        broadcast::channel::<String>(8).0
    }

    #[tokio::test]
    async fn handle_team_rejects_empty_and_absolute_dir() {
        let ctx = test_ctx(empty_cell(), ControlTenant::Workspace);
        match handle_team(
            TeamRequest {
                dir: "   ".to_string(),
                op: TeamOp::New,
                config_toml: None,
                script: false,
                window_id: None,
            },
            &ctx,
        )
        .await
        {
            ControlResponse::Error { message } => {
                assert!(message.contains("required"), "{message}")
            }
            ControlResponse::Ok { message } => panic!("unexpected ok: {message}"),
        }
        match handle_team(
            TeamRequest {
                dir: "/abs/team".to_string(),
                op: TeamOp::Load,
                config_toml: None,
                script: false,
                window_id: None,
            },
            &ctx,
        )
        .await
        {
            ControlResponse::Error { message } => {
                assert!(message.contains("workspace-relative"), "{message}")
            }
            ControlResponse::Ok { message } => panic!("unexpected ok: {message}"),
        }
    }

    #[tokio::test]
    async fn handle_team_new_requires_a_config() {
        let ctx = test_ctx(empty_cell(), ControlTenant::Workspace);
        match handle_team(
            TeamRequest {
                dir: "new-team-1".to_string(),
                op: TeamOp::New,
                config_toml: None,
                script: false,
                window_id: None,
            },
            &ctx,
        )
        .await
        {
            ControlResponse::Error { message } => {
                assert!(message.contains("needs a config"), "{message}")
            }
            ControlResponse::Ok { message } => panic!("unexpected ok: {message}"),
        }
    }

    #[tokio::test]
    async fn handle_team_new_script_emits_bootstrap_without_a_workspace() {
        // `--script` is a pure generator: it returns the script even with
        // no workspace cell bound (no filesystem I/O on this path).
        let ctx = test_ctx(empty_cell(), ControlTenant::Workspace);
        match handle_team(
            TeamRequest {
                dir: "new-team-1".to_string(),
                op: TeamOp::New,
                config_toml: Some(SAMPLE_TEAM_TOML.into()),
                script: true,
                window_id: None,
            },
            &ctx,
        )
        .await
        {
            ControlResponse::Ok { message } => {
                assert!(message.starts_with("#!/usr/bin/env bash"), "{message}");
                assert!(message.contains("--tab-name='@@Lead'"), "{message}");
                assert!(message.contains("--submit=codex"), "{message}");
            }
            ControlResponse::Error { message } => panic!("unexpected error: {message}"),
        }
    }

    #[tokio::test]
    async fn handle_team_new_rejects_invalid_toml() {
        let ctx = test_ctx(empty_cell(), ControlTenant::Workspace);
        match handle_team(
            TeamRequest {
                dir: "new-team-1".to_string(),
                op: TeamOp::New,
                config_toml: Some("this is not = = toml".into()),
                script: true,
                window_id: None,
            },
            &ctx,
        )
        .await
        {
            ControlResponse::Error { message } => {
                assert!(message.contains("invalid team config TOML"), "{message}")
            }
            ControlResponse::Ok { message } => panic!("unexpected ok: {message}"),
        }
    }

    #[tokio::test]
    async fn handle_team_new_rejects_a_config_that_fails_validation() {
        // Valid TOML, but zero members -> validation fails before any write.
        let cell = empty_cell();
        let toml_text = r#"
team_name = "alpha"
host_name = "Neo"
host_handle = "@@Neo"
created_at = "2026-05-29T00:00:00Z"
"#;
        match handle_team(
            TeamRequest {
                dir: "new-team-1".to_string(),
                op: TeamOp::New,
                config_toml: Some(toml_text.into()),
                script: true,
                window_id: None,
            },
            &test_ctx(cell, ControlTenant::Workspace),
        )
        .await
        {
            ControlResponse::Error { message } => {
                assert!(message.contains("between 1 and 9"), "{message}")
            }
            ControlResponse::Ok { message } => panic!("unexpected ok: {message}"),
        }
    }

    // A team whose members run a benign shell (blank command -> default
    // login shell), so `spawn_team` brings them up in CI without needing the
    // real agent binaries on PATH. Lead @@Lead (claude submit chord), worker
    // @@Alice (codex), plus a shell member @@Shell (no agent -> no poke).
    const SPAWNABLE_TEAM_TOML: &str = r#"
team_name = "spawnme"
host_name = "Neo"
host_handle = "@@Neo"
tab_group = "spawnme"
created_at = "2026-05-29T00:00:00Z"

# The submit agent is DERIVED from the command (+ a CHAN_AGENT env override).
# These fixtures use empty commands (so the spawn launches a harmless shell,
# not a real agent binary), and CHAN_AGENT to classify which members are
# agents vs the shell member. The command-sniff path is unit-tested in
# chan_shell::submit.
[[members]]
handle = "@@Alice"
command = ""
is_lead = false
env = { CHAN_AGENT = "codex" }

[[members]]
handle = "@@Lead"
command = ""
is_lead = true
env = { CHAN_AGENT = "claude" }

[[members]]
handle = "@@Shell"
command = ""
is_lead = false
"#;

    fn spawnable_config() -> TeamConfig {
        toml::from_str(SPAWNABLE_TEAM_TOML).expect("valid spawnable team config")
    }

    #[test]
    fn resolve_team_group_appends_suffix_on_collision() {
        let (_root, registry) = empty_registry();
        assert_eq!(
            resolve_team_group(&registry, "alpha"),
            "alpha",
            "no live group -> base verbatim"
        );
        // Bring up a session in group "alpha"; the next resolve must dodge it.
        let _h = registry
            .create(CreateOptions {
                size: PtySize {
                    cols: 80,
                    rows: 24,
                    pixel_width: 0,
                    pixel_height: 0,
                },
                tab_name: Some("@@x".into()),
                tab_group: Some("alpha".into()),
                window_id: None,
                mcp_env: true,
                cwd: None,
                command: None,
                env: Default::default(),
            })
            .expect("spawn collision session");
        assert_eq!(resolve_team_group(&registry, "alpha"), "alpha-2");
    }

    #[test]
    fn spawn_team_brings_up_lead_first_and_pokes_only_agents() {
        let (_root, registry) = empty_registry();
        let config = spawnable_config();
        let spawn = spawn_team(&registry, "new-team-1", &config, Some("win-spawn"));

        // Lead first, then roster order: @@Lead, @@Alice, @@Shell.
        assert_eq!(spawn.spawned, vec!["@@Lead", "@@Alice", "@@Shell"]);
        assert!(
            spawn.failed.is_empty(),
            "no spawn failed: {:?}",
            spawn.failed
        );
        assert_eq!(spawn.group, "spawnme");

        // Only the two AGENT members get an identity poke; the shell member
        // (no agent) does not. Lead's poke carries claude's chord, the
        // worker's carries codex's CR.
        assert_eq!(spawn.pokes.len(), 2, "agents only: {:?}", spawn.pokes);
        assert_eq!(spawn.pokes[0].0, "@@Lead");
        // claude is a SINGLE write ending in its modifyOtherKeys chord.
        assert_eq!(spawn.pokes[0].1.len(), 1, "claude poke is one write");
        assert!(
            spawn.pokes[0].1[0].ends_with("\x1b[27;9;13~"),
            "lead poke ends with claude chord: {:?}",
            spawn.pokes[0].1
        );
        assert!(
            spawn.pokes[0].1[0].contains("You are @@Lead"),
            "lead poke names the lead: {:?}",
            spawn.pokes[0].1
        );
        assert_eq!(spawn.pokes[1].0, "@@Alice");
        // codex is a SINGLE write (bracketed-paste wrap) ending in CR.
        assert_eq!(spawn.pokes[1].1.len(), 1, "codex poke is one write");
        assert!(
            spawn.pokes[1].1[0].ends_with('\r') && !spawn.pokes[1].1[0].ends_with("\x1b[27;9;13~"),
            "worker poke ends with codex CR: {:?}",
            spawn.pokes[1].1
        );

        // All three sessions live in the resolved group.
        let summaries = registry.session_summaries();
        assert_eq!(summaries.len(), 3);
        assert!(summaries.iter().all(|s| s.tab_group == "spawnme"));

        // Every spawned agent binds to the caller's window, so
        // `window_ids_matching` (the survey / pane-selector resolver) finds
        // it. A windowless spawn (None) would leave nothing to match.
        assert_eq!(
            registry.window_ids_matching(None, Some("spawnme")),
            vec!["win-spawn".to_string()],
            "agents are bound to the caller window"
        );
    }

    // The open_survey frame must carry the target tab as camelCase
    // `tabName`. A green compile alone would not catch a snake_case
    // drift, so pin the wire string here.
    #[test]
    fn open_survey_frame_serializes_tab_name_as_camel_case_tabname() {
        let spec = SurveySpec {
            survey_id: "sid-1".into(),
            title: None,
            body_markdown: "pick one".into(),
            options: vec!["a".into(), "b".into()],
            followup: None,
        };
        // `--tab-name=X` -> the frame carries `tabName`.
        let with_tab = serde_json::to_value(WindowCommand::OpenSurvey {
            survey: spec.clone(),
            tab_name: Some("@@Probe".into()),
        })
        .expect("serialize open_survey");
        assert_eq!(with_tab["command"], "open_survey");
        assert_eq!(with_tab["tabName"], "@@Probe");
        assert!(
            with_tab.get("tab_name").is_none(),
            "wire field must be camelCase tabName, not tab_name"
        );
        // No specific tab -> `tabName` is omitted; the SPA keeps its window-wide
        // fallback.
        let without_tab = serde_json::to_value(WindowCommand::OpenSurvey {
            survey: spec,
            tab_name: None,
        })
        .expect("serialize open_survey");
        assert!(
            without_tab.get("tabName").is_none(),
            "None tab_name omits the field"
        );
    }

    // A registry that advertises an MCP socket path, so `set_mcp_env` actually
    // stamps the CHAN_MCP_* descriptor when a member's mcp_env is on (the
    // `empty_registry` helper sets it to None, which no-ops the env even when
    // the toggle is on). The path need not point at a live socket: the child
    // only receives it as env, it does not dial it at spawn time.
    #[cfg(unix)]
    fn registry_with_mcp_socket() -> (tempfile::TempDir, TerminalRegistry) {
        use crate::config::TerminalConfig;
        use crate::terminal_sessions::RegistryConfig;
        let root = tempfile::tempdir().expect("workspace root");
        let registry = TerminalRegistry::new(RegistryConfig {
            workspace_root: root.path().to_path_buf(),
            mcp_socket_path: Some(std::path::PathBuf::from("/tmp/chan-test-mcp.sock")),
            control_socket_path: None,
            terminal: TerminalConfig::default(),
        });
        (root, registry)
    }

    #[cfg(unix)]
    fn probe_team_config(mcp_env: bool) -> TeamConfig {
        TeamConfig {
            team_name: "probe".into(),
            host_name: "Neo".into(),
            host_handle: "@@Neo".into(),
            tab_group: "probe".into(),
            auto_prefix_at: false,
            mcp_env,
            created_at: "2026-06-03T00:00:00Z".into(),
            members: vec![chan_workspace::Member {
                handle: "@@Probe".into(),
                // `${VAR:+set}` expands to "set" when CHAN_MCP_SERVER_JSON is
                // present and "" when absent, so the marker line records the
                // toggle outcome. `sleep` keeps the session alive to be read.
                command: "printf '<<MCP:%s>>\\n' \"${CHAN_MCP_SERVER_JSON:+set}\"; sleep 3".into(),
                env: Default::default(),
                is_lead: true,
                position: None,
            }],
        }
    }

    // E2E: the team config `mcp_env` toggle must flow all the way to the
    // spawned member's PTY env (TeamConfig.mcp_env -> CreateOptions.mcp_env in
    // spawn_team -> set_mcp_env on the child). mcp_env=true stamps
    // CHAN_MCP_SERVER_JSON; mcp_env=false omits it. Read off the member's PTY
    // scrollback via the marker the probe command prints.
    #[cfg(unix)]
    #[test]
    fn spawn_team_mcp_env_toggle_reaches_member_pty_env() {
        use std::time::{Duration, Instant};
        for mcp_env in [true, false] {
            let (_root, registry) = registry_with_mcp_socket();
            let config = probe_team_config(mcp_env);
            let spawn = spawn_team(&registry, "probe-team", &config, Some("win-probe"));
            assert_eq!(spawn.spawned, vec!["@@Probe"], "probe member spawned");

            let deadline = Instant::now() + Duration::from_secs(6);
            let mut out = String::new();
            loop {
                if let Ok(s) = term_scrollback(&registry, "@@Probe") {
                    out = s;
                    if out.contains("<<MCP:") {
                        break;
                    }
                }
                if Instant::now() >= deadline {
                    break;
                }
                std::thread::sleep(Duration::from_millis(100));
            }

            if mcp_env {
                assert!(
                    out.contains("<<MCP:set>>"),
                    "team mcp_env=true should stamp CHAN_MCP_SERVER_JSON on the \
                     member; scrollback: {out:?}"
                );
            } else {
                assert!(
                    out.contains("<<MCP:>>") && !out.contains("<<MCP:set>>"),
                    "team mcp_env=false should omit CHAN_MCP_SERVER_JSON on the \
                     member; scrollback: {out:?}"
                );
            }
        }
    }

    // A team of two shell members (one lead, neither an agent). spawn_and_
    // poke_team brings them up but pokes nobody, so it skips the boot-grace
    // wait and returns immediately - exercises the spawn path end-to-end
    // without a 3s sleep in the test.
    const SHELL_TEAM_TOML: &str = r#"
team_name = "shellsquad"
host_name = "Neo"
host_handle = "@@Neo"
tab_group = "shellsquad"
created_at = "2026-05-29T00:00:00Z"

[[members]]
handle = "@@Boss"
command = ""
is_lead = true

[[members]]
handle = "@@Hand"
command = ""
is_lead = false
"#;

    #[tokio::test]
    async fn spawn_and_poke_team_with_shell_members_skips_the_poke_wait() {
        let (_root, registry) = empty_registry();
        let registry = Arc::new(registry);
        let config: TeamConfig = toml::from_str(SHELL_TEAM_TOML).expect("valid shell team");
        match spawn_and_poke_team(&registry, "new-team-1", &config, None, &test_events()).await {
            ControlResponse::Ok { message } => {
                assert!(message.contains("shellsquad"), "{message}");
                assert!(message.contains("2 member(s) up"), "{message}");
                assert!(message.contains("poked 0 agent(s)"), "{message}");
            }
            ControlResponse::Error { message } => panic!("unexpected error: {message}"),
        }
        assert_eq!(registry.session_summaries().len(), 2);
    }

    #[tokio::test]
    async fn spawn_and_poke_team_surfaces_to_the_window() {
        // A windowed spawn pushes a `team_spawned` frame to that window
        // carrying the group + each member's tab name and live session id, so
        // the SPA can open a terminal tab attached to the session.
        let (_root, registry) = empty_registry();
        let registry = Arc::new(registry);
        let config: TeamConfig = toml::from_str(SHELL_TEAM_TOML).expect("valid shell team");
        let (events_tx, mut rx) = broadcast::channel::<String>(8);
        spawn_and_poke_team(&registry, "new-team-1", &config, Some("win-s1"), &events_tx).await;

        let frame: serde_json::Value =
            serde_json::from_str(&rx.try_recv().expect("team_spawned frame")).expect("json");
        assert_eq!(frame["type"], "window_command");
        assert_eq!(frame["window_id"], "win-s1");
        assert_eq!(frame["command"], "team_spawned");
        assert_eq!(frame["group"], "shellsquad");
        let members = frame["members"].as_array().expect("members array");
        assert_eq!(members.len(), 2);
        assert_eq!(members[0]["tab_name"], "@@Boss");
        assert!(
            members[0]["session_id"]
                .as_str()
                .is_some_and(|s| !s.is_empty()),
            "member carries a live session id: {members:?}"
        );
    }

    #[tokio::test]
    async fn spawn_and_poke_team_windowless_does_not_surface() {
        // No window to surface into -> no `team_spawned` push (the SPA learns
        // the sessions on its next attach, as before).
        let (_root, registry) = empty_registry();
        let registry = Arc::new(registry);
        let config: TeamConfig = toml::from_str(SHELL_TEAM_TOML).expect("valid shell team");
        let (events_tx, mut rx) = broadcast::channel::<String>(8);
        spawn_and_poke_team(&registry, "new-team-1", &config, None, &events_tx).await;
        assert!(
            rx.try_recv().is_err(),
            "a windowless spawn must not surface a team_spawned frame"
        );
    }

    /// A workspace cell with a SHELL_TEAM_TOML team already written under
    /// `dir`, for the `handle_team` Load tests. The indexer is real (the cell
    /// requires one) but never used here; both shell members spawn without an
    /// agent binary on PATH.
    fn bound_cell_with_shell_team(
        dir: &str,
    ) -> (
        tempfile::TempDir,
        tempfile::TempDir,
        Arc<RwLock<Option<WorkspaceCell>>>,
    ) {
        use crate::routes::team_config::write_team_config;
        let cfg = tempfile::tempdir().expect("config dir");
        let root = tempfile::tempdir().expect("workspace root");
        let lib =
            chan_workspace::Library::open_at(cfg.path().join("config.toml")).expect("library");
        lib.register_workspace(root.path())
            .expect("register workspace");
        let workspace = lib.open_workspace(root.path()).expect("open workspace");
        let config: TeamConfig = toml::from_str(SHELL_TEAM_TOML).expect("valid shell team");
        write_team_config(&workspace, dir, &config).expect("write team");
        let (index_tx, index_rx) = broadcast::channel::<chan_workspace::WatchEvent>(1);
        // Keep the channel open for the indexer's lifetime; the test never
        // sends on it.
        std::mem::forget(index_tx);
        let indexer = Arc::new(crate::indexer::Indexer::spawn(
            workspace.clone(),
            index_rx,
            false,
            chan_workspace::SearchAggression::Conservative,
            Arc::new(chan_workspace::NoProgress),
        ));
        let cell = Arc::new(RwLock::new(Some(WorkspaceCell {
            workspace,
            watch_handle: None,
            indexer,
        })));
        (cfg, root, cell)
    }

    #[tokio::test]
    async fn handle_team_load_spawns_the_saved_team() {
        // `cs terminal team load` brings the saved team UP, not just
        // summarizes it. The two shell members come live in the registry.
        let (_cfg, _root, cell) = bound_cell_with_shell_team("saved-team");
        let (_rroot, registry) = empty_registry();
        let registry = Arc::new(registry);
        let ctx = test_ctx(cell, ControlTenant::Workspace);
        assert!(
            ctx.terminal_registry.set(registry.clone()).is_ok(),
            "fresh registry cell"
        );
        match handle_team(
            TeamRequest {
                dir: "saved-team".to_string(),
                op: TeamOp::Load,
                config_toml: None,
                script: false,
                window_id: Some("win-load".to_string()),
            },
            &ctx,
        )
        .await
        {
            ControlResponse::Ok { message } => {
                assert!(message.contains("spawned"), "load spawns: {message}");
                assert!(message.contains("2 member(s) up"), "{message}");
            }
            ControlResponse::Error { message } => panic!("unexpected error: {message}"),
        }
        assert_eq!(
            registry.session_summaries().len(),
            2,
            "both shell members are live after a load"
        );
        // Window binding applies to Load too: the loaded team's agents bind
        // to the caller's window.
        assert_eq!(
            registry.window_ids_matching(None, Some("shellsquad")),
            vec!["win-load".to_string()],
            "loaded agents bind to the caller window"
        );
    }

    #[tokio::test]
    async fn handle_team_load_without_registry_falls_back_to_a_summary() {
        // A server with terminals disabled has nothing to spawn into, so Load
        // still validates + summarizes instead of erroring.
        let (_cfg, _root, cell) = bound_cell_with_shell_team("saved-team");
        match handle_team(
            TeamRequest {
                dir: "saved-team".to_string(),
                op: TeamOp::Load,
                config_toml: None,
                script: false,
                window_id: None,
            },
            &test_ctx(cell, ControlTenant::Workspace),
        )
        .await
        {
            ControlResponse::Ok { message } => {
                assert!(
                    message.contains("no terminal registry to spawn into"),
                    "{message}"
                );
                assert!(message.contains("2 member(s)"), "{message}");
            }
            ControlResponse::Error { message } => panic!("unexpected error: {message}"),
        }
    }

    #[test]
    fn team_spawn_summary_counts_up_poked_and_failed() {
        let spawn = TeamSpawn {
            group: "alpha".into(),
            spawned: vec!["@@Lead".into(), "@@A".into()],
            failed: vec![("@@B".into(), "no such file".into())],
            pokes: vec![("@@Lead".into(), vec!["hi\x1b[27;9;13~".into()])],
            members: vec![],
        };
        match team_spawn_summary("alpha", &spawn) {
            ControlResponse::Ok { message } => {
                assert!(message.contains("2 member(s) up"), "{message}");
                assert!(message.contains("poked 1 agent(s)"), "{message}");
                assert!(
                    message.contains("1 failed: @@B (no such file)"),
                    "{message}"
                );
            }
            ControlResponse::Error { message } => panic!("unexpected error: {message}"),
        }
    }

    #[test]
    fn team_spawn_summary_errors_when_nothing_came_up() {
        let spawn = TeamSpawn {
            group: "alpha".into(),
            spawned: vec![],
            failed: vec![("@@Lead".into(), "boom".into())],
            pokes: vec![],
            members: vec![],
        };
        match team_spawn_summary("alpha", &spawn) {
            ControlResponse::Error { message } => {
                assert!(message.contains("no member could be spawned"), "{message}");
                assert!(message.contains("@@Lead (boom)"), "{message}");
            }
            ControlResponse::Ok { message } => panic!("unexpected ok: {message}"),
        }
    }
}
