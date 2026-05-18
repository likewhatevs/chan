// chan-report public surface.
//
// Walks a directory, counts SLOC per file by language, computes
// per-language roll-ups and a COCOMO summary, and maintains the
// state incrementally so a single file change does not require a
// full rescan. The crate is I/O-free for state: callers
// (chan-drive) own persistence and call write_jsonl / load_jsonl
// when they decide to materialize a snapshot to disk.

#![forbid(unsafe_code)]

mod cocomo;
mod complexity;
mod count;
mod error;
mod jsonl;
mod summary;
mod walk;

pub use cocomo::{CocomoModel, CocomoParams, CocomoSummary};
pub use error::ChanReportError;
pub use jsonl::report_to_jsonl_string;
pub use summary::{FileStats, LanguageStats, Report, ReportMeta, Totals, SCHEMA_VERSION};

use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

use chrono::Utc;

use crate::walk::Filter;

/// Inputs to the initial walk and to the filter applied by
/// incremental updates.
#[derive(Debug, Clone)]
pub struct ReportOptions {
    pub root: PathBuf,
    pub follow_symlinks: bool,
    pub include_hidden: bool,
    pub respect_gitignore: bool,
    pub exclude_globs: Vec<String>,
    pub cocomo: CocomoParams,
}

impl ReportOptions {
    /// Construct with defaults: don't follow symlinks, skip
    /// hidden files, respect `.gitignore`, no extra exclude
    /// globs, organic-COCOMO cost model.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            follow_symlinks: false,
            include_hidden: false,
            respect_gitignore: true,
            exclude_globs: Vec::new(),
            cocomo: CocomoParams::default(),
        }
    }
}

/// What subset of the index a snapshot should roll up.
#[derive(Debug, Clone)]
pub enum Scope {
    /// Whole indexed root.
    All,
    /// Drive-relative POSIX prefix; matches the directory and
    /// everything under it. Empty string is equivalent to `All`.
    Prefix(String),
    /// Exact relative paths. Missing entries are silently ignored.
    Files(Vec<String>),
}

/// Result of an incremental mutation, used by the consumer to
/// decide whether the persisted JSONL needs rewriting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateOutcome {
    Inserted,
    Updated,
    Unchanged,
    Removed,
    Skipped,
}

/// In-memory per-file state plus the cached accept-filter. Single
/// owner per drive; chan-drive wraps this in its own lock.
pub struct Index {
    root: PathBuf,
    files: HashMap<String, FileStats>,
    filter: Filter,
}

impl Index {
    /// Walk `opts.root` and produce an index. Files the walker
    /// or counter rejects (unrecognized extension, binary,
    /// gitignored, hidden, oversize) are silently dropped.
    pub fn scan(opts: &ReportOptions) -> Result<Self, ChanReportError> {
        let rels = walk::walk_root(opts)?;
        let mut files = HashMap::with_capacity(rels.len());
        for rel in rels {
            if let Some(fs) = count::count_file_impl(&opts.root, &rel)? {
                files.insert(rel, fs);
            }
        }
        Ok(Self {
            root: opts.root.clone(),
            filter: Filter::build(opts)?,
            files,
        })
    }

    /// Re-count `rel` from disk and reconcile against the index.
    /// `rel` is drive-relative POSIX. Returns:
    ///
    ///   - `Inserted` when the file is new and the filter accepts it.
    ///   - `Updated` when stats changed.
    ///   - `Unchanged` when stats are byte-identical.
    ///   - `Removed` when the file vanished or the filter no
    ///     longer accepts it (and we held a row for it).
    ///   - `Skipped` when the filter rejected the path and we did
    ///     not hold a row for it.
    pub fn update(&mut self, rel: &str) -> Result<UpdateOutcome, ChanReportError> {
        if !self.filter.accepts(rel) {
            return Ok(if self.files.remove(rel).is_some() {
                UpdateOutcome::Removed
            } else {
                UpdateOutcome::Skipped
            });
        }
        match count::count_file_impl(&self.root, rel)? {
            Some(new_stats) => match self.files.get(rel) {
                Some(old) if old == &new_stats => Ok(UpdateOutcome::Unchanged),
                Some(_) => {
                    self.files.insert(rel.to_string(), new_stats);
                    Ok(UpdateOutcome::Updated)
                }
                None => {
                    self.files.insert(rel.to_string(), new_stats);
                    Ok(UpdateOutcome::Inserted)
                }
            },
            None => Ok(if self.files.remove(rel).is_some() {
                UpdateOutcome::Removed
            } else {
                UpdateOutcome::Skipped
            }),
        }
    }

    /// Drop a row unconditionally. `Removed` when a row existed,
    /// `Unchanged` otherwise. Does not touch disk.
    pub fn remove(&mut self, rel: &str) -> UpdateOutcome {
        if self.files.remove(rel).is_some() {
            UpdateOutcome::Removed
        } else {
            UpdateOutcome::Unchanged
        }
    }

    /// Treat a filesystem rename as `remove(from)` plus
    /// `update(to)`. Returns the outcome of the update half;
    /// callers infer "something happened" from this being
    /// non-`Unchanged` or from a prior row existing at `from`.
    pub fn rename(&mut self, from: &str, to: &str) -> Result<UpdateOutcome, ChanReportError> {
        let removed = self.files.remove(from).is_some();
        let out = self.update(to)?;
        // If we removed something at `from` but the destination
        // ended up `Unchanged` / `Skipped`, force at least
        // `Removed` so the writer flushes.
        if removed && matches!(out, UpdateOutcome::Unchanged | UpdateOutcome::Skipped) {
            return Ok(UpdateOutcome::Removed);
        }
        Ok(out)
    }

    /// Borrow the row for a specific file, if tracked.
    pub fn file(&self, rel: &str) -> Option<&FileStats> {
        self.files.get(rel)
    }

    /// Number of tracked files.
    pub fn len(&self) -> usize {
        self.files.len()
    }

    /// True when no files are tracked.
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// Build a `Report` covering the requested scope. Pure
    /// projection; does not mutate state.
    pub fn snapshot(&self, scope: &Scope, cocomo_params: &CocomoParams) -> Report {
        let files: Vec<FileStats> = match scope {
            Scope::All => self.files.values().cloned().collect(),
            Scope::Prefix(p) if p.is_empty() => self.files.values().cloned().collect(),
            Scope::Prefix(p) => {
                let needle = if p.ends_with('/') {
                    p.clone()
                } else {
                    format!("{}/", p)
                };
                self.files
                    .iter()
                    .filter(|(k, _)| *k == p || k.starts_with(&needle))
                    .map(|(_, v)| v.clone())
                    .collect()
            }
            Scope::Files(list) => list
                .iter()
                .filter_map(|p| self.files.get(p).cloned())
                .collect(),
        };

        let (by_language, totals) = roll_up(&files);
        let cocomo = cocomo::compute(totals.code, cocomo_params);

        let mut files_sorted = files;
        files_sorted.sort_by(|a, b| a.path.cmp(&b.path));

        Report {
            meta: ReportMeta {
                root: self.root.display().to_string(),
                generated_at: Utc::now().to_rfc3339(),
                schema: summary::SCHEMA_VERSION,
            },
            totals,
            by_language,
            files: files_sorted,
            cocomo,
        }
    }

    /// Serialize a scoped snapshot as JSONL to `w`. Consumers
    /// pair this with their own atomic-write helper to persist
    /// to disk.
    pub fn write_jsonl<W: Write>(
        &self,
        w: W,
        scope: &Scope,
        cocomo_params: &CocomoParams,
    ) -> Result<(), ChanReportError> {
        let report = self.snapshot(scope, cocomo_params);
        jsonl::write_report(
            w,
            &report.meta,
            &report.files,
            &report.by_language,
            &report.totals,
            &report.cocomo,
        )
    }

    /// Reconstruct an `Index` from a previously written JSONL
    /// stream. `opts` provides the live filter; the schema field
    /// in the loaded `meta` record must match the current build.
    pub fn load_jsonl<R: BufRead>(r: R, opts: &ReportOptions) -> Result<Self, ChanReportError> {
        let (meta, files) = jsonl::read_file_rows(r)?;
        if meta.schema != summary::SCHEMA_VERSION {
            return Err(ChanReportError::SchemaMismatch {
                expected: summary::SCHEMA_VERSION,
                found: meta.schema,
            });
        }
        let mut map = HashMap::with_capacity(files.len());
        for f in files {
            map.insert(f.path.clone(), f);
        }
        Ok(Self {
            root: opts.root.clone(),
            filter: Filter::build(opts)?,
            files: map,
        })
    }
}

/// One-shot helper for the common "scan once, get a report" flow.
/// Equivalent to `Index::scan(opts)?.snapshot(&Scope::All, &opts.cocomo)`.
pub fn run(opts: &ReportOptions) -> Result<Report, ChanReportError> {
    let idx = Index::scan(opts)?;
    Ok(idx.snapshot(&Scope::All, &opts.cocomo))
}

/// Stateless per-file count. Shared by `Index::scan` (parallel
/// over walked files) and `Index::update` (single file). Returns
/// `None` for files the counter skips: unrecognized extension,
/// binary, excluded by the walker's filter, or vanished.
pub fn count_file(root: &Path, rel: &str) -> Result<Option<FileStats>, ChanReportError> {
    count::count_file_impl(root, rel)
}

fn roll_up(files: &[FileStats]) -> (Vec<LanguageStats>, Totals) {
    let mut by_lang: HashMap<String, LanguageStats> = HashMap::new();
    let mut totals = Totals::default();
    for f in files {
        let entry = by_lang
            .entry(f.language.clone())
            .or_insert_with(|| LanguageStats {
                name: f.language.clone(),
                ..Default::default()
            });
        entry.files += 1;
        entry.bytes += f.bytes;
        entry.code += f.code;
        entry.comments += f.comments;
        entry.blanks += f.blanks;
        entry.complexity += f.complexity;

        totals.files += 1;
        totals.bytes += f.bytes;
        totals.code += f.code;
        totals.comments += f.comments;
        totals.blanks += f.blanks;
        totals.complexity += f.complexity;
    }
    let mut by_language: Vec<LanguageStats> = by_lang.into_values().collect();
    by_language.sort_by(|a, b| {
        b.bytes
            .cmp(&a.bytes)
            .then_with(|| b.files.cmp(&a.files))
            .then_with(|| a.name.cmp(&b.name))
    });
    (by_language, totals)
}
