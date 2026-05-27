//! Rich Prompt metadata sessions.
//!
//! Rich Prompts are terminal-owned draft-backed sessions. They live under
//! the per-workspace drafts metadata root as `rich-prompt-N/`, but an
//! active marker distinguishes them from old history-only
//! `rich-prompt-N/prompt.md` directories and ordinary drafts.

use std::fs;
use std::path::{Path, PathBuf};

use crate::drafts;
use crate::error::{ChanError, Result};
use crate::{fs_ops, trash};

pub const ACTIVE_MARKER: &str = ".chan-rich-prompt-active";
const RICH_PROMPT_PREFIX: &str = "rich-prompt";
const SPOOL_DIR: &str = "spool";
const PROCESS_FILE: &str = "process.md";
const DRAFT_FILE: &str = "draft.md";
const EVENTS_DIR: &str = "events";
const JOURNALS_DIR: &str = "journals";
const TASKS_DIR: &str = "tasks";

pub const DEFAULT_PROCESS_TEXT: &str = "\
# Rich Prompt Process\n\
\n\
This session is terminal-owned and draft-backed.\n\
\n\
- `draft.md` is the active prompt buffer.\n\
- `prompt-N.md` files are submitted prompt archives.\n\
- `spool/events/` carries event files.\n\
- `spool/journals/` carries agent journals.\n\
- `spool/tasks/` carries agent task files.\n\
\n\
Use `Drafts/...` paths through chan tools. They resolve to uncommitted metadata\n\
outside the workspace root until the host saves or promotes content intentionally.\n";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RichPromptSession {
    pub name: String,
    pub draft_path: String,
    pub workspace_path: String,
    pub events_path: String,
    pub process_path: String,
    pub workspace_abs: PathBuf,
    pub events_abs: PathBuf,
    pub submission_sequence: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RichPromptSubmitReport {
    pub name: String,
    pub archived_path: String,
    pub draft_path: String,
    pub submission_sequence: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RichPromptIssue {
    pub name: String,
    pub message: String,
}

pub fn create(
    drafts_dir: &Path,
    requested_name: Option<&str>,
    process_text: &str,
) -> Result<RichPromptSession> {
    fs::create_dir_all(drafts_dir).map_err(|e| {
        ChanError::Io(format!(
            "failed to create drafts directory {}: {e}",
            drafts_dir.display()
        ))
    })?;
    match requested_name {
        Some(name) => create_named(drafts_dir, name, process_text, false),
        None => {
            for i in 0..1000u32 {
                let candidate = if i == 0 {
                    RICH_PROMPT_PREFIX.to_string()
                } else {
                    format!("{RICH_PROMPT_PREFIX}-{i}")
                };
                match create_named(drafts_dir, &candidate, process_text, true) {
                    Ok(workspace) => return Ok(workspace),
                    Err(ChanError::PathAlreadyExists(_)) => continue,
                    Err(e) => return Err(e),
                }
            }
            Err(ChanError::Io(
                "failed to allocate rich prompt session name".into(),
            ))
        }
    }
}

pub fn inspect(drafts_dir: &Path, name: &str) -> Result<RichPromptSession> {
    validate_name(name)?;
    let root = drafts_dir.join(name);
    inspect_root(name, &root)
}

pub fn submit(
    drafts_dir: &Path,
    name: &str,
    content: &str,
    expected_sequence: u64,
    expected_mtime_ns: Option<i64>,
) -> Result<RichPromptSubmitReport> {
    let workspace = inspect(drafts_dir, name)?;
    if workspace.submission_sequence != expected_sequence {
        return Err(ChanError::WriteConflict {
            current_mtime_ns: None,
        });
    }
    let root = drafts_dir.join(name);
    let draft = root.join(DRAFT_FILE);
    if let Some(expected) = expected_mtime_ns {
        let meta = fs::symlink_metadata(&draft)
            .map_err(|e| broken(name, format!("failed to inspect draft.md: {e}")))?;
        if mtime_ns_std(&meta) != Some(expected) {
            return Err(ChanError::WriteConflict {
                current_mtime_ns: mtime_ns_std(&meta),
            });
        }
    }
    let next = expected_sequence.saturating_add(1);
    let archive_name = format!("prompt-{next}.md");
    let archive = root.join(&archive_name);
    if fs::symlink_metadata(&archive).is_ok() {
        return Err(ChanError::PathAlreadyExists(format!(
            "{}/{name}/{archive_name}",
            drafts::UNIFIED_DRAFTS_ROOT
        )));
    }
    fs_ops::atomic_write(&archive, content.as_bytes())?;
    fs_ops::atomic_write(&draft, b"")?;
    Ok(RichPromptSubmitReport {
        name: name.to_string(),
        archived_path: format!("{}/{name}/{archive_name}", drafts::UNIFIED_DRAFTS_ROOT),
        draft_path: format!("{}/{name}/{DRAFT_FILE}", drafts::UNIFIED_DRAFTS_ROOT),
        submission_sequence: next,
    })
}

pub fn discard(drafts_dir: &Path, draft_trash_dir: &Path, name: &str) -> Result<()> {
    inspect(drafts_dir, name)?;
    let src = drafts_dir.join(name);
    trash::move_into(
        draft_trash_dir,
        &src,
        &format!("{}/{name}", drafts::UNIFIED_DRAFTS_ROOT),
        true,
    )
}

pub fn preflight(drafts_dir: &Path) -> Result<Vec<RichPromptIssue>> {
    let rd = match fs::read_dir(drafts_dir) {
        Ok(rd) => rd,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => {
            return Err(ChanError::Io(format!(
                "failed to read drafts dir {}: {e}",
                drafts_dir.display()
            )))
        }
    };
    let mut issues = Vec::new();
    for entry in rd {
        let entry = match entry {
            Ok(entry) => entry,
            Err(e) => {
                issues.push(RichPromptIssue {
                    name: "<unknown>".to_string(),
                    message: format!("failed to read drafts entry: {e}"),
                });
                continue;
            }
        };
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = entry.path();
        if !owns_preflight(&name, &path) {
            continue;
        }
        match inspect_root(&name, &path) {
            Ok(_) => {}
            Err(ChanError::DraftBroken { message, .. }) => {
                issues.push(RichPromptIssue { name, message });
            }
            Err(e) => {
                issues.push(RichPromptIssue {
                    name,
                    message: e.to_string(),
                });
            }
        }
    }
    issues.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(issues)
}

pub fn has_active_marker_entry(path: &Path) -> bool {
    fs::symlink_metadata(path.join(ACTIVE_MARKER)).is_ok()
}

pub fn owns_preflight(name: &str, path: &Path) -> bool {
    has_active_marker_entry(path)
        || (is_rich_prompt_name(name) && !is_legacy_history_dir(name, path))
}

pub fn is_legacy_history_dir(name: &str, path: &Path) -> bool {
    is_rich_prompt_name(name)
        && !has_active_marker_entry(path)
        && fs::symlink_metadata(path.join(DRAFT_FILE)).is_err()
        && fs::symlink_metadata(path.join("prompt.md"))
            .map(|meta| meta.is_file() && !meta.file_type().is_symlink())
            .unwrap_or(false)
}

fn create_named(
    drafts_dir: &Path,
    name: &str,
    process_text: &str,
    auto_name: bool,
) -> Result<RichPromptSession> {
    validate_name(name)?;
    let root = drafts_dir.join(name);
    match fs::create_dir(&root) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists && auto_name => {
            return Err(ChanError::PathAlreadyExists(name.to_string()));
        }
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            return Err(ChanError::PathAlreadyExists(name.to_string()));
        }
        Err(e) => {
            return Err(ChanError::Io(format!(
                "failed to create rich prompt session {}: {e}",
                root.display()
            )));
        }
    }
    let result = create_session_contents(name, &root, process_text);
    if result.is_err() {
        let _ = fs::remove_dir_all(&root);
    }
    result
}

fn create_session_contents(
    name: &str,
    root: &Path,
    process_text: &str,
) -> Result<RichPromptSession> {
    let spool = root.join(SPOOL_DIR);
    fs::create_dir(&spool)
        .map_err(|e| ChanError::Io(format!("failed to create rich prompt spool dir: {e}")))?;
    for leaf in [EVENTS_DIR, JOURNALS_DIR, TASKS_DIR] {
        fs::create_dir(spool.join(leaf))
            .map_err(|e| ChanError::Io(format!("failed to create rich prompt {leaf} dir: {e}")))?;
    }
    fs_ops::atomic_write(&root.join(DRAFT_FILE), b"")?;
    fs_ops::atomic_write(&spool.join(PROCESS_FILE), process_text.as_bytes())?;
    fs_ops::atomic_write(&root.join(ACTIVE_MARKER), b"active\n")?;
    inspect_root(name, root)
}

fn inspect_root(name: &str, root: &Path) -> Result<RichPromptSession> {
    validate_name(name)?;
    let meta = fs::symlink_metadata(root).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            ChanError::Io(format!(
                "not found: rich prompt `{name}` at {}",
                root.display()
            ))
        } else {
            ChanError::Io(format!("failed to inspect rich prompt `{name}`: {e}"))
        }
    })?;
    if !meta.is_dir() || meta.file_type().is_symlink() {
        return Err(broken(name, "rich prompt root is not a directory"));
    }
    ensure_regular_file(name, &root.join(ACTIVE_MARKER), ACTIVE_MARKER)?;
    ensure_regular_file(name, &root.join(DRAFT_FILE), DRAFT_FILE)?;
    let spool = root.join(SPOOL_DIR);
    ensure_dir(name, &spool, SPOOL_DIR)?;
    ensure_regular_file(
        name,
        &spool.join(PROCESS_FILE),
        &format!("{SPOOL_DIR}/{PROCESS_FILE}"),
    )?;
    for leaf in [EVENTS_DIR, JOURNALS_DIR, TASKS_DIR] {
        ensure_dir(name, &spool.join(leaf), &format!("{SPOOL_DIR}/{leaf}"))?;
    }
    scan_for_unsafe_entries(name, root, Path::new(""))?;
    Ok(RichPromptSession {
        name: name.to_string(),
        draft_path: format!("{}/{name}/{DRAFT_FILE}", drafts::UNIFIED_DRAFTS_ROOT),
        workspace_path: format!("{}/{name}", drafts::UNIFIED_DRAFTS_ROOT),
        events_path: format!(
            "{}/{name}/{SPOOL_DIR}/{EVENTS_DIR}",
            drafts::UNIFIED_DRAFTS_ROOT
        ),
        process_path: format!(
            "{}/{name}/{SPOOL_DIR}/{PROCESS_FILE}",
            drafts::UNIFIED_DRAFTS_ROOT
        ),
        workspace_abs: root.to_path_buf(),
        events_abs: spool.join(EVENTS_DIR),
        submission_sequence: submission_sequence(name, root)?,
    })
}

fn ensure_regular_file(name: &str, path: &Path, rel: &str) -> Result<()> {
    let meta = fs::symlink_metadata(path)
        .map_err(|e| broken(name, format!("failed to inspect {rel}: {e}")))?;
    let ft = meta.file_type();
    if ft.is_file() && !ft.is_symlink() {
        return Ok(());
    }
    Err(broken(name, format!("{rel} is not a regular file")))
}

fn ensure_dir(name: &str, path: &Path, rel: &str) -> Result<()> {
    let meta = fs::symlink_metadata(path)
        .map_err(|e| broken(name, format!("failed to inspect {rel}: {e}")))?;
    let ft = meta.file_type();
    if ft.is_dir() && !ft.is_symlink() {
        return Ok(());
    }
    Err(broken(name, format!("{rel} is not a directory")))
}

fn scan_for_unsafe_entries(name: &str, root: &Path, rel_dir: &Path) -> Result<()> {
    let dir = root.join(rel_dir);
    let read = fs::read_dir(&dir)
        .map_err(|e| broken(name, format!("failed to read {}: {e}", dir.display())))?;
    for entry in read {
        let entry =
            entry.map_err(|e| broken(name, format!("failed to read {}: {e}", dir.display())))?;
        let rel = rel_dir.join(entry.file_name());
        let path = entry.path();
        let meta = fs::symlink_metadata(&path)
            .map_err(|e| broken(name, format!("failed to inspect {}: {e}", rel.display())))?;
        let ft = meta.file_type();
        if ft.is_symlink() {
            return Err(broken(name, format!("refusing symlink {}", rel.display())));
        }
        if ft.is_dir() {
            scan_for_unsafe_entries(name, root, &rel)?;
        } else if !ft.is_file() {
            return Err(broken(
                name,
                format!("refusing special file {}", rel.display()),
            ));
        }
    }
    Ok(())
}

fn submission_sequence(name: &str, root: &Path) -> Result<u64> {
    let read = fs::read_dir(root)
        .map_err(|e| broken(name, format!("failed to read {}: {e}", root.display())))?;
    let mut max_seen = 0u64;
    for entry in read {
        let entry =
            entry.map_err(|e| broken(name, format!("failed to read {}: {e}", root.display())))?;
        let Some(file_name) = entry.file_name().to_str().map(str::to_string) else {
            continue;
        };
        let Some(n) = file_name
            .strip_prefix("prompt-")
            .and_then(|s| s.strip_suffix(".md"))
            .and_then(|s| s.parse::<u64>().ok())
        else {
            continue;
        };
        let meta = entry
            .file_type()
            .map_err(|e| broken(name, format!("failed to inspect {file_name}: {e}")))?;
        if meta.is_file() {
            max_seen = max_seen.max(n);
        }
    }
    Ok(max_seen)
}

fn is_rich_prompt_name(name: &str) -> bool {
    name == RICH_PROMPT_PREFIX
        || name
            .strip_prefix("rich-prompt-")
            .is_some_and(|suffix| !suffix.is_empty() && suffix.chars().all(|c| c.is_ascii_digit()))
}

fn validate_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(ChanError::Io("rich prompt name cannot be empty".into()));
    }
    if name.contains('/') || name.contains('\\') {
        return Err(ChanError::Io(format!(
            "rich prompt name `{name}` must not contain path separators"
        )));
    }
    if name == "." || name == ".." {
        return Err(ChanError::Io(format!(
            "rich prompt name `{name}` is reserved"
        )));
    }
    if !is_rich_prompt_name(name) {
        return Err(ChanError::Io(format!(
            "rich prompt name `{name}` must use the rich-prompt prefix"
        )));
    }
    Ok(())
}

fn broken(name: &str, message: impl Into<String>) -> ChanError {
    ChanError::DraftBroken {
        name: name.to_string(),
        message: message.into(),
    }
}

fn mtime_ns_std(meta: &std::fs::Metadata) -> Option<i64> {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .and_then(|d| i64::try_from(d.as_nanos()).ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn create_builds_active_session() {
        let td = TempDir::new().unwrap();
        let workspace = create(td.path(), None, DEFAULT_PROCESS_TEXT).unwrap();

        assert_eq!(workspace.name, "rich-prompt");
        assert_eq!(workspace.draft_path, "Drafts/rich-prompt/draft.md");
        assert_eq!(workspace.events_path, "Drafts/rich-prompt/spool/events");
        assert!(td.path().join("rich-prompt/draft.md").is_file());
        assert!(td
            .path()
            .join("rich-prompt/.chan-rich-prompt-active")
            .is_file());
        assert!(td.path().join("rich-prompt/spool/process.md").is_file());
        assert!(td.path().join("rich-prompt/spool/events").is_dir());
        assert!(td.path().join("rich-prompt/spool/journals").is_dir());
        assert!(td.path().join("rich-prompt/spool/tasks").is_dir());
    }

    #[test]
    fn create_counts_past_collisions() {
        let td = TempDir::new().unwrap();
        create(td.path(), None, DEFAULT_PROCESS_TEXT).unwrap();
        let second = create(td.path(), None, DEFAULT_PROCESS_TEXT).unwrap();

        assert_eq!(second.name, "rich-prompt-1");
    }

    #[test]
    fn inspect_reports_missing_events_dir() {
        let td = TempDir::new().unwrap();
        create(td.path(), None, DEFAULT_PROCESS_TEXT).unwrap();
        fs::remove_dir(td.path().join("rich-prompt/spool/events")).unwrap();

        let err = inspect(td.path(), "rich-prompt").unwrap_err();

        assert!(err.to_string().contains("spool/events"));
    }

    #[test]
    #[cfg(unix)]
    fn inspect_rejects_fifo() {
        use std::process::Command;

        let td = TempDir::new().unwrap();
        create(td.path(), None, DEFAULT_PROCESS_TEXT).unwrap();
        let fifo = td.path().join("rich-prompt/spool/events/event-bad.md");
        let status = Command::new("mkfifo").arg(&fifo).status().unwrap();
        assert!(status.success());

        let err = inspect(td.path(), "rich-prompt").unwrap_err();

        assert!(err.to_string().contains("refusing special file"));
    }

    #[test]
    fn submit_archives_posted_content_and_resets_draft() {
        let td = TempDir::new().unwrap();
        create(td.path(), None, DEFAULT_PROCESS_TEXT).unwrap();
        fs_ops::atomic_write(&td.path().join("rich-prompt/draft.md"), b"stale").unwrap();

        let report = submit(td.path(), "rich-prompt", "fresh", 0, None).unwrap();

        assert_eq!(report.archived_path, "Drafts/rich-prompt/prompt-1.md");
        assert_eq!(report.submission_sequence, 1);
        assert_eq!(
            fs::read_to_string(td.path().join("rich-prompt/prompt-1.md")).unwrap(),
            "fresh"
        );
        assert_eq!(
            fs::read_to_string(td.path().join("rich-prompt/draft.md")).unwrap(),
            ""
        );
    }

    #[test]
    fn submit_rejects_sequence_mismatch() {
        let td = TempDir::new().unwrap();
        create(td.path(), None, DEFAULT_PROCESS_TEXT).unwrap();

        let err = submit(td.path(), "rich-prompt", "fresh", 2, None).unwrap_err();

        assert!(matches!(err, ChanError::WriteConflict { .. }));
    }

    #[test]
    fn preflight_only_reports_active_marker_dirs() {
        let td = TempDir::new().unwrap();
        fs::create_dir_all(td.path().join("rich-prompt")).unwrap();
        fs::write(td.path().join("rich-prompt/prompt.md"), "legacy").unwrap();
        create(td.path(), Some("rich-prompt-1"), DEFAULT_PROCESS_TEXT).unwrap();
        fs::remove_file(td.path().join("rich-prompt-1/draft.md")).unwrap();

        let issues = preflight(td.path()).unwrap();

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].name, "rich-prompt-1");
        assert!(issues[0].message.contains("draft.md"));
    }
}
