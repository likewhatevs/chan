// Public surface of the search-index module. Hybrid (BM25 + dense)
// retrieval over per-heading chunks.
//
// Layout:
//   bm25       lexical side, tantivy-backed
//   chunking   markdown -> Chunk[]
//   config     on-disk index config (model, chunking strategy, …)
//   embeddings fastembed-rs wrapper (gated by `embeddings`)
//   facade     high-level Index: build / search / forget
//   fusion     reciprocal-rank fusion (BM25 + semantic)
//   vectors    on-disk vector store + brute-force cosine
//
// All callers (the CLI's `chan index` / `chan search` and the
// chan-server background indexer) go through this module so search
// results match across surfaces.

pub mod bm25;
pub mod chunking;
pub mod config;
#[cfg(feature = "embeddings")]
pub mod embeddings;
pub mod facade;
pub mod fusion;
pub mod vectors;

pub use config::{Chunking, IndexConfig, DEFAULT_MODEL};
pub use facade::{
    BuildOptions, BuildProgress, BuildStage, BuildSummary, Hit, Index, IndexError, IndexStats,
    Mode, SearchResult,
};
