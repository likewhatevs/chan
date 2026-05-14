// High-level entry point for both the CLI (`chan index`, `chan
// search`) and the in-process server use cases. Composes BM25,
// embeddings, and the vector store. Hybrid retrieval (RRF fusion)
// is gated by the `embeddings` feature; without it the facade
// answers Hybrid / Semantic queries with `ready: false` and a
// BM25-only fallback so the UI can show a "BM25-only on this
// build" hint instead of erroring out.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

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
use crate::fs_ops::{self, WalkFilter};

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
    /// Persisted index config. Behind a Mutex because `build_all`
    /// (which only holds `&self` through Drive's `Arc<Index>`) needs
    /// to stamp `vectors_model` / `vectors_dim` after a successful
    /// embed pass. Reads in hot paths take a single lock per build,
    /// not per chunk: each pass snapshots the config once at the top.
    config: Mutex<IndexConfig>,
    bm25: Bm25Index,
    vectors: VectorStore,
    /// Lazily loaded: opening the embedder mmaps the safetensors
    /// weights and warms the device, and we don't want
    /// `chan search --mode bm25` to pay that cost. The Mutex
    /// serializes first-init so two threads racing here can't both
    /// download the model from HuggingFace; once the Arc is
    /// populated, every subsequent call clones it cheaply.
    #[cfg(feature = "embeddings")]
    embedder: Mutex<Option<Arc<Embedder>>>,
    /// Directory-name blocklist applied by `build_all`'s tree walk.
    /// Updated via `set_walk_filter`. Default is empty; the chan
    /// binary populates the noise list (`node_modules`, `target`,
    /// ...) via `Library::set_walk_filter` which Drive forwards
    /// here before each reindex.
    walk_filter: Mutex<Arc<WalkFilter>>,
}

impl std::fmt::Debug for Index {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut d = f.debug_struct("Index");
        let model = self
            .config
            .lock()
            .map(|c| c.model.clone())
            .unwrap_or_default();
        d.field("drive_root", &self.drive_root)
            .field("index_dir", &self.index_dir)
            .field("model", &model);
        #[cfg(feature = "embeddings")]
        d.field(
            "embedder_loaded",
            &self.embedder.lock().map(|g| g.is_some()).unwrap_or(false),
        );
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
            config.vectors_model = None;
            config.vectors_dim = None;
            config::save(index_dir, &config)?;
        }
        // Model drift: vectors on disk were produced by a different
        // model than what's now configured. The two are not
        // interchangeable (different semantic space, potentially
        // different dim), and mixing them would silently degrade
        // retrieval. Wipe `embeddings/` (BM25 is model-independent
        // so it stays) and clear the tracking fields; the next
        // reindex will repopulate against the new model.
        if let Some(prior) = config.vectors_model.as_deref() {
            if prior != config.model {
                tracing::warn!(
                    prior = %prior,
                    target = %config.model,
                    "index model changed since last embed; wiping embeddings/",
                );
                wipe_vectors_dir(index_dir)?;
                config.vectors_model = None;
                config.vectors_dim = None;
                config::save(index_dir, &config)?;
            }
        }
        let bm25 = Bm25Index::open(index_dir)?;
        let vectors = VectorStore::open(index_dir)?;
        Ok(Self {
            drive_root: drive_root.to_path_buf(),
            index_dir: index_dir.to_path_buf(),
            config: Mutex::new(config),
            bm25,
            vectors,
            #[cfg(feature = "embeddings")]
            embedder: Mutex::new(None),
            walk_filter: Mutex::new(Arc::new(WalkFilter::default())),
        })
    }

    /// Replace the directory-name blocklist for the next `build_all`.
    /// Subsequent walks consult the new filter; an in-flight build
    /// keeps its snapshot (the filter is sampled once at the top of
    /// `build_all`). Drive calls this from `reindex_with` before
    /// kicking off the build.
    pub fn set_walk_filter(&self, filter: Arc<WalkFilter>) {
        *self.walk_filter.lock().unwrap() = filter;
    }

    /// Re-open after wiping `index_dir`. Intended for `--rebuild`.
    pub fn rebuild(drive_root: &Path, index_dir: &Path) -> Result<Self, IndexError> {
        wipe_index_dir(index_dir)?;
        Self::open(drive_root, index_dir)
    }

    /// Snapshot of the persisted config. Callers get a clone so the
    /// lock isn't held across their use; the config is small.
    pub fn config(&self) -> IndexConfig {
        self.config.lock().unwrap().clone()
    }

    /// Persist a (possibly mutated) config. Used by the CLI when
    /// the user passes `--model X`. Switching model invalidates the
    /// existing vectors (different dim / different semantics) so
    /// we wipe the vector dir; BM25 is unaffected.
    pub fn set_model(&mut self, model: String) -> Result<(), IndexError> {
        let to_save = {
            let mut cfg = self.config.lock().unwrap();
            if model == cfg.model {
                return Ok(());
            }
            cfg.model = model;
            // The vectors_* stamp described what *was* on disk; the
            // wipe below makes that stamp invalid. Clear it so the
            // next Index::open's model-mismatch check (and any human
            // reading the TOML) cannot conclude we trust the empty
            // store.
            cfg.vectors_model = None;
            cfg.vectors_dim = None;
            cfg.clone()
        };
        config::save(&self.index_dir, &to_save)?;
        #[cfg(feature = "embeddings")]
        {
            *self.embedder.lock().unwrap() = None;
        }
        let vec_dir = self.index_dir.join("embeddings");
        if vec_dir.exists() {
            std::fs::remove_dir_all(&vec_dir)?;
        }
        self.vectors = VectorStore::open(&self.index_dir)?;
        Ok(())
    }

    /// Get-or-init the embedder. Errors propagate (e.g. unknown
    /// model id, model download failure, candle device init). The
    /// init step holds the Mutex across `Embedder::open`, which can
    /// take seconds on a cold cache (model download from
    /// HuggingFace), so concurrent first-callers serialize and only
    /// one download happens. Once populated, every call returns a
    /// cheap Arc clone and never enters the slow path again.
    #[cfg(feature = "embeddings")]
    fn embedder(&self) -> Result<Arc<Embedder>, IndexError> {
        let mut guard = self.embedder.lock().unwrap();
        if let Some(e) = guard.as_ref() {
            return Ok(Arc::clone(e));
        }
        let cache_dir = embeddings::global_models_dir();
        let model_id = self.config.lock().unwrap().model.clone();
        let e = Arc::new(Embedder::open(&model_id, &cache_dir)?);
        *guard = Some(Arc::clone(&e));
        Ok(e)
    }

    /// Walk the drive and re-index everything from scratch. If
    /// `cancel` is set to true mid-build, returns `Cancelled` without
    /// calling `commit()` so tantivy discards every pending write
    /// queued in this run; the on-disk index is left as it was at
    /// the start.
    pub fn build_all(
        &self,
        opts: BuildOptions,
        progress: &dyn crate::progress::ProgressCallback,
        cancel: Option<&AtomicBool>,
    ) -> Result<BuildSummary, IndexError> {
        use crate::progress::{ProgressEvent, ProgressStage};
        use std::sync::atomic::AtomicUsize;
        // Snapshot the config once. The build can run for minutes;
        // a concurrent `set_model` would otherwise see-saw what
        // every chunk gets stamped with, and the post-build stamp
        // would race against in-flight reads.
        let cfg_at_start = self.config.lock().unwrap().clone();
        let model_at_start = cfg_at_start.model.clone();
        let filter = Arc::clone(&self.walk_filter.lock().unwrap());
        let files = list_indexable(&self.drive_root, &filter)?;
        let total = files.len();
        let mut indexed = 0usize;
        let mut chunks_total = 0usize;
        let mut errors: Vec<(String, IndexError)> = Vec::new();

        // Embedding throughput is dominated by per-call dispatch
        // and kernel-launch overhead on the GPU side. Per-file
        // embed calls on a drive of small markdown files (typical:
        // ~30 chunks per file) leave that overhead unamortized and
        // run an order of magnitude slower than the hardware can
        // do. Accumulate chunks across files and flush in
        // `EMBED_BATCH_CHUNKS`-sized groups so each forward pass
        // gets enough work to fill the device.
        #[cfg(feature = "embeddings")]
        let do_vectors = opts.include_vectors;
        #[cfg(not(feature = "embeddings"))]
        let _ = opts.include_vectors;
        #[cfg(feature = "embeddings")]
        let mut pending: Vec<(String, Vec<chunking::Chunk>)> = Vec::new();
        #[cfg(feature = "embeddings")]
        let mut pending_chunks: usize = 0;

        // Parallel read + chunk pipeline. Workers pull file indices
        // from `next`, read the file off disk, parse it into chunks,
        // and ship the result over `tx`. The main thread drains `rx`
        // and is the only thing that touches the BM25 writer and the
        // embed batcher, so writer-mutex contention and embed-batch
        // ordering stay simple. Bounded channel (workers * 4) caps
        // resident chunk memory: roughly that many parsed files in
        // flight at once.
        //
        // Workers per call: `available_parallelism - 2`, clamped to
        // [1, 6]. Two cores held back so the server's tokio runtime
        // and the OS UI thread keep breathing during a reindex of a
        // large drive. The cap of 6 is empirical: above it tantivy's
        // internal indexing threads + the embed model's tokio runtime
        // start contending and wall-clock stops improving.
        let worker_count = effective_workers();
        let next = AtomicUsize::new(0);
        let (tx, rx) = std::sync::mpsc::sync_channel::<WorkerOut>(worker_count * 4);
        let chunking_cfg = cfg_at_start.chunking.clone();
        let drive_root = &self.drive_root;
        let files_ref = &files;

        let started = std::time::Instant::now();
        let drain_result: Result<(), IndexError> = std::thread::scope(|s| {
            for _ in 0..worker_count {
                let tx = tx.clone();
                let next = &next;
                let chunking_cfg = chunking_cfg.clone();
                s.spawn(move || loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    if i >= files_ref.len() {
                        break;
                    }
                    if let Some(c) = cancel {
                        if c.load(Ordering::Relaxed) {
                            break;
                        }
                    }
                    let rel = files_ref[i].clone();
                    let abs = drive_root.join(&rel);
                    let item = match std::fs::read_to_string(&abs) {
                        Ok(text) => WorkerOut {
                            rel,
                            result: Ok(chunking::chunk(&text, &chunking_cfg)),
                        },
                        Err(e) => WorkerOut {
                            rel,
                            result: Err(e),
                        },
                    };
                    if tx.send(item).is_err() {
                        break;
                    }
                });
            }
            drop(tx);

            // Drain results. Order is non-deterministic across files;
            // `seen` is a monotonic count of completions so progress
            // ticks march forward even when results land out of order.
            for (seen, msg) in (0_u64..).zip(rx) {
                if let Some(c) = cancel {
                    if c.load(Ordering::Relaxed) {
                        return Err(IndexError::Cancelled);
                    }
                }
                progress.on_progress(ProgressEvent {
                    stage: ProgressStage::IndexFile,
                    current: seen,
                    total: total as u64,
                    label: Some(msg.rel.clone()),
                    eta_secs: crate::progress::eta_secs_from(started, seen, total as u64),
                });
                let chunks = match msg.result {
                    Ok(c) => c,
                    Err(e) => {
                        errors.push((msg.rel, e.into()));
                        continue;
                    }
                };
                if let Err(e) = self.bm25.index_chunks(&msg.rel, &chunks) {
                    errors.push((msg.rel, e.into()));
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
                        if let Err(e) =
                            self.vectors
                                .replace_file(&msg.rel, &model_at_start, 0, vec![])
                        {
                            errors.push((msg.rel, e.into()));
                        }
                        continue;
                    }
                    pending_chunks += chunks.len();
                    let rel_for_label = msg.rel.clone();
                    pending.push((msg.rel, chunks));
                    if pending_chunks >= EMBED_BATCH_CHUNKS {
                        progress.on_progress(ProgressEvent {
                            stage: ProgressStage::EmbedBatch,
                            current: pending_chunks as u64,
                            total: EMBED_BATCH_CHUNKS as u64,
                            label: Some(format!("files={} last={rel_for_label}", pending.len())),
                            // EmbedBatch fires once per buffer flush, not
                            // per chunk, so a rate-based ETA across batches
                            // would track GPU step time and not give the UI
                            // anything actionable. Leave it to the
                            // IndexFile ticks to drive the bar.
                            eta_secs: None,
                        });
                        match self.flush_embed_batch(&mut pending, cancel, &model_at_start) {
                            Ok(errs) => errors.extend(errs),
                            Err(IndexError::Cancelled) => return Err(IndexError::Cancelled),
                            Err(e) => return Err(e),
                        }
                        pending_chunks = 0;
                    }
                }
                #[cfg(not(feature = "embeddings"))]
                {
                    let _ = msg.rel;
                }
            }
            Ok(())
        });
        drain_result?;

        // Tail flush for the leftover < EMBED_BATCH_CHUNKS group.
        #[cfg(feature = "embeddings")]
        if do_vectors && !pending.is_empty() {
            if let Some(c) = cancel {
                if c.load(Ordering::Relaxed) {
                    return Err(IndexError::Cancelled);
                }
            }
            let last = pending.last().map(|(r, _)| r.clone()).unwrap_or_default();
            progress.on_progress(ProgressEvent {
                stage: ProgressStage::EmbedBatch,
                current: pending_chunks as u64,
                total: EMBED_BATCH_CHUNKS as u64,
                label: Some(format!("tail files={} last={last}", pending.len())),
                eta_secs: None,
            });
            match self.flush_embed_batch(&mut pending, cancel, &model_at_start) {
                Ok(errs) => errors.extend(errs),
                Err(IndexError::Cancelled) => return Err(IndexError::Cancelled),
                Err(e) => return Err(e),
            }
        }

        if let Some(c) = cancel {
            if c.load(Ordering::Relaxed) {
                return Err(IndexError::Cancelled);
            }
        }
        // Drop vector shards for paths that survived a previous
        // build but are no longer on disk (file deleted while the
        // process was down, or a forget that crashed between vector
        // delete and BM25 commit). After this pass the two backends
        // converge on `files` as the source of truth. Errors here
        // are non-fatal: an orphan shard wastes disk and may surface
        // in semantic search as a hit pointing at a missing file,
        // but the next reindex will retry the cleanup.
        #[cfg(feature = "embeddings")]
        if do_vectors {
            let visited: std::collections::HashSet<&str> =
                files.iter().map(String::as_str).collect();
            for rel in self.vectors.known_paths() {
                if !visited.contains(rel.as_str()) {
                    if let Err(e) = self.vectors.delete_file(&rel) {
                        tracing::warn!(rel = %rel, ?e, "vector shard cleanup failed");
                    }
                }
            }
        }
        self.bm25.commit()?;
        // Stamp "what's on disk" so the next Index::open's model
        // mismatch check has something to compare against. We only
        // do this when vectors were configured for this build; a
        // BM25-only build (no `embeddings` feature, or zero
        // indexable files) leaves the tracking fields alone so a
        // subsequent vector build sets them honestly.
        //
        // Dim is read from the embedder when at least one chunk was
        // embedded this run; if nothing was embedded (empty drive,
        // every file produced zero chunks) we leave vectors_dim
        // unchanged and stamp vectors_model anyway, because an
        // empty vector store is trivially consistent with the
        // current model. The dim will be filled in on the next
        // build that actually produces vectors.
        #[cfg(feature = "embeddings")]
        if do_vectors {
            let to_save = {
                let mut cfg = self.config.lock().unwrap();
                cfg.vectors_model = Some(model_at_start.clone());
                if let Some(e) = self.embedder.lock().unwrap().as_ref() {
                    cfg.vectors_dim = Some(e.dim() as u32);
                }
                cfg.clone()
            };
            if let Err(e) = config::save(&self.index_dir, &to_save) {
                // Non-fatal: BM25 + tantivy commits already
                // succeeded. A missed stamp means the next open
                // sees vectors_model=None (or the previous value)
                // and may decide to wipe; the search still works.
                tracing::warn!(?e, "failed to persist vectors_model stamp after build");
            }
        }
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
    /// the whole batch. Returns `Cancelled` (without writing partial
    /// vectors) if `cancel` flips during one of the inner sub-batches.
    #[cfg(feature = "embeddings")]
    fn flush_embed_batch(
        &self,
        pending: &mut Vec<(String, Vec<chunking::Chunk>)>,
        cancel: Option<&AtomicBool>,
        model: &str,
    ) -> Result<Vec<(String, IndexError)>, IndexError> {
        let mut errors = Vec::new();
        if pending.is_empty() {
            return Ok(errors);
        }
        let embedder = match self.embedder() {
            Ok(e) => e,
            Err(e) => {
                let msg = e.to_string();
                for (rel, _) in pending.drain(..) {
                    errors.push((rel, IndexError::Embed(EmbedError::Candle(msg.clone()))));
                }
                return Ok(errors);
            }
        };
        let dim = embedder.dim();
        let bodies: Vec<&str> = pending
            .iter()
            .flat_map(|(_, chunks)| chunks.iter().map(|c| c.body.as_str()))
            .collect();
        let raw = match embedder.embed_documents_cancelable(&bodies, cancel) {
            Ok(v) => v,
            Err(EmbedError::Cancelled) => return Err(IndexError::Cancelled),
            Err(_) => {
                // Per-file fallback so a single bad file doesn't
                // discard the rest of the batch's vectors.
                for (rel, chunks) in pending.drain(..) {
                    if let Err(e) = self.embed_one_file(&rel, &chunks, dim, model) {
                        errors.push((rel, e));
                    }
                }
                return Ok(errors);
            }
        };
        let mut cursor = 0usize;
        for (rel, chunks) in pending.drain(..) {
            let n = chunks.len();
            let slice = raw[cursor..cursor + n].to_vec();
            cursor += n;
            let embedded = vectors::pair(&chunks, slice);
            if let Err(e) = self.vectors.replace_file(&rel, model, dim, embedded) {
                errors.push((rel, e.into()));
            }
        }
        Ok(errors)
    }

    #[cfg(feature = "embeddings")]
    fn embed_one_file(
        &self,
        rel: &str,
        chunks: &[chunking::Chunk],
        dim: usize,
        model: &str,
    ) -> Result<(), IndexError> {
        let bodies: Vec<&str> = chunks.iter().map(|c| c.body.as_str()).collect();
        let embedder = self.embedder()?;
        let raw = embedder.embed_documents(&bodies)?;
        let embedded = vectors::pair(chunks, raw);
        self.vectors.replace_file(rel, model, dim, embedded)?;
        Ok(())
    }

    /// One-file write path used by both `build_all` and `index_one`.
    /// Chunks once, persists vectors first, then hands the same
    /// chunks to BM25. Caller commits BM25; that commit is the
    /// durable boundary for the pair. A crash between vector
    /// persist and BM25 commit drops the BM25 write entirely
    /// (tantivy never persisted it) and leaves the vector shard on
    /// disk, which the next reindex overwrites. The opposite
    /// ordering would let a committed BM25 row reference a chunk
    /// whose vector never reached disk: silent semantic-search
    /// drift that the user only notices when results disappear.
    fn write_file(
        &self,
        rel_path: &str,
        text: &str,
        include_vectors: bool,
    ) -> Result<usize, IndexError> {
        // Snapshot the parts of config we need so a concurrent
        // `set_model` cannot rewrite the model id mid-call (which
        // would let us write a vector shard stamped with one
        // model id but produced by the embedder of another).
        let (model, chunking_cfg) = {
            let cfg = self.config.lock().unwrap();
            (cfg.model.clone(), cfg.chunking.clone())
        };
        let chunks = chunking::chunk(text, &chunking_cfg);
        // include_vectors is the caller's intent. When the binary
        // is built without `embeddings`, we never produce vectors
        // regardless. BM25-only is a working subset.
        #[cfg(not(feature = "embeddings"))]
        let _ = include_vectors;
        #[cfg(feature = "embeddings")]
        {
            if include_vectors {
                if chunks.is_empty() {
                    self.vectors.replace_file(rel_path, &model, 0, vec![])?;
                } else {
                    let embedder = self.embedder()?;
                    let dim = embedder.dim();
                    let bodies: Vec<&str> = chunks.iter().map(|c| c.body.as_str()).collect();
                    let vectors_raw = embedder.embed_documents(&bodies)?;
                    let embedded = vectors::pair(&chunks, vectors_raw);
                    self.vectors.replace_file(rel_path, &model, dim, embedded)?;
                }
            }
        }
        #[cfg(not(feature = "embeddings"))]
        let _ = model;
        self.bm25.index_file(rel_path, text, &chunking_cfg)?;
        Ok(chunks.len())
    }

    /// Re-index a single file (incremental). Used by the watcher
    /// hook. Caller supplies both `rel_path` and `text` so the read
    /// goes through the Drive sandbox (path safety, special-file
    /// refusal, editable-text gate). The index never opens user
    /// files directly outside `build_all`'s controlled walk.
    pub fn index_one(&self, rel_path: &str, text: &str) -> Result<usize, IndexError> {
        let n = self.write_file(rel_path, text, true)?;
        self.bm25.commit()?;
        Ok(n)
    }

    /// Drop a file from both indexes (e.g. after the file is
    /// removed on disk). Vectors first, then BM25 + commit. A
    /// crash between the two leaves the vector shard removed but
    /// BM25 still claiming the row; the next BM25 search keeps
    /// working and the next reindex repopulates vectors. The
    /// opposite ordering would commit a BM25 deletion while the
    /// vector shard outlived it, so semantic search would surface
    /// a hit pointing at a path BM25 (and the editor) considers
    /// gone.
    pub fn forget(&self, rel_path: &str) -> Result<(), IndexError> {
        self.vectors.delete_file(rel_path)?;
        self.bm25.delete_file(rel_path)?;
        self.bm25.commit()?;
        Ok(())
    }

    /// Batched `forget` for a directory delete: queue vector + BM25
    /// deletes for every path, commit BM25 once at the end. Same
    /// ordering invariants as `forget` (vectors first per path), so
    /// a crash mid-batch leaves orphan vector shards at worst; the
    /// next reindex's orphan-cleanup pass reclaims them. Empty input
    /// is a no-op (no commit, no churn on tantivy's segments).
    pub fn forget_many<I, S>(&self, paths: I) -> Result<(), IndexError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut any = false;
        for p in paths {
            let p = p.as_ref();
            self.vectors.delete_file(p)?;
            self.bm25.delete_file(p)?;
            any = true;
        }
        if any {
            self.bm25.commit()?;
        }
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
            model: self.config.lock().unwrap().model.clone(),
        }
    }
}

// Cross-file embedding batch size, in chunks. Tuned for candle +
// bge-small on Metal: large enough to amortize forward-pass setup
// over a useful work unit, small enough that working memory stays
// modest (~12 MB at 384-dim) on big drives. Only used when the
// `embeddings` feature is on; harmless otherwise.
#[cfg(feature = "embeddings")]
const EMBED_BATCH_CHUNKS: usize = 4096;

/// One worker -> main message. The worker is responsible only for
/// reading the file and parsing it into chunks; the writer side
/// (BM25 add_document, embed batching, vector writes) is the main
/// thread's job. Carrying the rel-path along lets the main thread
/// attribute IO errors back to a specific file without a side
/// channel.
struct WorkerOut {
    rel: String,
    result: std::io::Result<Vec<chunking::Chunk>>,
}

/// How many read+chunk workers `build_all` runs in parallel.
/// Reserves two cores for the rest of the process (the server's
/// tokio runtime, the UI thread, tantivy's internal indexing pool,
/// the OS) so reindex never starves foreground work. The upper
/// cap of 6 keeps the embedding model and tantivy's writer threads
/// from contending past the point where wall-clock improves.
fn effective_workers() -> usize {
    let cores = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(2);
    cores.saturating_sub(2).clamp(1, 6)
}

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

/// Wipe only the vector store (`embeddings/`), leaving BM25 and the
/// config alone. Used by `Index::open` on a model-id mismatch: the
/// BM25 segments are model-independent and have to survive so the
/// user keeps lexical search while the embeddings rebuild.
fn wipe_vectors_dir(index_dir: &Path) -> Result<(), IndexError> {
    let p = index_dir.join("embeddings");
    if p.exists() {
        std::fs::remove_dir_all(&p)?;
    }
    Ok(())
}

/// Walk the drive and return every indexable file (any
/// `FileClass::EditableText`: `.md` + `.txt` today) relative to
/// root, using forward-slash separators on all platforms (matches
/// the API's shape). Honors the caller-supplied `WalkFilter` so
/// blocked dir names (`node_modules`, ...) are never descended.
fn list_indexable(root: &Path, filter: &WalkFilter) -> Result<Vec<String>, IndexError> {
    let mut out: Vec<String> = fs_ops::walk_drive_filtered(root, filter)
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| {
            e.path()
                .strip_prefix(root)
                .ok()
                .map(|rel| rel.to_string_lossy().replace('\\', "/"))
        })
        .filter(|rel| fs_ops::is_editable_text(rel))
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
        let summary = idx
            .build_all(no_vectors(), &crate::progress::NoProgress, None)
            .unwrap();
        assert_eq!(summary.files, 2);
        assert_eq!(summary.indexed, 2);
        assert!(summary.errors.is_empty());
        let r = idx.search("apples", Mode::Bm25, 10).unwrap();
        assert_eq!(r.hits.len(), 1);
        assert_eq!(r.hits[0].path, "a.md");
    }

    #[test]
    fn parallel_build_indexes_every_file_and_emits_one_progress_per_file() {
        // build_all distributes work across N reader threads, so we
        // need to confirm two invariants the previous serial loop got
        // for free: every file ends up in the index regardless of
        // dispatch order, and the progress callback fires once per
        // file (`seen` is monotonic even when results land out of
        // order). 200 files is enough to make N>1 workers race.
        use std::sync::Mutex;
        let tmp = make_drive();
        for i in 0..200 {
            let path = tmp.path().join(format!("note-{i:03}.md"));
            std::fs::write(&path, format!("# note {i}\nbody-token-{i:03}\n")).unwrap();
        }
        let idx = Index::open(tmp.path(), &idx_dir(&tmp)).unwrap();
        let labels = Arc::new(Mutex::new(Vec::<String>::new()));
        let cb = {
            let labels = labels.clone();
            crate::progress::progress_fn(move |e| {
                if matches!(e.stage, crate::progress::ProgressStage::IndexFile) {
                    if let Some(l) = e.label {
                        labels.lock().unwrap().push(l);
                    }
                }
            })
        };
        let summary = idx.build_all(no_vectors(), &*cb, None).unwrap();
        assert_eq!(summary.files, 200);
        assert_eq!(summary.indexed, 200);
        assert!(
            summary.errors.is_empty(),
            "got errors: {:?}",
            summary.errors
        );
        // One progress label per file, no dupes, no drops.
        let seen = labels.lock().unwrap().clone();
        let mut uniq = seen.clone();
        uniq.sort();
        uniq.dedup();
        assert_eq!(
            uniq.len(),
            200,
            "expected 200 distinct labels, got {seen:?}"
        );
        // Every file is independently searchable.
        for i in 0..200 {
            let q = format!("body-token-{i:03}");
            let hits = idx.search(&q, Mode::Bm25, 5).unwrap().hits;
            assert_eq!(hits.len(), 1, "missing hit for {q}");
            assert_eq!(hits[0].path, format!("note-{i:03}.md"));
        }
    }

    #[test]
    fn forget_drops_chunks() {
        let tmp = make_drive();
        std::fs::write(tmp.path().join("a.md"), "unique-token here\n").unwrap();
        let idx = Index::open(tmp.path(), &idx_dir(&tmp)).unwrap();
        idx.build_all(no_vectors(), &crate::progress::NoProgress, None)
            .unwrap();
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
        idx.build_all(no_vectors(), &crate::progress::NoProgress, None)
            .unwrap();
        assert!(!idx.search("first", Mode::Bm25, 10).unwrap().hits.is_empty());
        drop(idx);
        let idx = Index::rebuild(tmp.path(), &dir).unwrap();
        assert!(idx.search("first", Mode::Bm25, 10).unwrap().hits.is_empty());
    }

    #[test]
    fn open_wipes_embeddings_when_configured_model_diverges_from_disk_stamp() {
        // Simulate the upgrade-path: a previous build stamped a
        // different model into the config. On open, the divergence
        // must trigger an embeddings-only wipe; BM25 must survive.
        let tmp = make_drive();
        std::fs::write(tmp.path().join("a.md"), "alpha unique-token\n").unwrap();
        let dir = idx_dir(&tmp);
        let idx = Index::open(tmp.path(), &dir).unwrap();
        idx.build_all(no_vectors(), &crate::progress::NoProgress, None)
            .unwrap();
        let bm25_before = idx.search("unique-token", Mode::Bm25, 10).unwrap();
        assert_eq!(bm25_before.hits.len(), 1);

        // Plant a fake vectors/ subdir to confirm the wipe targets it.
        let vec_dir = dir.join("embeddings");
        std::fs::create_dir_all(&vec_dir).unwrap();
        std::fs::write(vec_dir.join("planted"), b"junk").unwrap();

        // Stamp config as if a previous embed pass had used a
        // different model. Then close + re-open so the open-time
        // check fires.
        let cfg_on_disk = config::IndexConfig {
            schema_version: config::SCHEMA_VERSION,
            model: "BAAI/bge-small-en-v1.5".to_owned(),
            chunking: config::Chunking::default(),
            vectors_model: Some("BAAI/bge-large-en-v1.5".to_owned()),
            vectors_dim: Some(1024),
        };
        config::save(&dir, &cfg_on_disk).unwrap();
        drop(idx);

        let idx = Index::open(tmp.path(), &dir).unwrap();
        // BM25 search still works (segments untouched).
        let bm25_after = idx.search("unique-token", Mode::Bm25, 10).unwrap();
        assert_eq!(bm25_after.hits.len(), 1);
        // The planted embeddings dir was reclaimed.
        assert!(
            !vec_dir.join("planted").exists(),
            "embeddings dir must be wiped on model-id mismatch"
        );
        // Config tracking fields cleared and persisted.
        let cfg_after = config::load(&dir).unwrap();
        assert_eq!(cfg_after.vectors_model, None);
        assert_eq!(cfg_after.vectors_dim, None);
        assert_eq!(cfg_after.model, "BAAI/bge-small-en-v1.5");
    }

    #[test]
    fn open_leaves_everything_alone_when_disk_stamp_matches() {
        // Symmetric case: the stamp matches the configured model.
        // No wipe, no churn, no spurious config save.
        let tmp = make_drive();
        std::fs::write(tmp.path().join("a.md"), "alpha\n").unwrap();
        let dir = idx_dir(&tmp);
        let model = "BAAI/bge-small-en-v1.5".to_owned();
        let cfg_on_disk = config::IndexConfig {
            schema_version: config::SCHEMA_VERSION,
            model: model.clone(),
            chunking: config::Chunking::default(),
            vectors_model: Some(model.clone()),
            vectors_dim: Some(384),
        };
        std::fs::create_dir_all(&dir).unwrap();
        config::save(&dir, &cfg_on_disk).unwrap();
        // Plant a sentinel in embeddings/ that must survive the open.
        let vec_dir = dir.join("embeddings");
        std::fs::create_dir_all(&vec_dir).unwrap();
        std::fs::write(vec_dir.join("sentinel"), b"keep").unwrap();

        let _idx = Index::open(tmp.path(), &dir).unwrap();
        assert!(
            vec_dir.join("sentinel").exists(),
            "matching stamp must not trigger a wipe",
        );
        let cfg_after = config::load(&dir).unwrap();
        assert_eq!(cfg_after.vectors_model.as_deref(), Some(model.as_str()));
        assert_eq!(cfg_after.vectors_dim, Some(384));
    }

    #[test]
    fn schema_version_bump_clears_tracking_fields() {
        // A schema bump wipes everything, including the tracking
        // fields. Otherwise the post-wipe open would think the
        // vectors are still valid for the old model.
        let tmp = make_drive();
        let dir = idx_dir(&tmp);
        std::fs::create_dir_all(&dir).unwrap();
        let cfg_on_disk = config::IndexConfig {
            schema_version: config::SCHEMA_VERSION.saturating_sub(1),
            model: "BAAI/bge-small-en-v1.5".to_owned(),
            chunking: config::Chunking::default(),
            vectors_model: Some("BAAI/bge-small-en-v1.5".to_owned()),
            vectors_dim: Some(384),
        };
        config::save(&dir, &cfg_on_disk).unwrap();
        let _idx = Index::open(tmp.path(), &dir).unwrap();
        let cfg_after = config::load(&dir).unwrap();
        assert_eq!(cfg_after.schema_version, config::SCHEMA_VERSION);
        assert_eq!(cfg_after.vectors_model, None);
        assert_eq!(cfg_after.vectors_dim, None);
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
            .build_all(no_vectors(), &crate::progress::NoProgress, Some(&cancel))
            .unwrap_err();
        assert!(matches!(err, IndexError::Cancelled));
        // No commit happened; the index stays empty so an auto-rebuild
        // trigger (`indexed_docs == 0`) would re-fire on next boot.
        assert_eq!(idx.stats().indexed_docs, 0);
    }
}
