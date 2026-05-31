//! Team Work config, persisted INSIDE the workspace.
//!
//! Backs the Team Work dialog's New/Load flow. The dialog persists
//! its full state under a user-chosen workspace-RELATIVE directory
//! `{dir}/` (e.g. `new-team-1` or `teams/alpha`). The on-disk layout
//! is:
//!
//! ```text
//! {dir}/
//!   config.toml   the TeamConfig (users may hand-edit; revalidated
//!                 on every reload)
//!   bootstrap.md  generated team-wide process doc (tool-owned;
//!                 regenerated from config.toml on every write)
//!   tasks/        task-{from}-{to}-{n}.md  (owned by `to`, append-only)
//!   journals/     journal-{member}.md      (owned by each member)
//!   followups/    followup-{from}-{to}-{n}.md (owned by `to`)
//! ```
//!
//! All file I/O routes through `Workspace::{read_text,write_text,
//! create_dir}`, so the team config lives inside the same sandbox +
//! atomic-write contract as notes content. `config.toml` and the
//! generated `.md` both pass the editable-text gate. This is the
//! reverse of the earlier design, which deliberately wrote the config
//! to a user-chosen ABSOLUTE path outside the sandbox via `std::fs`;
//! that exception is gone. The team's `.md` docs are now indexed +
//! graphed like any other workspace content.

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chan_workspace::TeamConfig;
use serde::Deserialize;

use crate::error::err;
use crate::state::AppState;

/// `POST /api/team-config/read` body. `dir` is workspace-relative.
#[derive(Deserialize)]
pub struct ReadTeamConfigPayload {
    pub dir: String,
}

/// `POST /api/team-config/write` body. `dir` is workspace-relative;
/// `config` mirrors the SPA's `TeamConfigWire` shape 1:1 (it IS
/// `chan_workspace::TeamConfig`).
#[derive(Deserialize)]
pub struct WriteTeamConfigPayload {
    pub dir: String,
    pub config: TeamConfig,
}

/// Up-front guard so a bad `dir` produces a clean 400 message instead
/// of a sandbox error deeper in the write path. The Workspace sandbox
/// already refuses `..` traversal, so we only reject empty + absolute
/// here. Returns the trimmed dir on success.
fn require_relative_dir(dir: &str) -> Result<String, String> {
    let trimmed = dir.trim();
    if trimmed.is_empty() {
        return Err("team directory is required".into());
    }
    if trimmed.starts_with('/') {
        return Err(format!(
            "team directory must be workspace-relative, not absolute: {trimmed}"
        ));
    }
    Ok(trimmed.to_string())
}

/// Structural validation run on BOTH read and write. Returns the first
/// failure as a human-readable string that becomes the 400 body, so
/// the SPA's New/Load flow can surface it inline.
fn validate_team_config(config: &TeamConfig) -> Result<(), String> {
    if config.members.is_empty() || config.members.len() > 9 {
        return Err("team must have between 1 and 9 members".into());
    }
    let lead_count = config.members.iter().filter(|m| m.is_lead).count();
    if lead_count != 1 {
        return Err("exactly one member must be marked as lead".into());
    }
    if config.team_name.trim().is_empty() {
        return Err("team_name must not be empty".into());
    }
    if config.host_name.trim().is_empty() {
        return Err("host_name must not be empty".into());
    }
    if config.host_handle.trim().is_empty() {
        return Err("host_handle must not be empty".into());
    }
    if config.members.iter().any(|m| m.handle.trim().is_empty()) {
        return Err("every member must have a non-empty handle".into());
    }
    Ok(())
}

/// `POST /api/team-config/read` - read `{dir}/config.toml`, parse +
/// validate it. 400 on missing file / invalid TOML / failed
/// validation so the Load flow surfaces the message inline.
pub async fn api_team_config_read(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ReadTeamConfigPayload>,
) -> Response {
    let dir = match require_relative_dir(&payload.dir) {
        Ok(d) => d,
        Err(msg) => return err(StatusCode::BAD_REQUEST, msg),
    };
    let workspace = state.workspace();
    let result = tokio::task::spawn_blocking(move || read_team_config(&workspace, &dir)).await;
    match result {
        Ok(Ok(config)) => Json(config).into_response(),
        Ok(Err(msg)) => err(StatusCode::BAD_REQUEST, msg),
        Err(join) => err(StatusCode::INTERNAL_SERVER_ERROR, join.to_string()),
    }
}

/// `POST /api/team-config/write` - validate, create the `{dir}/` tree,
/// then write `config.toml` + the regenerated `bootstrap.md` through
/// the Workspace sandbox. 200 `{}` on success; 400 on bad dir / failed
/// validation / write failure.
pub async fn api_team_config_write(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<WriteTeamConfigPayload>,
) -> Response {
    let dir = match require_relative_dir(&payload.dir) {
        Ok(d) => d,
        Err(msg) => return err(StatusCode::BAD_REQUEST, msg),
    };
    let config = payload.config;
    let workspace = state.workspace();
    let result =
        tokio::task::spawn_blocking(move || write_team_config(&workspace, &dir, &config)).await;
    match result {
        Ok(Ok(())) => Json(serde_json::json!({})).into_response(),
        Ok(Err(msg)) => err(StatusCode::BAD_REQUEST, msg),
        Err(join) => err(StatusCode::INTERNAL_SERVER_ERROR, join.to_string()),
    }
}

fn read_team_config(
    workspace: &chan_workspace::Workspace,
    dir: &str,
) -> Result<TeamConfig, String> {
    let rel = format!("{dir}/config.toml");
    let text = workspace
        .read_text(&rel)
        .map_err(|e| format!("cannot read {rel}: {e}"))?;
    let config = toml::from_str::<TeamConfig>(&text)
        .map_err(|e| format!("invalid team config at {rel}: {e}"))?;
    validate_team_config(&config)?;
    Ok(config)
}

fn write_team_config(
    workspace: &chan_workspace::Workspace,
    dir: &str,
    config: &TeamConfig,
) -> Result<(), String> {
    validate_team_config(config)?;
    let toml_text =
        toml::to_string_pretty(config).map_err(|e| format!("cannot serialize team config: {e}"))?;

    // create_dir is create_dir_all, so creating the leaf subdirs also
    // materializes the `{dir}` parent. We still create `{dir}` first
    // for clarity and to fail early with a clean message on a bad path.
    for sub in [
        dir.to_string(),
        format!("{dir}/tasks"),
        format!("{dir}/journals"),
        format!("{dir}/followups"),
    ] {
        workspace
            .create_dir(&sub)
            .map_err(|e| format!("cannot create {sub}: {e}"))?;
    }

    let config_rel = format!("{dir}/config.toml");
    workspace
        .write_text(&config_rel, &toml_text)
        .map_err(|e| format!("cannot write {config_rel}: {e}"))?;

    let bootstrap_rel = format!("{dir}/bootstrap.md");
    let bootstrap = generate_bootstrap_md(dir, config);
    workspace
        .write_text(&bootstrap_rel, &bootstrap)
        .map_err(|e| format!("cannot write {bootstrap_rel}: {e}"))?;

    Ok(())
}

/// Generate the team-wide `bootstrap.md` process doc from `config`.
/// Tool-owned artifact, regenerated on every write. ASCII only, no em
/// dashes. `team_dir` is the workspace-relative dir the team lives in.
fn generate_bootstrap_md(team_dir: &str, config: &TeamConfig) -> String {
    let team_name = &config.team_name;
    let host_handle = &config.host_handle;
    let host_name = &config.host_name;
    // Validation guarantees exactly one lead by the time a write
    // reaches here; fall back to the first member defensively so the
    // generator never panics on a hand-built config that skipped the
    // gate (e.g. a future direct caller).
    let lead = config
        .members
        .iter()
        .find(|m| m.is_lead)
        .or_else(|| config.members.first());
    let lead_handle = lead.map(|m| m.handle.as_str()).unwrap_or("@@Lead");

    let mut out = String::new();
    out.push_str(&format!("# {team_name} - team bootstrap\n\n"));
    out.push_str(&format!(
        "Generated for the {team_name} team. created_at: {}.\n\n",
        config.created_at
    ));

    out.push_str("## Who we are\n\n");
    out.push_str(&format!(
        "- Host: {host_handle} ({host_name}). The host sets scope and is the only\n  \
         one who acts outside the team; reach the host through {lead_handle}.\n"
    ));
    out.push_str(&format!(
        "- Lead: {lead_handle}. Distributes tasks, sequences the work, and\n  \
         aggregates requests for the host.\n\n"
    ));

    out.push_str("## Roster\n\n");
    out.push_str(&render_roster(config));
    out.push('\n');

    out.push_str("## How we work\n\n");
    out.push_str(&format!(
        "- Workers hold and wait for {lead_handle} to distribute tasks. Do not\n  \
         start until you are poked with your task path.\n"
    ));
    out.push_str(&format!(
        "- {lead_handle} cuts a task into {team_dir}/tasks/task-{{from}}-{{to}}-{{n}}.md\n  \
         (owned by the recipient, N is an atomic increment, append-only) and\n  \
         pokes the recipient.\n"
    ));
    out.push_str(&format!(
        "- On completion, cut a task back to {lead_handle} in the same place and\n  \
         format, then poke back.\n"
    ));
    out.push_str(&format!(
        "- Keep a running log in {team_dir}/journals/journal-{{your-name}}.md\n  \
         (owned by you, append-only).\n"
    ));
    out.push_str(&format!(
        "- Most worker-to-host communication routes through {lead_handle}, who\n  \
         aggregates requests for {host_handle}.\n\n"
    ));

    out.push_str("## The poke 1-liner\n\n");
    out.push_str(
        "Pokes are one-line pointers, not fat context. The context lives in the\n\
         task file you point to.\n\n",
    );
    out.push_str(
        "    cs terminal write --tab-name=<target> $'poke from <me>: <1-line>; read <path>\\x1b[27;9;13~'\n\n",
    );
    out.push_str(
        "The trailing \\x1b[27;9;13~ is the Meta+Enter submit chord; a bare newline\n\
         parks the poke unsubmitted in the target's compose box.\n\n",
    );

    out.push_str("## Files\n\n");
    out.push_str(
        "- config.toml    the team config (you may hand-edit; revalidated on reload)\n\
         - bootstrap.md   this file (generated from config.toml)\n\
         - tasks/         task-{from}-{to}-{n}.md, owned by the recipient, append-only\n\
         - journals/      journal-{member}.md, owned by each member, append-only\n\
         - followups/     followup-{from}-{to}-{n}.md, owned by the recipient\n\n",
    );
    out.push_str(
        "Task and followup filenames use the bare name (handle without the @@),\n\
         e.g. tasks/task-Lead-LaneA-1.md.\n",
    );

    out
}

/// Render the roster as a pure-ASCII table (handle | command | role),
/// columns padded to content width, targeting <=80 cols. One row per
/// member; role is "lead" or "worker".
fn render_roster(config: &TeamConfig) -> String {
    const H_HANDLE: &str = "handle";
    const H_COMMAND: &str = "command";
    const H_ROLE: &str = "role";

    let rows: Vec<(String, String, &'static str)> = config
        .members
        .iter()
        .map(|m| {
            let role = if m.is_lead { "lead" } else { "worker" };
            (m.handle.clone(), m.command.clone(), role)
        })
        .collect();

    let w_handle = rows
        .iter()
        .map(|(h, _, _)| h.len())
        .chain(std::iter::once(H_HANDLE.len()))
        .max()
        .unwrap_or(H_HANDLE.len());
    let w_command = rows
        .iter()
        .map(|(_, c, _)| c.len())
        .chain(std::iter::once(H_COMMAND.len()))
        .max()
        .unwrap_or(H_COMMAND.len());
    let w_role = rows
        .iter()
        .map(|(_, _, r)| r.len())
        .chain(std::iter::once(H_ROLE.len()))
        .max()
        .unwrap_or(H_ROLE.len());

    let border = format!(
        "+{}+{}+{}+\n",
        "-".repeat(w_handle + 2),
        "-".repeat(w_command + 2),
        "-".repeat(w_role + 2),
    );

    let mut out = String::new();
    out.push_str(&border);
    out.push_str(&format!(
        "| {H_HANDLE:<w_handle$} | {H_COMMAND:<w_command$} | {H_ROLE:<w_role$} |\n"
    ));
    out.push_str(&border);
    for (handle, command, role) in &rows {
        out.push_str(&format!(
            "| {handle:<w_handle$} | {command:<w_command$} | {role:<w_role$} |\n"
        ));
    }
    out.push_str(&border);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use chan_workspace::teams::Member;

    fn sample_config() -> TeamConfig {
        TeamConfig {
            team_name: "alpha".into(),
            host_name: "Neo".into(),
            host_handle: "@@Neo".into(),
            tab_group: "alpha".into(),
            auto_prefix_at: true,
            created_at: "2026-05-29T00:00:00Z".into(),
            members: vec![
                Member {
                    handle: "@@Lead".into(),
                    command: "claude".into(),
                    env: std::collections::BTreeMap::from([(
                        "CHAN_TAB_NAME".to_string(),
                        "@@Lead".to_string(),
                    )]),
                    is_lead: true,
                    position: None,
                },
                Member {
                    handle: "@@LaneA".into(),
                    command: "codex".into(),
                    env: std::collections::BTreeMap::new(),
                    is_lead: false,
                    position: None,
                },
            ],
        }
    }

    fn test_workspace() -> (
        tempfile::TempDir,
        tempfile::TempDir,
        Arc<chan_workspace::Workspace>,
    ) {
        let cfg = tempfile::TempDir::new().unwrap();
        let root = tempfile::TempDir::new().unwrap();
        let lib = chan_workspace::Library::open_at(cfg.path().join("config.toml")).unwrap();
        lib.register_workspace(root.path()).unwrap();
        let workspace = lib.open_workspace(root.path()).unwrap();
        // Keep the TempDirs alive for the duration of the test by
        // returning them alongside the workspace.
        (cfg, root, workspace)
    }

    #[test]
    fn write_then_read_round_trips_including_tab_group() {
        let (_cfg, _root, workspace) = test_workspace();
        let config = sample_config();

        write_team_config(&workspace, "new-team-1", &config).unwrap();
        let read = read_team_config(&workspace, "new-team-1").unwrap();

        assert_eq!(read, config, "read config must equal written config");
        assert_eq!(read.tab_group, "alpha", "tab_group must round-trip");
    }

    #[test]
    fn write_creates_config_bootstrap_and_subdirs() {
        let (_cfg, _root, workspace) = test_workspace();
        write_team_config(&workspace, "teams/alpha", &sample_config()).unwrap();

        assert!(workspace.exists("teams/alpha/config.toml"));
        assert!(workspace.exists("teams/alpha/bootstrap.md"));
        assert!(workspace.stat("teams/alpha/tasks").unwrap().is_dir);
        assert!(workspace.stat("teams/alpha/journals").unwrap().is_dir);
        assert!(workspace.stat("teams/alpha/followups").unwrap().is_dir);
    }

    #[test]
    fn bootstrap_contains_team_host_lead_and_poke_chord() {
        let (_cfg, _root, workspace) = test_workspace();
        write_team_config(&workspace, "new-team-1", &sample_config()).unwrap();
        let bootstrap = workspace.read_text("new-team-1/bootstrap.md").unwrap();

        assert!(bootstrap.contains("alpha"), "team name present");
        assert!(bootstrap.contains("@@Neo"), "host handle present");
        assert!(bootstrap.contains("@@Lead"), "lead handle present");
        assert!(
            bootstrap.contains("\\x1b[27;9;13~"),
            "poke submit chord literal present"
        );
        // No em dashes; ASCII only.
        assert!(!bootstrap.contains('\u{2014}'), "no em dashes");
        assert!(bootstrap.is_ascii(), "bootstrap must be pure ASCII");
    }

    #[test]
    fn validate_rejects_zero_members() {
        let mut config = sample_config();
        config.members.clear();
        let err = validate_team_config(&config).unwrap_err();
        assert!(err.contains("between 1 and 9"), "got: {err}");
    }

    #[test]
    fn validate_rejects_ten_members() {
        let mut config = sample_config();
        // 10 members, exactly one lead, all handles non-empty.
        config.members = (0..10)
            .map(|i| Member {
                handle: format!("@@M{i}"),
                command: "claude".into(),
                env: std::collections::BTreeMap::new(),
                is_lead: i == 0,
                position: None,
            })
            .collect();
        let err = validate_team_config(&config).unwrap_err();
        assert!(err.contains("between 1 and 9"), "got: {err}");
    }

    #[test]
    fn validate_rejects_zero_leads() {
        let mut config = sample_config();
        for m in &mut config.members {
            m.is_lead = false;
        }
        let err = validate_team_config(&config).unwrap_err();
        assert!(err.contains("exactly one member"), "got: {err}");
    }

    #[test]
    fn validate_rejects_two_leads() {
        let mut config = sample_config();
        for m in &mut config.members {
            m.is_lead = true;
        }
        let err = validate_team_config(&config).unwrap_err();
        assert!(err.contains("exactly one member"), "got: {err}");
    }

    #[test]
    fn read_errors_on_missing_config() {
        let (_cfg, _root, workspace) = test_workspace();
        let err = read_team_config(&workspace, "no-such-team").unwrap_err();
        assert!(
            err.contains("cannot read"),
            "expected read-failure message, got {err}"
        );
    }

    #[test]
    fn require_relative_dir_rejects_empty_and_absolute() {
        assert!(require_relative_dir("   ").is_err());
        assert!(require_relative_dir("/tmp/new-team-1").is_err());
        assert_eq!(require_relative_dir(" new-team-1 ").unwrap(), "new-team-1");
        assert_eq!(require_relative_dir("teams/alpha").unwrap(), "teams/alpha");
    }
}
