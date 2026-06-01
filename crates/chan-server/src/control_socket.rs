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
use chan_shell::{apply_submit_chord, SubmitAgent, SurveyReply, SurveySpec, TeamOp};

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
    },
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

#[cfg(unix)]
pub fn start(
    socket_path: PathBuf,
    workspace_cell: Arc<RwLock<Option<WorkspaceCell>>>,
    events_tx: broadcast::Sender<String>,
    self_writes: Arc<crate::self_writes::SelfWrites>,
    terminal_registry: TerminalRegistryCell,
    survey_bus: Arc<crate::survey::SurveyBus>,
) -> std::io::Result<ControlHandle> {
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
            let workspace_cell = workspace_cell.clone();
            let events_tx = events_tx.clone();
            let self_writes = self_writes.clone();
            let terminal_registry = terminal_registry.clone();
            let survey_bus = survey_bus.clone();
            tokio::spawn(async move {
                let (read, mut write) = stream.into_split();
                let mut reader = BufReader::new(read);
                let mut line = String::new();
                let response = match reader.read_line(&mut line).await {
                    Ok(0) => ControlResponse::Error {
                        message: "empty control request".into(),
                    },
                    Ok(_) => match serde_json::from_str::<ControlRequest>(&line) {
                        Ok(req) => {
                            handle_request(
                                req,
                                &workspace_cell,
                                &events_tx,
                                &self_writes,
                                terminal_registry.get(),
                                &survey_bus,
                            )
                            .await
                        }
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
pub fn start(
    _socket_path: PathBuf,
    _workspace_cell: Arc<RwLock<Option<WorkspaceCell>>>,
    _events_tx: broadcast::Sender<String>,
    _self_writes: Arc<crate::self_writes::SelfWrites>,
    _terminal_registry: TerminalRegistryCell,
    _survey_bus: Arc<crate::survey::SurveyBus>,
) -> std::io::Result<ControlHandle> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "control socket requires unix-domain sockets",
    ))
}

// Async because of the one blocking variant (`TermSurvey`); every other
// arm returns synchronously without awaiting.
#[cfg(unix)]
async fn handle_request(
    req: ControlRequest,
    workspace_cell: &Arc<RwLock<Option<WorkspaceCell>>>,
    events_tx: &broadcast::Sender<String>,
    self_writes: &crate::self_writes::SelfWrites,
    terminal_registry: Option<&Arc<TerminalRegistry>>,
    survey_bus: &Arc<crate::survey::SurveyBus>,
) -> ControlResponse {
    match req {
        ControlRequest::OpenPath { window_id, path } => {
            if let Err(message) = require_window_id(&window_id) {
                return ControlResponse::Error { message };
            }
            let workspace = match workspace_from_cell(workspace_cell) {
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
            let workspace = match workspace_from_cell(workspace_cell) {
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
            let workspace = match workspace_from_cell(workspace_cell) {
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
        ControlRequest::Search { query, limit } => {
            let workspace = match workspace_from_cell(workspace_cell) {
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
        } => {
            handle_team(
                workspace_cell,
                terminal_registry,
                &dir,
                op,
                config_toml,
                script,
            )
            .await
        }
    }
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
async fn handle_team(
    workspace_cell: &Arc<RwLock<Option<WorkspaceCell>>>,
    terminal_registry: Option<&Arc<TerminalRegistry>>,
    dir: &str,
    op: TeamOp,
    config_toml: Option<String>,
    script: bool,
) -> ControlResponse {
    use crate::routes::team_config::{
        ensure_created_at, generate_bootstrap_script, read_team_config, validate_team_config,
        write_team_config,
    };

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
            let workspace = match workspace_from_cell(workspace_cell) {
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
            spawn_and_poke_team(registry, dir, &config).await
        }
        TeamOp::Load => {
            let workspace = match workspace_from_cell(workspace_cell) {
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
            let lead = config
                .members
                .iter()
                .find(|m| m.is_lead)
                .map(|m| m.handle.as_str())
                .unwrap_or("?");
            ControlResponse::Ok {
                message: format!(
                    "team {:?} at {dir}: {} member(s), lead {lead}",
                    config.team_name,
                    config.members.len()
                ),
            }
        }
    }
}

/// Boot grace between spawning the team's agents and poking their compose
/// boxes. Matches the `--script` form's inline `sleep 3`: a freshly-spawned
/// agent needs a moment before its compose box accepts input, else the
/// identity poke lands mid-startup and is lost. This is the one magic number
/// in the spawn path; the Wave-2 live smoke validates it.
#[cfg(unix)]
const TEAM_SPAWN_POKE_GRACE: std::time::Duration = std::time::Duration::from_secs(3);

/// What a server-side team spawn produced: the resolved group, the handles
/// that came up, the ones that failed (with the spawn error), and the
/// per-agent identity pokes to deliver after the boot grace.
#[cfg(unix)]
struct TeamSpawn {
    group: String,
    spawned: Vec<String>,
    failed: Vec<(String, String)>,
    /// `(handle, payload)` for each AGENT member; the payload is the
    /// identity prompt with the agent's submit chord already appended.
    pokes: Vec<(String, String)>,
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
fn spawn_team(registry: &TerminalRegistry, dir: &str, config: &TeamConfig) -> TeamSpawn {
    use crate::routes::team_config::{identity_prompt, lead_first_order, team_base_group};

    let group = resolve_team_group(registry, team_base_group(config));
    let mut spawned = Vec::new();
    let mut failed = Vec::new();
    let mut pokes = Vec::new();
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
            window_id: None,
            mcp_env: true,
            cwd: None,
            command,
            env: m.env.clone(),
        };
        match registry.create(opts) {
            Ok(_handle) => {
                // Drop the attach handle: the session stays in the registry
                // map, and the boot-grace poke re-resolves it by
                // tab-name + group.
                spawned.push(m.handle.clone());
                if let Some(agent) = m.agent.as_deref().and_then(SubmitAgent::from_agent_name) {
                    let payload = apply_submit_chord(identity_prompt(config, dir, m), Some(agent));
                    pokes.push((m.handle.clone(), payload));
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
) -> ControlResponse {
    let spawn = spawn_team(registry, dir, config);

    // Let the agents come up before poking their compose boxes, then deliver
    // each agent its identity prompt + submit chord. A shell member has no
    // compose box, so it has no poke entry; an all-shell team (or a fully
    // failed spawn) skips the wait entirely.
    if !spawn.spawned.is_empty() && !spawn.pokes.is_empty() {
        tokio::time::sleep(TEAM_SPAWN_POKE_GRACE).await;
        for (handle, payload) in &spawn.pokes {
            registry.write_input_matching(Some(handle), Some(&spawn.group), payload.as_bytes());
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

/// The stdout line the CLI prints for a completed survey: the bare chosen
/// option label, or the `new follow up file created: ...` line on `[F]`.
#[cfg(unix)]
fn format_survey_reply(reply: &SurveyReply) -> String {
    match reply {
        SurveyReply::Option { option_label, .. } => option_label.clone(),
        SurveyReply::Followup { followup_path, .. } => {
            format!("new follow up file created: {followup_path}")
        }
    }
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
) -> Result<Arc<Workspace>, String> {
    let cell = workspace_cell
        .read()
        .map_err(|_| "workspace cell lock poisoned".to_string())?;
    let cell = cell
        .as_ref()
        .ok_or_else(|| "workspace cell unavailable".to_string())?;
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
    let _ = events_tx.send(raw);
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

/// Category 2: write raw bytes to the matching live PTY sessions. At
/// least one selector is required so a missing filter cannot fan out to
/// every terminal by accident.
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
    let written = registry.write_input_matching(tab_name, tab_group, data.as_bytes());
    if written == 0 {
        return Err("no live terminal session matched".into());
    }
    Ok(format!("wrote to {written} terminal session(s)"))
}

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
        let self_writes = crate::self_writes::SelfWrites::new();
        let (tx, _) = broadcast::channel(1);
        let survey_bus = Arc::new(crate::survey::SurveyBus::new());

        let response = handle_request(
            ControlRequest::OpenPath {
                window_id: "window-a".to_string(),
                path: PathBuf::from("/tmp/note.md"),
            },
            &workspace_cell,
            &tx,
            &self_writes,
            None,
            &survey_bus,
        )
        .await;

        match response {
            ControlResponse::Error { message } => {
                assert_eq!(message, "workspace cell lock poisoned");
            }
            ControlResponse::Ok { message } => panic!("unexpected ok response: {message}"),
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
handle = "@@LaneA"
command = "codex"
is_lead = false
agent = "codex"
"#;

    fn empty_cell() -> Arc<RwLock<Option<WorkspaceCell>>> {
        Arc::new(RwLock::new(None))
    }

    #[tokio::test]
    async fn handle_team_rejects_empty_and_absolute_dir() {
        let cell = empty_cell();
        match handle_team(&cell, None, "   ", TeamOp::New, None, false).await {
            ControlResponse::Error { message } => {
                assert!(message.contains("required"), "{message}")
            }
            ControlResponse::Ok { message } => panic!("unexpected ok: {message}"),
        }
        match handle_team(&cell, None, "/abs/team", TeamOp::Load, None, false).await {
            ControlResponse::Error { message } => {
                assert!(message.contains("workspace-relative"), "{message}")
            }
            ControlResponse::Ok { message } => panic!("unexpected ok: {message}"),
        }
    }

    #[tokio::test]
    async fn handle_team_new_requires_a_config() {
        let cell = empty_cell();
        match handle_team(&cell, None, "new-team-1", TeamOp::New, None, false).await {
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
        let cell = empty_cell();
        match handle_team(
            &cell,
            None,
            "new-team-1",
            TeamOp::New,
            Some(SAMPLE_TEAM_TOML.into()),
            true,
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
        let cell = empty_cell();
        match handle_team(
            &cell,
            None,
            "new-team-1",
            TeamOp::New,
            Some("this is not = = toml".into()),
            true,
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
            &cell,
            None,
            "new-team-1",
            TeamOp::New,
            Some(toml_text.into()),
            true,
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
    // @@LaneA (codex), plus a shell member @@Shell (no agent -> no poke).
    const SPAWNABLE_TEAM_TOML: &str = r#"
team_name = "spawnme"
host_name = "Neo"
host_handle = "@@Neo"
tab_group = "spawnme"
created_at = "2026-05-29T00:00:00Z"

[[members]]
handle = "@@LaneA"
command = ""
is_lead = false
agent = "codex"

[[members]]
handle = "@@Lead"
command = ""
is_lead = true
agent = "claude"

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
        let spawn = spawn_team(&registry, "new-team-1", &config);

        // Lead first, then roster order: @@Lead, @@LaneA, @@Shell.
        assert_eq!(spawn.spawned, vec!["@@Lead", "@@LaneA", "@@Shell"]);
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
        assert!(
            spawn.pokes[0].1.ends_with("\x1b[27;9;13~"),
            "lead poke ends with claude chord: {:?}",
            spawn.pokes[0].1
        );
        assert!(
            spawn.pokes[0].1.contains("You are @@Lead"),
            "lead poke names the lead: {:?}",
            spawn.pokes[0].1
        );
        assert_eq!(spawn.pokes[1].0, "@@LaneA");
        assert!(
            spawn.pokes[1].1.ends_with('\r') && !spawn.pokes[1].1.ends_with("\x1b[27;9;13~"),
            "worker poke ends with codex CR: {:?}",
            spawn.pokes[1].1
        );

        // All three sessions live in the resolved group.
        let summaries = registry.session_summaries();
        assert_eq!(summaries.len(), 3);
        assert!(summaries.iter().all(|s| s.tab_group == "spawnme"));
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
        match spawn_and_poke_team(&registry, "new-team-1", &config).await {
            ControlResponse::Ok { message } => {
                assert!(message.contains("shellsquad"), "{message}");
                assert!(message.contains("2 member(s) up"), "{message}");
                assert!(message.contains("poked 0 agent(s)"), "{message}");
            }
            ControlResponse::Error { message } => panic!("unexpected error: {message}"),
        }
        assert_eq!(registry.session_summaries().len(), 2);
    }

    #[test]
    fn team_spawn_summary_counts_up_poked_and_failed() {
        let spawn = TeamSpawn {
            group: "alpha".into(),
            spawned: vec!["@@Lead".into(), "@@A".into()],
            failed: vec![("@@B".into(), "no such file".into())],
            pokes: vec![("@@Lead".into(), "hi\x1b[27;9;13~".into())],
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
