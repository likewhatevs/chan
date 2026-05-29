//! phase-13-r2 `lane-a-A3`: path-based chan-team.toml read/write.
//!
//! Backs the Team Work dialog's New/Load config flow. The dialog
//! persists its full state to a user-chosen `chan-team.toml`
//! (default `/tmp/new-team-1/chan-team.toml`) on Bootstrap, and
//! re-reads it to prepopulate the form on Load.
//!
//! WHY this bypasses `Workspace::write_text` / the workspace
//! sandbox (the @@Alex-authorized exception to "all fs ops route
//! through Workspace", request risk #6):
//!
//! `chan-team.toml` is app-level dev-orchestration config, not
//! notes content. It lives at a user-chosen ABSOLUTE path
//! (default `/tmp`), deliberately OUTSIDE the notes-content
//! sandbox so a team config can describe a team that operates
//! across directories independent of any single workspace root.
//! The sandbox refuses absolute paths outside the workspace by
//! design, so routing through it cannot serve this feature. This
//! is consistent with chan's loopback single-user threat model:
//! the embedded terminal already grants full shell access to the
//! same machine, so a single additional read/write at a path the
//! user typed adds no new exposure. We use `std::fs` directly and
//! restrict input to absolute paths.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chan_workspace::TeamConfig;
use serde::Deserialize;
use std::path::{Path, PathBuf};

use crate::error::err;

/// `POST /api/team-config/read` body.
#[derive(Deserialize)]
pub struct ReadTeamConfigPayload {
    pub path: String,
}

/// `POST /api/team-config/write` body. `config` mirrors the SPA's
/// `TeamConfigWire` shape 1:1 (it IS `chan_workspace::TeamConfig`).
#[derive(Deserialize)]
pub struct WriteTeamConfigPayload {
    pub path: String,
    pub config: TeamConfig,
}

/// Reject relative / empty paths up front. We do NOT canonicalize
/// or sandbox beyond "must be absolute" because this is the
/// deliberate out-of-sandbox app-config path (see module docs).
/// Single-user loopback means traversal is not a privilege-
/// escalation concern; rejecting non-absolute input is enough to
/// keep the contract unambiguous (the dialog always sends an
/// absolute path).
fn require_absolute(path: &str) -> Result<PathBuf, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("path is required".into());
    }
    let p = PathBuf::from(trimmed);
    if !p.is_absolute() {
        return Err(format!("path must be absolute: {trimmed}"));
    }
    Ok(p)
}

/// `POST /api/team-config/read` - parse the TOML at the absolute
/// path into a `TeamConfig`. 400 on missing / unreadable / invalid
/// TOML / wrong shape so the Load flow can surface the message
/// inline + reject the path.
pub async fn api_team_config_read(Json(payload): Json<ReadTeamConfigPayload>) -> Response {
    let path = match require_absolute(&payload.path) {
        Ok(p) => p,
        Err(msg) => return err(StatusCode::BAD_REQUEST, msg),
    };
    let result = tokio::task::spawn_blocking(move || read_team_config(&path)).await;
    match result {
        Ok(Ok(config)) => Json(config).into_response(),
        Ok(Err(msg)) => err(StatusCode::BAD_REQUEST, msg),
        Err(join) => err(StatusCode::INTERNAL_SERVER_ERROR, join.to_string()),
    }
}

/// `POST /api/team-config/write` - serialize the config to TOML +
/// write it to the absolute path, creating parent dirs. Atomic via
/// write-temp-then-rename so a partial write never leaves a
/// truncated config behind. 200 `{}` on success, 400 on bad path /
/// write failure.
pub async fn api_team_config_write(Json(payload): Json<WriteTeamConfigPayload>) -> Response {
    let path = match require_absolute(&payload.path) {
        Ok(p) => p,
        Err(msg) => return err(StatusCode::BAD_REQUEST, msg),
    };
    let config = payload.config;
    let result = tokio::task::spawn_blocking(move || write_team_config(&path, &config)).await;
    match result {
        Ok(Ok(())) => Json(serde_json::json!({})).into_response(),
        Ok(Err(msg)) => err(StatusCode::BAD_REQUEST, msg),
        Err(join) => err(StatusCode::INTERNAL_SERVER_ERROR, join.to_string()),
    }
}

fn read_team_config(path: &Path) -> Result<TeamConfig, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    toml::from_str::<TeamConfig>(&text)
        .map_err(|e| format!("invalid team config at {}: {e}", path.display()))
}

fn write_team_config(path: &Path, config: &TeamConfig) -> Result<(), String> {
    let text =
        toml::to_string_pretty(config).map_err(|e| format!("cannot serialize team config: {e}"))?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("cannot create {}: {e}", parent.display()))?;
    }
    // Write-temp-then-rename for atomicity: the rename is a single
    // syscall so a reader either sees the old file or the complete
    // new one, never a torn write. The temp sits beside the target
    // so the rename stays on the same filesystem.
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let tmp = dir.join(format!(
        ".{}.tmp",
        path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("chan-team")
    ));
    std::fs::write(&tmp, text.as_bytes())
        .map_err(|e| format!("cannot write {}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, path).map_err(|e| {
        // Clean up the temp on rename failure so we don't leave
        // litter beside the target.
        let _ = std::fs::remove_file(&tmp);
        format!("cannot finalize {}: {e}", path.display())
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chan_workspace::teams::Member;
    use tempfile::TempDir;

    fn sample_config() -> TeamConfig {
        TeamConfig {
            team_name: "alpha".into(),
            host_name: "Neo".into(),
            host_handle: "@@Neo".into(),
            auto_prefix_at: true,
            created_at: "2026-05-29T00:00:00Z".into(),
            members: vec![Member {
                handle: "@@Lead".into(),
                command: "claude".into(),
                env: std::collections::BTreeMap::from([(
                    "CHAN_TAB_NAME".to_string(),
                    "@@Lead".to_string(),
                )]),
                is_lead: true,
                position: None,
            }],
        }
    }

    #[test]
    fn write_then_read_round_trips() {
        let dir = TempDir::new().unwrap();
        // Nested path the dialog hands us: the write step must
        // create the missing parent dir (the default flow is
        // `/tmp/new-team-1/chan-team.toml` where new-team-1 does
        // not exist yet).
        let path = dir.path().join("new-team-1").join("chan-team.toml");
        let config = sample_config();
        write_team_config(&path, &config).unwrap();
        assert!(path.exists(), "config file must exist after write");
        let read = read_team_config(&path).unwrap();
        assert_eq!(read, config, "read config must equal written config");
    }

    #[test]
    fn read_rejects_invalid_toml() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("chan-team.toml");
        std::fs::write(&path, b"this is = not [valid").unwrap();
        let err = read_team_config(&path).unwrap_err();
        assert!(
            err.contains("invalid team config"),
            "expected invalid-config message, got {err}"
        );
    }

    #[test]
    fn read_rejects_missing_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("does-not-exist.toml");
        let err = read_team_config(&path).unwrap_err();
        assert!(
            err.contains("cannot read"),
            "expected read-failure message, got {err}"
        );
    }

    #[test]
    fn require_absolute_rejects_relative_and_empty() {
        assert!(require_absolute("relative/path.toml").is_err());
        assert!(require_absolute("   ").is_err());
        assert!(require_absolute("/tmp/new-team-1/chan-team.toml").is_ok());
    }
}
