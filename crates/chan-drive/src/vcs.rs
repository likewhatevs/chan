// VCS-parent detection: is the drive path inside a Git / Mercurial /
// Subversion working tree?
//
// Used by `chan serve` to nudge users toward serving the repo root
// instead of an arbitrary subdir. If the drive's notes live inside
// `~/code/myproject/docs/notes`, the user almost always wants
// `~/code/myproject` as the drive root so cross-file links, graph,
// and search cover the whole project.
//
// Detection is a pure stat-walk; we never invoke `git`/`hg`/`svn`
// binaries and never read repository contents. Stops at mount
// boundaries, at `$HOME` (which is never inspected), and at the
// filesystem root.

use std::path::{Path, PathBuf};

/// Which VCS produced the match.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VcsKind {
    Git,
    Mercurial,
    Subversion,
}

impl VcsKind {
    /// Stable lowercase identifier for CLI error markers and
    /// structured logs. Wire-format; do not translate or rename.
    pub fn as_str(&self) -> &'static str {
        match self {
            VcsKind::Git => "git",
            VcsKind::Mercurial => "hg",
            VcsKind::Subversion => "svn",
        }
    }
}

/// A VCS working tree that contains the queried path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VcsParent {
    pub kind: VcsKind,
    /// Canonical path to the working-tree root (the directory that
    /// holds `.git` / `.hg` / `.svn`).
    pub repo_root: PathBuf,
}

/// Find the nearest VCS working tree strictly *above* `path`.
///
/// Returns `None` when no VCS marker is found, when the walk crosses
/// a filesystem mount boundary, when it reaches `$HOME`, or when it
/// reaches the filesystem root. The input path itself is NOT checked:
/// if the user picks the repo root as the drive root, the function
/// returns `None` so the caller can proceed silently.
///
/// Detection looks for, at each strict ancestor:
///
///   - `.git` (file OR directory; the file form covers worktrees and
///     submodules whose `.git` is a pointer to a gitdir elsewhere),
///   - `.hg/` directory (Mercurial),
///   - `.svn/` directory (Subversion 1.7+ keeps it at the repo root).
///
/// `$HOME` is deliberately excluded so a dotfiles-managed-as-git
/// home directory does not block every drive the user has.
///
/// Pure stat calls; never spawns external processes.
pub fn detect_parent_vcs(path: &Path) -> Option<VcsParent> {
    detect_parent_vcs_with_home(path, dirs::home_dir())
}

/// Test seam: the same algorithm as [`detect_parent_vcs`], but with
/// an explicit `home` override so tests can drive the `$HOME` stop
/// without touching the developer's real home directory.
pub(crate) fn detect_parent_vcs_with_home(path: &Path, home: Option<PathBuf>) -> Option<VcsParent> {
    let start = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let home_canon = home.and_then(|h| std::fs::canonicalize(&h).ok());
    let start_dev = dev_of(&start);

    // Skip the leaf: only inspect strict ancestors. If the leaf is
    // itself a repo root, we want to return None so `chan serve`
    // proceeds without prompting the user.
    let mut iter = start.ancestors();
    let _leaf = iter.next();

    for dir in iter {
        // Mount-boundary stop. If we have a `start_dev` and the
        // ancestor lives on a different filesystem, don't cross it:
        // a `.git` on the host won't be reachable from a drive
        // sitting on a separate mount and suggesting it would just
        // confuse the user.
        if let (Some(a), Some(b)) = (start_dev, dev_of(dir)) {
            if a != b {
                return None;
            }
        }
        // `$HOME` stop. Don't inspect home itself; a dotfiles-as-git
        // setup is unrelated to drive-root selection.
        if let Some(h) = &home_canon {
            if dir == h {
                return None;
            }
        }
        if let Some(kind) = vcs_kind_at(dir) {
            return Some(VcsParent {
                kind,
                repo_root: dir.to_path_buf(),
            });
        }
    }
    None
}

fn vcs_kind_at(dir: &Path) -> Option<VcsKind> {
    // `.git` may be a directory (regular repo) or a regular file
    // (worktree / submodule pointing to a gitdir elsewhere). Both
    // mark a working-tree boundary; `exists()` covers both without
    // reading the contents.
    if dir.join(".git").exists() {
        return Some(VcsKind::Git);
    }
    if dir.join(".hg").is_dir() {
        return Some(VcsKind::Mercurial);
    }
    if dir.join(".svn").is_dir() {
        return Some(VcsKind::Subversion);
    }
    None
}

#[cfg(unix)]
fn dev_of(path: &Path) -> Option<u64> {
    use std::os::unix::fs::MetadataExt;
    std::fs::metadata(path).ok().map(|m| m.dev())
}

#[cfg(not(unix))]
fn dev_of(_path: &Path) -> Option<u64> {
    // No portable `st_dev` equivalent on Windows in stable std.
    // Skip the mount-boundary check there; the home + FS-root
    // stops still apply.
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn mkdir(p: &Path) {
        std::fs::create_dir_all(p).unwrap();
    }

    fn canon(p: &Path) -> PathBuf {
        std::fs::canonicalize(p).unwrap()
    }

    #[test]
    fn none_when_no_vcs() {
        let tmp = TempDir::new().unwrap();
        let drive = tmp.path().join("notes");
        mkdir(&drive);
        assert!(detect_parent_vcs_with_home(&drive, None).is_none());
    }

    #[test]
    fn detects_git_dir_at_ancestor() {
        let tmp = TempDir::new().unwrap();
        mkdir(&tmp.path().join(".git"));
        let drive = tmp.path().join("docs/notes");
        mkdir(&drive);
        let got = detect_parent_vcs_with_home(&drive, None).unwrap();
        assert_eq!(got.kind, VcsKind::Git);
        assert_eq!(canon(&got.repo_root), canon(tmp.path()));
    }

    #[test]
    fn detects_git_file_at_ancestor() {
        // Submodule / worktree shape: `.git` is a regular file
        // containing `gitdir: ...`, not a directory.
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join(".git"), b"gitdir: /elsewhere\n").unwrap();
        let drive = tmp.path().join("docs");
        mkdir(&drive);
        let got = detect_parent_vcs_with_home(&drive, None).unwrap();
        assert_eq!(got.kind, VcsKind::Git);
    }

    #[test]
    fn detects_hg() {
        let tmp = TempDir::new().unwrap();
        mkdir(&tmp.path().join(".hg"));
        let drive = tmp.path().join("docs");
        mkdir(&drive);
        let got = detect_parent_vcs_with_home(&drive, None).unwrap();
        assert_eq!(got.kind, VcsKind::Mercurial);
    }

    #[test]
    fn detects_svn() {
        let tmp = TempDir::new().unwrap();
        mkdir(&tmp.path().join(".svn"));
        let drive = tmp.path().join("docs");
        mkdir(&drive);
        let got = detect_parent_vcs_with_home(&drive, None).unwrap();
        assert_eq!(got.kind, VcsKind::Subversion);
    }

    #[test]
    fn skips_leaf_when_path_is_repo_root() {
        // When the drive root is the repo root itself, the function
        // must return None: the caller is meant to proceed silently
        // because there's no better parent to suggest.
        let tmp = TempDir::new().unwrap();
        mkdir(&tmp.path().join(".git"));
        assert!(detect_parent_vcs_with_home(tmp.path(), None).is_none());
    }

    #[test]
    fn home_stop_filters_dotfiles_as_git() {
        // `.git` at a synthetic "home" must NOT be reported when the
        // drive is a child of that home: dotfiles-managed-as-git in
        // the user's home directory is common and unrelated to
        // drive-root selection.
        let tmp = TempDir::new().unwrap();
        let fake_home = tmp.path().join("home");
        mkdir(&fake_home);
        mkdir(&fake_home.join(".git"));
        let drive = fake_home.join("notes");
        mkdir(&drive);
        let got = detect_parent_vcs_with_home(&drive, Some(fake_home));
        assert!(got.is_none(), "expected None, got {got:?}");
    }

    #[test]
    fn finds_repo_below_home() {
        // Standard case: clone under home, drive is a subdir of the
        // clone. We want the clone reported as the repo root.
        let tmp = TempDir::new().unwrap();
        let fake_home = tmp.path().join("home");
        let clone = fake_home.join("code/myproject");
        mkdir(&clone.join(".git"));
        let drive = clone.join("docs/notes");
        mkdir(&drive);
        let got = detect_parent_vcs_with_home(&drive, Some(fake_home)).unwrap();
        assert_eq!(got.kind, VcsKind::Git);
        assert_eq!(canon(&got.repo_root), canon(&clone));
    }

    #[test]
    fn handles_nonexistent_leaf() {
        // `chan serve` may pass a path that doesn't exist yet
        // (auto-creation happens later in cmd_serve). Detection
        // should still walk the existing ancestors.
        let tmp = TempDir::new().unwrap();
        mkdir(&tmp.path().join(".git"));
        let nonexistent = tmp.path().join("not-yet/sub");
        let got = detect_parent_vcs_with_home(&nonexistent, None).unwrap();
        assert_eq!(got.kind, VcsKind::Git);
    }

    #[test]
    fn vcs_kind_str_is_stable() {
        // The CLI error marker format pins these strings as wire
        // format; renaming the enum must not change them.
        assert_eq!(VcsKind::Git.as_str(), "git");
        assert_eq!(VcsKind::Mercurial.as_str(), "hg");
        assert_eq!(VcsKind::Subversion.as_str(), "svn");
    }
}
