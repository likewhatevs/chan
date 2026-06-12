//! End-to-end indexing benchmark.
//!
//! Measures wall-clock time to index a realistic workspace END-TO-END in two
//! modes, timing the structural index and the chan-report scan separately
//! so chan-report's marginal cost is isolated (not inferred from the gap
//! between two whole reindexes, which would also fold in filesystem-cache
//! warmth). Mode 1 (WITHOUT chan-report) is the structural index only
//! (graph rebuild + BM25 build_all) via `Workspace::reindex`. Mode 2 (WITH
//! chan-report) is the same structural reindex, then the chan-report
//! language analysis full scan (SLOC / language / COCOMO) forced via
//! `Workspace::report()` after `set_reports_enabled(true)`, timed on its own.
//!
//! EMBEDDINGS: this benchmark MUST run with the `embeddings` feature
//! compiled OUT (`--no-default-features`). `Workspace::reindex` builds with
//! `BuildOptions::default()` (`include_vectors = true`) and there is no
//! public Workspace API to reindex BM25-only, so with the feature ON and a
//! bge model present in the per-machine cache the reindex runs candle
//! inference on every chunk, which dominates the wall time, hiding the
//! structural plus chan-report cost this benchmark is meant to measure
//! (embeddings must stay disabled entirely). With the feature OFF the
//! embed code is absent entirely, so `index_stats().indexed_vectors == 0`
//! holds by construction. The test ASSERTS that, and FAILS loudly if run
//! with embeddings on against a machine that has a cached model, so you
//! cannot accidentally measure the wrong thing.
//!
//! IGNORED by default so it never runs in CI (it copies a real repo tree).
//! Run it explicitly with embeddings off:
//!
//!   cargo test -p chan-workspace --no-default-features --test index_bench \
//!     -- --ignored --nocapture
//!
//! `CHAN_BENCH_REPO=/path/to/a/repo` overrides the workspace source; with no
//! env var it defaults to the chan-workspace crate's own repo root (the
//! workspace this test ships in), benchmarking a filtered copy of THIS
//! repo as the test workspace.
//!
//! `CHAN_BENCH_MAX_FILES` caps how many tracked files are copied (default
//! 250, `0` = the whole tree). With embeddings off (as required) even the
//! whole ~1370-file tree reindexes in a few seconds, so `0` is the
//! representative run; the cap exists only to bound a run further if you
//! want.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use chan_workspace::Library;
use tempfile::TempDir;

/// Resolve the repo to benchmark: `CHAN_BENCH_REPO` if set, else walk up
/// from the crate dir to the repo root (the dir containing `.git`).
fn bench_repo() -> PathBuf {
    if let Ok(p) = std::env::var("CHAN_BENCH_REPO") {
        return PathBuf::from(p);
    }
    // CARGO_MANIFEST_DIR is .../crates/chan-workspace; the repo root is two up.
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    while !dir.join(".git").exists() {
        if !dir.pop() {
            panic!("could not locate a .git repo root above CARGO_MANIFEST_DIR");
        }
    }
    dir
}

/// Cap on how many git-tracked files to copy into the test workspace.
/// `CHAN_BENCH_MAX_FILES` overrides; 0 means "no cap (the whole repo)".
///
/// Default 250 indexes a realistic, reproducible slice of THIS repo. With
/// embeddings off (as this benchmark requires) the whole ~1370-file tree
/// (~9.5 MB of markdown journals plus the TS/Rust/Svelte source) also
/// reindexes in a few seconds, so `CHAN_BENCH_MAX_FILES=0` is the
/// representative run; the cap just bounds it further if wanted.
fn max_files() -> usize {
    std::env::var("CHAN_BENCH_MAX_FILES")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(250)
}

/// Make a SHALLOW copy of `repo` into `dest`: only git-tracked files, so
/// `target/`, `.git/` internals, and anything gitignored are excluded.
/// This honors the same spirit as the unified ignore set (the index then
/// also prunes node_modules/target/venv/etc via WalkFilter, but tracked
/// files already exclude the big build dirs). Copies at most `cap` files
/// (0 = no cap) so a run stays bounded; returns the file count copied.
fn copy_tracked_files(repo: &Path, dest: &Path, cap: usize) -> usize {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["ls-files", "-z"])
        .output()
        .expect("git ls-files");
    assert!(out.status.success(), "git ls-files failed");
    let mut count = 0usize;
    for rel in out.stdout.split(|&b| b == 0) {
        if rel.is_empty() {
            continue;
        }
        if cap != 0 && count >= cap {
            break;
        }
        let rel = Path::new(std::str::from_utf8(rel).expect("utf8 path"));
        let src = repo.join(rel);
        // Skip anything not a regular file (submodule gitlinks, etc).
        let Ok(meta) = std::fs::symlink_metadata(&src) else {
            continue;
        };
        if !meta.file_type().is_file() {
            continue;
        }
        let dst = dest.join(rel);
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        if std::fs::copy(&src, &dst).is_ok() {
            count += 1;
        }
    }
    count
}

/// One end-to-end index of a freshly-opened workspace over `workspace_root`.
/// The reindex (structural: graph rebuild + BM25 build_all) and the
/// chan-report language scan are timed SEPARATELY so chan-report's cost
/// is isolated rather than inferred from the gap between two whole
/// reindexes (which would also fold in filesystem-cache warmth
/// differences). `report_ms` is `None` when `with_report` is false.
struct IndexResult {
    files: usize,
    indexed_md: usize,
    reindex_ms: u128,
    report_ms: Option<u128>,
    indexed_vectors: u64,
}

fn index_once(cfg_dir: &Path, workspace_root: &Path, with_report: bool) -> IndexResult {
    let lib = Library::open_at(cfg_dir.join("config.toml")).unwrap();
    lib.register_workspace(workspace_root).unwrap();
    let workspace = lib.open_workspace(workspace_root).unwrap();

    if with_report {
        workspace.set_reports_enabled(true).unwrap();
    }

    // Structural index: this is the same work in both modes.
    let start = Instant::now();
    let summary = workspace.reindex(None).unwrap();
    let reindex_ms = start.elapsed().as_millis();

    // chan-report initial full scan (lazy on first call). Timed on its
    // own so the WITH-report number is the marginal chan-report cost,
    // not a second full reindex.
    let report_ms = if with_report {
        let rs = Instant::now();
        let _ = workspace.report().unwrap();
        Some(rs.elapsed().as_millis())
    } else {
        None
    };

    let stats = workspace.index_stats().unwrap();
    IndexResult {
        files: summary.files,
        indexed_md: summary.indexed,
        reindex_ms,
        report_ms,
        indexed_vectors: stats.indexed_vectors,
    }
}

#[test]
#[ignore = "benchmark: copies a real repo tree; run with --ignored --nocapture"]
fn end_to_end_index_with_and_without_report() {
    let repo = bench_repo();
    eprintln!("[bench] repo = {}", repo.display());

    // Shallow copy once; reuse the same on-disk tree for both modes (each
    // mode opens its OWN workspace over a fresh COPY so neither warms the
    // other's index/report state).
    let staging = TempDir::new().unwrap();
    let src_tree = staging.path().join("tree");
    std::fs::create_dir_all(&src_tree).unwrap();
    let cap = max_files();
    let copied = copy_tracked_files(&repo, &src_tree, cap);
    eprintln!(
        "[bench] copied {copied} git-tracked files into the test workspace (cap={})",
        if cap == 0 {
            "none".to_string()
        } else {
            cap.to_string()
        }
    );
    assert!(
        copied > 50,
        "expected a non-trivial repo; got {copied} files"
    );

    // Mode 1: WITHOUT chan-report (structural index only).
    let cfg1 = TempDir::new().unwrap();
    let workspace1 = staging.path().join("workspace-noreport");
    fs_copy_dir(&src_tree, &workspace1);
    let r1 = index_once(cfg1.path(), &workspace1, false);

    // Mode 2: WITH chan-report (structural index + language analysis).
    let cfg2 = TempDir::new().unwrap();
    let workspace2 = staging.path().join("workspace-report");
    fs_copy_dir(&src_tree, &workspace2);
    let r2 = index_once(cfg2.path(), &workspace2, true);

    // Embeddings must NOT have run in either mode.
    assert_eq!(
        r1.indexed_vectors, 0,
        "WITHOUT-report run embedded vectors; expected 0"
    );
    assert_eq!(
        r2.indexed_vectors, 0,
        "WITH-report run embedded vectors; expected 0"
    );

    let report_ms = r2.report_ms.unwrap_or(0);
    let with_total = r2.reindex_ms + report_ms;
    eprintln!("\n=== END-TO-END INDEXING BENCHMARK (bge embeddings DISABLED) ===");
    eprintln!("repo files copied (git-tracked): {copied}");
    eprintln!(
        "WITHOUT chan-report: files={} indexed_md={} vectors={}  reindex={} ms",
        r1.files, r1.indexed_md, r1.indexed_vectors, r1.reindex_ms
    );
    eprintln!(
        "WITH    chan-report: files={} indexed_md={} vectors={}  reindex={} ms + report={} ms = {} ms",
        r2.files, r2.indexed_md, r2.indexed_vectors, r2.reindex_ms, report_ms, with_total
    );
    eprintln!(
        "chan-report marginal cost: {} ms ({:.2}x of the structural reindex)",
        report_ms,
        report_ms as f64 / r1.reindex_ms.max(1) as f64
    );
    eprintln!("================================================================\n");
}

/// Recursive directory copy of regular files (the staged tree is already
/// filtered to tracked files, so no special-file handling needed).
fn fs_copy_dir(src: &Path, dst: &Path) {
    std::fs::create_dir_all(dst).unwrap();
    for entry in std::fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let ft = entry.file_type().unwrap();
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if ft.is_dir() {
            fs_copy_dir(&from, &to);
        } else if ft.is_file() {
            std::fs::copy(&from, &to).unwrap();
        }
    }
}
