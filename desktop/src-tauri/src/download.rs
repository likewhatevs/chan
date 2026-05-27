// Desktop-native "Download" save target.
//
// In the browser an `<a download>` click hands the bytes to the
// browser's own download manager, which shows progress + drops the
// file in the user's Downloads folder. chan-desktop's WKWebView /
// WebView2 has no such download manager UI, so a plain `<a download>`
// in the desktop shell silently does nothing useful. Bug 2 (round-1)
// asks for a native download indicator that mimics the browser.
//
// The split is: the SPA fetches the file over the same loopback
// connection it already uses (via XHR, so it gets download progress
// for the in-app indicator), then hands the finished bytes to this
// command, which writes them to the OS Downloads folder and returns
// the saved path so the SPA can show "Saved to <path>". Keeping the
// byte transfer in JS reuses the upload progress pattern and avoids a
// second loopback fetch from Rust; workspace content is notes-scale, so
// buffering the blob in memory is acceptable. (A "reveal in Finder"
// action is a future addition: reveal_in_finder is currently only in
// the launcher window's ACL, not workspace windows where the inspector
// lives.)

use std::{fs, path::PathBuf};

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct SavedDownload {
    /// Absolute path the file was written to (after de-duplicating
    /// against an existing name).
    pub path: String,
}

/// Write `bytes` to the user's Downloads folder under `filename`,
/// de-duplicating the name if it already exists, and return the saved
/// path. The SPA passes a filename it already sanitized
/// (`downloadFilename`), but we defensively strip path separators so a
/// crafted name can't escape the Downloads folder.
#[tauri::command]
pub fn save_file_to_downloads(filename: String, bytes: Vec<u8>) -> Result<SavedDownload, String> {
    let dir = downloads_dir().ok_or_else(|| "could not resolve a Downloads folder".to_string())?;
    fs::create_dir_all(&dir).map_err(|e| format!("creating {}: {e}", dir.display()))?;

    let safe = sanitize_filename(&filename);
    let target = unique_path(&dir, &safe);
    fs::write(&target, &bytes).map_err(|e| format!("writing {}: {e}", target.display()))?;

    Ok(SavedDownload {
        path: target.display().to_string(),
    })
}

/// The OS Downloads folder, falling back to the home directory when
/// the platform has no distinct Downloads dir.
fn downloads_dir() -> Option<PathBuf> {
    dirs::download_dir().or_else(dirs::home_dir)
}

/// Reduce `filename` to a single bare name with no path separators or
/// control characters, so a value coming from the webview can never
/// redirect the write outside the Downloads folder. Empty / dot-only
/// names fall back to a generic label.
fn sanitize_filename(filename: &str) -> String {
    let base = filename
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(filename)
        .trim();
    let cleaned: String = base
        .chars()
        .map(|c| if c.is_control() { '_' } else { c })
        .collect();
    if cleaned.is_empty() || cleaned == "." || cleaned == ".." {
        "download".to_string()
    } else {
        cleaned
    }
}

/// `dir/name`, or `dir/name (N).ext` for the smallest N >= 1 that does
/// not already exist. Mirrors the browser download manager's
/// "file (1).txt" de-duplication so a second download of the same file
/// never silently overwrites the first.
fn unique_path(dir: &std::path::Path, name: &str) -> PathBuf {
    let candidate = dir.join(name);
    if !candidate.exists() {
        return candidate;
    }
    let (stem, ext) = split_ext(name);
    for n in 1..=9999 {
        let alt = match &ext {
            Some(ext) => format!("{stem} ({n}).{ext}"),
            None => format!("{stem} ({n})"),
        };
        let candidate = dir.join(&alt);
        if !candidate.exists() {
            return candidate;
        }
    }
    // Pathological: 9999 collisions. Fall back to the bare name and let
    // the write overwrite rather than loop forever.
    dir.join(name)
}

/// Split `name` into (stem, Some(ext)) on the LAST dot, or (name, None)
/// when there is no usable extension. A leading-dot name like
/// `.gitignore` has no extension (stem == name) so de-duplication
/// produces `.gitignore (1)`, not ` (1).gitignore`.
fn split_ext(name: &str) -> (String, Option<String>) {
    match name.rfind('.') {
        Some(idx) if idx > 0 && idx < name.len() - 1 => {
            (name[..idx].to_string(), Some(name[idx + 1..].to_string()))
        }
        _ => (name.to_string(), None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_strips_path_separators() {
        assert_eq!(sanitize_filename("../../etc/passwd"), "passwd");
        assert_eq!(sanitize_filename("a/b/c.md"), "c.md");
        assert_eq!(sanitize_filename("plain.txt"), "plain.txt");
        assert_eq!(sanitize_filename(""), "download");
        assert_eq!(sanitize_filename("   "), "download");
        assert_eq!(sanitize_filename(".."), "download");
    }

    #[test]
    fn split_ext_handles_dotfiles_and_extensions() {
        assert_eq!(split_ext("a.md"), ("a".to_string(), Some("md".to_string())));
        assert_eq!(
            split_ext("archive.tar"),
            ("archive".to_string(), Some("tar".to_string()))
        );
        assert_eq!(split_ext("noext"), ("noext".to_string(), None));
        assert_eq!(split_ext(".gitignore"), (".gitignore".to_string(), None));
        assert_eq!(
            split_ext("trailingdot."),
            ("trailingdot.".to_string(), None)
        );
    }

    #[test]
    fn unique_path_dedupes_against_existing() {
        let tmp = std::env::temp_dir().join(format!("chan-dl-test-{}", std::process::id()));
        fs::create_dir_all(&tmp).unwrap();
        let first = unique_path(&tmp, "note.md");
        assert_eq!(first, tmp.join("note.md"));
        fs::write(&first, b"x").unwrap();
        let second = unique_path(&tmp, "note.md");
        assert_eq!(second, tmp.join("note (1).md"));
        fs::remove_dir_all(&tmp).ok();
    }
}
