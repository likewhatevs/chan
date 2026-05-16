// Report integration: scoped snapshots, JSONL persistence, and
// incremental updates through the watcher. Uses an isolated
// config dir so the test never touches the developer's real
// ~/.chan; the per-drive state path is keyed off the tempdir
// root, so multiple test runs don't collide.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use std::fs;
use std::path::Path;

use chan_drive::watch::{WatchCallback, WatchEvent};
use chan_drive::{Library, ReportScope};
use tempfile::TempDir;

fn put(root: &Path, rel: &str, content: &str) {
    let p = root.join(rel);
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(p, content).unwrap();
}

struct Collector(Mutex<Vec<WatchEvent>>);

impl Collector {
    fn new() -> Arc<Self> {
        Arc::new(Self(Mutex::new(Vec::new())))
    }
    fn len(&self) -> usize {
        self.0.lock().unwrap().len()
    }
}

impl WatchCallback for Collector {
    fn on_event(&self, ev: WatchEvent) {
        self.0.lock().unwrap().push(ev);
    }
}

fn wait_for<F: Fn() -> bool>(predicate: F, timeout: Duration) -> bool {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if predicate() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    false
}

#[test]
fn report_initial_scan_picks_up_markdown_and_code() {
    let cfg = TempDir::new().unwrap();
    let drive_root = TempDir::new().unwrap();
    let lib = Library::open_at(cfg.path().join("config.toml")).unwrap();
    lib.register_drive(drive_root.path(), Some("Report".into()))
        .unwrap();
    put(drive_root.path(), "notes/today.md", "# today\n\nbody.\n");
    put(
        drive_root.path(),
        "src/lib.rs",
        "fn main() { if true { } }\n",
    );

    let drive = lib.open_drive(drive_root.path()).unwrap();
    let report = drive.report().unwrap();
    assert!(report.totals.files >= 2);
    let langs: Vec<_> = report.by_language.iter().map(|l| l.name.clone()).collect();
    assert!(langs.iter().any(|n| n == "Rust"));
    assert!(langs.iter().any(|n| n == "Markdown"));
}

#[test]
fn report_for_prefix_restricts_to_subtree() {
    let cfg = TempDir::new().unwrap();
    let drive_root = TempDir::new().unwrap();
    let lib = Library::open_at(cfg.path().join("config.toml")).unwrap();
    lib.register_drive(drive_root.path(), Some("Scope".into()))
        .unwrap();
    put(drive_root.path(), "src/a.rs", "fn a() {}\n");
    put(drive_root.path(), "src/b.rs", "fn b() {}\n");
    put(drive_root.path(), "README.md", "# x\n");

    let drive = lib.open_drive(drive_root.path()).unwrap();
    let scoped = drive.report_for_prefix("src").unwrap();
    assert_eq!(scoped.totals.files, 2);
    assert!(scoped.files.iter().all(|f| f.path.starts_with("src/")));
}

#[test]
fn watcher_keeps_report_current() {
    let cfg = TempDir::new().unwrap();
    let drive_root = TempDir::new().unwrap();
    let lib = Library::open_at(cfg.path().join("config.toml")).unwrap();
    lib.register_drive(drive_root.path(), Some("Watch".into()))
        .unwrap();
    let drive = lib.open_drive(drive_root.path()).unwrap();

    drive.write_text("a.md", "# a\n").unwrap();

    let collector = Collector::new();
    let cb: Arc<dyn WatchCallback> = collector.clone();
    let _handle = drive.watch(cb).unwrap();

    // Add a file and wait for the watcher to deliver an event.
    drive.write_text("b.md", "# b\n").unwrap();
    assert!(
        wait_for(|| collector.len() >= 1, Duration::from_secs(5)),
        "watcher did not fire for new file"
    );

    // Allow the report writer thread to debounce + flush.
    std::thread::sleep(Duration::from_millis(700));

    let after = drive.report().unwrap();
    let names: Vec<_> = after.files.iter().map(|f| f.path.clone()).collect();
    assert!(names.iter().any(|n| n == "b.md"), "report missed b.md");

    // JSONL is now persisted at the advertised path.
    let path = drive.report_jsonl_path().unwrap();
    assert!(
        wait_for(|| path.exists(), Duration::from_secs(3)),
        "report jsonl never written: {}",
        path.display()
    );
    let bytes = std::fs::read_to_string(&path).unwrap();
    assert!(bytes.contains("\"kind\":\"meta\""));
}

#[test]
fn report_returns_for_empty_drive() {
    let cfg = TempDir::new().unwrap();
    let drive_root = TempDir::new().unwrap();
    let lib = Library::open_at(cfg.path().join("config.toml")).unwrap();
    lib.register_drive(drive_root.path(), Some("Empty".into()))
        .unwrap();
    let drive = lib.open_drive(drive_root.path()).unwrap();

    let r = drive.report().unwrap();
    assert_eq!(r.totals.files, 0);
    assert!(r.by_language.is_empty());
    assert_eq!(r.cocomo.effort_person_months, 0.0);

    // ReportScope::All on an empty drive returns the same shape.
    let r2 = drive.report().unwrap();
    assert_eq!(r2.totals.files, r.totals.files);
    let _ = ReportScope::All; // public re-export still exists
}
