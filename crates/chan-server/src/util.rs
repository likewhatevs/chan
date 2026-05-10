//! Small route-shared helpers: filename slugs, opaque-JSON responses,
//! markdown heading sniffing.

use axum::http::header;
use axum::response::{IntoResponse, Response};

/// Wrap an opaque blob in an `application/json` response. We don't
/// re-parse + re-serialize because the blob may be large and we
/// trust whoever wrote it (Drive::put_*) handed back exactly what
/// they got. If the blob isn't JSON the client sees the raw bytes
/// with the wrong content-type, which is acceptable for opaque
/// storage that the frontend writes itself.
pub fn raw_json_response(bytes: Vec<u8>) -> Response {
    (
        [(header::CONTENT_TYPE, "application/json; charset=utf-8")],
        bytes,
    )
        .into_response()
}

/// Pull a level-1 heading from a single line. Returns `None` for any
/// line that isn't `# heading-text`. Leading whitespace is tolerated;
/// trailing `#` runs (`# title #`) are trimmed.
pub fn extract_h1(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    let stripped = trimmed.strip_prefix("# ")?;
    let s = stripped.trim().trim_end_matches('#').trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

/// Strip a string into a filesystem-safe slug. Keeps ASCII alnum,
/// '-', '_'; collapses everything else to '-'; trims leading and
/// trailing dashes; clamps to 80 chars (safe under chan-drive's
/// blob key length and most filesystems' name limits).
pub fn slugify_for_filename(s: &str) -> String {
    let mut out = String::with_capacity(s.len().min(80));
    let mut last_dash = true;
    for c in s.chars() {
        if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
            out.push(c);
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
        if out.len() >= 80 {
            break;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    while out.starts_with('-') {
        out.remove(0);
    }
    out
}

/// Fallback name when no header / explicit name was provided:
/// `answer-<unix-seconds>`. Uses the system clock; tests should
/// pass `name` to keep filenames deterministic.
pub fn timestamp_slug() -> String {
    format!("answer-{}", crate::signal::now_unix_secs())
}

/// Split `foo.bar.PNG` into (`"foo.bar"`, Some("PNG")). Bare
/// names with no `.` return (input, None). Hidden files like
/// `.gitignore` are treated as having no extension (`.gitignore`,
/// None) so we don't produce a garbage extension.
pub fn split_filename(name: &str) -> (&str, Option<&str>) {
    if name.starts_with('.') {
        return (name, None);
    }
    match name.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() && !ext.is_empty() => (stem, Some(ext)),
        _ => (name, None),
    }
}
