// candle-backed embedder. Replaces the previous fastembed + ort
// stack with a pure-Rust transformer runtime so release builds are
// a single static binary on every platform: no prebuilt
// onnxruntime download at build time, no `libwebgpu_dawn.dylib`
// next to the binary at runtime, and no rpath / install_name_tool
// post-processing.
//
// Backends:
//   - macOS: candle's Metal backend (objc2-metal -> Metal.framework).
//     Always linked when the `metal` feature is on, which the
//     chan binary forwards automatically on macOS targets.
//   - Linux + `--features cuda`: candle's CUDA backend (cudarc).
//   - everything else: CPU.
//
// `CHAN_DISABLE_GPU=1` forces CPU at runtime without rebuilding,
// useful for benchmarking or for working around a flaky GPU path
// on a particular machine.
//
// Models we accept are listed in `MODELS`. Unknown ids are rejected
// at open time so the user gets a clear error instead of a panic
// deep inside candle. The embedding dimension is read from the
// model's own `config.json::hidden_size` so adding a new BGE
// variant only means adding one row.
//
// On-disk layout: hf-hub stores the model under
// `<cache_dir>/models--<org>--<name>/snapshots/<rev>/`. We resolve
// `config.json`, `tokenizer.json`, and `model.safetensors` from
// there. The cache is per-machine (immutable model files,
// identical across drives).

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use candle_core::{Device, IndexOp, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{BertModel, Config, DTYPE};
use hf_hub::api::sync::ApiBuilder;
use thiserror::Error;
use tokenizers::{PaddingParams, PaddingStrategy, Tokenizer, TruncationParams, TruncationStrategy};

#[derive(Debug, Error)]
pub enum EmbedError {
    #[error("unknown embedding model: {0}")]
    UnknownModel(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("candle: {0}")]
    Candle(String),
    #[error("tokenizer: {0}")]
    Tokenizer(String),
    #[error("hf-hub: {0}")]
    HfHub(String),
    #[error("config decode: {0}")]
    Config(String),
    #[error("operation cancelled")]
    Cancelled,
}

fn candle_err<E: std::fmt::Display>(e: E) -> EmbedError {
    EmbedError::Candle(e.to_string())
}
fn tok_err<E: std::fmt::Display>(e: E) -> EmbedError {
    EmbedError::Tokenizer(e.to_string())
}

/// Maximum input length in tokens. BGE family is 512.
const MAX_SEQ_LEN: usize = 512;

/// Per-forward-pass batch cap. The indexer hands us thousands of
/// chunks at a time (cross-file batching for throughput); running
/// them as one tensor would allocate
/// `[N, 512, hidden] * num_layers * activations` on the device, which
/// exhausts GPU memory and stalls Metal indefinitely on large drives.
/// 32 keeps activation memory under ~25 MB per layer for bge-small
/// while still amortizing kernel-launch overhead. Tune up cautiously
/// on machines with more VRAM.
const INFER_BATCH: usize = 32;

/// Loaded transformer + tokenizer. The Mutex wraps both because
/// `BertModel::forward` and tokenization are `&self` but we want
/// to keep the call site cheap; the rest of the codebase expects
/// `&Embedder` access. Contention is light: query embeds are tiny
/// and the indexer batches.
pub struct Embedder {
    model_id: String,
    inner: Mutex<Inner>,
    dim: usize,
}

struct Inner {
    model: BertModel,
    tokenizer: Tokenizer,
    device: Device,
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
    /// Open `model_id` (e.g. `BAAI/bge-small-en-v1.5`). Pulls
    /// `config.json` + `tokenizer.json` + `model.safetensors` from
    /// HuggingFace into `cache_dir` on first use; subsequent opens
    /// are local and fast.
    pub fn open(model_id: &str, cache_dir: &Path) -> Result<Self, EmbedError> {
        let _ = lookup_model(model_id)?;
        std::fs::create_dir_all(cache_dir)?;

        let api = ApiBuilder::new()
            .with_cache_dir(cache_dir.to_path_buf())
            .with_progress(true)
            .build()
            .map_err(|e| EmbedError::HfHub(e.to_string()))?;
        let repo = api.model(model_id.to_owned());

        let config_path = repo
            .get("config.json")
            .map_err(|e| EmbedError::HfHub(e.to_string()))?;
        let tokenizer_path = repo
            .get("tokenizer.json")
            .map_err(|e| EmbedError::HfHub(e.to_string()))?;
        let weights_path = repo
            .get("model.safetensors")
            .map_err(|e| EmbedError::HfHub(e.to_string()))?;

        let config_raw = std::fs::read_to_string(&config_path)?;
        let config: Config =
            serde_json::from_str(&config_raw).map_err(|e| EmbedError::Config(e.to_string()))?;
        let dim = config.hidden_size;

        let device = select_device();
        // Safe: safetensors files are mmap-friendly; candle reads
        // them read-only and doesn't mutate the backing pages.
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[weights_path], DTYPE, &device)
                .map_err(candle_err)?
        };
        let model = BertModel::load(vb, &config).map_err(candle_err)?;

        let mut tokenizer = Tokenizer::from_file(&tokenizer_path).map_err(tok_err)?;
        // Pad to the longest in the batch (BERT requires uniform
        // length per batch). Truncate to the model's max position
        // embedding window to match HF's default policy.
        let pad_id = tokenizer.get_padding().map(|p| p.pad_id).unwrap_or(0);
        tokenizer
            .with_padding(Some(PaddingParams {
                strategy: PaddingStrategy::BatchLongest,
                pad_id,
                pad_type_id: 0,
                pad_token: "[PAD]".to_string(),
                ..Default::default()
            }))
            .with_truncation(Some(TruncationParams {
                max_length: MAX_SEQ_LEN.min(config.max_position_embeddings),
                strategy: TruncationStrategy::LongestFirst,
                ..Default::default()
            }))
            .map_err(tok_err)?;

        Ok(Self {
            model_id: model_id.to_owned(),
            inner: Mutex::new(Inner {
                model,
                tokenizer,
                device,
            }),
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

    /// Embed a batch of documents. Internally splits into
    /// `INFER_BATCH`-sized forward passes so the caller can hand us
    /// thousands of chunks at a time without blowing GPU memory.
    /// Caller may pass a cancel flag; checked between sub-batches
    /// so a `chan index` Ctrl+C interrupts within ~one forward pass
    /// instead of waiting for the next file boundary.
    pub fn embed_documents<S: AsRef<str> + Send + Sync>(
        &self,
        docs: &[S],
    ) -> Result<Vec<Vec<f32>>, EmbedError> {
        self.embed_documents_cancelable(docs, None)
    }

    /// Cancellable variant. Returns `Cancelled` between sub-batches
    /// when `cancel` is set. The plain `embed_documents` is a thin
    /// wrapper that passes `None`.
    pub fn embed_documents_cancelable<S: AsRef<str> + Send + Sync>(
        &self,
        docs: &[S],
        cancel: Option<&std::sync::atomic::AtomicBool>,
    ) -> Result<Vec<Vec<f32>>, EmbedError> {
        if docs.is_empty() {
            return Ok(Vec::new());
        }
        let texts: Vec<&str> = docs.iter().map(|s| s.as_ref()).collect();
        let mut guard = self.inner.lock().unwrap();
        let mut out: Vec<Vec<f32>> = Vec::with_capacity(texts.len());
        for slice in texts.chunks(INFER_BATCH) {
            if let Some(c) = cancel {
                if c.load(std::sync::atomic::Ordering::Relaxed) {
                    return Err(EmbedError::Cancelled);
                }
            }
            let rows = embed_with(&mut guard, slice)?;
            out.extend(rows);
        }
        Ok(out)
    }

    /// Single-query embedding. Same path as documents (BGE family
    /// doesn't need a query-specific prefix for retrieval).
    pub fn embed_query(&self, q: &str) -> Result<Vec<f32>, EmbedError> {
        let mut v = self.embed_documents(&[q])?;
        Ok(v.pop().unwrap_or_default())
    }
}

fn embed_with(inner: &mut Inner, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbedError> {
    let encodings = inner
        .tokenizer
        .encode_batch(texts.to_vec(), true)
        .map_err(tok_err)?;

    let device = &inner.device;
    let mut id_rows: Vec<Tensor> = Vec::with_capacity(encodings.len());
    let mut mask_rows: Vec<Tensor> = Vec::with_capacity(encodings.len());
    for enc in &encodings {
        let ids: Vec<u32> = enc.get_ids().to_vec();
        let mask: Vec<u32> = enc.get_attention_mask().to_vec();
        id_rows.push(Tensor::new(ids.as_slice(), device).map_err(candle_err)?);
        mask_rows.push(Tensor::new(mask.as_slice(), device).map_err(candle_err)?);
    }
    let token_ids = Tensor::stack(&id_rows, 0).map_err(candle_err)?;
    let attention_mask = Tensor::stack(&mask_rows, 0).map_err(candle_err)?;
    let token_type_ids = token_ids.zeros_like().map_err(candle_err)?;

    // [batch, seq_len, hidden] last hidden state.
    let hidden = inner
        .model
        .forward(&token_ids, &token_type_ids, Some(&attention_mask))
        .map_err(candle_err)?;
    // BGE family is CLS-pooled per the HF model card. Take the
    // first token of every row, then L2-normalize so cosine becomes
    // a dot product downstream.
    let pooled = hidden.i((.., 0)).map_err(candle_err)?;
    let normed = l2_normalize(&pooled)?;

    let rows: Vec<Vec<f32>> = normed.to_vec2::<f32>().map_err(candle_err)?;
    Ok(rows)
}

fn l2_normalize(t: &Tensor) -> Result<Tensor, EmbedError> {
    let squared = t.sqr().map_err(candle_err)?;
    let sums = squared.sum_keepdim(1).map_err(candle_err)?;
    let norms = sums.sqrt().map_err(candle_err)?;
    t.broadcast_div(&norms).map_err(candle_err)
}

/// Pick the best available accelerator. CHAN_DISABLE_GPU=1 forces
/// CPU at runtime. macOS uses Metal, Linux + `cuda` feature uses
/// CUDA, everything else is CPU.
fn select_device() -> Device {
    if std::env::var_os("CHAN_DISABLE_GPU").is_some() {
        tracing::info!("embedder: GPU disabled via CHAN_DISABLE_GPU, using CPU");
        return Device::Cpu;
    }
    #[cfg(all(target_os = "macos", feature = "metal"))]
    {
        match Device::new_metal(0) {
            Ok(d) => {
                tracing::info!("embedder: Metal backend enabled");
                return d;
            }
            Err(e) => {
                tracing::warn!("embedder: Metal init failed ({e}); falling back to CPU");
            }
        }
    }
    #[cfg(all(target_os = "linux", feature = "cuda"))]
    {
        match Device::new_cuda(0) {
            Ok(d) => {
                tracing::info!("embedder: CUDA backend enabled");
                return d;
            }
            Err(e) => {
                tracing::warn!("embedder: CUDA init failed ({e}); falling back to CPU");
            }
        }
    }
    Device::Cpu
}

/// Models we explicitly accept. The dim column is informational
/// (we read the real value from each model's config.json at load)
/// but kept here so unknown-model errors can hint at the right
/// spelling. Add a row to support a new BGE variant without
/// touching the rest of the file.
const MODELS: &[(&str, usize)] = &[
    ("BAAI/bge-small-en-v1.5", 384),
    ("BAAI/bge-base-en-v1.5", 768),
    ("BAAI/bge-large-en-v1.5", 1024),
    // BGE-M3 is multilingual; useful when notes mix languages.
    ("BAAI/bge-m3", 1024),
];

fn lookup_model(id: &str) -> Result<usize, EmbedError> {
    for (name, dim) in MODELS {
        if *name == id {
            return Ok(*dim);
        }
    }
    Err(EmbedError::UnknownModel(id.to_owned()))
}

/// Per-machine model cache. macOS: `~/Library/Caches/chan/models`;
/// Linux: `$XDG_CACHE_HOME/chan/models`; Windows:
/// `%LOCALAPPDATA%/chan/models`. Falls back to the system temp dir
/// if `dirs::cache_dir()` is unavailable; hf-hub will then re-download
/// into the temp dir on each launch but search still works.
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
        let err = lookup_model("not-a-model").unwrap_err();
        assert!(matches!(err, EmbedError::UnknownModel(_)));
    }

    #[test]
    fn known_models_resolve() {
        assert!(lookup_model("BAAI/bge-small-en-v1.5").is_ok());
        assert!(lookup_model("BAAI/bge-m3").is_ok());
    }

    /// Gated on CHAN_RUN_MODEL_TESTS=1 so CI doesn't pull 130 MB
    /// every run. Locally: `CHAN_RUN_MODEL_TESTS=1 cargo test`.
    fn run_model_tests() -> bool {
        std::env::var_os("CHAN_RUN_MODEL_TESTS").is_some()
    }

    #[test]
    fn outputs_are_unit_vectors() {
        if !run_model_tests() {
            return;
        }
        let tmp = tempfile::tempdir().unwrap();
        let e = Embedder::open("BAAI/bge-small-en-v1.5", tmp.path()).unwrap();
        let v = e
            .embed_documents(&["hello world", "another sentence"])
            .unwrap();
        for row in &v {
            let n: f32 = row.iter().map(|x| x * x).sum::<f32>().sqrt();
            assert!((n - 1.0).abs() < 1e-4, "row not unit-norm: {n}");
        }
    }

    #[test]
    fn batched_equals_per_row() {
        if !run_model_tests() {
            return;
        }
        let tmp = tempfile::tempdir().unwrap();
        let e = Embedder::open("BAAI/bge-small-en-v1.5", tmp.path()).unwrap();
        let texts = ["alpha", "beta", "gamma"];
        let batched = e.embed_documents(&texts).unwrap();
        for (i, t) in texts.iter().enumerate() {
            let single = e.embed_query(t).unwrap();
            let dot: f32 = batched[i].iter().zip(&single).map(|(a, b)| a * b).sum();
            assert!(
                dot > 0.999,
                "batched vs single mismatch for {t:?}: cos={dot}"
            );
        }
    }

    #[test]
    fn deterministic() {
        if !run_model_tests() {
            return;
        }
        let tmp = tempfile::tempdir().unwrap();
        let e = Embedder::open("BAAI/bge-small-en-v1.5", tmp.path()).unwrap();
        let a = e.embed_query("the quick brown fox").unwrap();
        let b = e.embed_query("the quick brown fox").unwrap();
        assert_eq!(a, b);
    }
}
