use serde::{Deserialize, Serialize};

use crate::cocomo::CocomoSummary;

/// Source-code-shaped classification axis. The graph overhaul's
/// G6 colour scheme needs to distinguish markdown notes from
/// source-code files (different colour for each); this enum is
/// the axis chan-report owns.
///
/// `chan-workspace::FileClass` is the orthogonal IO-contract axis
/// (Image / Pdf / Other for non-source files); the graph indexer
/// composes the two: chan-report's `FileBucket` for the files it
/// tracks (markdown + source code); chan-workspace's `FileClass` for
/// everything else (media / binary / unknown).
///
/// Only the buckets chan-report can populate from `tokei`'s
/// language detection live here. Binary / Media / Other are NOT
/// in this enum because chan-report doesn't track those files; the
/// graph indexer reads `chan_workspace::classify()` for those.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FileBucket {
    /// `.md` files. Notes. Graph G6 colour: orange.
    Markdown,
    /// Anything else `tokei` recognizes: Rust, Python, TypeScript,
    /// JSON, TOML, shell scripts, Makefile, Dockerfile, LICENSE,
    /// .gitignore, etc. The `language` is `tokei::LanguageType::name()`
    /// so consumers can group / display per-language. Graph G6
    /// colour: royalblue.
    SourceCode { language: String },
}

/// Single `meta` record at the head of a JSONL file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportMeta {
    pub root: String,
    pub generated_at: String,
    pub schema: u32,
}

/// Per-file row. Mirrors the JSONL `kind: "file"` shape.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct FileStats {
    pub path: String,
    pub language: String,
    pub code: u64,
    pub comments: u64,
    pub blanks: u64,
    pub complexity: u64,
    pub bytes: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mtime: Option<String>,
    /// Source-code-shaped classification axis for the graph's G6
    /// colour scheme. Optional + serde-skipped when None so older
    /// JSONL files (written before this field existed)
    /// load cleanly under the same SCHEMA_VERSION; the field
    /// just defaults to `None` on those rows + the consumer
    /// falls back to inspecting `language` directly.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bucket: Option<FileBucket>,
}

/// Per-language roll-up.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct LanguageStats {
    pub name: String,
    pub files: u64,
    #[serde(default)]
    pub bytes: u64,
    pub code: u64,
    pub comments: u64,
    pub blanks: u64,
    pub complexity: u64,
}

/// Whole-scope roll-up.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct Totals {
    pub files: u64,
    #[serde(default)]
    pub bytes: u64,
    pub code: u64,
    pub comments: u64,
    pub blanks: u64,
    pub complexity: u64,
}

/// Owned report returned by `Index::snapshot`. Plain data; safe
/// to serialize to anything serde supports.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub meta: ReportMeta,
    pub totals: Totals,
    pub by_language: Vec<LanguageStats>,
    pub files: Vec<FileStats>,
    pub cocomo: CocomoSummary,
}

/// Current JSONL schema version. Bump when a non-additive change
/// lands in the on-disk format; `Index::load_jsonl` rejects files
/// stamped with a different value via `ChanReportError::SchemaMismatch`.
pub const SCHEMA_VERSION: u32 = 1;
