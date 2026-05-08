// High-level entry point for both the CLI (`chan index`, `chan
// search`) and the in-process server use cases. Composes BM25,
// embeddings, and the vector store. Hybrid retrieval (RRF fusion)
// is gated by the `embeddings` feature; without it the facade
// answers Hybrid / Semantic queries with `ready: false` and a
// BM25-only fallback so the UI can show a "BM25-only on this
// build" hint instead of erroring out.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(feature = "embeddings")]
use std::sync::OnceLock;

use serde::Serialize;
use thiserror::Error;

use super::bm25::{Bm25Error, Bm25Index};
use super::chunking;
use super::config::{self, ConfigError, IndexConfig};
#[cfg(feature = "embeddings")]
use super::embeddings::{self, EmbedError, Embedder};
#[cfg(feature = "embeddings")]
use super::fusion;
#[cfg(feature = "embeddings")]
use super::vectors;
use super::vectors::{VectorError, VectorStore};
use crate::error::ChanError;
use crate::fs_ops;

#[derive(Debug, Error)]
pub enum IndexError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error(transparent)]
    Bm25(#[from] Bm25Error),
    #[cfg(feature = "embeddings")]
    #[error(transparent)]
    Embed(#[from] EmbedError),
    #[error(transparent)]
    Vector(#[from] VectorError),
    #[error(transparent)]
    Chan(#[from] ChanError),
    #[error("operation cancelled")]
    Cancelled,
}

/// Which retrieval mode to run.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Bm25,
    Semantic,
    Hybrid,
}

impl Mode {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "bm25" => Some(Mode::Bm25),
            "semantic" => Some(Mode::Semantic),
            "hybrid" => Some(Mode::Hybrid),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Mode::Bm25 => "bm25",
            Mode::Semantic => "semantic",
            Mode::Hybrid => "hybrid",
        }
    }
}

/// Unified search hit. Both BM25 and semantic results are converted
/// to this shape before being returned to the CLI / API.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Hit {
    pub path: String,
    pub chunk_id: String,
    pub heading: String,
    pub start_line: u64,
    pub snippet: String,
    pub score: f32,
}

impl From<super::bm25::Hit> for Hit {
    fn from(h: super::bm25::Hit) -> Self {
        Self {
            path: h.path,
            chunk_id: h.chunk_id,
            heading: h.heading,
            start_line: h.start_line,
            snippet: h.snippet,
            score: h.score,
        }
    }
}

impl From<super::vectors::Hit> for Hit {
    fn from(h: super::vectors::Hit) -> Self {
        Self {
            path: h.path,
            chunk_id: h.chunk_id,
            heading: h.heading,
            start_line: h.start_line,
            snippet: h.snippet,
            score: h.score,
        }
    }
}

/// Search-result envelope used by both the CLI and the API.
#[derive(Debug, Clone, Serialize)]
pub struct SearchResult {
    pub ready: bool,
    pub mode: &'static str,
    pub hits: Vec<Hit>,
}

/// The big handle. One per drive per process.
pub struct Index {
    drive_root: PathBuf,
    index_dir: PathBuf,
    config: IndexConfig,
    bm25: Bm25Index,
    vectors: VectorStore,
    /// Lazily loaded: opening fastembed touches a large model file
    /// and we don't want `chan search --mode bm25` to pay that cost.
    #[cfg(feature = "embeddings")]
    embedder: OnceLock<Embedder>,
}

impl std::fmt::Debug for Index {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut d = f.debug_struct("Index");
        d.field("drive_root", &self.drive_root)
            .field("index_dir", &self.index_dir)
            .field("model", &self.config.model);
        #[cfg(feature = "embeddings")]
        d.field("embedder_loaded", &self.embedder.get().is_some());
        d.finish()
    }
}

impl Index {
    /// Open (or create) the index for `drive_root`, with storage
    /// rooted at `index_dir`. The two directories are decoupled:
    /// `drive_root` is where the markdown lives (read-only for
    /// the indexer), `index_dir` is where tantivy + vectors live
    /// (per-drive, in the global cache; resolved by
    /// `crate::paths::drive_paths`). Tests pass a tempdir for both.
    pub fn open(drive_root: &Path, index_dir: &Path) -> Result<Self, IndexError> {
        std::fs::create_dir_all(index_dir)?;
        let mut config = config::load(index_dir)?;
        if !config::config_path(index_dir).exists() {
            config::save(index_dir, &config)?;
        }
        // Schema-version drift forces a clean rebuild. We do this by
        // wiping the on-disk dir before opening tantivy; the config
        // itself is rewritten with the current SCHEMA_VERSION.
        if config.schema_version != config::SCHEMA_VERSION {
            wipe_index_dir(index_dir)?;
            config.schema_version = config::SCHEMA_VERSION;
            config::save(index_dir, &config)?;
        }
        let bm25 = Bm25Index::open(index_dir)?;
        let vectors = VectorStore::open(index_dir)?;
        Ok(Self {
            drive_root: drive_root.to_path_buf(),
            index_dir: index_dir.to_path_buf(),
            config,
            bm25,
            vectors,
            #[cfg(feature = "embeddings")]
            embedder: OnceLock::new(),
        })
    }

    /// Re-open after wiping `index_dir`. Intended for `--rebuild`.
    pub fn rebuild(drive_root: &Path, index_dir: &Path) -> Result<Self, IndexError> {
        wipe_index_dir(index_dir)?;
        Self::open(drive_root, index_dir)
    }

    pub fn config(&self) -> &IndexConfig {
        &self.config
    }

    /// Persist a (possibly mutated) config. Used by the CLI when
    /// the user passes `--model X`. Switching model invalidates the
    /// existing vectors (different dim / different semantics) so
    /// we wipe the vector dir; BM25 is unaffected.
    pub fn set_model(&mut self, model: String) -> Result<(), IndexError> {
        if model == self.config.model {
            return Ok(());
        }
        self.config.model = model;
        config::save(&self.index_dir, &self.config)?;
        #[cfg(feature = "embeddings")]
        {
            self.embedder = OnceLock::new();
        }
        let vec_dir = self.index_dir.join("embeddings");
        if vec_dir.exists() {
            std::fs::remove_dir_all(&vec_dir)?;
        }
        self.vectors = VectorStore::open(&self.index_dir)?;
        Ok(())
    }

    /// Get-or-init the embedder. Errors propagate (e.g. unknown
    /// model id, model download failure, ONNX runtime missing).
    #[cfg(feature = "embeddings")]
    fn embedder(&self) -> Result<&Embedder, IndexError> {
        if let Some(e) = self.embedder.get() {
            return Ok(e);
        }
        let cache_dir = embeddings::global_models_dir();
        let e = Embedder::open(&self.config.model, &cache_dir)?;
        let _ = self.embedder.set(e);
        Ok(self.embedder.get().unwrap())
    }

    /// Walk the drive and re-index everything from scratch. If
    /// `cancel` is set to true mid-build, returns `Cancelled` without
    /// calling `commit()` so tantivy discards every pending write
    /// queued in this run; the on-disk index is left as it was at
    /// the start.
    pub fn build_all<F>(
        &self,
        opts: BuildOptions,
        mut on_progress: F,
        cancel: Option<&AtomicBool>,
    ) -> Result<BuildSummary, IndexError>
    where
        F: FnMut(BuildProgress<'_>),
    {
        let files = list_markdown(&self.drive_root)?;
        let total = files.len();
        let mut indexed = 0usize;
        let mut chunks_total = 0usize;
        let mut errors: Vec<(String, IndexError)> = Vec::new();

        // Embedding throughput is dominated by ONNX session +
        // intra-batch parallelism overhead. Per-file embed calls on
        // a drive of small markdown files (typical: ~30 chunks per
        // file) leave that overhead unamortized and run an order of
        // magnitude slower than the hardware can do. Accumulate
        // chunks across files and flush in `EMBED_BATCH_CHUNKS`-sized
        // groups so each embed call gets enough work for fastembed's
        // internal batcher.
        #[cfg(feature = "embeddings")]
        let do_vectors = opts.include_vectors;
        #[cfg(not(feature = "embeddings"))]
        let _ = opts.include_vectors;
        #[cfg(feature = "embeddings")]
        let mut pending: Vec<(String, Vec<chunking::Chunk>)> = Vec::new();
        #[cfg(feature = "embeddings")]
        let mut pending_chunks: usize = 0;

        for (i, rel) in files.iter().enumerate() {
            if let Some(c) = cancel {
                if c.load(Ordering::Relaxed) {
                    return Err(IndexError::Cancelled);
                }
            }
            on_progress(BuildProgress {
                index: i,
                total,
                path: rel,
                stage: BuildStage::File,
            });
            let abs = self.drive_root.join(rel);
            let text = match std::fs::read_to_string(&abs) {
                Ok(s) => s,
                Err(e) => {
                    errors.push((rel.clone(), e.into()));
                    continue;
                }
            };
            let chunks = chunking::chunk(&text, &self.config.chunking);
            if let Err(e) = self.bm25.index_file(rel, &text, &self.config.chunking) {
                errors.push((rel.clone(), e.into()));
                continue;
            }
            indexed += 1;
            chunks_total += chunks.len();

            #[cfg(feature = "embeddings")]
            if do_vectors {
                if chunks.is_empty() {
                    // Stale-vector cleanup for files that became
                    // empty since the last build. `replace_file`
                    // with an empty vec deletes the on-disk shard.
                    if let Err(e) = self
                        .vectors
                        .replace_file(rel, &self.config.model, 0, vec![])
                    {
                        errors.push((rel.clone(), e.into()));
                    }
                    continue;
                }
                pending_chunks += chunks.len();
                pending.push((rel.clone(), chunks));
                if pending_chunks >= EMBED_BATCH_CHUNKS {
                    on_progress(BuildProgress {
                        index: i,
                        total,
                        path: rel,
                        stage: BuildStage::EmbedBatch {
                            chunks: pending_chunks,
                            files: pending.len(),
                        },
                    });
                    errors.extend(self.flush_embed_batch(&mut pending));
                    pending_chunks = 0;
                }
            }
        }

        // Tail flush for the leftover < EMBED_BATCH_CHUNKS group.
        #[cfg(feature = "embeddings")]
        if do_vectors && !pending.is_empty() {
            if let Some(c) = cancel {
                if c.load(Ordering::Relaxed) {
                    return Err(IndexError::Cancelled);
                }
            }
            let last = pending.last().map(|(r, _)| r.clone()).unwrap_or_default();
            on_progress(BuildProgress {
                index: total.saturating_sub(1),
                total,
                path: &last,
                stage: BuildStage::EmbedBatch {
                    chunks: pending_chunks,
                    files: pending.len(),
                },
            });
            errors.extend(self.flush_embed_batch(&mut pending));
        }

        if let Some(c) = cancel {
            if c.load(Ordering::Relaxed) {
                return Err(IndexError::Cancelled);
            }
        }
        self.bm25.commit()?;
        Ok(BuildSummary {
            files: total,
            indexed,
            chunks: chunks_total,
            errors,
        })
    }

    /// Embed every pending chunk in one call, then split the result
    /// back per file and write each file's vectors. Drains `pending`.
    /// On batch failure, falls back to per-file embedding so errors
    /// can be attributed to the offending file rather than poisoning
    /// the whole batch.
    #[cfg(feature = "embeddings")]
    fn flush_embed_batch(
        &self,
        pending: &mut Vec<(String, Vec<chunking::Chunk>)>,
    ) -> Vec<(String, IndexError)> {
        let mut errors = Vec::new();
        if pending.is_empty() {
            return errors;
        }
        let embedder = match self.embedder() {
            Ok(e) => e,
            Err(e) => {
                let msg = e.to_string();
                for (rel, _) in pending.drain(..) {
                    errors.push((rel, IndexError::Embed(EmbedError::Fastembed(msg.clone()))));
                }
                return errors;
            }
        };
        let dim = embedder.dim();
        let bodies: Vec<&str> = pending
            .iter()
            .flat_map(|(_, chunks)| chunks.iter().map(|c| c.body.as_str()))
            .collect();
        let raw = match embedder.embed_documents(&bodies) {
            Ok(v) => v,
            Err(_) => {
                // Per-file fallback so a single bad file doesn't
                // discard the rest of the batch's vectors.
                for (rel, chunks) in pending.drain(..) {
                    if let Err(e) = self.embed_one_file(&rel, &chunks, dim) {
                        errors.push((rel, e));
                    }
                }
                return errors;
            }
        };
        let mut cursor = 0usize;
        for (rel, chunks) in pending.drain(..) {
            let n = chunks.len();
            let slice = raw[cursor..cursor + n].to_vec();
            cursor += n;
            let embedded = vectors::pair(&chunks, slice);
            if let Err(e) = self
                .vectors
                .replace_file(&rel, &self.config.model, dim, embedded)
            {
                errors.push((rel, e.into()));
            }
        }
        errors
    }

    #[cfg(feature = "embeddings")]
    fn embed_one_file(
        &self,
        rel: &str,
        chunks: &[chunking::Chunk],
        dim: usize,
    ) -> Result<(), IndexError> {
        let bodies: Vec<&str> = chunks.iter().map(|c| c.body.as_str()).collect();
        let embedder = self.embedder()?;
        let raw = embedder.embed_documents(&bodies)?;
        let embedded = vectors::pair(chunks, raw);
        self.vectors
            .replace_file(rel, &self.config.model, dim, embedded)?;
        Ok(())
    }

    /// One-file write path used by both `build_all` and `index_one`.
    /// Chunks once, hands the same chunks to BM25 and (optionally)
    /// to the vector store. Caller commits BM25.
    fn write_file(
        &self,
        rel_path: &str,
        text: &str,
        include_vectors: bool,
    ) -> Result<usize, IndexError> {
        let chunks = chunking::chunk(text, &self.config.chunking);
        self.bm25
            .index_file(rel_path, text, &self.config.chunking)?;
        // include_vectors is the caller's intent. When the binary
        // is built without `embeddings`, we never produce vectors
        // regardless. BM25-only is a working subset.
        #[cfg(not(feature = "embeddings"))]
        let _ = include_vectors;
        #[cfg(feature = "embeddings")]
        {
            if !include_vectors {
                return Ok(chunks.len());
            }
            if chunks.is_empty() {
                self.vectors
                    .replace_file(rel_path, &self.config.model, 0, vec![])?;
                return Ok(0);
            }
            let embedder = self.embedder()?;
            let dim = embedder.dim();
            let bodies: Vec<&str> = chunks.iter().map(|c| c.body.as_str()).collect();
            let vectors_raw = embedder.embed_documents(&bodies)?;
            let embedded = vectors::pair(&chunks, vectors_raw);
            self.vectors
                .replace_file(rel_path, &self.config.model, dim, embedded)?;
        }
        Ok(chunks.len())
    }

    /// Re-index a single file (incremental). Used by the watcher
    /// hook. Always writes both indexes; if you need bm25-only at
    /// watcher time, gate at the caller.
    pub fn index_one(&self, rel_path: &str) -> Result<usize, IndexError> {
        let abs = self.drive_root.join(rel_path);
        if !abs.is_file() {
            return self.forget(rel_path).map(|_| 0);
        }
        let text = std::fs::read_to_string(&abs)?;
        let n = self.write_file(rel_path, &text, true)?;
        self.bm25.commit()?;
        Ok(n)
    }

    /// Drop a file from both indexes (e.g. after the file is
    /// removed on disk).
    pub fn forget(&self, rel_path: &str) -> Result<(), IndexError> {
        self.bm25.delete_file(rel_path)?;
        self.bm25.commit()?;
        self.vectors.delete_file(rel_path)?;
        Ok(())
    }

    /// Run a query.
    pub fn search(
        &self,
        query: &str,
        mode: Mode,
        limit: usize,
    ) -> Result<SearchResult, IndexError> {
        match mode {
            Mode::Bm25 => Ok(SearchResult {
                ready: true,
                mode: mode.label(),
                hits: self
                    .bm25
                    .search(query, limit)?
                    .into_iter()
                    .map(Into::into)
                    .collect(),
            }),
            #[cfg(feature = "embeddings")]
            Mode::Semantic => {
                let qv = self.embedder()?.embed_query(query)?;
                let hits = self
                    .vectors
                    .search(&qv, limit)
                    .into_iter()
                    .map(Into::into)
                    .collect();
                Ok(SearchResult {
                    ready: true,
                    mode: mode.label(),
                    hits,
                })
            }
            #[cfg(feature = "embeddings")]
            Mode::Hybrid => {
                // Over-fetch each side so RRF has material to fuse.
                // 2x the user-requested limit, with a floor of 20.
                let buffer = (limit * 2).max(20);
                let bm25_hits: Vec<Hit> = self
                    .bm25
                    .search(query, buffer)?
                    .into_iter()
                    .map(Into::into)
                    .collect();
                let qv = self.embedder()?.embed_query(query)?;
                let sem_hits: Vec<Hit> = self
                    .vectors
                    .search(&qv, buffer)
                    .into_iter()
                    .map(Into::into)
                    .collect();
                let fused = fusion::rrf(&[bm25_hits, sem_hits], limit);
                Ok(SearchResult {
                    ready: true,
                    mode: mode.label(),
                    hits: fused,
                })
            }
            // Without `embeddings`, semantic and hybrid collapse
            // to BM25 with `ready: false` so the UI can show a
            // "search index is text-only on this build" hint.
            #[cfg(not(feature = "embeddings"))]
            Mode::Semantic | Mode::Hybrid => Ok(SearchResult {
                ready: false,
                mode: mode.label(),
                hits: self
                    .bm25
                    .search(query, limit)?
                    .into_iter()
                    .map(Into::into)
                    .collect(),
            }),
        }
    }

    /// Stats for the API status endpoint.
    pub fn stats(&self) -> IndexStats {
        IndexStats {
            ready: true,
            indexed_docs: self.bm25.doc_count(),
            indexed_vectors: self.vectors.chunk_count() as u64,
            model: self.config.model.clone(),
        }
    }
}

// Cross-file embedding batch size, in chunks. Tuned for fastembed
// + bge-small on CPU: large enough that ONNX session overhead is
// amortized over a useful work unit, small enough that working
// memory stays modest (~12 MB at 384-dim) on big drives. Only used
// when the `embeddings` feature is on; harmless otherwise.
#[cfg(feature = "embeddings")]
const EMBED_BATCH_CHUNKS: usize = 4096;

/// Knobs for `Index::build_all`.
#[derive(Debug, Clone, Copy)]
pub struct BuildOptions {
    /// When `false`, skip embeddings (`chan index --mode bm25` and
    /// unit tests). Default: `true`.
    pub include_vectors: bool,
}

impl Default for BuildOptions {
    fn default() -> Self {
        Self {
            include_vectors: true,
        }
    }
}

#[derive(Debug)]
pub struct BuildProgress<'a> {
    pub index: usize,
    pub total: usize,
    pub path: &'a str,
    pub stage: BuildStage,
}

/// Which step of `build_all` the progress callback is reporting.
/// `File` fires per file before the read+chunk+BM25 step. `EmbedBatch`
/// fires once per cross-file embedding flush, which can be the
/// long-running pause on a CPU-only embedder; without surfacing it
/// the CLI's progress line would look stuck on whatever file
/// happened to push the buffer past the batch threshold.
#[derive(Debug, Clone, Copy)]
pub enum BuildStage {
    File,
    EmbedBatch { chunks: usize, files: usize },
}

#[derive(Debug)]
pub struct BuildSummary {
    pub files: usize,
    pub indexed: usize,
    pub chunks: usize,
    pub errors: Vec<(String, IndexError)>,
}

#[derive(Debug, Clone, Serialize)]
pub struct IndexStats {
    pub ready: bool,
    /// Number of BM25-indexed chunks.
    pub indexed_docs: u64,
    /// Number of chunks with embeddings on disk. May lag
    /// indexed_docs briefly during a partial build, or be 0 if no
    /// embedder has run yet for this drive.
    pub indexed_vectors: u64,
    pub model: String,
}

fn wipe_index_dir(index_dir: &Path) -> Result<(), IndexError> {
    // Model weights live in the per-machine cache (see
    // `embeddings::global_models_dir`), so a per-drive wipe never
    // touches them. We only nuke the indexable state: `bm25/`,
    // `embeddings/`, and the config (recreated on next open).
    for sub in ["bm25", "embeddings"] {
        let p = index_dir.join(sub);
        if p.exists() {
            std::fs::remove_dir_all(&p)?;
        }
    }
    let cfg = index_dir.join("config.toml");
    if cfg.exists() {
        std::fs::remove_file(&cfg)?;
    }
    Ok(())
}

/// Walk the drive and return every `.md` file relative to root,
/// using forward-slash separators on all platforms (matches the
/// API's shape).
fn list_markdown(root: &Path) -> Result<Vec<String>, IndexError> {
    let mut out: Vec<String> = fs_ops::walk_drive(root)
        .filter(|e| e.file_type().is_file())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("md"))
        .filter_map(|e| {
            e.path()
                .strip_prefix(root)
                .ok()
                .map(|rel| rel.to_string_lossy().replace('\\', "/"))
        })
        .collect();
    out.sort();
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_drive() -> TempDir {
        TempDir::new().unwrap()
    }

    fn idx_dir(tmp: &TempDir) -> PathBuf {
        tmp.path().join("idx")
    }

    fn no_vectors() -> BuildOptions {
        BuildOptions {
            include_vectors: false,
        }
    }

    #[test]
    fn build_then_search_end_to_end() {
        let tmp = make_drive();
        std::fs::write(tmp.path().join("a.md"), "# alpha\nfoo apples\n").unwrap();
        std::fs::write(tmp.path().join("b.md"), "# beta\nbar bananas\n").unwrap();
        let idx = Index::open(tmp.path(), &idx_dir(&tmp)).unwrap();
        let summary = idx.build_all(no_vectors(), |_| {}, None).unwrap();
        assert_eq!(summary.files, 2);
        assert_eq!(summary.indexed, 2);
        assert!(summary.errors.is_empty());
        let r = idx.search("apples", Mode::Bm25, 10).unwrap();
        assert_eq!(r.hits.len(), 1);
        assert_eq!(r.hits[0].path, "a.md");
    }

    #[test]
    fn forget_drops_chunks() {
        let tmp = make_drive();
        std::fs::write(tmp.path().join("a.md"), "unique-token here\n").unwrap();
        let idx = Index::open(tmp.path(), &idx_dir(&tmp)).unwrap();
        idx.build_all(no_vectors(), |_| {}, None).unwrap();
        assert!(!idx
            .search("unique-token", Mode::Bm25, 10)
            .unwrap()
            .hits
            .is_empty());
        idx.forget("a.md").unwrap();
        assert!(idx
            .search("unique-token", Mode::Bm25, 10)
            .unwrap()
            .hits
            .is_empty());
    }

    #[test]
    fn rebuild_clears_old_data() {
        let tmp = make_drive();
        std::fs::write(tmp.path().join("a.md"), "first content\n").unwrap();
        let dir = idx_dir(&tmp);
        let idx = Index::open(tmp.path(), &dir).unwrap();
        idx.build_all(no_vectors(), |_| {}, None).unwrap();
        assert!(!idx.search("first", Mode::Bm25, 10).unwrap().hits.is_empty());
        drop(idx);
        let idx = Index::rebuild(tmp.path(), &dir).unwrap();
        assert!(idx.search("first", Mode::Bm25, 10).unwrap().hits.is_empty());
    }

    #[test]
    fn build_all_honors_cancel_and_skips_commit() {
        // Pre-flagged cancel should bail before any file is written
        // and before commit. The on-disk index must still be empty.
        let tmp = make_drive();
        std::fs::write(tmp.path().join("a.md"), "alpha unique-token\n").unwrap();
        std::fs::write(tmp.path().join("b.md"), "beta\n").unwrap();
        let idx = Index::open(tmp.path(), &idx_dir(&tmp)).unwrap();
        let cancel = AtomicBool::new(true);
        let err = idx
            .build_all(no_vectors(), |_| {}, Some(&cancel))
            .unwrap_err();
        assert!(matches!(err, IndexError::Cancelled));
        // No commit happened; the index stays empty so an auto-rebuild
        // trigger (`indexed_docs == 0`) would re-fire on next boot.
        assert_eq!(idx.stats().indexed_docs, 0);
    }
}
