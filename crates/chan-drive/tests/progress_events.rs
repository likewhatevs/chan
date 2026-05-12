// End-to-end test: every long-running Drive / Library entry point
// with a `_with` variant must emit ProgressEvents through the
// supplied ProgressCallback. The point of this test isn't to pin
// the event count exactly (the per-file granularity is an
// implementation detail callers shouldn't depend on) but to verify
// each stage actually fires at least once, so a future refactor
// that silently drops the emits gets caught.

use std::sync::Mutex;

use chan_drive::{
    progress_fn, Library, NoProgress, ProgressCallback, ProgressEvent, ProgressStage,
};
use tempfile::TempDir;

struct Collector(Mutex<Vec<ProgressEvent>>);
impl Collector {
    fn new() -> Self {
        Self(Mutex::new(Vec::new()))
    }
    fn stages(&self) -> Vec<ProgressStage> {
        self.0.lock().unwrap().iter().map(|e| e.stage).collect()
    }
}
impl ProgressCallback for Collector {
    fn on_progress(&self, e: ProgressEvent) {
        self.0.lock().unwrap().push(e);
    }
}

#[test]
fn reindex_emits_graph_and_index_stages() {
    let cfg = TempDir::new().unwrap();
    let drive_root = TempDir::new().unwrap();
    let lib = Library::open_at(cfg.path().join("config.toml")).unwrap();
    lib.register_drive(drive_root.path(), Some("Prog".into()))
        .unwrap();
    let drive = lib.open_drive(drive_root.path()).unwrap();
    drive
        .write_text("intro.md", "# Intro\n\nHello\n")
        .unwrap();
    drive
        .write_text("notes/x.md", "# X\n\nhi\n")
        .unwrap();
    drive
        .write_text("notes/y.txt", "plain\n")
        .unwrap();

    let cb = Collector::new();
    drive.reindex_with(None, &cb).unwrap();
    let stages = cb.stages();
    assert!(
        stages.contains(&ProgressStage::GraphRebuild),
        "expected GraphRebuild event; got {stages:?}",
    );
    assert!(
        stages.contains(&ProgressStage::IndexFile),
        "expected IndexFile event; got {stages:?}",
    );

    // current/total invariants: every event should have current < total
    // (or total == 0 for indeterminate stages, but reindex always
    // knows its total).
    for e in cb.0.lock().unwrap().iter() {
        if matches!(e.stage, ProgressStage::GraphRebuild | ProgressStage::IndexFile) {
            assert!(
                e.total > 0,
                "stage {:?} should know its total; event = {e:?}",
                e.stage,
            );
            assert!(
                e.current < e.total,
                "current >= total in {e:?} (events are 0-indexed)",
            );
            assert!(e.label.is_some(), "file-level events should carry a path label");
        }
    }
}

#[test]
fn no_progress_passes_through_silently() {
    // The no-arg `reindex` delegates to `reindex_with(..., &NoProgress)`.
    // Calling reindex_with directly with &NoProgress must produce the
    // same outcome with zero callback overhead (no panics, no
    // observable side effects).
    let cfg = TempDir::new().unwrap();
    let drive_root = TempDir::new().unwrap();
    let lib = Library::open_at(cfg.path().join("config.toml")).unwrap();
    lib.register_drive(drive_root.path(), Some("Silent".into()))
        .unwrap();
    let drive = lib.open_drive(drive_root.path()).unwrap();
    drive.write_text("a.md", "# a\n").unwrap();
    let s1 = drive.reindex_with(None, &NoProgress).unwrap();
    assert_eq!(s1.indexed, 1);
}

#[test]
fn rename_with_link_rewrite_with_emits_rewrite_progress() {
    let cfg = TempDir::new().unwrap();
    let drive_root = TempDir::new().unwrap();
    let lib = Library::open_at(cfg.path().join("config.toml")).unwrap();
    lib.register_drive(drive_root.path(), Some("Rename".into()))
        .unwrap();
    let drive = lib.open_drive(drive_root.path()).unwrap();
    // Two source files reference the target by markdown link so
    // the rewriter has work to do per source.
    drive
        .write_text(
            "src1.md",
            "# S1\n\nlink to [target](old/target.md)\n",
        )
        .unwrap();
    drive
        .write_text(
            "src2.md",
            "# S2\n\nalso linking [target](old/target.md)\n",
        )
        .unwrap();
    drive
        .write_text("old/target.md", "# Target\n\nbody\n")
        .unwrap();
    drive.reindex(None).unwrap();

    let cb = Collector::new();
    drive
        .rename_with_link_rewrite_with("old/target.md", "new/target.md", &cb)
        .unwrap();
    let stages = cb.stages();
    assert!(
        stages.contains(&ProgressStage::RenameRewrite),
        "expected RenameRewrite events; got {stages:?}",
    );
}

#[test]
fn import_contacts_with_emits_per_contact_progress() {
    use chan_drive::{Contact, EmailAddress, ImportOpts};
    let cfg = TempDir::new().unwrap();
    let drive_root = TempDir::new().unwrap();
    let lib = Library::open_at(cfg.path().join("config.toml")).unwrap();
    lib.register_drive(drive_root.path(), Some("Imp".into()))
        .unwrap();
    let drive = lib.open_drive(drive_root.path()).unwrap();

    let contacts = vec![
        Contact {
            display_name: "Alice Anderson".into(),
            emails: vec![EmailAddress {
                label: None,
                value: "alice@example.com".into(),
            }],
            ..Default::default()
        },
        Contact {
            display_name: "Bob Brown".into(),
            ..Default::default()
        },
    ];
    let cb = Collector::new();
    let summary = drive
        .import_contacts_with(
            "Contacts",
            contacts,
            ImportOpts { overwrite: false },
            &cb,
        )
        .unwrap();
    assert_eq!(summary.outcomes.len(), 2);
    let import_events: Vec<_> = cb
        .0
        .lock()
        .unwrap()
        .iter()
        .filter(|e| e.stage == ProgressStage::Import)
        .cloned()
        .collect();
    assert_eq!(
        import_events.len(),
        2,
        "expected one Import event per contact",
    );
    assert_eq!(import_events[0].current, 0);
    assert_eq!(import_events[0].total, 2);
    assert_eq!(import_events[1].current, 1);
    assert_eq!(import_events[1].total, 2);
}

#[test]
fn reset_drive_with_emits_one_event_per_subsystem() {
    use chan_drive::ResetMode;
    let cfg = TempDir::new().unwrap();
    let drive_root = TempDir::new().unwrap();
    let lib = Library::open_at(cfg.path().join("config.toml")).unwrap();
    lib.register_drive(drive_root.path(), Some("R".into()))
        .unwrap();
    {
        // Open + write so the per-drive state dirs exist on disk.
        let drive = lib.open_drive(drive_root.path()).unwrap();
        drive.write_text("a.md", "# a\n").unwrap();
        drive.reindex(None).unwrap();
        // Drop the drive so the writer flock is released before reset.
    }

    let cb = Collector::new();
    lib.reset_drive_with(drive_root.path(), ResetMode::State, &cb)
        .unwrap();
    let reset_events: Vec<_> = cb
        .0
        .lock()
        .unwrap()
        .iter()
        .filter(|e| e.stage == ProgressStage::Reset)
        .cloned()
        .collect();
    // Five subsystems: index, graph, sessions, assistant, tokens.
    assert_eq!(reset_events.len(), 5, "got events: {reset_events:?}");
    let labels: Vec<_> = reset_events
        .iter()
        .filter_map(|e| e.label.clone())
        .collect();
    assert!(labels.contains(&"index".to_string()));
    assert!(labels.contains(&"graph".to_string()));
    assert!(labels.contains(&"sessions".to_string()));
    assert!(labels.contains(&"assistant".to_string()));
    assert!(labels.contains(&"tokens".to_string()));
}

#[test]
fn progress_fn_adapter_lets_closures_be_callbacks() {
    let cfg = TempDir::new().unwrap();
    let drive_root = TempDir::new().unwrap();
    let lib = Library::open_at(cfg.path().join("config.toml")).unwrap();
    lib.register_drive(drive_root.path(), Some("Fn".into()))
        .unwrap();
    let drive = lib.open_drive(drive_root.path()).unwrap();
    drive.write_text("a.md", "# a\n").unwrap();

    let counter = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let cb = {
        let counter = counter.clone();
        progress_fn(move |_e| {
            counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        })
    };
    drive.reindex_with(None, &*cb).unwrap();
    assert!(
        counter.load(std::sync::atomic::Ordering::SeqCst) > 0,
        "closure-backed callback should have fired at least once",
    );
}
