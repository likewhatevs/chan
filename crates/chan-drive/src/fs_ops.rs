// Filesystem helpers for the drive:
//   - atomic_write: tmpfile + fsync + rename. Used everywhere that
//     touches a file on behalf of the user. We never want a half-
//     written note.
//   - resolve_safe / resolve_safe_strict: normalize a request path
//     and reject anything that escapes the drive root via `..` or
//     a symlink pointing outside.
//   - ensure_regular_file: lstat-based gate that rejects symlinks,
//     FIFOs, sockets, char/block devices, and directories before
//     we open a path for read or write. Without it, opening a FIFO
//     blocks waiting for a writer; opening /dev/zero never returns;
//     opening through a symlink can escape the drive sandbox.
//   - is_editable_text: extension whitelist gate.
//   - walk_drive / list_tree: recursive listing scoped to the drive,
//     skipping `.git/` and `.chan/` at any depth and dropping non-
//     regular non-dir entries (symlinks, devices, sockets) so the
//     UI tree and the indexer never see them.
//
// Symlink policy: we never traverse a symlink that points outside
// the drive's canonical root. Symlinks that resolve back inside
// the drive are also rejected by default for the read/write API
// (final-component check via lstat) so a user's intentional
// `today.md -> 2026-05-06.md` doesn't silently get clobbered by
// atomic_write, and so reads always see real files. A future
// follower-mode could relax this once we've thought through the
// editor UX.

use std::path::{Component, Path, PathBuf};

use serde::Serialize;
use walkdir::{DirEntry, WalkDir};

use crate::error::{ChanError, Result};

/// True for paths inside the drive-internal `.chan/` dir. chan-drive
/// never writes there now (per-drive state lives outside the user's
/// notes tree). The check stays as a defensive filter so a stray
/// `.chan/` from an older install or a third-party tool never
/// surfaces in the file tree or gets indexed.
pub fn is_chan_internal(rel: &str) -> bool {
    rel == ".chan" || rel.starts_with(".chan/")
}

/// True for paths whose class is editable through the UTF-8 text
/// gate (`Drive::read_text` / `Drive::write_text`). This is the
/// **editor**'s gate: every kind of textual file users can round-
/// trip through a UTF-8 buffer (markdown-class .md/.txt plus
/// arbitrary source / config text files like .py, .json, .yaml,
/// Makefile, Dockerfile, ...).
///
/// For the **indexer / graph** gate (markdown-class only, the
/// subset chan parses for headings / links / tokens) use
/// `is_indexable_text` instead. A `.py` file is `is_editable_text`
/// but not `is_indexable_text`.
pub fn is_editable_text(rel: &str) -> bool {
    matches!(classify(rel), FileClass::EditableText | FileClass::Text)
}

/// True for paths whose class is markdown-style content the indexer
/// and graph parse (`.md` / `.txt` today, i.e. `FileClass::EditableText`).
/// Drives every per-file ingestion path: tantivy index entries,
/// graph nodes, link / token / heading extraction, link-rewrite on
/// rename, reindex-after-restore, etc.
///
/// Distinct from `is_editable_text`, which widens to any text file
/// the editor can edit. Arbitrary source-class text (`FileClass::Text`)
/// is editable but is **not** indexed today: false positives like
/// `#include` looking like a `#tag` would pollute the graph. Phase
/// 3 may revisit indexing source-class text as plain full-text;
/// until then this predicate stays narrow.
pub fn is_indexable_text(rel: &str) -> bool {
    matches!(classify(rel), FileClass::EditableText)
}

/// Coarse content class derived from a path's extension and (for
/// well-known no-extension files) basename. Drives:
///   - which files the editor reads/writes through the UTF-8 gate
///     (`read_text` / `write_text`): any `EditableText` or `Text`.
///   - which files the search index + graph ingest: `EditableText`
///     only (markdown-class .md / .txt). See `is_indexable_text`.
///   - which files the editor previews as media (`Image`, `Pdf`).
///   - everything else falls through to `Other`: still walkable,
///     readable as bytes, renameable / removeable, but opaque to
///     the editor and the indexer.
///
/// Extension matching is ASCII case-insensitive. Files with an
/// extension we don't recognize fall back to a basename check
/// against well-known textual filenames (Makefile, Dockerfile,
/// LICENSE, .gitignore, ...). No content sniffing in v1: phase
/// 1.5 may add a "read first N bytes, treat as Text if valid
/// UTF-8 and no NUL" fallback for genuinely unknown files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileClass {
    /// `.md`, `.txt`. Editable through `read_text` / `write_text`,
    /// indexed by tantivy, parsed for graph edges + headings.
    EditableText,
    /// Source / config text the editor can edit but the indexer
    /// does not parse. Covers programming-language extensions
    /// (.py, .rs, .c, .go, ...), config formats (.toml, .yaml,
    /// .json, .ini, ...), shell scripts, web sources, build files,
    /// and well-known no-extension files (Makefile, Dockerfile,
    /// LICENSE, .gitignore, ...).
    Text,
    /// `.png`, `.jpg`, `.jpeg`, `.gif`, `.webp`, `.svg`, `.avif`.
    /// Read-only via `read` / `write_bytes`; the editor's inspector
    /// pane previews these inline.
    Image,
    /// `.pdf`. Same I/O contract as `Image`; reserved as a separate
    /// class so the editor renders it through a dedicated viewer
    /// (browser PDF.js / inspector preview) rather than as a generic
    /// download.
    Pdf,
    /// Anything else (archives, audio, video, fonts, files with an
    /// unknown extension and no known basename). Walkable, byte-
    /// readable, renameable. Not indexed, not editable as text.
    Other,
}

/// Classify a relative path by extension first, then by basename
/// (for well-known no-extension files). See `FileClass` for the
/// downstream behaviour each class triggers.
pub fn classify(rel: &str) -> FileClass {
    // Extension path: rsplit_once('.') splits "Cargo.toml" into
    // ("Cargo", "toml"); for ".gitignore" it splits into
    // ("", "gitignore"). Both shapes feed `classify_ext` after
    // ASCII-lowercasing.
    if let Some((_, ext)) = rel.rsplit_once('.') {
        if !ext.is_empty() {
            let lower = ext.to_ascii_lowercase();
            if let Some(c) = classify_ext(&lower) {
                return c;
            }
            // Extension is present but not in the known set. Fall
            // through to the basename check so a file like
            // ".gitignore" still resolves via its full name.
        }
    }
    let basename = rel.rsplit('/').next().unwrap_or(rel);
    if let Some(c) = classify_basename(basename) {
        return c;
    }
    FileClass::Other
}

/// Extension -> FileClass for known suffixes. `ext` must already
/// be ASCII-lowercased. Returns `None` for anything not on the
/// whitelist so `classify` can fall back to the basename check.
fn classify_ext(ext: &str) -> Option<FileClass> {
    let class = match ext {
        // Markdown-class: indexed + graph-parsed.
        "md" | "txt" => FileClass::EditableText,

        // Media + dedicated preview classes.
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "avif" => FileClass::Image,
        "pdf" => FileClass::Pdf,

        // Source / config text. Editable, not indexed.
        //
        // The list errs on the side of inclusion for common
        // programming languages and config formats; obscure or
        // ambiguous extensions stay out and either resolve via
        // basename ("Makefile") or fall to `Other` (we'd rather
        // refuse a sketchy ext than risk opening a binary as
        // text). Content-sniffing is a follow-up.
        "rs" | "py" | "pyi" | "pyx" | "c" | "cc" | "cpp" | "cxx" | "h" | "hh" | "hpp" | "hxx"
        | "m" | "mm" | "go" | "java" | "kt" | "kts" | "swift" | "js" | "jsx" | "ts" | "tsx"
        | "mjs" | "cjs" | "rb" | "php" | "pl" | "pm" | "lua" | "r" | "scala" | "sc" | "clj"
        | "cljs" | "cljc" | "ml" | "mli" | "hs" | "lhs" | "erl" | "hrl" | "ex" | "exs" | "dart"
        | "nim" | "zig" | "vue" | "svelte" | "astro" | "elm" | "fs" | "fsi" | "fsx" | "tcl"
        | "awk" | "asm" | "vb" | "sh" | "bash" | "zsh" | "fish" | "ksh" | "csh" | "tcsh"
        | "ps1" | "psm1" | "psd1" | "toml" | "yaml" | "yml" | "json" | "json5" | "jsonl"
        | "ndjson" | "ini" | "cfg" | "conf" | "properties" | "env" | "envrc" | "lock" | "html"
        | "htm" | "css" | "scss" | "sass" | "less" | "xml" | "xhtml" | "xsl" | "xslt" | "rss"
        | "atom" | "csv" | "tsv" | "sql" | "log" | "mk" | "mak" | "cmake" | "bzl" | "ninja"
        | "gradle" | "patch" | "diff" | "rst" | "adoc" | "asciidoc" | "org" | "tex" | "latex"
        | "ltx" | "bib" | "gitignore" | "gitattributes" | "editorconfig" | "npmrc" | "nvmrc"
        | "babelrc" | "prettierrc" | "eslintrc" | "eslintignore" | "dockerignore" => {
            FileClass::Text
        }

        _ => return None,
    };
    Some(class)
}

/// Basename -> FileClass for well-known no-extension or all-caps
/// filenames. Used as the fallback when the extension is missing
/// or unknown; returns `None` so `classify` can settle on
/// `Other` when nothing matches.
fn classify_basename(name: &str) -> Option<FileClass> {
    let class = match name {
        "Makefile" | "GNUmakefile" | "BSDmakefile" | "makefile" => FileClass::Text,
        "Dockerfile" | "Containerfile" => FileClass::Text,
        "Rakefile" | "Gemfile" | "Procfile" | "Justfile" | "Vagrantfile" | "Berksfile"
        | "Brewfile" => FileClass::Text,
        "LICENSE" | "LICENCE" | "COPYING" | "COPYRIGHT" | "NOTICE" | "AUTHORS" | "CONTRIBUTORS"
        | "README" | "TODO" | "NEWS" | "CHANGELOG" | "HISTORY" | "INSTALL" | "MANIFEST" => {
            FileClass::Text
        }
        _ => return None,
    };
    Some(class)
}

/// Caller-supplied list of directory names that the indexing /
/// graph-rebuild walks should not descend into. Matched at any
/// depth by exact basename (case-sensitive on Unix, case-folded
/// on Windows via `eq_ignore_ascii_case`). Empty by default in
/// chan-core; the chan binary populates it with a noise list
/// (`node_modules`, `target`, `__pycache__`, ...) read from its
/// config so a user pointing chan at a source tree doesn't burn
/// CPU indexing dependencies.
///
/// What this is NOT:
///   - Glob matching. A future variant can grow that. v1 is
///     basename equality so the chan config stays simple and
///     the walker stays cheap.
///   - A trash / lock / sandbox gate. `.git` / `.chan` skip is
///     hardcoded in `walk_drive`; those are invariants, not
///     policy. The filter is purely additive on top.
///   - A read/write/list gate. `Drive::list_tree` (the editor's
///     file-tree view) stays unfiltered so the user can still
///     open files inside a "noisy" dir on demand. Only the
///     reindex paths consult the filter.
#[derive(Debug, Clone, Default)]
pub struct WalkFilter {
    /// Directory basenames to skip at any depth.
    pub excluded_dir_names: Vec<String>,
}

impl WalkFilter {
    pub fn new<I, S>(names: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            excluded_dir_names: names.into_iter().map(Into::into).collect(),
        }
    }

    /// True when `name` is an excluded directory basename.
    pub fn is_excluded(&self, name: &str) -> bool {
        self.excluded_dir_names
            .iter()
            .any(|n| n.eq_ignore_ascii_case(name))
    }
}

/// Recursive walker rooted at `root` that:
///   - skips `.git/` and `.chan/` at any depth;
///   - never follows symlinks (`walkdir` default; we set it
///     explicitly so a future maintainer cannot flip it without
///     understanding what they're trading away);
///   - drops non-regular non-directory entries (symlinks, FIFOs,
///     sockets, char/block devices) at iteration time so the
///     listing and the indexer only ever see real files and dirs.
///
/// Per-entry errors are logged and skipped.
pub fn walk_drive(root: &Path) -> impl Iterator<Item = DirEntry> {
    walk_drive_with(root, None)
}

/// Variant of `walk_drive` that additionally skips any directory
/// whose basename is in `filter.excluded_dir_names`. Used by the
/// reindex paths (graph rebuild + index facade). Pass `None` to
/// get the same behavior as `walk_drive`.
pub fn walk_drive_filtered<'a>(
    root: &Path,
    filter: &'a WalkFilter,
) -> impl Iterator<Item = DirEntry> + 'a {
    walk_drive_with(root, Some(filter))
}

fn walk_drive_with<'a>(
    root: &Path,
    filter: Option<&'a WalkFilter>,
) -> impl Iterator<Item = DirEntry> + 'a {
    WalkDir::new(root)
        .min_depth(1)
        .follow_links(false)
        .same_file_system(true)
        .into_iter()
        .filter_entry(move |e| {
            if !e.file_type().is_dir() {
                return true;
            }
            let n = e.file_name().to_string_lossy();
            if n == ".git" || n == ".chan" {
                return false;
            }
            if let Some(f) = filter {
                if f.is_excluded(&n) {
                    return false;
                }
            }
            true
        })
        .filter_map(|res| match res {
            Ok(e) => Some(e),
            Err(e) => {
                tracing::warn!("walkdir error: {e}");
                None
            }
        })
        .filter(|e| {
            let ft = e.file_type();
            // Keep dirs (we descend into them) and regular files.
            // Drop symlinks (regardless of where they point),
            // devices, sockets, and FIFOs.
            ft.is_dir() || ft.is_file()
        })
}

/// Atomically write `bytes` to `path`. Creates parent directories.
///
/// 1) Capture the target's mode + xattrs if it already exists. The
///    tmpfile we're about to create inherits the umask default, so
///    without this step every overwrite drops the file's mode back
///    to 0600 (or whatever umask gives) and strips xattrs. Editors
///    that rely on Finder tags / SELinux labels / capabilities would
///    silently lose them on every save. Best-effort: a failure to
///    read xattrs (permission, fs without xattr support) is logged
///    and skipped, not an error.
/// 2) Open a NamedTempFile in the same directory as `path`.
/// 3) Write all bytes.
/// 4) fsync the tempfile so the data is durable before the rename.
/// 5) Atomically rename over `path`.
/// 6) Re-apply the captured mode + xattrs. Mode is best-effort
///    (warn-on-failure) for the same reason: an exotic fs that
///    refuses chmod must not block a save.
/// 7) fsync the parent directory so the new dirent is durable too.
///    Without (7), POSIX permits the rename to be lost on power loss
///    even though the file's data was sync'd. ext4/xfs/btrfs/APFS all
///    need this for true atomic-write semantics.
pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let preserved = capture_metadata(path);
    let mut tmp = tempfile::NamedTempFile::new_in(dir)?;
    use std::io::Write;
    tmp.write_all(bytes)?;
    tmp.as_file().sync_all()?;
    tmp.persist(path)
        .map_err(|e| ChanError::Io(e.error.to_string()))?;
    apply_metadata(path, preserved);
    sync_dir(dir)?;
    Ok(())
}

/// Mode + xattrs captured before an atomic_write that overwrites an
/// existing file. None when the target doesn't exist or when stat
/// fails (we silently start fresh in those cases).
#[cfg(unix)]
struct PreservedMeta {
    mode: u32,
    xattrs: Vec<(std::ffi::OsString, Vec<u8>)>,
}

#[cfg(not(unix))]
struct PreservedMeta;

#[cfg(unix)]
fn capture_metadata(path: &Path) -> Option<PreservedMeta> {
    use std::os::unix::fs::PermissionsExt;
    let meta = std::fs::symlink_metadata(path).ok()?;
    if !meta.is_file() || meta.file_type().is_symlink() {
        return None;
    }
    let mode = meta.permissions().mode();
    let xattrs = read_xattrs(path);
    Some(PreservedMeta { mode, xattrs })
}

#[cfg(not(unix))]
fn capture_metadata(_path: &Path) -> Option<PreservedMeta> {
    None
}

#[cfg(unix)]
fn apply_metadata(path: &Path, preserved: Option<PreservedMeta>) {
    use std::os::unix::fs::PermissionsExt;
    let Some(p) = preserved else { return };
    if let Err(e) = std::fs::set_permissions(path, std::fs::Permissions::from_mode(p.mode)) {
        tracing::warn!(?path, mode = %format!("{:o}", p.mode), ?e, "atomic_write: chmod failed");
    }
    write_xattrs(path, &p.xattrs);
}

#[cfg(not(unix))]
fn apply_metadata(_path: &Path, _preserved: Option<PreservedMeta>) {}

#[cfg(any(
    target_os = "linux",
    target_os = "macos",
    target_os = "freebsd",
    target_os = "netbsd"
))]
fn read_xattrs(path: &Path) -> Vec<(std::ffi::OsString, Vec<u8>)> {
    let names = match xattr::list(path) {
        Ok(it) => it,
        Err(e) => {
            // ENOTSUP is normal on tmpfs/FAT/SMB; don't pollute logs.
            if !matches!(e.raw_os_error(), Some(libc_enotsup) if libc_enotsup == enotsup_errno()) {
                tracing::debug!(?path, ?e, "atomic_write: xattr list failed");
            }
            return Vec::new();
        }
    };
    let mut out = Vec::new();
    for name in names {
        match xattr::get(path, &name) {
            Ok(Some(value)) => out.push((name, value)),
            Ok(None) => {}
            Err(e) => tracing::debug!(?path, ?name, ?e, "atomic_write: xattr get failed"),
        }
    }
    out
}

#[cfg(any(
    target_os = "linux",
    target_os = "macos",
    target_os = "freebsd",
    target_os = "netbsd"
))]
fn write_xattrs(path: &Path, xattrs: &[(std::ffi::OsString, Vec<u8>)]) {
    for (name, value) in xattrs {
        if let Err(e) = xattr::set(path, name, value) {
            // Re-applying namespaced xattrs (e.g. security.selinux on
            // a fs without SELinux) can fail without that being our
            // fault; best-effort, warn at debug.
            tracing::debug!(?path, ?name, ?e, "atomic_write: xattr set failed");
        }
    }
}

#[cfg(any(
    target_os = "linux",
    target_os = "macos",
    target_os = "freebsd",
    target_os = "netbsd"
))]
fn enotsup_errno() -> i32 {
    // ENOTSUP and EOPNOTSUPP share a number on Linux but differ on
    // BSDs; both indicate "filesystem doesn't support xattrs".
    #[cfg(target_os = "linux")]
    {
        95 // ENOTSUP / EOPNOTSUPP on Linux
    }
    #[cfg(any(target_os = "macos", target_os = "freebsd", target_os = "netbsd"))]
    {
        45 // ENOTSUP on macOS/BSD
    }
}

#[cfg(all(
    unix,
    not(any(
        target_os = "linux",
        target_os = "macos",
        target_os = "freebsd",
        target_os = "netbsd"
    ))
))]
fn read_xattrs(_path: &Path) -> Vec<(std::ffi::OsString, Vec<u8>)> {
    Vec::new()
}

#[cfg(all(
    unix,
    not(any(
        target_os = "linux",
        target_os = "macos",
        target_os = "freebsd",
        target_os = "netbsd"
    ))
))]
fn write_xattrs(_path: &Path, _xattrs: &[(std::ffi::OsString, Vec<u8>)]) {}

/// fsync a directory so a freshly-created or freshly-renamed entry
/// in it becomes durable. Unix-only: opening a directory and calling
/// `FlushFileBuffers` is not supported on Windows, where NTFS commits
/// dirent changes through the journal as part of the rename itself.
#[cfg(unix)]
fn sync_dir(dir: &Path) -> Result<()> {
    let f = std::fs::File::open(dir)?;
    f.sync_all()?;
    Ok(())
}

#[cfg(not(unix))]
fn sync_dir(_dir: &Path) -> Result<()> {
    Ok(())
}

/// fsync a single file's data + metadata. Used by callers that
/// produced a file via `fs::copy` (which does NOT fsync) and need
/// it durable before the next ordering point. Unix-only; the
/// non-unix path is a no-op for the same reason as `sync_dir`.
#[cfg(unix)]
pub(crate) fn sync_file(path: &Path) -> Result<()> {
    let f = std::fs::File::open(path)?;
    f.sync_all()?;
    Ok(())
}

#[cfg(not(unix))]
pub(crate) fn sync_file(_path: &Path) -> Result<()> {
    Ok(())
}

/// Recursively fsync every regular file under `root`, then fsync
/// `root` itself. Used after a recursive copy (trash move / restore
/// across filesystems) so the whole subtree is durable before we
/// commit a "this is moved" marker. Symlinks/special files inside
/// the tree are skipped; chan-drive never creates them.
///
/// Best-effort: per-entry walkdir or fsync errors are logged at
/// warn and the walk continues. The caller already wrote the
/// bytes; the worst that a missed fsync does is widen the window
/// in which a power loss could lose data for that one file. A
/// hard error here would abort the surrounding trash op for a
/// single permission-denied entry, which is a worse trade than
/// "the durable barrier is one file weaker than ideal."
#[cfg(unix)]
pub(crate) fn sync_tree(root: &Path) -> Result<()> {
    let mut errors = 0usize;
    for entry in WalkDir::new(root).follow_links(false) {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!(?root, ?e, "sync_tree: walk error; continuing");
                errors += 1;
                continue;
            }
        };
        let ft = entry.file_type();
        let res = if ft.is_file() {
            sync_file(entry.path())
        } else if ft.is_dir() {
            sync_dir(entry.path())
        } else {
            Ok(())
        };
        if let Err(e) = res {
            tracing::warn!(path = ?entry.path(), ?e, "sync_tree: fsync failed; continuing");
            errors += 1;
        }
    }
    if errors > 0 {
        tracing::warn!(?root, errors, "sync_tree: completed with per-entry errors");
    }
    Ok(())
}

#[cfg(not(unix))]
pub(crate) fn sync_tree(_root: &Path) -> Result<()> {
    Ok(())
}

/// Human-readable name for a file type, used in error messages and
/// log lines. Covers the unix-only special types behind cfg(unix);
/// other platforms collapse them under "unknown" since std doesn't
/// surface them through `FileType` directly.
pub fn describe_file_kind(ft: &std::fs::FileType) -> &'static str {
    if ft.is_dir() {
        return "directory";
    }
    if ft.is_symlink() {
        return "symlink";
    }
    if ft.is_file() {
        return "regular";
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::FileTypeExt;
        if ft.is_fifo() {
            return "fifo";
        }
        if ft.is_socket() {
            return "socket";
        }
        if ft.is_char_device() {
            return "char_device";
        }
        if ft.is_block_device() {
            return "block_device";
        }
    }
    "unknown"
}

/// Reject anything that isn't a regular file. Uses `lstat` semantics
/// (`symlink_metadata`) so a symlink target's kind cannot mask the
/// link itself. Call this before opening a path for read or write
/// so the layer never:
///
///   - blocks forever on a FIFO with no writer;
///   - drains `/dev/zero` or the like into a buffer;
///   - sends ioctl-shaped reads to a char/block device;
///   - follows a symlink and writes through it (atomic_write
///     replaces the symlink itself, but `read_text` and friends
///     would happily resolve through one).
///
/// Returns `Ok(())` when the path is a regular file. `ENOENT`
/// propagates as a normal `Io` error so callers can distinguish
/// "no file" from "wrong file type".
pub fn ensure_regular_file(path: &Path) -> Result<()> {
    let meta = std::fs::symlink_metadata(path)?;
    let ft = meta.file_type();
    if ft.is_file() && !ft.is_symlink() {
        return Ok(());
    }
    Err(ChanError::SpecialFile {
        kind: describe_file_kind(&ft).to_string(),
        path: path.to_path_buf(),
    })
}

/// Stricter resolve: lexical `resolve_safe` plus a canonical-form
/// check that the deepest existing ancestor still lives under the
/// canonical drive root. Catches the case where a mid-path
/// component is a symlink pointing outside the drive (e.g. a user
/// has `Backup -> /Volumes/external` inside their drive, and a
/// caller asks to write `Backup/today.md`; we refuse).
///
/// For paths that don't exist yet (typical for create/write), we
/// canonicalize the deepest existing ancestor instead of the leaf.
/// This mirrors what the kernel will do when it walks the path
/// during the actual open call.
///
/// Each call canonicalizes the drive root. Hot-path callers that
/// hold a long-lived `Drive` should use `resolve_safe_strict_canon`
/// with a cached canonical root instead, so cloud-synced drive
/// roots (iCloud / Google Drive / Dropbox) do not pay an FS-provider
/// round trip on every read or write.
pub fn resolve_safe_strict(root: &Path, requested: &str) -> Result<PathBuf> {
    let root_canon = root
        .canonicalize()
        .map_err(|e| ChanError::Io(format!("canonicalize drive root: {e}")))?;
    resolve_safe_strict_canon(root, &root_canon, requested)
}

/// Same gate as `resolve_safe_strict` but takes a pre-canonicalized
/// drive root so the caller doesn't pay a `canonicalize` syscall on
/// every entry point. The canonical root MUST be the canonicalize
/// of `root`; passing anything else lets paths escape the sandbox.
pub fn resolve_safe_strict_canon(
    root: &Path,
    root_canon: &Path,
    requested: &str,
) -> Result<PathBuf> {
    let joined = resolve_safe(root, requested)?;

    // Find the deepest ancestor of `joined` that already exists,
    // canonicalize it, and check it stays under root_canon.
    let mut probe: &Path = &joined;
    let canon_ancestor = loop {
        match probe.canonicalize() {
            Ok(c) => break c,
            Err(_) => match probe.parent() {
                Some(p) => probe = p,
                // We walked past the drive root without finding
                // anything that canonicalizes; treat as escape.
                None => return Err(ChanError::SymlinkEscape(joined)),
            },
        }
    };

    if !canon_ancestor.starts_with(root_canon) {
        return Err(ChanError::SymlinkEscape(joined));
    }
    Ok(joined)
}

/// Validate a request rel-path for use with the cap-std sandboxed
/// `Dir`. Strips a leading `/`, refuses empty / `..` traversal /
/// absolute paths, and returns a `PathBuf` of pure `Component::Normal`
/// segments. cap-std performs the actual TOCTOU-free walk; this
/// helper just gives us cleaner error variants than mapping
/// cap-std's generic `io::Error`s.
pub fn validate_rel(requested: &str) -> Result<PathBuf> {
    let trimmed = requested.trim_start_matches('/');
    if trimmed.is_empty() {
        return Err(ChanError::PathEmpty);
    }
    let raw = Path::new(trimmed);
    let mut out = PathBuf::new();
    for component in raw.components() {
        match component {
            Component::Normal(c) => out.push(c),
            Component::CurDir => {}
            _ => return Err(ChanError::PathEscape),
        }
    }
    if out.as_os_str().is_empty() {
        return Err(ChanError::PathEmpty);
    }
    Ok(out)
}

/// Atomic write into a cap-std `Dir`: tmpfile in the same
/// directory, write, fsync, atomic rename over `rel`. cap-tempfile
/// handles the rename; we add the dir fsync for the same reason
/// `atomic_write` does (POSIX permits a rename to be lost on power
/// loss without a parent-dir fsync). Mode + xattrs are preserved
/// for an existing target via the same capture/apply pattern as
/// `atomic_write`.
///
/// `rel` must already be `validate_rel`-ed; cap-std would refuse a
/// bad path anyway, but the explicit gate keeps our error mapping
/// crisp.
pub fn atomic_write_in(dir: &cap_std::fs::Dir, rel: &Path, bytes: &[u8]) -> Result<()> {
    use std::io::Write;

    if let Some(parent) = rel.parent() {
        if !parent.as_os_str().is_empty() {
            dir.create_dir_all(parent).map_err(|e| map_cap(e, rel))?;
        }
    }
    let preserved = capture_metadata_in(dir, rel);
    // cap-tempfile creates the temp file in the same dir as `rel`'s
    // parent so the eventual rename stays same-fs. Pass the parent;
    // for top-level files that's the drive root itself.
    let parent = rel.parent().filter(|p| !p.as_os_str().is_empty());
    let parent_dir;
    let target_dir: &cap_std::fs::Dir = match parent {
        Some(p) => {
            parent_dir = dir.open_dir(p).map_err(|e| map_cap(e, rel))?;
            &parent_dir
        }
        None => dir,
    };
    let leaf = rel.file_name().ok_or(ChanError::PathEmpty)?;
    let mut tmp = cap_tempfile::TempFile::new(target_dir).map_err(|e| map_cap(e, rel))?;
    tmp.write_all(bytes)
        .map_err(|e| ChanError::Io(format!("write: {e}")))?;
    tmp.as_file()
        .sync_all()
        .map_err(|e| ChanError::Io(format!("fsync tmp: {e}")))?;
    tmp.replace(leaf).map_err(|e| map_cap(e, rel))?;
    apply_metadata_in(dir, rel, preserved);
    sync_dir_handle(target_dir)?;
    Ok(())
}

/// Map a cap-std `io::Error` into our error enum, distinguishing
/// "you tried to escape the sandbox" from generic I/O. cap-std
/// signals an escape via the message string (see `map_cap_err` in
/// `drive.rs` for the symmetric mapping on the Drive side).
fn map_cap(err: std::io::Error, rel: &Path) -> ChanError {
    let msg = err.to_string();
    if msg.contains("outside of the filesystem") || msg.contains("path escape") {
        return ChanError::SymlinkEscape(rel.to_path_buf());
    }
    ChanError::Io(msg)
}

#[cfg(unix)]
fn capture_metadata_in(dir: &cap_std::fs::Dir, rel: &Path) -> Option<PreservedMeta> {
    use cap_std::fs::MetadataExt;
    let meta = dir.symlink_metadata(rel).ok()?;
    if !meta.is_file() || meta.file_type().is_symlink() {
        return None;
    }
    let mode = meta.mode();
    // For xattrs we need the absolute path; cap-std's File doesn't
    // currently expose fgetxattr in a portable way, and falling back
    // to lookup via the underlying fd is platform-specific. Best-
    // effort: read xattrs through the in-process abs path computed
    // from the dir handle. On systems where this isn't supported the
    // shim returns empty.
    let xattrs = read_xattrs_via_fd(dir, rel);
    Some(PreservedMeta { mode, xattrs })
}

#[cfg(not(unix))]
fn capture_metadata_in(_dir: &cap_std::fs::Dir, _rel: &Path) -> Option<PreservedMeta> {
    None
}

#[cfg(unix)]
fn apply_metadata_in(dir: &cap_std::fs::Dir, rel: &Path, preserved: Option<PreservedMeta>) {
    use std::os::unix::fs::PermissionsExt;
    let Some(p) = preserved else { return };
    if let Ok(file) = dir.open(rel) {
        let perms = cap_std::fs::Permissions::from_std(std::fs::Permissions::from_mode(p.mode));
        if let Err(e) = file.set_permissions(perms) {
            tracing::warn!(?rel, mode = %format!("{:o}", p.mode), ?e, "atomic_write_in: chmod failed");
        }
    }
    write_xattrs_via_fd(dir, rel, &p.xattrs);
}

#[cfg(not(unix))]
fn apply_metadata_in(_dir: &cap_std::fs::Dir, _rel: &Path, _preserved: Option<PreservedMeta>) {}

#[cfg(any(
    target_os = "linux",
    target_os = "macos",
    target_os = "freebsd",
    target_os = "netbsd"
))]
fn read_xattrs_via_fd(dir: &cap_std::fs::Dir, rel: &Path) -> Vec<(std::ffi::OsString, Vec<u8>)> {
    use xattr::FileExt;
    // Open through cap-std (sandbox-validated), then dup into a
    // std::fs::File so we can call xattr's FileExt. xattr's FileExt
    // is implemented on std::fs::File only; the dup keeps the
    // operation fd-bound (no abs-path round-trip) while satisfying
    // the trait impl.
    let Some(file) = open_std_file_through_dir(dir, rel) else {
        return Vec::new();
    };
    let names = match file.list_xattr() {
        Ok(it) => it,
        Err(e) => {
            tracing::debug!(?rel, ?e, "atomic_write_in: xattr list failed");
            return Vec::new();
        }
    };
    let mut out = Vec::new();
    for name in names {
        match file.get_xattr(&name) {
            Ok(Some(v)) => out.push((name, v)),
            Ok(None) => {}
            Err(e) => tracing::debug!(?rel, ?name, ?e, "atomic_write_in: xattr get failed"),
        }
    }
    out
}

#[cfg(any(
    target_os = "linux",
    target_os = "macos",
    target_os = "freebsd",
    target_os = "netbsd"
))]
fn write_xattrs_via_fd(
    dir: &cap_std::fs::Dir,
    rel: &Path,
    xattrs: &[(std::ffi::OsString, Vec<u8>)],
) {
    use xattr::FileExt;
    let Some(file) = open_std_file_through_dir(dir, rel) else {
        return;
    };
    for (name, value) in xattrs {
        if let Err(e) = file.set_xattr(name, value) {
            tracing::debug!(?rel, ?name, ?e, "atomic_write_in: xattr set failed");
        }
    }
}

/// Open `rel` through the cap-std `Dir` and dup the fd into a
/// `std::fs::File`. Used as a bridge to crates (like `xattr`) that
/// expect `std::fs::File` for their `FileExt` impls.
#[cfg(unix)]
fn open_std_file_through_dir(dir: &cap_std::fs::Dir, rel: &Path) -> Option<std::fs::File> {
    use std::os::fd::AsFd;
    let cap_file = dir.open(rel).ok()?;
    let owned = cap_file.as_fd().try_clone_to_owned().ok()?;
    Some(std::fs::File::from(owned))
}

#[cfg(all(
    unix,
    not(any(
        target_os = "linux",
        target_os = "macos",
        target_os = "freebsd",
        target_os = "netbsd"
    ))
))]
fn read_xattrs_via_fd(_dir: &cap_std::fs::Dir, _rel: &Path) -> Vec<(std::ffi::OsString, Vec<u8>)> {
    Vec::new()
}

#[cfg(all(
    unix,
    not(any(
        target_os = "linux",
        target_os = "macos",
        target_os = "freebsd",
        target_os = "netbsd"
    ))
))]
fn write_xattrs_via_fd(
    _dir: &cap_std::fs::Dir,
    _rel: &Path,
    _xattrs: &[(std::ffi::OsString, Vec<u8>)],
) {
}

/// Fsync a cap-std `Dir` so a fresh dirent inside it becomes
/// durable. On Windows this is a no-op (NTFS commits dirent changes
/// through the journal as part of the rename itself).
///
/// Linux quirk: `Dir::open_ambient_dir` opens directories with
/// `O_PATH` via cap-primitives, and an `O_PATH` fd does not support
/// `fsync` (returns `EBADF`). Dup'ing the fd preserves `O_PATH`, so a
/// straight `try_clone_to_owned` + `sync_all` fails on Linux. We
/// re-open the same dir via `/proc/self/fd/<n>` to get a fresh
/// non-`O_PATH` fd that supports `fsync`. Other unixes (macOS, BSDs)
/// don't carry `O_PATH`, so the dup path is fine there.
#[cfg(target_os = "linux")]
pub(crate) fn sync_dir_handle(dir: &cap_std::fs::Dir) -> Result<()> {
    use std::os::fd::AsRawFd;
    let raw = dir.as_raw_fd();
    let proc_path = format!("/proc/self/fd/{raw}");
    let f = std::fs::File::open(&proc_path)
        .map_err(|e| ChanError::Io(format!("reopen dir via procfs for fsync: {e}")))?;
    f.sync_all()
        .map_err(|e| ChanError::Io(format!("fsync dir: {e}")))?;
    Ok(())
}

#[cfg(all(unix, not(target_os = "linux")))]
pub(crate) fn sync_dir_handle(dir: &cap_std::fs::Dir) -> Result<()> {
    use std::os::fd::AsFd;
    let owned = dir
        .as_fd()
        .try_clone_to_owned()
        .map_err(|e| ChanError::Io(format!("dup dir fd: {e}")))?;
    let f: std::fs::File = owned.into();
    f.sync_all()
        .map_err(|e| ChanError::Io(format!("fsync dir: {e}")))?;
    Ok(())
}

#[cfg(not(unix))]
pub(crate) fn sync_dir_handle(_dir: &cap_std::fs::Dir) -> Result<()> {
    Ok(())
}

/// Take an untrusted request path (`notes/x.md` or `../etc/passwd`)
/// and join it onto the drive root, rejecting any traversal that
/// escapes the root. Returns the absolute joined path.
///
/// This is a LEXICAL check only: it does not detect mid-path
/// symlinks pointing outside the drive. Use `resolve_safe_strict`
/// for that. We keep this as a fast-path for tests and for the
/// strict variant's first leg.
pub fn resolve_safe(root: &Path, requested: &str) -> Result<PathBuf> {
    let requested = requested.trim_start_matches('/');
    if requested.is_empty() {
        return Err(ChanError::PathEmpty);
    }
    let raw = Path::new(requested);
    let mut joined = PathBuf::from(root);
    for component in raw.components() {
        match component {
            Component::Normal(c) => joined.push(c),
            Component::CurDir => {}
            _ => return Err(ChanError::PathEscape),
        }
    }
    Ok(joined)
}

/// One entry in the file tree. Path is relative to the drive root
/// using `/` separators on all platforms (stable JSON shape).
#[derive(Debug, Clone, Serialize)]
pub struct TreeEntry {
    pub path: String,
    pub is_dir: bool,
    /// Last modification time as Unix seconds. None if unavailable.
    pub mtime: Option<i64>,
    /// File size in bytes. 0 for directories.
    pub size: u64,
}

/// Hard cap on entries returned by `list_tree`. 500k covers any
/// realistic notes drive with margin (Obsidian-shaped vaults run a
/// few thousand files; the largest in-the-wild notes corpora hit
/// low six figures). Past this we refuse to allocate the result
/// vec rather than OOM the editor; the user has either pointed
/// chan at the wrong directory (e.g. `~`) or has a cleanup job.
pub const LIST_TREE_LIMIT: usize = 500_000;

/// Hard cap on entries returned by a single-directory `list`. 50k
/// fits any directory a human could reasonably navigate; beyond
/// that the editor's tree view is unusable anyway.
pub const LIST_DIR_LIMIT: usize = 50_000;

/// Recursively list everything under `root`. Skips `.git/` and
/// `.chan/` at any depth. Errors with `ListingTooLarge` once the
/// walker sees more than `LIST_TREE_LIMIT` entries, so a runaway
/// or mis-pointed drive never OOMs the caller.
pub fn list_tree(root: &Path) -> Result<Vec<TreeEntry>> {
    list_tree_inner(root, root, 1, None)
}

/// Variant of `list_tree` that also applies a caller-supplied
/// directory-name blocklist. Used by the reindex paths so a
/// `node_modules/` under the drive doesn't force the graph
/// rebuild to walk a hundred thousand README.md files. The
/// editor's tree view keeps using the unfiltered `list_tree`.
pub fn list_tree_filtered(root: &Path, filter: &WalkFilter) -> Result<Vec<TreeEntry>> {
    list_tree_inner(root, root, 1, Some(filter))
}

/// Variant of `list_tree` scoped to the subtree at `subtree_abs`,
/// which must be `root` or a descendant. Walks only that subtree;
/// returned `TreeEntry.path` values stay relative to `root` so
/// callers see the same shape as `list_tree`.
///
/// When `subtree_abs` points at a regular file, the result is a
/// single-entry vec for that file. When it points at a directory,
/// the directory itself is included followed by its descendants
/// (matching the previous client-side filter `e.path == p ||
/// e.path.starts_with(format!("{p}/"))`). When the path doesn't
/// exist on disk, returns `Ok(vec![])` rather than an error so the
/// `list_files` tool can return an empty listing instead of a hard
/// failure when a model probes a non-existent prefix.
///
/// `LIST_TREE_LIMIT` still applies, in case a misconfigured prefix
/// covers the whole drive (e.g. the user pointed chan at `~`).
pub fn list_tree_prefix(root: &Path, subtree_abs: &Path) -> Result<Vec<TreeEntry>> {
    if !subtree_abs.exists() {
        return Ok(Vec::new());
    }
    // min_depth=0 so the prefix entry itself is included; the
    // legacy client-side filter in chan-llm did the same. Files
    // come back as their single self-entry; directories come back
    // with their own entry plus descendants.
    list_tree_inner(root, subtree_abs, 0, None)
}

fn list_tree_inner(
    root: &Path,
    walk_from: &Path,
    min_depth: usize,
    filter: Option<&WalkFilter>,
) -> Result<Vec<TreeEntry>> {
    let mut out = Vec::new();
    let iter: Box<dyn Iterator<Item = DirEntry>> = if walk_from == root {
        // Same shape the public `walk_drive` / `walk_drive_filtered`
        // helpers offer; reuse them so the .git / .chan and special-
        // file filters live in one place.
        match filter {
            Some(f) => Box::new(walk_drive_filtered(root, f)),
            None => Box::new(walk_drive(root)),
        }
    } else {
        // Subtree walk. Inline the same filter chain `walk_drive`
        // applies, with `min_depth` overridden so the prefix entry
        // itself is included for callers that want to see it.
        let walker = WalkDir::new(walk_from)
            .min_depth(min_depth)
            .follow_links(false)
            .same_file_system(true)
            .into_iter()
            .filter_entry(|e| {
                if !e.file_type().is_dir() {
                    return true;
                }
                let n = e.file_name().to_string_lossy();
                n != ".git" && n != ".chan"
            })
            .filter_map(|res| match res {
                Ok(e) => Some(e),
                Err(e) => {
                    tracing::warn!("walkdir error: {e}");
                    None
                }
            })
            .filter(|e| {
                let ft = e.file_type();
                ft.is_dir() || ft.is_file()
            });
        Box::new(walker)
    };
    for entry in iter {
        if out.len() >= LIST_TREE_LIMIT {
            return Err(ChanError::ListingTooLarge {
                observed: out.len(),
                limit: LIST_TREE_LIMIT,
            });
        }
        let rel = entry
            .path()
            .strip_prefix(root)
            .map_err(|_| ChanError::PathEscape)?;
        let path_str = rel.to_string_lossy().replace('\\', "/");
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!(?path_str, ?e, "metadata failed; skipping");
                continue;
            }
        };
        let mtime = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64);
        out.push(TreeEntry {
            path: path_str,
            is_dir: meta.is_dir(),
            mtime,
            size: if meta.is_dir() { 0 } else { meta.len() },
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn atomic_write_creates_dirs() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("a/b/c.txt");
        atomic_write(&p, b"hello").unwrap();
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "hello");
    }

    #[test]
    fn atomic_write_overwrites() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("x.txt");
        atomic_write(&p, b"v1").unwrap();
        atomic_write(&p, b"v2").unwrap();
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "v2");
    }

    #[cfg(unix)]
    #[test]
    fn atomic_write_preserves_mode_across_overwrites() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("note.md");
        atomic_write(&p, b"v1").unwrap();
        // User chmods to 0644 (a-w for group/world is the typical
        // share setting). The next save must keep that.
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o644)).unwrap();
        atomic_write(&p, b"v2").unwrap();
        let mode = std::fs::metadata(&p).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o644, "mode dropped on overwrite");
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[test]
    fn atomic_write_preserves_user_xattr_across_overwrites() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("note.md");
        atomic_write(&p, b"v1").unwrap();
        // user.* is the unprivileged xattr namespace on Linux and a
        // valid attr on macOS HFS+/APFS. We test against it because
        // it doesn't require any privilege and is supported on the
        // target's standard test filesystem.
        let key = "user.chan.test";
        if let Err(_e) = xattr::set(&p, key, b"hello") {
            // tmpfs / sandboxed CI may reject user.* xattrs; skip.
            return;
        }
        atomic_write(&p, b"v2").unwrap();
        let got = xattr::get(&p, key).unwrap();
        assert_eq!(
            got.as_deref(),
            Some(&b"hello"[..]),
            "xattr lost on overwrite"
        );
    }

    #[test]
    fn list_tree_skips_internal_dirs() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join(".chan")).unwrap();
        std::fs::create_dir_all(tmp.path().join(".git/objects")).unwrap();
        std::fs::write(tmp.path().join(".chan/x"), b"").unwrap();
        std::fs::write(tmp.path().join(".git/HEAD"), b"").unwrap();
        std::fs::write(tmp.path().join("note.md"), b"hi").unwrap();
        let tree = list_tree(tmp.path()).unwrap();
        let paths: Vec<_> = tree.iter().map(|e| e.path.as_str()).collect();
        assert!(paths.contains(&"note.md"));
        assert!(!paths.iter().any(|p| p.starts_with(".chan")));
        assert!(!paths.iter().any(|p| p.starts_with(".git")));
    }

    #[test]
    fn list_tree_prefix_walks_only_subtree() {
        let tmp = TempDir::new().unwrap();
        // Layout:
        //   notes/
        //     a.md
        //     deep/b.md
        //   other/c.md
        //   top.md
        std::fs::create_dir_all(tmp.path().join("notes/deep")).unwrap();
        std::fs::create_dir_all(tmp.path().join("other")).unwrap();
        std::fs::write(tmp.path().join("notes/a.md"), b"a").unwrap();
        std::fs::write(tmp.path().join("notes/deep/b.md"), b"b").unwrap();
        std::fs::write(tmp.path().join("other/c.md"), b"c").unwrap();
        std::fs::write(tmp.path().join("top.md"), b"t").unwrap();

        let entries = list_tree_prefix(tmp.path(), &tmp.path().join("notes")).unwrap();
        let paths: Vec<_> = entries.iter().map(|e| e.path.as_str()).collect();
        // The prefix entry itself is included (matches the legacy
        // client-side filter), plus everything under it.
        assert!(paths.contains(&"notes"), "got {paths:?}");
        assert!(paths.contains(&"notes/a.md"));
        assert!(paths.contains(&"notes/deep"));
        assert!(paths.contains(&"notes/deep/b.md"));
        // Nothing outside the subtree leaks in.
        assert!(!paths.iter().any(|p| p.starts_with("other")));
        assert!(!paths.contains(&"top.md"));
    }

    #[test]
    fn list_tree_prefix_returns_single_entry_for_file() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("note.md"), b"hi").unwrap();
        let entries = list_tree_prefix(tmp.path(), &tmp.path().join("note.md")).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].path, "note.md");
        assert!(!entries[0].is_dir);
    }

    #[test]
    fn list_tree_prefix_missing_path_returns_empty() {
        let tmp = TempDir::new().unwrap();
        // The model passes a nonexistent prefix (typo, stale path);
        // we want an empty listing rather than a hard error.
        let entries = list_tree_prefix(tmp.path(), &tmp.path().join("nope")).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn resolve_safe_rejects_traversal() {
        let tmp = TempDir::new().unwrap();
        assert!(matches!(
            resolve_safe(tmp.path(), "../escape").unwrap_err(),
            ChanError::PathEscape
        ));
        assert!(matches!(
            resolve_safe(tmp.path(), "a/../b").unwrap_err(),
            ChanError::PathEscape
        ));
    }

    #[test]
    fn resolve_safe_accepts_normal() {
        let tmp = TempDir::new().unwrap();
        let r = resolve_safe(tmp.path(), "notes/x.md").unwrap();
        assert!(r.starts_with(tmp.path()));
        assert!(r.ends_with("notes/x.md"));
    }

    #[test]
    fn is_editable_text_covers_markdown_and_text() {
        // Markdown-class.
        assert!(is_editable_text("note.md"));
        assert!(is_editable_text("a/b/c.txt"));
        assert!(is_editable_text("README.MD"));
        // Text-class (source / config / shell / well-known
        // basenames). All editable through the UTF-8 gate.
        assert!(is_editable_text("src/main.py"));
        assert!(is_editable_text("Cargo.toml"));
        assert!(is_editable_text("config.yaml"));
        assert!(is_editable_text("script.sh"));
        assert!(is_editable_text(".gitignore"));
        assert!(is_editable_text("Makefile"));
        assert!(is_editable_text("Dockerfile"));
        assert!(is_editable_text("LICENSE"));
        // Non-textual.
        assert!(!is_editable_text("image.png"));
        assert!(!is_editable_text("doc.pdf"));
        assert!(!is_editable_text("archive.zip"));
        assert!(!is_editable_text("song.mp3"));
        assert!(!is_editable_text(""));
    }

    #[test]
    fn is_indexable_text_stays_markdown_only() {
        // Markdown-class is indexable + graph-parsed.
        assert!(is_indexable_text("note.md"));
        assert!(is_indexable_text("a/b/c.txt"));
        assert!(is_indexable_text("README.MD"));
        // Text-class is editable but **not** indexed: avoids
        // `#include` looking like a `#tag` and similar false
        // positives.
        assert!(!is_indexable_text("src/main.py"));
        assert!(!is_indexable_text("Cargo.toml"));
        assert!(!is_indexable_text("Makefile"));
        // Everything else stays excluded.
        assert!(!is_indexable_text("image.png"));
        assert!(!is_indexable_text("doc.pdf"));
        assert!(!is_indexable_text("song.mp3"));
        assert!(!is_indexable_text(""));
    }

    #[test]
    fn classify_covers_each_class() {
        // EditableText (markdown-class).
        assert_eq!(classify("note.md"), FileClass::EditableText);
        assert_eq!(classify("a/b/c.txt"), FileClass::EditableText);
        assert_eq!(classify("README.MD"), FileClass::EditableText);
        // Image.
        assert_eq!(classify("img.png"), FileClass::Image);
        assert_eq!(classify("photo.JPG"), FileClass::Image);
        assert_eq!(classify("icon.svg"), FileClass::Image);
        assert_eq!(classify("anim.gif"), FileClass::Image);
        assert_eq!(classify("modern.webp"), FileClass::Image);
        assert_eq!(classify("new.avif"), FileClass::Image);
        // Pdf.
        assert_eq!(classify("doc.pdf"), FileClass::Pdf);
        assert_eq!(classify("paper.PDF"), FileClass::Pdf);
        // Text: source code.
        assert_eq!(classify("script.sh"), FileClass::Text);
        assert_eq!(classify("main.py"), FileClass::Text);
        assert_eq!(classify("lib.rs"), FileClass::Text);
        assert_eq!(classify("App.tsx"), FileClass::Text);
        assert_eq!(classify("server.go"), FileClass::Text);
        // Text: config + data.
        assert_eq!(classify("Cargo.toml"), FileClass::Text);
        assert_eq!(classify("config.yaml"), FileClass::Text);
        assert_eq!(classify("package.json"), FileClass::Text);
        assert_eq!(classify("data.csv"), FileClass::Text);
        assert_eq!(classify("schema.sql"), FileClass::Text);
        // Text: web / markup.
        assert_eq!(classify("index.html"), FileClass::Text);
        assert_eq!(classify("style.css"), FileClass::Text);
        assert_eq!(classify("readme.rst"), FileClass::Text);
        // Text: well-known basenames (no ext or all-caps).
        assert_eq!(classify("Makefile"), FileClass::Text);
        assert_eq!(classify("notes/Dockerfile"), FileClass::Text);
        assert_eq!(classify("LICENSE"), FileClass::Text);
        assert_eq!(classify("CHANGELOG"), FileClass::Text);
        // Text: dotfiles.
        assert_eq!(classify(".gitignore"), FileClass::Text);
        assert_eq!(classify(".editorconfig"), FileClass::Text);
        // Other: opaque binaries and unknowns.
        assert_eq!(classify("archive.zip"), FileClass::Other);
        assert_eq!(classify("song.mp3"), FileClass::Other);
        assert_eq!(classify("font.ttf"), FileClass::Other);
        assert_eq!(classify(""), FileClass::Other);
        // Case insensitivity for known textual extensions.
        assert_eq!(classify("Main.PY"), FileClass::Text);
        assert_eq!(classify("Build.YAML"), FileClass::Text);
    }

    #[test]
    fn ensure_regular_file_accepts_regular() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("a.md");
        std::fs::write(&p, b"hi").unwrap();
        ensure_regular_file(&p).unwrap();
    }

    #[test]
    fn ensure_regular_file_rejects_directory() {
        let tmp = TempDir::new().unwrap();
        let err = ensure_regular_file(tmp.path()).unwrap_err();
        match err {
            ChanError::SpecialFile { kind, .. } => assert_eq!(kind, "directory"),
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[cfg(unix)]
    #[test]
    fn ensure_regular_file_rejects_symlink() {
        use std::os::unix::fs::symlink;
        let tmp = TempDir::new().unwrap();
        let target = tmp.path().join("real.md");
        let link = tmp.path().join("link.md");
        std::fs::write(&target, b"hi").unwrap();
        symlink(&target, &link).unwrap();
        let err = ensure_regular_file(&link).unwrap_err();
        match err {
            ChanError::SpecialFile { kind, .. } => assert_eq!(kind, "symlink"),
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[cfg(unix)]
    #[test]
    fn ensure_regular_file_rejects_unix_socket() {
        use std::os::unix::net::UnixListener;
        let tmp = TempDir::new().unwrap();
        let sock = tmp.path().join("s");
        let _l = UnixListener::bind(&sock).unwrap();
        let err = ensure_regular_file(&sock).unwrap_err();
        match err {
            ChanError::SpecialFile { kind, .. } => assert_eq!(kind, "socket"),
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[cfg(unix)]
    #[test]
    fn resolve_safe_strict_rejects_midpath_symlink_to_outside() {
        use std::os::unix::fs::symlink;
        let outside = TempDir::new().unwrap();
        let drive = TempDir::new().unwrap();
        // Backup -> outside dir.
        symlink(outside.path(), drive.path().join("Backup")).unwrap();
        let err = resolve_safe_strict(drive.path(), "Backup/today.md").unwrap_err();
        assert!(matches!(err, ChanError::SymlinkEscape(_)));
    }

    #[cfg(unix)]
    #[test]
    fn resolve_safe_strict_allows_symlink_pointing_inside() {
        use std::os::unix::fs::symlink;
        let drive = TempDir::new().unwrap();
        std::fs::create_dir(drive.path().join("real")).unwrap();
        // alias -> ./real, both under the drive. The strict resolve
        // doesn't reject in-drive symlinks; the per-path lstat gate
        // in Drive::read_text / write_text is what catches them as
        // a final-component policy.
        symlink("real", drive.path().join("alias")).unwrap();
        resolve_safe_strict(drive.path(), "alias/x.md").unwrap();
    }

    #[test]
    fn resolve_safe_strict_passes_normal_path() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir(tmp.path().join("notes")).unwrap();
        resolve_safe_strict(tmp.path(), "notes/x.md").unwrap();
    }

    #[test]
    fn walk_drive_filtered_skips_named_dirs_at_any_depth() {
        // node_modules at the root and again nested under web/ both
        // get pruned: the indexing walker should never descend into
        // them. .git stays hardcoded in the unfiltered walk, so we
        // confirm the filter is additive, not replacing.
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join("notes")).unwrap();
        std::fs::write(tmp.path().join("notes/a.md"), b"a").unwrap();
        std::fs::create_dir_all(tmp.path().join("node_modules/pkg")).unwrap();
        std::fs::write(tmp.path().join("node_modules/pkg/README.md"), b"junk").unwrap();
        std::fs::create_dir_all(tmp.path().join("web/node_modules/x")).unwrap();
        std::fs::write(tmp.path().join("web/node_modules/x/README.md"), b"junk").unwrap();
        std::fs::create_dir_all(tmp.path().join(".git")).unwrap();
        std::fs::write(tmp.path().join(".git/HEAD"), b"junk").unwrap();
        let filter = WalkFilter::new(["node_modules"]);
        let names: Vec<_> = walk_drive_filtered(tmp.path(), &filter)
            .map(|e| {
                e.path()
                    .strip_prefix(tmp.path())
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect();
        assert!(names.iter().any(|n| n == "notes/a.md"));
        assert!(!names.iter().any(|n| n.contains("node_modules")));
        assert!(!names.iter().any(|n| n.contains(".git")));
    }

    #[test]
    fn walk_filter_is_case_insensitive_on_dir_basename() {
        // Same dir name with different casing matches the filter.
        // Defensive: macOS' default volume is case-insensitive, so a
        // user-visible "Node_Modules" should not slip through.
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join("Node_Modules/pkg")).unwrap();
        std::fs::write(tmp.path().join("Node_Modules/pkg/README.md"), b"junk").unwrap();
        let filter = WalkFilter::new(["node_modules"]);
        let count = walk_drive_filtered(tmp.path(), &filter)
            .filter(|e| e.file_type().is_file())
            .count();
        assert_eq!(count, 0);
    }

    #[cfg(unix)]
    #[test]
    fn walk_drive_drops_symlinks_and_special_files() {
        use std::os::unix::fs::symlink;
        use std::os::unix::net::UnixListener;
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("note.md"), b"hi").unwrap();
        symlink("note.md", tmp.path().join("alias.md")).unwrap();
        let _l = UnixListener::bind(tmp.path().join("sock")).unwrap();
        let names: Vec<_> = walk_drive(tmp.path())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        assert!(names.contains(&"note.md".to_string()));
        assert!(!names.contains(&"alias.md".to_string()));
        assert!(!names.contains(&"sock".to_string()));
    }
}
