// End-to-end test pinning the file-type policy: which extensions
// are editable text, which are media (Image, Pdf), and how each
// class flows through walk / index / graph.

use chan_drive::{classify, ChanError, FileClass, Library, SearchOpts};
use tempfile::TempDir;

#[test]
fn file_type_policy_end_to_end() {
    let cfg = TempDir::new().unwrap();
    let drive_root = TempDir::new().unwrap();

    let lib = Library::open_at(cfg.path().join("config.toml")).unwrap();
    lib.register_drive(drive_root.path(), Some("Types".into()))
        .unwrap();
    let drive = lib.open_drive(drive_root.path()).unwrap();

    // Editable-text: .md and .txt go through write_text.
    drive
        .write_text(
            "notes/intro.md",
            "# Intro\n\nSee ![diagram](../media/diagram.png) and \
             [whitepaper](../docs/spec.pdf).\n\nMore in notes/notes.txt.\n",
        )
        .unwrap();
    drive
        .write_text(
            "notes/notes.txt",
            "shopping list: bread, milk, walnuts, sourdough\n",
        )
        .unwrap();

    // Text class: source / config / shell / well-known basenames
    // are editable through write_text but not indexed. The body
    // carries a unique sentinel token (`xyzzysentinel`) so the
    // "Text-class not searchable" assertion below has something
    // grep-shaped to look for without colliding with markdown
    // chunk content.
    drive
        .write_text("src/main.py", "def main():\n    return 'xyzzysentinel'\n")
        .unwrap();
    drive
        .write_text("Cargo.toml", "[package]\nname = \"x\"\n")
        .unwrap();
    drive.write_text("Makefile", "all:\n\techo hi\n").unwrap();

    // Media classes: png + pdf are written as opaque bytes.
    drive
        .write_bytes("media/diagram.png", &[0x89, 0x50, 0x4e, 0x47])
        .unwrap();
    drive
        .write_bytes("docs/spec.pdf", b"%PDF-1.7\n%fake\n")
        .unwrap();
    // Other: a binary that doesn't fit any category.
    drive
        .write_bytes("downloads/song.mp3", &[0xff, 0xfb, 0x90])
        .unwrap();

    // write_text is rejected for non-textual (Image/Pdf/Other) extensions.
    let err = drive.write_text("media/diagram.png", "nope").unwrap_err();
    assert!(
        matches!(err, ChanError::NotEditableText(_)),
        "expected NotEditableText, got {err:?}",
    );
    let err = drive.write_text("downloads/song.mp3", "nope").unwrap_err();
    assert!(matches!(err, ChanError::NotEditableText(_)));

    // list_tree returns every regular file regardless of class.
    let tree = drive.list_tree().unwrap();
    let paths: Vec<&str> = tree.iter().map(|e| e.path.as_str()).collect();
    for expected in [
        "notes/intro.md",
        "notes/notes.txt",
        "src/main.py",
        "Cargo.toml",
        "Makefile",
        "media/diagram.png",
        "docs/spec.pdf",
        "downloads/song.mp3",
    ] {
        assert!(paths.contains(&expected), "missing {expected} in {paths:?}");
    }

    // classify is the single source of truth the editor will use.
    assert_eq!(classify("notes/intro.md"), FileClass::EditableText);
    assert_eq!(classify("notes/notes.txt"), FileClass::EditableText);
    assert_eq!(classify("src/main.py"), FileClass::Text);
    assert_eq!(classify("Cargo.toml"), FileClass::Text);
    assert_eq!(classify("Makefile"), FileClass::Text);
    assert_eq!(classify("media/diagram.png"), FileClass::Image);
    assert_eq!(classify("docs/spec.pdf"), FileClass::Pdf);
    assert_eq!(classify("downloads/song.mp3"), FileClass::Other);

    // Reindex: only EditableText (.md + .txt) reach the index;
    // Text-class files (.py / Cargo.toml / Makefile) are walked
    // but stay out of the index by design. The image / pdf / mp3
    // are likewise excluded.
    let summary = drive.reindex(None).unwrap();
    assert_eq!(summary.files, 2, "indexer should ingest .md + .txt only");
    assert_eq!(summary.indexed, 2);
    assert!(summary.errors.is_empty());

    // Full-text search finds tokens from both editable kinds.
    let md_hits = drive.search("Intro", &SearchOpts::default()).unwrap();
    assert!(md_hits.hits.iter().any(|h| h.path == "notes/intro.md"));
    let txt_hits = drive.search("sourdough", &SearchOpts::default()).unwrap();
    assert!(
        txt_hits.hits.iter().any(|h| h.path == "notes/notes.txt"),
        ".txt content should be searchable: hits = {:?}",
        txt_hits.hits,
    );

    // Text-class content is editable but **not** searchable: the
    // unique sentinel from main.py must not surface anywhere.
    let py_hits = drive
        .search("xyzzysentinel", &SearchOpts::default())
        .unwrap();
    assert!(
        py_hits.hits.is_empty(),
        "Text-class file leaked into search: {:?}",
        py_hits.hits,
    );

    // Search never returns a non-editable-text path.
    for needle in ["spec", "diagram", "song", "mp3"] {
        let res = drive.search(needle, &SearchOpts::default()).unwrap();
        for hit in &res.hits {
            let c = classify(&hit.path);
            assert_eq!(
                c,
                FileClass::EditableText,
                "search returned non-editable {} (class={:?})",
                hit.path,
                c,
            );
        }
    }

    // Graph: nodes are .md + .txt only; images/pdfs do NOT get a
    // node row. The graph DOES carry edges that point at them
    // (e.g. an `![]()` embed), so the backlink-from-media query
    // still works.
    let g = drive.graph().unwrap();
    let files = g.files().unwrap();
    assert!(files.iter().any(|p| p == "notes/intro.md"));
    assert!(files.iter().any(|p| p == "notes/notes.txt"));
    assert!(
        !files.iter().any(|p| p == "media/diagram.png"),
        "image must not appear in graph nodes; found {files:?}",
    );
    assert!(
        !files.iter().any(|p| p == "docs/spec.pdf"),
        "pdf must not appear in graph nodes; found {files:?}",
    );
    // Text-class files are editable but stay out of the graph
    // for the same reason they stay out of the index.
    assert!(
        !files.iter().any(|p| p == "src/main.py"),
        "Text-class .py must not appear in graph nodes; found {files:?}",
    );
    assert!(
        !files.iter().any(|p| p == "Cargo.toml"),
        "Text-class .toml must not appear in graph nodes; found {files:?}",
    );
    assert!(
        !files.iter().any(|p| p == "Makefile"),
        "Text-class Makefile must not appear in graph nodes; found {files:?}",
    );

    // Edges to the image / pdf are stored (relative href collapsed
    // to drive-rooted form by `normalize_href`).
    let neighbors = g.neighbors("notes/intro.md").unwrap();
    let dsts: Vec<&str> = neighbors.iter().map(|e| e.dst.as_str()).collect();
    assert!(
        dsts.contains(&"media/diagram.png"),
        "image embed edge missing; dsts = {dsts:?}",
    );
    assert!(
        dsts.contains(&"docs/spec.pdf"),
        "pdf link edge missing; dsts = {dsts:?}",
    );

    // Backlinks from the image perspective surface the embedding file.
    let img_backlinks = g.backlinks("media/diagram.png").unwrap();
    let src_paths: Vec<&str> = img_backlinks.iter().map(|e| e.src.as_str()).collect();
    assert_eq!(src_paths, vec!["notes/intro.md"]);

    // Read back: every class is byte-readable.
    assert!(!drive.read("media/diagram.png").unwrap().is_empty());
    assert!(!drive.read("docs/spec.pdf").unwrap().is_empty());
    assert!(!drive.read("downloads/song.mp3").unwrap().is_empty());

    // Rename of an image: filesystem moves, no link rewrite expected
    // for the binary itself (only editable-text source bodies get
    // rewritten). The renamed file is still walkable.
    drive
        .rename("media/diagram.png", "media/diagram-v2.png")
        .unwrap();
    let tree2 = drive.list_tree().unwrap();
    let paths2: Vec<&str> = tree2.iter().map(|e| e.path.as_str()).collect();
    assert!(paths2.contains(&"media/diagram-v2.png"));
    assert!(!paths2.contains(&"media/diagram.png"));
}
