# candle switch plan

Follow-up to the WebGPU/Metal embedder change. Replace fastembed +
ort with candle so embeddings run on native Metal / CUDA kernels
without the Dawn middle layer or the runtime dylib + rpath hack.

## why

- WebGPU EP works but ships `libwebgpu_dawn.dylib` (~16 MB) that has
  to live next to the binary, with `@executable_path` rpath patched
  in via `install_name_tool`. Breaks the "single static binary" goal
  for macOS releases.
- pyke's ort prebuilt for `aarch64-apple-darwin` has no CoreML
  variant, only `none` and `+wgpu`. Building onnxruntime from source
  to get CoreML/ANE is rejected: 30-60 min build, Xcode toolchain
  required, breaks `cargo build, done`.
- candle is HF's pure-Rust ML lib. Native Metal kernels for
  transformer encoders. Linked against system frameworks only
  (Metal.framework on macOS, libcuda on Linux). No build-time
  binary download. Smaller dep tree than fastembed+ort.
- Empirically, candle's Metal path tends to match or beat ORT's
  WebGPU EP for small transformers; CUDA path is on par with ORT.

## scope

- Replaces only `chan-drive/src/index/embeddings.rs`.
- `Embedder`'s public API is unchanged. No caller changes.
- Cross-file embed batching (this session's `build_all` rework) is
  backend-agnostic and stays.

## what to drop

- `fastembed` dependency.
- `ort` direct dep (`[target.'cfg(target_os = "macos")'.dependencies]`
  and `[target.'cfg(target_os = "linux")'.dependencies]`).
- chan-drive `cuda` cargo feature (replaced by passthrough to
  candle's `cuda` feature).
- Release-time rpath hack and the `libwebgpu_dawn.dylib` copy step.
  After the switch, `cargo build --release` produces a working
  binary with no post-processing.

## what to add

- `candle-core`, `candle-nn`, `candle-transformers` — workspace deps.
- `tokenizers` — HF tokenizers (BERT WordPiece for the BGE family).
- `hf-hub` — one-shot model + tokenizer download. Cache under the
  existing `global_models_dir()` so models from prior fastembed
  installs aren't re-downloaded if their layout overlaps; otherwise
  a fresh `chan/models/candle/` subdir is fine.
- New `gpu` cargo feature on chan-drive that flips candle's
  `metal` / `cuda` features. Default-on for macOS, opt-in for Linux.

## API parity contract

Drop-in. Callers (`facade.rs::Index::write_file`,
`facade.rs::flush_embed_batch`, `facade.rs::embed_one_file`) must
not change.

```rust
impl Embedder {
    pub fn open(model_id: &str, cache_dir: &Path) -> Result<Self, EmbedError>;
    pub fn embed_documents<S: AsRef<str> + Send + Sync>(
        &self,
        docs: &[S],
    ) -> Result<Vec<Vec<f32>>, EmbedError>;
    pub fn embed_query(&self, q: &str) -> Result<Vec<f32>, EmbedError>;
    pub fn dim(&self) -> usize;
    pub fn model_id(&self) -> &str;
}
```

## architecture

1. **Load**: read `config.json` and `tokenizer.json` from HF Hub for
   the model_id. Load weights from `model.safetensors`. Build a
   `BertModel` (candle-transformers) on the selected `Device`.
2. **Tokenize**: `tokenizers::Tokenizer::encode_batch` produces
   input_ids / attention_mask / token_type_ids tensors.
3. **Forward**: `BertModel::forward(input_ids, token_type_ids,
   attention_mask)` returns last_hidden_state.
4. **Pool**: BGE uses CLS-token pooled output (verify against HF
   model card and `1_Pooling/config.json`). L2-normalize.
5. **Return**: `Vec<Vec<f32>>`, one row per input.

## backend selection

```rust
#[cfg(all(target_os = "macos", feature = "gpu"))]
let device = candle_core::Device::new_metal(0)?;
#[cfg(all(target_os = "linux", feature = "gpu", feature = "cuda"))]
let device = candle_core::Device::new_cuda(0)?;
#[cfg(not(any(
    all(target_os = "macos", feature = "gpu"),
    all(target_os = "linux", feature = "gpu", feature = "cuda"),
)))]
let device = candle_core::Device::Cpu;
```

`CHAN_DISABLE_GPU=1` env var forces `Device::Cpu` at runtime.

## models to support

Same set as today's `model_for`:

| model_id                       | dim  | notes                  |
|--------------------------------|------|------------------------|
| BAAI/bge-small-en-v1.5         | 384  | default                |
| BAAI/bge-base-en-v1.5          | 768  |                        |
| BAAI/bge-large-en-v1.5         | 1024 |                        |
| BAAI/bge-m3                    | 1024 | multilingual           |

Read dim from `config.json::hidden_size` at load instead of
hardcoding per-enum.

## tests

- L2 norm of every output ≈ 1.0 (within 1e-5).
- Determinism: identical input → identical output.
- Cross-file batched embed equals per-file embed for the same
  inputs (batching invariant; key correctness check).
- Golden cosine similarity: precompute one (input, expected_vector)
  pair from `sentence-transformers` Python and assert
  `cos(candle_out, expected) > 0.999`. Catches subtle pooling /
  tokenizer drift.
- Gate model-download tests behind `CHAN_RUN_MODEL_TESTS=1` so CI
  doesn't pull 130 MB on every run.

## schema migration

Bump `SCHEMA_VERSION` in `chan-drive/src/index/config.rs`. On-disk
vectors written by fastembed get wiped on first open with the new
build; reindex regenerates them. Note in the commit body so users
know to expect a one-time reindex on upgrade.

## perf targets (back-of-envelope)

- M2 Max (Metal): bge-small 200–500 chunks/sec.
- NVIDIA desktop (CUDA): 1000+ chunks/sec.
- CPU fallback: parity with today's fastembed CPU (~50 files/min on
  small markdown).

## rough order of work

1. Add candle / tokenizers / hf-hub deps in chan-drive Cargo.toml.
2. Rewrite `embeddings.rs::Embedder` (consider doing it in a new
   `embedder.rs` and deleting `embeddings.rs` at the end so the
   diff is readable).
3. Map `model_id` → candle config (mirror current `model_for`).
4. Wire backend selection via cfg + env-var fallback.
5. Drop ort dep + delete `with_accelerator`.
6. Bump `SCHEMA_VERSION`.
7. Tests (L2 norm, determinism, batching invariant, golden vector).
8. Remove `libwebgpu_dawn.dylib` bundling + rpath patch from any
   release scripts (audit `Makefile` and `scripts/`).
9. Smoke: `chan index ./docs` against wikimd; compare to today's
   WebGPU baseline.

## open questions to resolve in the new session

- BGE pooling: CLS-token vs mean. Confirm against HF
  `1_Pooling/config.json` for each supported model.
- `BertModel::forward` exact signature in the candle version we
  pin; whether attention mask is `Option<&Tensor>` or required.
- Max sequence length: BGE family is 512; current chunking already
  produces small chunks but tokenizer truncation policy must match
  what HF does (truncate=longest_first, max_length=512).
- Thread safety: candle `Device` + `BertModel` under our existing
  `Mutex<Embedder>` wrapper. Confirm no deadlocks under concurrent
  `embed_query` from server routes while a `build_all` is running.

## what to keep from this session

- Cross-file embed batching in `build_all` (4096 chunks/flush).
- `BuildStage::EmbedBatch` progress events and the CLI rendering.
- `Drive::reindex_with(cancel, on_progress)`.
- Boot auto-rebuild trigger firing on empty-graph in
  `chan-server/src/indexer.rs`.

These are backend-agnostic; the candle switch should not touch
them.
