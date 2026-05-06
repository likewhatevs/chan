// End-to-end smoke test for the public API. Uses an isolated
// config dir so it doesn't touch the developer's real ~/.chan.

use chan_core::{Library, SearchOpts};
use tempfile::TempDir;

#[test]
fn end_to_end_register_open_write_search() {
    let cfg = TempDir::new().unwrap();
    let drive_root = TempDir::new().unwrap();

    let lib = Library::open_at(cfg.path().join("config.toml")).unwrap();
    lib.register_drive(drive_root.path(), Some("Smoke".into()))
        .unwrap();

    let drive = lib.open_drive(drive_root.path()).unwrap();
    drive.write_text("intro.md", "# Hello\n\nWelcome.").unwrap();
    drive
        .write_text("recipes/pasta.md", "# Carbonara\n")
        .unwrap();

    let entries = drive.list_tree().unwrap();
    let paths: Vec<_> = entries.iter().map(|e| e.path.clone()).collect();
    assert!(paths.iter().any(|p| p == "intro.md"));
    assert!(paths.iter().any(|p| p == "recipes/pasta.md"));

    // Search is a stub; just exercise the path. Real ranking
    // arrives once tantivy is wired in.
    let res = drive.search("hello", &SearchOpts::default()).unwrap();
    assert_eq!(res.hits.len(), 0);

    // Graph opens cleanly and reports zero relations on a fresh drive.
    let g = drive.graph().unwrap();
    assert!(g.tags().unwrap().is_empty());
}
