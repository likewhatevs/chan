use serde::{Deserialize, Serialize};

use crate::cocomo::CocomoSummary;

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
}

/// Per-language roll-up.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct LanguageStats {
    pub name: String,
    pub files: u64,
    pub code: u64,
    pub comments: u64,
    pub blanks: u64,
    pub complexity: u64,
}

/// Whole-scope roll-up.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct Totals {
    pub files: u64,
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
