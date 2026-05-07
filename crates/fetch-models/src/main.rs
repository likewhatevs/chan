// Pre-fetches the default embedding model and writes it as a
// single zstd-compressed tarball at
// `crates/chan-server/resources/models.tar.zst`. chan-server's
// release build calls `include_bytes!` on the tarball; the seeder
// at first server launch zstd-decodes + untars the blob into the
// per-machine cache (~/Library/Caches/chan/models on macOS) so
// users never block on a HuggingFace download.
//
// Two-stage:
//
//   1. Run fastembed against a stable staging dir under
//      `target/fetch-models-cache/` so re-runs are fast (HF cache
//      hit-or-skip). cargo-clean wipes it; that's intentional, the
//      next build re-downloads.
//   2. tar+zstd encode the staging dir into the embed bundle.
//      Drops `*.lock` and `**/blobs/**` along the way so the
//      bundle doesn't carry every model file twice (snapshots/
//      symlinks already follow into blobs at copy time).
//
// Run from the workspace root via `make models` or
// `cargo run -p fetch-models`. Idempotent: re-running with the
// model already cached AND the tarball up-to-date is a fast
// no-op. Honors `HTTPS_PROXY` / `HTTP_PROXY` for restricted
// networks; fastembed's underlying HTTP client picks them up.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chan_core::index::embeddings::Embedder;
use chan_core::DEFAULT_MODEL;

/// Compression level. zstd's range is 1..=22; 19 sits at the
/// "max ratio with reasonable encode time" sweet spot for blobs
/// of this size (one-shot encode, no realtime constraint).
/// Anything higher only buys ~1% smaller for >2x encode time.
const ZSTD_LEVEL: i32 = 19;

fn main() -> Result<()> {
    let staging = staging_dir();
    let bundle = bundle_path();

    if let Some(parent) = bundle.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    std::fs::create_dir_all(&staging).with_context(|| format!("create {}", staging.display()))?;

    if let Some((var, val)) = active_proxy() {
        eprintln!("fetch-models: using {var}={val}");
    }
    eprintln!(
        "fetch-models: seeding {DEFAULT_MODEL} into {}",
        staging.display()
    );

    // Open the embedder pointing at the staging dir. fastembed
    // downloads the model there if missing; if already present
    // from a prior run it skips the network and returns instantly.
    Embedder::open(DEFAULT_MODEL, &staging).context("download default embedding model")?;

    eprintln!("fetch-models: encoding bundle to {}", bundle.display());
    encode_tar_zst(&staging, &bundle)?;
    let size = std::fs::metadata(&bundle)
        .map(|m| m.len())
        .unwrap_or_default();
    eprintln!(
        "fetch-models: done ({} -> {})",
        staging.display(),
        humanize(size)
    );
    Ok(())
}

/// Stable staging dir for fastembed's HF cache. Lives under
/// `target/` so cargo-clean wipes it; survives normal builds so
/// re-runs of fetch-models hit the on-disk cache and skip the
/// network. Keep this OUT of `crates/chan-server/resources/` so
/// the only thing under that dir is the final bundle.
fn staging_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("target")
        .join("fetch-models-cache")
}

fn bundle_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("chan-server")
        .join("resources")
        .join("models.tar.zst")
}

/// Walk `src` and emit a zstd-compressed tar archive at `dst`.
/// Skips hf-hub's bookkeeping cruft (`*.lock`, `*.no_exists`) and
/// the `blobs/` subdir (snapshots/ symlinks already point to the
/// same bytes; tar follows symlinks by default, so blobs/ would
/// double the archive).
fn encode_tar_zst(src: &Path, dst: &Path) -> Result<()> {
    // Atomic write: encode to a sibling tmp file, fsync, rename.
    // Avoids leaving a half-written bundle on disk if the encode
    // fails partway through (compounding `cargo build` confusion).
    let tmp = dst.with_extension("zst.tmp");
    {
        let file =
            std::fs::File::create(&tmp).with_context(|| format!("create {}", tmp.display()))?;
        let zenc = zstd::Encoder::new(file, ZSTD_LEVEL)
            .context("init zstd encoder")?
            .auto_finish();
        let mut tarw = tar::Builder::new(zenc);
        tarw.follow_symlinks(true);
        // Walk the staging tree explicitly so we can filter
        // entries; `Builder::append_dir_all` would happily include
        // blobs/ and lock files.
        for entry in walk_files(src)? {
            let rel = entry
                .strip_prefix(src)
                .with_context(|| format!("strip {}", entry.display()))?;
            if should_skip(rel) {
                continue;
            }
            tarw.append_path_with_name(&entry, rel)
                .with_context(|| format!("append {}", entry.display()))?;
        }
        tarw.finish().context("finalize tar")?;
        // Drop the encoder (auto_finish flushes zstd footer).
    }
    std::fs::rename(&tmp, dst)
        .with_context(|| format!("rename {} -> {}", tmp.display(), dst.display()))?;
    Ok(())
}

/// Recursive file walk (no external dep). Only yields regular
/// files; tar's append-with-name handles paths that don't exist
/// at the root.
fn walk_files(root: &Path) -> Result<Vec<PathBuf>> {
    fn rec(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
        for entry in
            std::fs::read_dir(dir).with_context(|| format!("read_dir {}", dir.display()))?
        {
            let entry = entry?;
            let path = entry.path();
            let ft = entry
                .file_type()
                .with_context(|| format!("file_type {}", path.display()))?;
            if ft.is_dir() {
                rec(&path, out)?;
            } else if ft.is_file() || ft.is_symlink() {
                // Symlinks: tar's `follow_symlinks(true)` resolves
                // them at append time, so we list them here and
                // tar serializes the target's bytes.
                out.push(path);
            }
        }
        Ok(())
    }
    let mut out = Vec::new();
    rec(root, &mut out)?;
    out.sort();
    Ok(out)
}

/// Filenames hf-hub emits that we don't want in the bundle.
fn should_skip(rel: &Path) -> bool {
    let s = rel.to_string_lossy();
    if s.contains("/blobs/") || s.starts_with("blobs/") {
        return true;
    }
    if s.ends_with(".lock") || s.ends_with(".no_exists") {
        return true;
    }
    false
}

fn humanize(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    let b = bytes as f64;
    if b >= MB {
        format!("{:.1} MB", b / MB)
    } else if b >= KB {
        format!("{:.1} KB", b / KB)
    } else {
        format!("{bytes} B")
    }
}

/// Report which (if any) HTTP proxy env var is in effect, with
/// HTTPS_PROXY taking precedence (fastembed uses HTTPS to hit the
/// HuggingFace CDN).
fn active_proxy() -> Option<(&'static str, String)> {
    for var in ["HTTPS_PROXY", "https_proxy", "HTTP_PROXY", "http_proxy"] {
        if let Ok(v) = std::env::var(var) {
            if !v.is_empty() {
                return Some((var, v));
            }
        }
    }
    None
}
