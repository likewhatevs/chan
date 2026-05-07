// Thin wrapper around fastembed-rs. Loads ONNX models on demand
// (cached per-machine under `dirs::cache_dir()/chan/models/`) and
// exposes a small `Embedder` interface used by the vector side of
// the index.
//
// The cache is per-machine (not per-drive) because models are
// immutable (~130 MB for the default BGE-small) and identical
// across every drive on a machine.
//
// Models we explicitly map are listed in `model_for`. Unknown model
// ids are rejected at open time so the user gets an actionable
// error instead of a panic deep inside fastembed.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EmbedError {
    #[error("unknown embedding model: {0}")]
    UnknownModel(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("fastembed: {0}")]
    Fastembed(String),
}

/// Stringify anything that fastembed returns (it surfaces
/// anyhow::Error). Avoids pulling anyhow as a chan-core dep just
/// for this one boundary.
fn stringify<E: std::fmt::Display>(e: E) -> EmbedError {
    EmbedError::Fastembed(e.to_string())
}

/// Loaded fastembed model. Holds the ONNX session behind a Mutex
/// because TextEmbedding::embed takes `&mut self` while the rest
/// of the codebase wants `&Embedder` to satisfy `&self` callers.
pub struct Embedder {
    model_id: String,
    inner: Mutex<TextEmbedding>,
    dim: usize,
}

impl std::fmt::Debug for Embedder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Embedder")
            .field("model_id", &self.model_id)
            .field("dim", &self.dim)
            .finish()
    }
}

impl Embedder {
    /// Open `model_id` (e.g. `BAAI/bge-small-en-v1.5`). Downloads
    /// the model into `cache_dir` on first use; subsequent opens
    /// are fast.
    pub fn open(model_id: &str, cache_dir: &Path) -> Result<Self, EmbedError> {
        let kind = model_for(model_id)?;
        std::fs::create_dir_all(cache_dir)?;
        let opts = InitOptions::new(kind.clone())
            .with_cache_dir(cache_dir.to_path_buf())
            .with_show_download_progress(true);
        let inner = TextEmbedding::try_new(opts).map_err(stringify)?;
        let dim = embedding_dim(&kind);
        Ok(Self {
            model_id: model_id.to_owned(),
            inner: Mutex::new(inner),
            dim,
        })
    }

    pub fn model_id(&self) -> &str {
        &self.model_id
    }

    /// Embedding dimension. Useful so callers can sanity-check
    /// stored vectors before searching.
    pub fn dim(&self) -> usize {
        self.dim
    }

    /// Embed a batch of documents. fastembed handles its own
    /// batching internally; pass the whole batch.
    pub fn embed_documents<S: AsRef<str> + Send + Sync>(
        &self,
        docs: &[S],
    ) -> Result<Vec<Vec<f32>>, EmbedError> {
        if docs.is_empty() {
            return Ok(Vec::new());
        }
        let owned: Vec<String> = docs.iter().map(|s| s.as_ref().to_owned()).collect();
        let mut guard = self.inner.lock().unwrap();
        guard.embed(owned, None).map_err(stringify)
    }

    /// Single-query embedding. Same path as documents (BGE family
    /// doesn't need a different prefix for queries).
    pub fn embed_query(&self, q: &str) -> Result<Vec<f32>, EmbedError> {
        let mut v = self.embed_documents(&[q.to_owned()])?;
        Ok(v.pop().unwrap_or_default())
    }
}

/// Map a HuggingFace-style model id to fastembed's enum. Add
/// entries here as you want to support more models without touching
/// the rest of the code.
fn model_for(id: &str) -> Result<EmbeddingModel, EmbedError> {
    match id {
        "BAAI/bge-small-en-v1.5" => Ok(EmbeddingModel::BGESmallENV15),
        "BAAI/bge-base-en-v1.5" => Ok(EmbeddingModel::BGEBaseENV15),
        "BAAI/bge-large-en-v1.5" => Ok(EmbeddingModel::BGELargeENV15),
        // BGE-M3 is multilingual; useful when notes mix languages.
        "BAAI/bge-m3" | "Xenova/bge-m3" => Ok(EmbeddingModel::BGEM3),
        _ => Err(EmbedError::UnknownModel(id.to_owned())),
    }
}

/// Hard-coded dimensions for the models we accept. Centralizing
/// this lets us validate stored vectors without instantiating the
/// model.
fn embedding_dim(m: &EmbeddingModel) -> usize {
    match m {
        EmbeddingModel::BGESmallENV15 => 384,
        EmbeddingModel::BGEBaseENV15 => 768,
        EmbeddingModel::BGELargeENV15 => 1024,
        EmbeddingModel::BGEM3 => 1024,
        // Conservative fallback: 384 (smallest seen). Adding new
        // models above is cheap and avoids this branch.
        _ => 384,
    }
}

/// Per-machine model cache. macOS: `~/Library/Caches/chan/models`;
/// Linux: `$XDG_CACHE_HOME/chan/models`; Windows:
/// `%LOCALAPPDATA%/chan/models`. Falls back to the system temp dir
/// if `dirs::cache_dir()` is unavailable; fastembed will then
/// re-download into the temp dir on each launch but search still
/// works.
pub fn global_models_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("chan")
        .join("models")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_model_is_error() {
        let err = model_for("not-a-model").unwrap_err();
        assert!(matches!(err, EmbedError::UnknownModel(_)));
    }

    #[test]
    fn known_models_resolve() {
        assert!(model_for("BAAI/bge-small-en-v1.5").is_ok());
        assert!(model_for("BAAI/bge-m3").is_ok());
    }
}
