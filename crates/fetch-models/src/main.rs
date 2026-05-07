// Pre-fetches the default embedding model into
// `crates/chan-server/resources/models/` so chan-server's rust-embed
// can bundle it into the release binary. At first launch the server
// extracts the bundle into the per-machine cache (~/Library/Caches
// /chan/models on macOS) so users never block on a HuggingFace
// download.
//
// Run from the workspace root via `make models` or
// `cargo run -p fetch-models`. Idempotent: re-running with the
// model already present is a fast no-op.
//
// Proxy: fastembed's underlying HTTP client (hf-hub via ureq)
// reads `HTTPS_PROXY` / `HTTP_PROXY` from the environment, so a
// pre-set
//
//     HTTPS_PROXY=http://proxy.corp:3128 make models
//
// works without a code change. We log the active proxy on stderr
// when one is set so a network-restricted CI run shows it.

use std::path::PathBuf;

use anyhow::{Context, Result};
use chan_core::index::embeddings::Embedder;
use chan_core::DEFAULT_MODEL;

fn main() -> Result<()> {
    let resource_dir = resource_models_dir();
    std::fs::create_dir_all(&resource_dir)
        .with_context(|| format!("create {}", resource_dir.display()))?;

    if let Some((var, val)) = active_proxy() {
        eprintln!("fetch-models: using {var}={val}");
    }
    eprintln!(
        "fetch-models: seeding {DEFAULT_MODEL} into {}",
        resource_dir.display()
    );

    // Open the embedder pointing at the resource dir as cache.
    // fastembed downloads the model there if missing; if already
    // present from a prior run it skips the network and returns
    // instantly.
    Embedder::open(DEFAULT_MODEL, &resource_dir).context("download default embedding model")?;

    eprintln!("fetch-models: done");
    Ok(())
}

/// Resolve `<workspace-root>/crates/chan-server/resources/models`
/// from this crate's manifest dir. cargo sets `CARGO_MANIFEST_DIR`
/// to `<workspace>/crates/fetch-models`.
fn resource_models_dir() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir)
        .join("..")
        .join("chan-server")
        .join("resources")
        .join("models")
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
