// Search API surface and (initially) BM25-only implementation.
//
// The public types live here even when the `search` feature is off
// so callers can keep referring to `SearchOpts` etc. without
// cfg-guards everywhere; only the index implementation is gated.
//
// Rough plan once the impl lands:
//   - tantivy schema: path (TEXT, stored), heading_path (TEXT),
//     body (TEXT, indexed), mtime (I64), file_id (TEXT, kept).
//   - chunking is per-section (split on ATX headings) so a hit
//     can carry the heading-stack as breadcrumb context.
//   - schema_version field on disk; mismatched versions trigger a
//     full rebuild on next open.
//
// Hybrid (BM25 + dense) lives behind a future `embeddings` feature
// when fastembed-rs (or a CoreML / NNAPI alternative) is wired in.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::Result;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SearchMode {
    /// BM25 only. Available everywhere.
    #[default]
    Bm25,
    /// BM25 + dense retrieval, fused. Requires the embeddings
    /// feature; the index falls back to BM25 if disabled.
    Hybrid,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchOpts {
    pub mode: SearchMode,
    /// Hard cap on results returned. Defaults to 50 when 0.
    pub limit: u32,
    /// Optional subdir scope (relative to drive root). When set,
    /// only paths under this prefix are returned. None = whole drive.
    pub scope: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResults {
    pub hits: Vec<Hit>,
    pub total: u32,
    pub mode_used: SearchMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hit {
    pub path: String,
    pub score: f32,
    pub snippets: Vec<Snippet>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snippet {
    /// ATX heading stack leading to this snippet, e.g.
    /// `["Recipes", "Pasta", "Carbonara"]`. Empty for top-of-file
    /// content.
    pub heading_path: Vec<String>,
    /// Highlighted excerpt around the match. Plain text; the UI
    /// is responsible for rendering markdown.
    pub text: String,
}

/// Index handle. One per Drive open; not Clone (carries the
/// writer lock lifecycle internally).
pub struct Index {
    /// Drive root, kept for scoping/file-id purposes.
    #[allow(dead_code)]
    drive_root: std::path::PathBuf,
    /// Index dir from `paths::drive_paths`.
    #[allow(dead_code)]
    index_dir: std::path::PathBuf,
}

impl Index {
    /// Open or create the search index for this drive. Schema
    /// mismatches trigger a full rebuild on the next `reindex()`.
    pub fn open(drive_root: &Path, index_dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(index_dir)?;
        Ok(Self {
            drive_root: drive_root.to_path_buf(),
            index_dir: index_dir.to_path_buf(),
        })
    }

    /// Run a query. The current implementation is a no-op stub
    /// (always returns empty results); wiring tantivy is the next
    /// session's work.
    pub fn search(&self, _query: &str, opts: &SearchOpts) -> Result<SearchResults> {
        Ok(SearchResults {
            hits: Vec::new(),
            total: 0,
            mode_used: opts.mode,
        })
    }

    /// Re-index the whole drive from scratch. Stub for now.
    pub fn reindex(&self) -> Result<IndexStats> {
        Ok(IndexStats::default())
    }

    /// Incrementally update the index for a single file. Stub for now.
    pub fn upsert(&self, _rel: &str) -> Result<()> {
        Ok(())
    }

    /// Drop a file from the index.
    pub fn remove(&self, _rel: &str) -> Result<()> {
        Ok(())
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IndexStats {
    pub files_indexed: u32,
    pub chunks_indexed: u32,
    pub elapsed_ms: u64,
}
