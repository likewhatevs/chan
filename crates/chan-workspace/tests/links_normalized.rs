// End-to-end: drive-rooted (`/images/x`) and parent-relative
// (`../images/x`) markdown links resolve to the same backlink target
// as bare drive-relative paths after `reindex`. Regresses the
// "0 backlinks on an embedded image" symptom.

use chan_workspace::Library;
use tempfile::TempDir;

#[test]
fn abs_and_parent_relative_image_links_both_backlink_to_same_node() {
    let cfg = TempDir::new().unwrap();
    let drive_root = TempDir::new().unwrap();

    let lib = Library::open_at(cfg.path().join("config.toml")).unwrap();
    lib.register_workspace(drive_root.path()).unwrap();
    let drive = lib.open_workspace(drive_root.path()).unwrap();

    // One image placeholder at the drive root referenced by three
    // sources that each pick a different href shape:
    //   - notes/abs.md         uses /images/foo.png  (drive-rooted)
    //   - notes/rel.md         uses ../images/foo.png (parent-relative)
    //   - notes/wiki.md        uses [[/images/foo.png]] (wiki, abs)
    // All three should backlink to the same canonical
    // `images/foo.png` dst. Pre-fix, only the bare drive-relative
    // form matched the node and the inspector showed 0 backlinks.
    drive.write_bytes("images/foo.png", b"\x89PNG\r\n").unwrap();
    drive
        .write_text("notes/abs.md", "# Abs\n\n![cat](/images/foo.png)\n")
        .unwrap();
    drive
        .write_text("notes/rel.md", "# Rel\n\n![cat](../images/foo.png)\n")
        .unwrap();
    drive
        .write_text("notes/wiki.md", "# Wiki\n\n[[/images/foo.png]]\n")
        .unwrap();

    drive.reindex(None).unwrap();

    let g = drive.graph().unwrap();
    let back = g.backlinks("images/foo.png").unwrap();
    let mut srcs: Vec<&str> = back.iter().map(|e| e.src.as_str()).collect();
    srcs.sort();
    assert_eq!(
        srcs,
        vec!["notes/abs.md", "notes/rel.md", "notes/wiki.md"],
        "all three href shapes should backlink to the same image node",
    );

    // Negative control: the literal stale strings must not survive
    // as their own dst nodes; only the normalized form should.
    assert!(g.backlinks("/images/foo.png").unwrap().is_empty());
    assert!(g.backlinks("../images/foo.png").unwrap().is_empty());
}

#[test]
fn drive_escape_link_is_dropped() {
    let cfg = TempDir::new().unwrap();
    let drive_root = TempDir::new().unwrap();
    let lib = Library::open_at(cfg.path().join("config.toml")).unwrap();
    lib.register_workspace(drive_root.path()).unwrap();
    let drive = lib.open_workspace(drive_root.path()).unwrap();

    // `../../etc/passwd` from a depth-1 file pops past the drive
    // root and the indexer must drop the edge entirely rather than
    // store a literal escape path.
    drive
        .write_text(
            "notes/post.md",
            "[escape](../../etc/passwd) and [ok](/x.md)\n",
        )
        .unwrap();
    drive.reindex(None).unwrap();
    let g = drive.graph().unwrap();
    let neighbors = g.neighbors("notes/post.md").unwrap();
    let dsts: Vec<&str> = neighbors.iter().map(|e| e.dst.as_str()).collect();
    assert_eq!(dsts, vec!["x.md"]);
}
