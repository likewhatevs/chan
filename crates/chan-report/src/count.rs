// Per-file counting via `tokei`.
//
// Detects language (extension + shebang), counts code / comments
// / blanks, computes a complexity score, and reads basic metadata
// (bytes, mtime). Pure function: takes a root + relative path,
// returns the resulting row or `None` when the file is
// unrecognized, oversize, vanished, or fails to parse.

use std::fs;
use std::io::Read;
use std::path::Path;

use chrono::{DateTime, Utc};
use tokei::{Config, LanguageType};

use crate::complexity;
use crate::error::ChanReportError;
use crate::summary::{FileBucket, FileStats};

/// Classify a tokei-recognized language into the
/// source-code-shaped bucket axis. Markdown is the only special
/// case (the graph colour scheme distinguishes notes from
/// source); everything else `tokei` recognizes falls under
/// `SourceCode { language: <tokei name> }`.
///
/// Binary / Media / Other don't appear here because chan-report
/// doesn't track those file kinds; the graph indexer composes
/// chan-report's bucket with `chan_workspace::classify()` (the
/// IO-contract axis) for those.
fn classify_bucket(language: LanguageType) -> FileBucket {
    match language {
        LanguageType::Markdown => FileBucket::Markdown,
        other => FileBucket::SourceCode {
            language: other.name().to_string(),
        },
    }
}

/// Files larger than this skip the in-memory read (and therefore
/// complexity scoring). tokei still counts them via its streaming
/// path. Tuned so a 16 MiB markdown file doesn't blow up RSS on
/// the watcher thread.
const READ_CAP: u64 = 16 * 1024 * 1024;

pub(crate) fn count_file_impl(
    root: &Path,
    rel: &str,
) -> Result<Option<FileStats>, ChanReportError> {
    let abs = root.join(rel);

    let meta = match fs::symlink_metadata(&abs) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(ChanReportError::Io(e.to_string())),
    };
    if !meta.is_file() {
        // Symlinks, sockets, devices: not interesting.
        return Ok(None);
    }
    let bytes = meta.len();

    let cfg = Config::default();
    let language = match LanguageType::from_path(&abs, &cfg) {
        Some(l) => l,
        None => return Ok(None),
    };

    // Two paths: small files we read into memory once and feed to
    // both the counter and the complexity scorer. Large files
    // stream through tokei and skip complexity (heuristic anyway,
    // not worth the second pass on multi-MB files).
    let (code, comments, blanks, complexity_score) = if bytes <= READ_CAP {
        match read_text(&abs) {
            Some(content) => {
                let stats = language.parse_from_str(content.clone(), &cfg);
                let cx = complexity::score(language.name(), &content);
                (
                    stats.code as u64,
                    stats.comments as u64,
                    stats.blanks as u64,
                    cx,
                )
            }
            None => {
                // Non-UTF8 or read error; fall back to tokei's
                // path-based parse so binary files still get
                // counted (well, classified as binary and skipped).
                match language.parse(abs.clone(), &cfg) {
                    Ok(r) => (
                        r.stats.code as u64,
                        r.stats.comments as u64,
                        r.stats.blanks as u64,
                        0,
                    ),
                    Err(_) => return Ok(None),
                }
            }
        }
    } else {
        match language.parse(abs.clone(), &cfg) {
            Ok(r) => (
                r.stats.code as u64,
                r.stats.comments as u64,
                r.stats.blanks as u64,
                0,
            ),
            Err(_) => return Ok(None),
        }
    };

    let mtime = meta
        .modified()
        .ok()
        .map(|t| DateTime::<Utc>::from(t).to_rfc3339());

    Ok(Some(FileStats {
        path: rel.to_string(),
        language: language.name().to_string(),
        code,
        comments,
        blanks,
        complexity: complexity_score,
        bytes,
        mtime,
        bucket: Some(classify_bucket(language)),
    }))
}

/// Best-effort UTF-8 read. Returns `None` for non-UTF8 content,
/// I/O errors, or anything unexpected. The caller falls back to
/// tokei's streaming path-based parse.
fn read_text(abs: &Path) -> Option<String> {
    let mut f = fs::File::open(abs).ok()?;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf).ok()?;
    String::from_utf8(buf).ok()
}
