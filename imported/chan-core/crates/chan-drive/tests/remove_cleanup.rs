// End-to-end test for the remove/restore + index/graph cleanup
// contract: deleting a file or directory through the Drive API
// must drop the corresponding graph rows and search-index entries
// without waiting for the next reindex. Restoring the same entry
// must repopulate them.

use chan_drive::{Library, SearchOpts};
use tempfile::TempDir;

#[test]
fn remove_single_file_drops_graph_and_index() {
    let cfg = TempDir::new().unwrap();
    let drive_root = TempDir::new().unwrap();
    let lib = Library::open_at(cfg.path().join("config.toml")).unwrap();
    lib.register_drive(drive_root.path(), Some("Del".into()))
        .unwrap();
    let drive = lib.open_drive(drive_root.path()).unwrap();

    // Markdown link with explicit `.md` so the stored edge `dst`
    // matches the path we'll later pass to `remove`. Wiki links
    // (`[[notes/x]]`) intentionally store an extensionless `dst`
    // (resolved through `Drive::resolve_link` at query time), so
    // they wouldn't be cleaned up by a path-prefix delete; that's
    // a separate concern from this test.
    drive
        .write_text(
            "intro.md",
            "# Intro\n\nLinks to [x](notes/x.md). unique-intro-token\n",
        )
        .unwrap();
    drive
        .write_text("notes/x.md", "# X\n\n#tagged Body of x. unique-x-token\n")
        .unwrap();
    drive.reindex(None).unwrap();

    // Sanity: both files indexed + linked.
    let g = drive.graph().unwrap();
    assert!(g.files().unwrap().iter().any(|p| p == "notes/x.md"));
    assert_eq!(
        g.backlinks("notes/x.md").unwrap().len(),
        1,
        "intro -> notes/x.md markdown link should be a backlink",
    );
    assert!(!drive
        .search("unique-x-token", &SearchOpts::default())
        .unwrap()
        .hits
        .is_empty());

    // Remove the target. Graph + index must reflect it immediately.
    drive.remove("notes/x.md").unwrap();

    let files = g.files().unwrap();
    assert!(
        !files.iter().any(|p| p == "notes/x.md"),
        "graph node not dropped after remove: {files:?}",
    );
    // Inbound edge from intro.md -> notes/x.md is intentionally
    // preserved: it describes intro.md's body, which still contains
    // the link text. `backlinks("notes/x.md")` returns it as a
    // "broken link" until the source is edited or the entry is
    // restored.
    assert_eq!(
        g.backlinks("notes/x.md").unwrap().len(),
        1,
        "inbound edge from intro.md should survive (it reflects intro.md's body)",
    );
    let hits = drive
        .search("unique-x-token", &SearchOpts::default())
        .unwrap()
        .hits;
    assert!(
        hits.is_empty(),
        "search must not return removed file: {hits:?}",
    );

    // Restoring brings everything back, no manual reindex.
    let id = drive.trash_list().unwrap()[0].id.clone();
    drive.trash_restore(&id).unwrap();

    assert!(drive
        .graph()
        .unwrap()
        .files()
        .unwrap()
        .iter()
        .any(|p| p == "notes/x.md"));
    assert!(!drive
        .search("unique-x-token", &SearchOpts::default())
        .unwrap()
        .hits
        .is_empty());
    assert_eq!(
        drive
            .graph()
            .unwrap()
            .backlinks("notes/x.md")
            .unwrap()
            .len(),
        1,
        "backlink should still be present after restore",
    );
}

#[test]
fn remove_directory_cascades_through_graph_and_index() {
    let cfg = TempDir::new().unwrap();
    let drive_root = TempDir::new().unwrap();
    let lib = Library::open_at(cfg.path().join("config.toml")).unwrap();
    lib.register_drive(drive_root.path(), Some("DelDir".into()))
        .unwrap();
    let drive = lib.open_drive(drive_root.path()).unwrap();

    // A subtree mixing editable text, an image, and a PDF. An outside
    // file links into the subtree to verify backlinks clear too.
    drive
        .write_text(
            "outside.md",
            "# Outside\n\n![diag](notes/media/diagram.png) and [[notes/inner]]\n",
        )
        .unwrap();
    drive
        .write_text(
            "notes/inner.md",
            "# Inner\n\nUnique-inner-token. See ./other.txt\n",
        )
        .unwrap();
    drive
        .write_text("notes/other.txt", "plain text body with sourdough\n")
        .unwrap();
    drive
        .write_bytes("notes/media/diagram.png", &[0x89, 0x50, 0x4e, 0x47])
        .unwrap();
    drive
        .write_bytes("notes/media/spec.pdf", b"%PDF-1.7\n")
        .unwrap();
    drive.reindex(None).unwrap();

    // Sanity: every editable-text node present, backlink to image exists.
    let g = drive.graph().unwrap();
    let files0 = g.files().unwrap();
    assert!(files0.iter().any(|p| p == "outside.md"));
    assert!(files0.iter().any(|p| p == "notes/inner.md"));
    assert!(files0.iter().any(|p| p == "notes/other.txt"));
    let img_back = g.backlinks("notes/media/diagram.png").unwrap();
    assert_eq!(img_back.len(), 1, "outside.md should backlink the image");

    // Remove the entire `notes` directory.
    drive.remove("notes").unwrap();

    let files1 = g.files().unwrap();
    for gone in ["notes/inner.md", "notes/other.txt"] {
        assert!(
            !files1.iter().any(|p| p == gone),
            "node {gone} still present after dir remove: {files1:?}",
        );
    }
    assert!(
        files1.iter().any(|p| p == "outside.md"),
        "outside.md must survive a sibling-dir remove",
    );
    // The cross-subtree edge outside.md -> notes/media/diagram.png
    // is preserved: outside.md's body still embeds the image; the
    // edge correctly describes that. The target is a broken link
    // until the subtree is restored or outside.md is edited.
    assert_eq!(
        g.backlinks("notes/media/diagram.png").unwrap().len(),
        1,
        "inbound edge to removed image should survive (reflects outside.md's body)",
    );

    // Search: nothing under notes/ should hit, outside.md still does.
    let inner_hits = drive
        .search("Unique-inner-token", &SearchOpts::default())
        .unwrap();
    assert!(
        inner_hits.hits.is_empty(),
        "stale BM25 row for notes/inner.md"
    );
    let txt_hits = drive.search("sourdough", &SearchOpts::default()).unwrap();
    assert!(
        txt_hits.hits.is_empty(),
        "stale BM25 row for notes/other.txt"
    );
    let outside_hits = drive.search("Outside", &SearchOpts::default()).unwrap();
    assert!(
        outside_hits.hits.iter().any(|h| h.path == "outside.md"),
        "outside file should still be searchable after sibling-dir remove",
    );

    // Restore brings the subtree back with graph + index repopulated.
    let id = drive.trash_list().unwrap()[0].id.clone();
    drive.trash_restore(&id).unwrap();
    let files2 = drive.graph().unwrap().files().unwrap();
    for back in ["notes/inner.md", "notes/other.txt"] {
        assert!(
            files2.iter().any(|p| p == back),
            "{back} did not return to graph after restore: {files2:?}",
        );
    }
    assert!(!drive
        .search("Unique-inner-token", &SearchOpts::default())
        .unwrap()
        .hits
        .is_empty());
    assert!(!drive
        .search("sourdough", &SearchOpts::default())
        .unwrap()
        .hits
        .is_empty());
}

#[test]
fn remove_non_editable_file_keeps_inbound_edges() {
    // Removing an image preserves the embedding markdown's outgoing
    // edge: that edge correctly describes the source body. The
    // image has no node row anyway (only editable-text files do),
    // so there's nothing to drop on the graph node side.
    let cfg = TempDir::new().unwrap();
    let drive_root = TempDir::new().unwrap();
    let lib = Library::open_at(cfg.path().join("config.toml")).unwrap();
    lib.register_drive(drive_root.path(), Some("Img".into()))
        .unwrap();
    let drive = lib.open_drive(drive_root.path()).unwrap();

    drive
        .write_text("post.md", "# Post\n\n![diag](media/diagram.png)\n")
        .unwrap();
    drive
        .write_bytes("media/diagram.png", &[0x89, 0x50, 0x4e, 0x47])
        .unwrap();
    drive.reindex(None).unwrap();

    let g = drive.graph().unwrap();
    assert_eq!(g.backlinks("media/diagram.png").unwrap().len(), 1);

    drive.remove("media/diagram.png").unwrap();
    // The inbound edge survives the remove; it describes post.md's
    // body, which still says ![](media/diagram.png).
    assert_eq!(
        g.backlinks("media/diagram.png").unwrap().len(),
        1,
        "inbound edge from post.md should survive removing the image",
    );
    let post_edges: Vec<String> = g
        .neighbors("post.md")
        .unwrap()
        .into_iter()
        .map(|e| e.dst)
        .collect();
    assert!(
        post_edges.contains(&"media/diagram.png".to_string()),
        "post.md's outgoing edge is preserved: {post_edges:?}",
    );
}
