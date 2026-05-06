// End-to-end smoke test for the public API. Uses an isolated
// config dir so it doesn't touch the developer's real ~/.chan.

use chan_core::{Library, SearchOpts};
use tempfile::TempDir;

#[test]
fn end_to_end_register_open_write_index_search_graph() {
    let cfg = TempDir::new().unwrap();
    let drive_root = TempDir::new().unwrap();

    let lib = Library::open_at(cfg.path().join("config.toml")).unwrap();
    lib.register_drive(drive_root.path(), Some("Smoke".into()))
        .unwrap();

    let drive = lib.open_drive(drive_root.path()).unwrap();
    drive
        .write_text(
            "intro.md",
            "# Welcome\n\nSee [[recipes/pasta]] for #cooking inspiration.\n",
        )
        .unwrap();
    drive
        .write_text(
            "recipes/pasta.md",
            "# Carbonara\n\nClassic #italian recipe; talk to @@alice.\n",
        )
        .unwrap();

    // Tree listing returns both files (sans .chan / .git).
    let entries = drive.list_tree().unwrap();
    let paths: Vec<_> = entries.iter().map(|e| e.path.clone()).collect();
    assert!(paths.iter().any(|p| p == "intro.md"));
    assert!(paths.iter().any(|p| p == "recipes/pasta.md"));

    // Reindex builds both the search index and the graph.
    let stats = drive.reindex().unwrap();
    assert_eq!(stats.files_indexed, 2);
    assert_eq!(stats.files_skipped, 0);

    // Full-text search hits the body content.
    let res = drive.search("carbonara", &SearchOpts::default()).unwrap();
    assert_eq!(res.hits.len(), 1);
    assert_eq!(res.hits[0].path, "recipes/pasta.md");

    // Scope filter narrows to a subdir.
    let scoped = drive
        .search(
            "italian",
            &SearchOpts {
                scope: Some("recipes".into()),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(scoped.hits.len(), 1);

    // Graph: tags from both files surface; backlinks resolve.
    let g = drive.graph().unwrap();
    let tags = g.tags().unwrap();
    let names: Vec<_> = tags.iter().map(|t| t.name.as_str()).collect();
    assert!(names.contains(&"cooking"));
    assert!(names.contains(&"italian"));

    let cooking_files = g.files_with_tag("cooking").unwrap();
    assert_eq!(cooking_files, vec!["intro.md".to_string()]);

    // Wiki link from intro -> recipes/pasta should appear as
    // a backlink on recipes/pasta. The link target is "recipes/pasta"
    // (without .md); resolving it to a real file is a higher-level
    // concern, so we check the raw stored edge.
    let neighbors = g.neighbors("intro.md").unwrap();
    let dsts: Vec<_> = neighbors.iter().map(|e| e.dst.as_str()).collect();
    assert!(dsts.contains(&"recipes/pasta"));

    // Headings stored per file.
    let h = g.headings_of("recipes/pasta.md").unwrap();
    assert_eq!(h.len(), 1);
    assert_eq!(h[0].text, "Carbonara");
    assert_eq!(h[0].level, 1);
    assert_eq!(h[0].anchor, "carbonara");

    // Single-file update path: edit intro, re-index just that file.
    drive
        .write_text("intro.md", "# Welcome\n\nNo more cooking talk.\n")
        .unwrap();
    drive.index_file("intro.md").unwrap();
    let cooking_files = g.files_with_tag("cooking").unwrap();
    assert!(cooking_files.is_empty(), "tag should drop after re-index");

    // Forget a file: drops from index and graph.
    drive.forget_file("recipes/pasta.md").unwrap();
    let res = drive.search("carbonara", &SearchOpts::default()).unwrap();
    assert!(res.hits.is_empty());
    let italian_files = g.files_with_tag("italian").unwrap();
    assert!(italian_files.is_empty());
}
