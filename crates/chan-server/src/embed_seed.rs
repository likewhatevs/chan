//! Seed the per-machine embedding-model cache from a baked-in tarball.
//!
//! The release build embeds a zstd-encoded tar of the hf-hub cache layout
//! at `resources/models.tar.zst` (produced by the `fetch-models` helper).
//! On first server launch we extract it into the global model cache so
//! users never block on a HuggingFace download. Plain `cargo build`
//! ships an empty stub: the seeder treats that as "no embedded model"
//! and falls back to hf-hub's network path.
//!
//! Whole module gated on `embeddings`; without the feature, neither the
//! bytes nor the decoder land in the binary.

#![cfg(feature = "embeddings")]

use std::path::Path;

/// Compressed copy of the default embedding model, baked at build
/// time. Empty in dev builds (the build.rs stub); real release
/// builds run `make models` first which writes the actual ~80 MB
/// tarball before linking.
static MODEL_BUNDLE: &[u8] = include_bytes!("../resources/models.tar.zst");

/// Extract the compressed default-model bundle into the per-machine
/// model cache (resolved by chan-drive's `global_models_dir`) the
/// first time the server boots on this machine. Skipped on every
/// subsequent boot: the presence of any file under the target dir
/// is taken as "already seeded".
///
/// Errors are logged but do not block startup: if the seed fails
/// the runtime path falls back to hf-hub's HuggingFace download,
/// the same UX as a dev build.
pub fn seed_models_from_bundle() {
    if MODEL_BUNDLE.is_empty() {
        // Dev build (or --no-default-features upstream of release):
        // nothing to extract. Runtime download path applies.
        return;
    }
    let target = chan_drive::index::embeddings::global_models_dir();
    if bundle_already_seeded(&target) {
        return;
    }
    if let Err(e) = std::fs::create_dir_all(&target) {
        tracing::warn!("seed-models: create {}: {e}", target.display());
        return;
    }
    match extract_bundle(&target) {
        Ok(count) => tracing::info!(
            "seed-models: extracted {count} files into {}",
            target.display()
        ),
        Err(e) => tracing::warn!("seed-models: extract failed: {e}"),
    }
}

/// zstd-decode + untar `MODEL_BUNDLE` into `target`. Returns the
/// number of files written. Reports any error verbatim; the caller
/// downgrades it to a warning so a corrupt bundle never blocks
/// server start.
fn extract_bundle(target: &Path) -> std::io::Result<usize> {
    let zr = zstd::Decoder::new(MODEL_BUNDLE)?;
    let mut tar = tar::Archive::new(zr);
    // Default unpack semantics are correct for the hf-hub layout:
    // creates intermediate dirs, preserves file mode, doesn't
    // follow symlinks (tar's encode side already resolved them
    // into regular files when fetch-models built the bundle).
    let mut count = 0usize;
    for entry in tar.entries()? {
        let mut entry = entry?;
        // Sanity: refuse path traversal. tar::Archive::unpack does
        // this implicitly; we replicate by skipping entries whose
        // post-cleanup path would escape `target`.
        let path = entry.path()?.into_owned();
        if path
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            tracing::warn!("seed-models: skipping suspicious entry {path:?}");
            continue;
        }
        entry.unpack_in(target)?;
        count += 1;
    }
    Ok(count)
}

/// Treat the cache as seeded if any file is already present
/// underneath the target dir. Checks recursively because hf-hub's
/// layout is `<target>/models--<org>--<name>/{blobs,snapshots}/...`;
/// a top-level read_dir would find the wrapping directory and miss
/// a partial seed. A single regular file is enough proof.
fn bundle_already_seeded(target: &Path) -> bool {
    fn any_file(dir: &Path) -> bool {
        let it = match std::fs::read_dir(dir) {
            Ok(it) => it,
            Err(_) => return false,
        };
        for entry in it.flatten() {
            let ft = match entry.file_type() {
                Ok(ft) => ft,
                Err(_) => continue,
            };
            if ft.is_file() {
                return true;
            }
            if ft.is_dir() && any_file(&entry.path()) {
                return true;
            }
        }
        false
    }
    target.exists() && any_file(target)
}
