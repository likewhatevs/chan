// End-to-end import: parse a Google CSV blob, write per-contact
// markdown files into a chosen drive folder, verify the on-disk
// shape, then re-run with overwrite vs. skip semantics.
//
// Mirrors the smoke.rs pattern: isolated config dir via Library::open_at
// so this never touches the developer's real ~/.chan.

use chan_drive::contacts::{google::parse_google_csv, ImportOpts};
use chan_drive::Library;
use tempfile::TempDir;

const CSV: &str = "\
Name,Given Name,Family Name,Notes,Group Membership,\
E-mail 1 - Type,E-mail 1 - Value,\
Phone 1 - Type,Phone 1 - Value,\
Organization 1 - Name,Organization 1 - Title
\"Jane Doe\",Jane,Doe,\"Met at FOSDEM\",\"* myContacts ::: Friends\",Home,jane@x.com,Mobile,+1-555-0100,Acme,Engineer
\"Bob Smith\",Bob,Smith,,\"* myContacts\",Work,bob@y.com,,,,
,,,,,,,,,,
";

#[test]
fn end_to_end_import_into_drive() {
    let cfg = TempDir::new().unwrap();
    let drive_root = TempDir::new().unwrap();
    let lib = Library::open_at(cfg.path().join("config.toml")).unwrap();
    lib.register_drive(drive_root.path(), Some("ImportTest".into()))
        .unwrap();
    let drive = lib.open_drive(drive_root.path()).unwrap();

    // Parse + import.
    let contacts = parse_google_csv(CSV.as_bytes()).unwrap();
    assert_eq!(contacts.len(), 2, "blank row should be skipped");
    let summary = drive
        .import_contacts("Contacts", contacts, ImportOpts::default())
        .unwrap();
    let counts = summary.counts();
    assert_eq!(counts.wrote, 2);
    assert_eq!(counts.skipped, 0);
    assert_eq!(counts.failed, 0);

    // Files exist under the chosen dir.
    assert!(drive.exists("Contacts/Jane Doe.md"));
    assert!(drive.exists("Contacts/Bob Smith.md"));

    // Frontmatter is what we promised.
    let jane = drive.read_text("Contacts/Jane Doe.md").unwrap();
    assert!(jane.starts_with("---\n"));
    assert!(jane.contains("kind: contact"));
    assert!(jane.contains("provider: google"));
    assert!(jane.contains("display_name: \"Jane Doe\""));
    assert!(jane.contains("jane@x.com"));
    assert!(jane.contains("# Jane Doe"));
    assert!(jane.contains("Met at FOSDEM"));
    // System "* myContacts" label dropped; user-set "Friends" kept.
    assert!(jane.contains("Friends"));
    assert!(!jane.contains("myContacts"));
}

#[test]
fn re_import_default_skips_existing() {
    let cfg = TempDir::new().unwrap();
    let drive_root = TempDir::new().unwrap();
    let lib = Library::open_at(cfg.path().join("config.toml")).unwrap();
    lib.register_drive(drive_root.path(), Some("Reimport".into()))
        .unwrap();
    let drive = lib.open_drive(drive_root.path()).unwrap();

    let first = parse_google_csv(CSV.as_bytes()).unwrap();
    let _ = drive
        .import_contacts("Contacts", first, ImportOpts::default())
        .unwrap();

    // Mutate Jane's file so we can detect whether the second run
    // touched it (default: no).
    drive
        .write_text("Contacts/Jane Doe.md", "user-edited body\n")
        .unwrap();

    let second = parse_google_csv(CSV.as_bytes()).unwrap();
    let summary = drive
        .import_contacts("Contacts", second, ImportOpts::default())
        .unwrap();
    let counts = summary.counts();
    assert_eq!(counts.skipped, 2);
    assert_eq!(counts.wrote, 0);
    assert_eq!(counts.overwrote, 0);

    let jane = drive.read_text("Contacts/Jane Doe.md").unwrap();
    assert_eq!(jane, "user-edited body\n", "skip must not touch the file");
}

#[test]
fn re_import_with_overwrite_replaces() {
    let cfg = TempDir::new().unwrap();
    let drive_root = TempDir::new().unwrap();
    let lib = Library::open_at(cfg.path().join("config.toml")).unwrap();
    lib.register_drive(drive_root.path(), Some("Overwrite".into()))
        .unwrap();
    let drive = lib.open_drive(drive_root.path()).unwrap();

    let first = parse_google_csv(CSV.as_bytes()).unwrap();
    let _ = drive
        .import_contacts("Contacts", first, ImportOpts::default())
        .unwrap();

    drive
        .write_text("Contacts/Jane Doe.md", "user-edited body\n")
        .unwrap();

    let second = parse_google_csv(CSV.as_bytes()).unwrap();
    let summary = drive
        .import_contacts("Contacts", second, ImportOpts { overwrite: true })
        .unwrap();
    let counts = summary.counts();
    assert_eq!(counts.overwrote, 2);
    assert_eq!(counts.skipped, 0);
    assert_eq!(counts.wrote, 0);

    let jane = drive.read_text("Contacts/Jane Doe.md").unwrap();
    assert!(jane.contains("# Jane Doe"));
    assert!(!jane.contains("user-edited body"));
}

#[test]
fn import_into_drive_root() {
    let cfg = TempDir::new().unwrap();
    let drive_root = TempDir::new().unwrap();
    let lib = Library::open_at(cfg.path().join("config.toml")).unwrap();
    lib.register_drive(drive_root.path(), Some("Root".into()))
        .unwrap();
    let drive = lib.open_drive(drive_root.path()).unwrap();

    let contacts = parse_google_csv(CSV.as_bytes()).unwrap();
    let summary = drive
        .import_contacts("", contacts, ImportOpts::default())
        .unwrap();
    assert_eq!(summary.counts().wrote, 2);
    assert!(drive.exists("Jane Doe.md"));
    assert!(drive.exists("Bob Smith.md"));
}
