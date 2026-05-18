// End-to-end coverage: scan, incremental update, JSONL round-trip,
// scope filtering. Uses tempdirs so nothing touches the developer's
// real filesystem.

use std::fs;
use std::io::Cursor;

use chan_report::{
    CocomoParams, Index, Report, ReportOptions, Scope, UpdateOutcome, SCHEMA_VERSION,
};

use tempfile::tempdir;

fn write(root: &std::path::Path, rel: &str, content: &str) {
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}

#[test]
fn scan_picks_up_known_languages() {
    let d = tempdir().unwrap();
    write(
        d.path(),
        "main.rs",
        "fn main() {\n    if true { println!(); }\n}\n",
    );
    write(d.path(), "notes.md", "# heading\n\ntext.\n");
    write(d.path(), "tool.py", "def foo():\n    return 1\n");

    let idx = Index::scan(&ReportOptions::new(d.path())).unwrap();
    assert_eq!(idx.len(), 3, "expected three counted files");

    let rs = idx.file("main.rs").expect("main.rs tracked");
    assert_eq!(rs.language, "Rust");
    assert!(rs.code >= 1);
    assert!(rs.complexity >= 1, "if-branch should contribute");
}

#[test]
fn update_outcomes_are_distinct() {
    let d = tempdir().unwrap();
    write(d.path(), "a.rs", "fn a() {}\n");
    let mut idx = Index::scan(&ReportOptions::new(d.path())).unwrap();

    write(d.path(), "b.rs", "fn b() {}\n");
    assert_eq!(idx.update("b.rs").unwrap(), UpdateOutcome::Inserted);

    // Re-running with no on-disk change should be Unchanged.
    assert_eq!(idx.update("a.rs").unwrap(), UpdateOutcome::Unchanged);

    write(d.path(), "a.rs", "fn a() { let _ = 1; }\n");
    assert_eq!(idx.update("a.rs").unwrap(), UpdateOutcome::Updated);

    fs::remove_file(d.path().join("b.rs")).unwrap();
    assert_eq!(idx.update("b.rs").unwrap(), UpdateOutcome::Removed);

    // remove() on something we never had: Unchanged.
    assert_eq!(idx.remove("never.rs"), UpdateOutcome::Unchanged);
}

#[test]
fn rename_moves_row() {
    let d = tempdir().unwrap();
    write(d.path(), "old.rs", "fn x() {}\n");
    let mut idx = Index::scan(&ReportOptions::new(d.path())).unwrap();
    assert!(idx.file("old.rs").is_some());

    fs::rename(d.path().join("old.rs"), d.path().join("new.rs")).unwrap();
    let out = idx.rename("old.rs", "new.rs").unwrap();
    // Either Inserted (rename hit the update path) or Removed (if
    // counter rejected). Both should result in the row moving.
    assert!(matches!(
        out,
        UpdateOutcome::Inserted | UpdateOutcome::Updated
    ));
    assert!(idx.file("old.rs").is_none());
    assert!(idx.file("new.rs").is_some());
}

#[test]
fn jsonl_round_trip_preserves_file_rows() {
    let d = tempdir().unwrap();
    write(d.path(), "x.py", "def foo():\n    pass\n");
    write(d.path(), "y.md", "# y\n");
    let idx = Index::scan(&ReportOptions::new(d.path())).unwrap();

    let mut buf = Vec::new();
    idx.write_jsonl(&mut buf, &Scope::All, &CocomoParams::default())
        .unwrap();

    // Smoke check: the first line is the meta record.
    let text = std::str::from_utf8(&buf).unwrap();
    let first = text.lines().next().unwrap();
    assert!(first.contains("\"kind\":\"meta\""));
    assert!(first.contains(&format!("\"schema\":{}", SCHEMA_VERSION)));

    let opts = ReportOptions::new(d.path());
    let reloaded = Index::load_jsonl(Cursor::new(buf), &opts).unwrap();
    assert_eq!(reloaded.len(), idx.len());
    assert_eq!(reloaded.file("x.py"), idx.file("x.py"));
}

#[test]
fn scope_prefix_restricts_rollups() {
    let d = tempdir().unwrap();
    write(d.path(), "src/lib.rs", "fn x() { if true {} }\n");
    write(d.path(), "src/main.rs", "fn main() {}\n");
    write(d.path(), "README.md", "# h\n");

    let idx = Index::scan(&ReportOptions::new(d.path())).unwrap();
    let scoped: Report = idx.snapshot(&Scope::Prefix("src".into()), &CocomoParams::default());
    assert_eq!(scoped.files.len(), 2);
    assert!(scoped.files.iter().all(|f| f.path.starts_with("src/")));
    assert_eq!(scoped.totals.files, 2);
    assert!(scoped.totals.bytes > 0);
    // by_language for the scope contains Rust only.
    assert_eq!(scoped.by_language.len(), 1);
    assert_eq!(scoped.by_language[0].name, "Rust");
    assert!(scoped.by_language[0].bytes > 0);
}

#[test]
fn scope_files_picks_explicit_rows() {
    let d = tempdir().unwrap();
    write(d.path(), "a.rs", "fn a() {}\n");
    write(d.path(), "b.rs", "fn b() {}\n");
    write(d.path(), "c.rs", "fn c() {}\n");
    let idx = Index::scan(&ReportOptions::new(d.path())).unwrap();

    let scoped = idx.snapshot(
        &Scope::Files(vec!["a.rs".into(), "c.rs".into(), "missing.rs".into()]),
        &CocomoParams::default(),
    );
    assert_eq!(scoped.files.len(), 2);
    assert_eq!(scoped.totals.files, 2);
}

#[test]
fn gitignore_filters_during_scan_and_update() {
    let d = tempdir().unwrap();
    write(d.path(), ".gitignore", "target/\n");
    write(d.path(), "src/lib.rs", "fn x() {}\n");
    write(d.path(), "target/junk.rs", "fn junk() {}\n");

    let idx = Index::scan(&ReportOptions::new(d.path())).unwrap();
    assert!(idx.file("src/lib.rs").is_some());
    assert!(idx.file("target/junk.rs").is_none());

    // Incremental path should also reject the ignored file.
    let mut idx = idx;
    let outcome = idx.update("target/junk.rs").unwrap();
    assert_eq!(outcome, UpdateOutcome::Skipped);
}

#[test]
fn schema_mismatch_is_reported() {
    let d = tempdir().unwrap();
    let opts = ReportOptions::new(d.path());
    let bogus = format!(
        "{{\"kind\":\"meta\",\"schema\":{},\"root\":\"/x\",\"generated_at\":\"2026-01-01T00:00:00Z\"}}\n",
        SCHEMA_VERSION + 99
    );
    let err = Index::load_jsonl(Cursor::new(bogus), &opts).err().unwrap();
    assert!(matches!(
        err,
        chan_report::ChanReportError::SchemaMismatch { .. }
    ));
}
