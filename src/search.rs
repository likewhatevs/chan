// Search index, BM25-only at v1, backed by tantivy.
//
// Schema (one document per file at v1; per-section chunking is a
// follow-up that adds heading_path and a chunk-id field):
//
//   path     STRING | STORED          POSIX rel path; primary key
//   filename TEXT                     basename stem of path; lets
//                                     "carbonara" match
//                                     `recipes/carbonara.md` even
//                                     when the body never mentions
//                                     it. Not stored: the path
//                                     field already carries the
//                                     display value.
//   title    TEXT   | STORED          top-level h1 if present
//   body     TEXT   | STORED          full file body (post-frontmatter)
//   headings TEXT                     newline-joined heading texts;
//                                     surfaces section titles in
//                                     general search hits without
//                                     duplicating the body. Not
//                                     stored.
//   mtime    I64    | STORED | FAST   Unix seconds; for rebuild
//                                     hints and future filtering
//
// Schema versioning lives in `<index_dir>/.schema_version` (single
// integer, as text). On `Index::open`, a missing or mismatched
// version wipes the index dir before tantivy opens it. This lets us
// add fields, switch tokenizers, or change chunking and have stale
// indexes rebuild on next open without manual intervention.
//
// SCHEMA_VERSION history:
//   1 -> 2  no-op stub to real schema (path/title/body/mtime).
//   2 -> 3  added `filename` and `headings` for the [[ link picker
//           and broader free-text findability.

use std::fs;
use std::path::{Path, PathBuf};
#[cfg(feature = "search")]
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

#[cfg(feature = "search")]
use tantivy::{
    collector::TopDocs,
    doc,
    query::QueryParser,
    schema::{Field, Schema, Value, FAST, STORED, STRING, TEXT},
    snippet::SnippetGenerator,
    Index as TantivyIndex, IndexReader, ReloadPolicy, TantivyDocument, Term,
};

#[cfg(feature = "search")]
use crate::error::ChanError;
use crate::error::Result;

#[cfg(feature = "search")]
const SCHEMA_VERSION: u32 = 3;
#[cfg(feature = "search")]
const VERSION_FILE: &str = ".schema_version";

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
    /// ATX heading stack leading to this snippet. Empty for v1
    /// (per-file granularity). Filled once per-section chunking
    /// lands.
    pub heading_path: Vec<String>,
    /// Excerpt with the match. Plain text; UI renders.
    pub text: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IndexStats {
    pub files_indexed: u32,
    pub files_skipped: u32,
    pub elapsed_ms: u64,
}

/// One document fed into the index. Built by Drive::index_file,
/// passed to `Index::reindex_iter` for bulk rebuilds.
#[derive(Debug, Clone)]
pub struct IndexDoc {
    pub path: String,
    /// File's display title (h1 / frontmatter `title`).
    pub title: Option<String>,
    /// Full file body (post-frontmatter).
    pub body: String,
    /// Last modification time, Unix seconds.
    pub mtime: Option<i64>,
    /// Basename of `path` with the extension stripped. Indexed as
    /// a text field so users typing the filename find the file even
    /// when the body never mentions it.
    pub filename: String,
    /// Newline-joined heading texts. Indexed as a text field so
    /// section titles surface in general queries.
    pub headings: String,
}

/// Index handle. One per Drive open.
#[cfg(feature = "search")]
pub struct Index {
    /// Drive root, kept for diagnostics. Not used at query time;
    /// tantivy works against the index dir we passed to `open`.
    #[allow(dead_code)]
    drive_root: PathBuf,
    inner: TantivyIndex,
    reader: IndexReader,
    /// Serializes writer construction. Tantivy's writer holds a
    /// directory-level lock; this Mutex prevents two threads in
    /// the same process from racing to grab it.
    writer_slot: Mutex<()>,
    schema_fields: SchemaFields,
}

#[cfg(feature = "search")]
#[derive(Clone)]
struct SchemaFields {
    path: Field,
    filename: Field,
    title: Field,
    body: Field,
    headings: Field,
    mtime: Field,
}

#[cfg(feature = "search")]
impl Index {
    pub fn open(drive_root: &Path, index_dir: &Path) -> Result<Self> {
        fs::create_dir_all(index_dir)?;
        ensure_schema_version(index_dir)?;

        let (schema, fields) = build_schema();
        let inner = TantivyIndex::open_or_create(
            tantivy::directory::MmapDirectory::open(index_dir)
                .map_err(|e| ChanError::Search(e.to_string()))?,
            schema,
        )
        .map_err(|e| ChanError::Search(e.to_string()))?;
        let reader = inner
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()
            .map_err(|e| ChanError::Search(e.to_string()))?;
        Ok(Self {
            drive_root: drive_root.to_path_buf(),
            inner,
            reader,
            writer_slot: Mutex::new(()),
            schema_fields: fields,
        })
    }

    /// Replace the document for `rel` in the index. Single commit.
    /// For bulk operations use `reindex_iter`.
    pub fn upsert(&self, doc: &IndexDoc) -> Result<()> {
        let _g = self.writer_slot.lock().unwrap();
        let mut w = self
            .inner
            .writer::<TantivyDocument>(50_000_000)
            .map_err(|e| ChanError::Search(e.to_string()))?;
        write_doc(&mut w, &self.schema_fields, doc);
        w.commit().map_err(|e| ChanError::Search(e.to_string()))?;
        self.reader
            .reload()
            .map_err(|e| ChanError::Search(e.to_string()))?;
        Ok(())
    }

    /// Drop a file from the index.
    pub fn remove(&self, rel: &str) -> Result<()> {
        let _g = self.writer_slot.lock().unwrap();
        let mut w = self
            .inner
            .writer::<TantivyDocument>(50_000_000)
            .map_err(|e| ChanError::Search(e.to_string()))?;
        w.delete_term(Term::from_field_text(self.schema_fields.path, rel));
        w.commit().map_err(|e| ChanError::Search(e.to_string()))?;
        self.reader
            .reload()
            .map_err(|e| ChanError::Search(e.to_string()))?;
        Ok(())
    }

    /// Wipe everything and rebuild from `docs`. Single commit at
    /// the end; tantivy fsyncs once.
    pub fn reindex_iter<I>(&self, docs: I) -> Result<IndexStats>
    where
        I: IntoIterator<Item = IndexDoc>,
    {
        let start = std::time::Instant::now();
        let _g = self.writer_slot.lock().unwrap();
        let mut w = self
            .inner
            .writer::<TantivyDocument>(50_000_000)
            .map_err(|e| ChanError::Search(e.to_string()))?;
        w.delete_all_documents()
            .map_err(|e| ChanError::Search(e.to_string()))?;
        let mut indexed = 0u32;
        for d in docs {
            write_doc(&mut w, &self.schema_fields, &d);
            indexed += 1;
        }
        w.commit().map_err(|e| ChanError::Search(e.to_string()))?;
        self.reader
            .reload()
            .map_err(|e| ChanError::Search(e.to_string()))?;
        Ok(IndexStats {
            files_indexed: indexed,
            files_skipped: 0,
            elapsed_ms: start.elapsed().as_millis() as u64,
        })
    }

    pub fn search(&self, query: &str, opts: &SearchOpts) -> Result<SearchResults> {
        let limit = if opts.limit == 0 { 50 } else { opts.limit } as usize;
        // When scope filtering is on, over-fetch so post-filter has
        // material to return up to `limit` results.
        let fetch = if opts.scope.is_some() {
            limit * 4
        } else {
            limit
        };

        let searcher = self.reader.searcher();
        // Free-text search runs across filename, title, headings,
        // and body. Field boosts could weight filename / title above
        // body for typeahead-style queries; left at 1.0 for v1, the
        // BM25 length normalization already tilts short fields up.
        let qp = QueryParser::for_index(
            &self.inner,
            vec![
                self.schema_fields.filename,
                self.schema_fields.title,
                self.schema_fields.headings,
                self.schema_fields.body,
            ],
        );
        let parsed = qp
            .parse_query(query)
            .map_err(|e| ChanError::Search(e.to_string()))?;
        let top = searcher
            .search(&parsed, &TopDocs::with_limit(fetch))
            .map_err(|e| ChanError::Search(e.to_string()))?;

        let snippet_gen = SnippetGenerator::create(&searcher, &parsed, self.schema_fields.body)
            .map_err(|e| ChanError::Search(e.to_string()))?;

        let mut hits = Vec::with_capacity(limit);
        for (score, addr) in top {
            let doc: TantivyDocument = searcher
                .doc::<TantivyDocument>(addr)
                .map_err(|e| ChanError::Search(e.to_string()))?;
            let path = doc
                .get_first(self.schema_fields.path)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if let Some(scope) = &opts.scope {
                if !path_under(&path, scope) {
                    continue;
                }
            }
            let snippet = snippet_gen.snippet_from_doc(&doc);
            let text = if snippet.is_empty() {
                doc.get_first(self.schema_fields.body)
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .chars()
                    .take(200)
                    .collect()
            } else {
                snippet.to_html()
            };
            hits.push(Hit {
                path,
                score,
                snippets: vec![Snippet {
                    heading_path: Vec::new(),
                    text,
                }],
            });
            if hits.len() >= limit {
                break;
            }
        }

        let total = hits.len() as u32;
        Ok(SearchResults {
            hits,
            total,
            mode_used: SearchMode::Bm25,
        })
    }
}

#[cfg(feature = "search")]
fn build_schema() -> (Schema, SchemaFields) {
    let mut sb = Schema::builder();
    let path = sb.add_text_field("path", STRING | STORED);
    // filename + headings are indexed but not stored: the path
    // field already carries the display value, and storing the
    // joined heading list would duplicate the body.
    let filename = sb.add_text_field("filename", TEXT);
    let title = sb.add_text_field("title", TEXT | STORED);
    let body = sb.add_text_field("body", TEXT | STORED);
    let headings = sb.add_text_field("headings", TEXT);
    let mtime = sb.add_i64_field("mtime", FAST | STORED);
    let schema = sb.build();
    (
        schema,
        SchemaFields {
            path,
            filename,
            title,
            body,
            headings,
            mtime,
        },
    )
}

#[cfg(feature = "search")]
fn write_doc(w: &mut tantivy::IndexWriter<TantivyDocument>, f: &SchemaFields, d: &IndexDoc) {
    w.delete_term(Term::from_field_text(f.path, &d.path));
    let mut td = doc!(
        f.path => d.path.clone(),
        f.filename => d.filename.clone(),
        f.body => d.body.clone(),
        f.headings => d.headings.clone(),
        f.mtime => d.mtime.unwrap_or(0),
    );
    if let Some(t) = &d.title {
        td.add_text(f.title, t);
    }
    let _ = w.add_document(td);
}

#[cfg(feature = "search")]
fn path_under(path: &str, scope: &str) -> bool {
    let scope = scope.trim_matches('/');
    if scope.is_empty() {
        return true;
    }
    let prefix = format!("{scope}/");
    path == scope || path.starts_with(&prefix)
}

/// Read `<index_dir>/.schema_version`. If missing or mismatched,
/// nuke everything in `index_dir` (except the version file itself,
/// which we rewrite). The next caller's tantivy open then sees a
/// clean directory.
#[cfg(feature = "search")]
fn ensure_schema_version(index_dir: &Path) -> Result<()> {
    let vfile = index_dir.join(VERSION_FILE);
    let observed: Option<u32> = fs::read_to_string(&vfile)
        .ok()
        .and_then(|s| s.trim().parse().ok());
    if observed == Some(SCHEMA_VERSION) {
        return Ok(());
    }
    // Wipe everything in the dir, then rewrite the version file.
    if let Ok(rd) = fs::read_dir(index_dir) {
        for entry in rd.flatten() {
            let p = entry.path();
            if p.is_dir() {
                let _ = fs::remove_dir_all(&p);
            } else {
                let _ = fs::remove_file(&p);
            }
        }
    }
    crate::fs_ops::atomic_write(&vfile, SCHEMA_VERSION.to_string().as_bytes())?;
    Ok(())
}

#[cfg(not(feature = "search"))]
pub struct Index {
    #[allow(dead_code)]
    drive_root: PathBuf,
    #[allow(dead_code)]
    index_dir: PathBuf,
}

#[cfg(not(feature = "search"))]
impl Index {
    pub fn open(drive_root: &Path, index_dir: &Path) -> Result<Self> {
        fs::create_dir_all(index_dir)?;
        Ok(Self {
            drive_root: drive_root.to_path_buf(),
            index_dir: index_dir.to_path_buf(),
        })
    }

    pub fn upsert(&self, _doc: &IndexDoc) -> Result<()> {
        Ok(())
    }

    pub fn remove(&self, _rel: &str) -> Result<()> {
        Ok(())
    }

    pub fn reindex_iter<I>(&self, _docs: I) -> Result<IndexStats>
    where
        I: IntoIterator<Item = IndexDoc>,
    {
        Ok(IndexStats::default())
    }

    pub fn search(&self, _query: &str, opts: &SearchOpts) -> Result<SearchResults> {
        Ok(SearchResults {
            hits: Vec::new(),
            total: 0,
            mode_used: opts.mode,
        })
    }
}

#[cfg(all(test, feature = "search"))]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn path_under_matches_prefix() {
        assert!(path_under("recipes/pasta.md", "recipes"));
        assert!(path_under("recipes", "recipes"));
        assert!(!path_under("recipes-old/pasta.md", "recipes"));
        assert!(path_under("notes/x.md", ""));
    }

    fn doc(path: &str, title: Option<&str>, body: &str) -> IndexDoc {
        IndexDoc {
            path: path.to_string(),
            title: title.map(str::to_owned),
            body: body.to_string(),
            mtime: None,
            filename: std::path::Path::new(path)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string(),
            headings: String::new(),
        }
    }

    #[test]
    fn upsert_then_search_finds_doc() {
        let drive_root = TempDir::new().unwrap();
        let idx_dir = TempDir::new().unwrap();
        let idx = Index::open(drive_root.path(), idx_dir.path()).unwrap();
        idx.upsert(&doc(
            "intro.md",
            Some("Hello"),
            "Welcome to the carbonara recipe collection.",
        ))
        .unwrap();
        let res = idx.search("carbonara", &SearchOpts::default()).unwrap();
        assert_eq!(res.hits.len(), 1);
        assert_eq!(res.hits[0].path, "intro.md");
    }

    #[test]
    fn remove_drops_doc() {
        let drive_root = TempDir::new().unwrap();
        let idx_dir = TempDir::new().unwrap();
        let idx = Index::open(drive_root.path(), idx_dir.path()).unwrap();
        idx.upsert(&doc("a.md", None, "carbonara")).unwrap();
        idx.remove("a.md").unwrap();
        let res = idx.search("carbonara", &SearchOpts::default()).unwrap();
        assert_eq!(res.hits.len(), 0);
    }

    #[test]
    fn scope_filters_results() {
        let drive_root = TempDir::new().unwrap();
        let idx_dir = TempDir::new().unwrap();
        let idx = Index::open(drive_root.path(), idx_dir.path()).unwrap();
        for (path, body) in [
            ("recipes/pasta.md", "carbonara"),
            ("notes/cooking.md", "carbonara"),
        ] {
            idx.upsert(&doc(path, None, body)).unwrap();
        }
        let opts = SearchOpts {
            scope: Some("recipes".into()),
            ..Default::default()
        };
        let res = idx.search("carbonara", &opts).unwrap();
        assert_eq!(res.hits.len(), 1);
        assert_eq!(res.hits[0].path, "recipes/pasta.md");
    }

    #[test]
    fn search_matches_filename_without_body_mention() {
        // The body never mentions "carbonara"; the filename does.
        // The filename field is what makes the hit possible.
        let drive_root = TempDir::new().unwrap();
        let idx_dir = TempDir::new().unwrap();
        let idx = Index::open(drive_root.path(), idx_dir.path()).unwrap();
        idx.upsert(&doc(
            "recipes/carbonara.md",
            Some("Pasta night"),
            "tonight we tried something different.",
        ))
        .unwrap();
        let res = idx.search("carbonara", &SearchOpts::default()).unwrap();
        assert_eq!(res.hits.len(), 1);
        assert_eq!(res.hits[0].path, "recipes/carbonara.md");
    }

    #[test]
    fn search_matches_heading_text() {
        let drive_root = TempDir::new().unwrap();
        let idx_dir = TempDir::new().unwrap();
        let idx = Index::open(drive_root.path(), idx_dir.path()).unwrap();
        let mut d = doc("notes/2026-05-06.md", Some("Pasta night"), "the body.");
        d.headings = "Intro\nCarbonara variant\nLeftovers".into();
        idx.upsert(&d).unwrap();
        let res = idx.search("variant", &SearchOpts::default()).unwrap();
        assert_eq!(res.hits.len(), 1);
        assert_eq!(res.hits[0].path, "notes/2026-05-06.md");
    }

    #[test]
    fn schema_version_wipe_rebuilds_clean() {
        let drive_root = TempDir::new().unwrap();
        let idx_dir = TempDir::new().unwrap();
        // Open + write a doc.
        {
            let idx = Index::open(drive_root.path(), idx_dir.path()).unwrap();
            idx.upsert(&doc("a.md", None, "stale data")).unwrap();
        }
        // Bump the on-disk version to something else, simulating a
        // schema migration.
        std::fs::write(idx_dir.path().join(VERSION_FILE), "999").unwrap();
        // Re-open: should wipe and start clean.
        let idx = Index::open(drive_root.path(), idx_dir.path()).unwrap();
        let res = idx.search("stale", &SearchOpts::default()).unwrap();
        assert_eq!(res.hits.len(), 0);
    }
}
