// BM25 side of the search index, backed by tantivy. Owns the
// `bm25/` subdirectory under the per-drive index dir (resolved
// by `crate::paths::drive_paths`).
//
// Each indexed unit is one chunk (see `index::chunking`). We delete
// by `path` so re-indexing a file is "remove all its chunks, then
// add the new ones" without tracking individual chunk ids.
//
// The body field is STORED so snippet generation is self-contained
// (no second read of the source file at query time). Index size
// roughly doubles vs not storing; for personal-notes corpora that's
// still tiny (~MBs).

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::Serialize;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::{Field, Schema, SchemaBuilder, Value, FAST, INDEXED, STORED, STRING, TEXT};
use tantivy::snippet::SnippetGenerator;
use tantivy::{doc, Index, IndexReader, IndexWriter, ReloadPolicy, TantivyDocument, Term};
use thiserror::Error;

use super::chunking::{self, Chunk};
use super::config::Chunking;

/// Memory budget per writer batch. tantivy's recommendation is
/// 50 MB minimum; our corpora are small so this is more than enough.
const WRITER_BUDGET: usize = 50_000_000;

/// One search hit. Field naming matches the API response shape so
/// the server layer can serialize these directly.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Hit {
    pub path: String,
    pub chunk_id: String,
    pub heading: String,
    pub start_line: u64,
    pub snippet: String,
    pub score: f32,
}

#[derive(Debug, Error)]
pub enum Bm25Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    OpenDir(#[from] tantivy::directory::error::OpenDirectoryError),
    #[error(transparent)]
    Tantivy(#[from] tantivy::TantivyError),
    #[error("query parse: {0}")]
    QueryParse(#[from] tantivy::query::QueryParserError),
}

/// Field handles, kept together so we don't pluck them out of the
/// schema on every operation.
struct Fields {
    path: Field,
    chunk_id: Field,
    heading: Field,
    body: Field,
    start_line: Field,
    depth: Field,
}

/// BM25 index. Holds an open writer (single-thread per tantivy
/// contract; Mutex-wrapped so the rest of the codebase can share a
/// `&Bm25Index`) and a shareable reader.
pub struct Bm25Index {
    index: Index,
    writer: Mutex<IndexWriter>,
    reader: IndexReader,
    fields: Fields,
}

impl std::fmt::Debug for Bm25Index {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Bm25Index").finish()
    }
}

impl Bm25Index {
    /// Open the BM25 index under `index_dir`, creating it on first
    /// use. We manage its `bm25/` subdir.
    pub fn open(index_dir: &Path) -> Result<Self, Bm25Error> {
        let dir = bm25_dir(index_dir);
        std::fs::create_dir_all(&dir)?;
        let schema = build_schema();
        let fields = pluck_fields(&schema);
        // tantivy's open_or_create errors if the dir was used with a
        // different schema. We don't migrate yet (schema_version
        // bump => full rebuild); the caller (index::Index) is
        // responsible for clearing the dir before bumps.
        let index = Index::open_or_create(tantivy::directory::MmapDirectory::open(&dir)?, schema)?;
        let writer = index.writer(WRITER_BUDGET)?;
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()?;
        Ok(Self {
            index,
            writer: Mutex::new(writer),
            reader,
            fields,
        })
    }

    /// Erase every document for `rel_path`. Idempotent.
    pub fn delete_file(&self, rel_path: &str) -> Result<(), Bm25Error> {
        let writer = self.writer.lock().unwrap();
        writer.delete_term(Term::from_field_text(self.fields.path, rel_path));
        Ok(())
    }

    /// Re-index a single file. Deletes its previous chunks and writes
    /// new ones according to `chunking`. Caller commits.
    pub fn index_file(
        &self,
        rel_path: &str,
        content: &str,
        chunking: &Chunking,
    ) -> Result<usize, Bm25Error> {
        self.delete_file(rel_path)?;
        let chunks = chunking::chunk(content, chunking);
        if chunks.is_empty() {
            return Ok(0);
        }
        let writer = self.writer.lock().unwrap();
        for c in &chunks {
            self.add_chunk(&writer, rel_path, c)?;
        }
        Ok(chunks.len())
    }

    fn add_chunk(&self, writer: &IndexWriter, rel_path: &str, c: &Chunk) -> Result<(), Bm25Error> {
        writer.add_document(doc!(
            self.fields.path => rel_path,
            self.fields.chunk_id => c.id.as_str(),
            self.fields.heading => c.heading.as_str(),
            self.fields.body => c.body.as_str(),
            self.fields.start_line => c.start_line as u64,
            self.fields.depth => u64::from(c.depth),
        ))?;
        Ok(())
    }

    /// Flush pending writes. Call after a batch of `index_file` /
    /// `delete_file`. Synchronously reloads the reader so subsequent
    /// `search` calls see the new data; tantivy's default reload
    /// policy is async, which is wrong for our caller's expectations.
    pub fn commit(&self) -> Result<(), Bm25Error> {
        let mut writer = self.writer.lock().unwrap();
        writer.commit()?;
        self.reader.reload()?;
        Ok(())
    }

    /// Search `body` + `heading` with BM25 ranking. Returns up to
    /// `limit` hits.
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<Hit>, Bm25Error> {
        let searcher = self.reader.searcher();
        let parser =
            QueryParser::for_index(&self.index, vec![self.fields.body, self.fields.heading]);
        let parsed = parser.parse_query(query)?;
        let top = searcher.search(&parsed, &TopDocs::with_limit(limit))?;
        let snippet_gen = SnippetGenerator::create(&searcher, &parsed, self.fields.body)?;
        let mut hits = Vec::with_capacity(top.len());
        for (score, doc_addr) in top {
            let doc: TantivyDocument = searcher.doc(doc_addr)?;
            let snippet = snippet_gen.snippet_from_doc(&doc).to_html();
            hits.push(Hit {
                path: get_text(&doc, self.fields.path),
                chunk_id: get_text(&doc, self.fields.chunk_id),
                heading: get_text(&doc, self.fields.heading),
                start_line: get_u64(&doc, self.fields.start_line),
                snippet,
                score,
            });
        }
        Ok(hits)
    }

    /// Total indexed-document count. Used by the status endpoint.
    pub fn doc_count(&self) -> u64 {
        self.reader.searcher().num_docs()
    }
}

fn bm25_dir(index_dir: &Path) -> PathBuf {
    index_dir.join("bm25")
}

fn build_schema() -> Schema {
    let mut sb = SchemaBuilder::default();
    // STRING (single-token, exact-match) is what we need for delete-
    // by-path. INDEXED gives us the term lookup; STORED so we can
    // return it in hits; FAST in case we want to facet by file later.
    sb.add_text_field("path", STRING | STORED | FAST);
    sb.add_text_field("chunk_id", STRING | STORED);
    sb.add_text_field("heading", TEXT | STORED);
    sb.add_text_field("body", TEXT | STORED);
    sb.add_u64_field("start_line", INDEXED | STORED);
    sb.add_u64_field("depth", INDEXED | STORED);
    sb.build()
}

fn pluck_fields(schema: &Schema) -> Fields {
    Fields {
        path: schema.get_field("path").unwrap(),
        chunk_id: schema.get_field("chunk_id").unwrap(),
        heading: schema.get_field("heading").unwrap(),
        body: schema.get_field("body").unwrap(),
        start_line: schema.get_field("start_line").unwrap(),
        depth: schema.get_field("depth").unwrap(),
    }
}

fn get_text(doc: &TantivyDocument, field: Field) -> String {
    doc.get_first(field)
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_owned()
}

fn get_u64(doc: &TantivyDocument, field: Field) -> u64 {
    doc.get_first(field).and_then(|v| v.as_u64()).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn fresh() -> (TempDir, Bm25Index) {
        let tmp = TempDir::new().unwrap();
        let idx = Bm25Index::open(tmp.path()).unwrap();
        (tmp, idx)
    }

    #[test]
    fn empty_index_returns_no_hits() {
        let (_tmp, idx) = fresh();
        let hits = idx.search("anything", 10).unwrap();
        assert!(hits.is_empty());
        assert_eq!(idx.doc_count(), 0);
    }

    #[test]
    fn index_then_search() {
        let (_tmp, idx) = fresh();
        let body =
            "# notes\n\nThis page talks about apples and bananas.\n\n## fruit\nMore on apples.\n";
        idx.index_file("notes.md", body, &Chunking::Headings)
            .unwrap();
        idx.commit().unwrap();
        let hits = idx.search("apples", 10).unwrap();
        assert!(!hits.is_empty());
        assert!(hits.iter().all(|h| h.path == "notes.md"));
        assert_eq!(hits.len(), 2);
    }

    #[test]
    fn re_index_replaces_old_chunks() {
        let (_tmp, idx) = fresh();
        idx.index_file("a.md", "# old\nfoo bar\n", &Chunking::Headings)
            .unwrap();
        idx.commit().unwrap();
        assert!(!idx.search("foo", 10).unwrap().is_empty());

        idx.index_file("a.md", "# new\nbaz qux\n", &Chunking::Headings)
            .unwrap();
        idx.commit().unwrap();
        assert!(idx.search("foo", 10).unwrap().is_empty());
        assert!(!idx.search("baz", 10).unwrap().is_empty());
    }

    #[test]
    fn delete_file_removes_chunks() {
        let (_tmp, idx) = fresh();
        idx.index_file("a.md", "# h\nunique-term\n", &Chunking::Headings)
            .unwrap();
        idx.commit().unwrap();
        idx.delete_file("a.md").unwrap();
        idx.commit().unwrap();
        assert!(idx.search("unique-term", 10).unwrap().is_empty());
    }

    #[test]
    fn snippet_highlights_match() {
        let (_tmp, idx) = fresh();
        idx.index_file(
            "a.md",
            "# top\nThe quick brown fox jumps over the lazy dog.\n",
            &Chunking::Headings,
        )
        .unwrap();
        idx.commit().unwrap();
        let hits = idx.search("fox", 10).unwrap();
        assert_eq!(hits.len(), 1);
        assert!(hits[0].snippet.contains("<b>fox</b>"));
    }
}
