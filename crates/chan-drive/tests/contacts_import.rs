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

#[test]
fn imported_contacts_classified_as_contact_nodes_after_index() {
    // Phase 4 wiring: imported notes carry chan.kind: contact in
    // their frontmatter, and the indexer should pick that up so
    // Drive::contacts returns them and a regular .md does not.
    let cfg = TempDir::new().unwrap();
    let drive_root = TempDir::new().unwrap();
    let lib = Library::open_at(cfg.path().join("config.toml")).unwrap();
    lib.register_drive(drive_root.path(), Some("ContactsGraph".into()))
        .unwrap();
    let drive = lib.open_drive(drive_root.path()).unwrap();

    // One contact import + one plain note. Indexing both should
    // surface the contact only via Drive::contacts while both
    // appear in Drive::list_tree.
    let contacts = parse_google_csv(CSV.as_bytes()).unwrap();
    drive
        .import_contacts("Contacts", contacts, ImportOpts::default())
        .unwrap();
    drive
        .write_text("notes/journal.md", "# Journal\n\nUnrelated.\n")
        .unwrap();

    drive.reindex(None).unwrap();

    let contacts = drive.contacts().unwrap();
    let paths: Vec<_> = contacts.iter().map(|c| c.rel_path.clone()).collect();
    assert!(paths.contains(&"Contacts/Jane Doe.md".to_string()));
    assert!(paths.contains(&"Contacts/Bob Smith.md".to_string()));
    assert!(!paths.contains(&"notes/journal.md".to_string()));
    // Title comes from the # H1 the emitter puts in.
    let jane = contacts
        .iter()
        .find(|c| c.rel_path == "Contacts/Jane Doe.md")
        .unwrap();
    assert_eq!(jane.title.as_deref(), Some("Jane Doe"));
    assert_eq!(jane.basename, "Jane Doe.md");
}

#[test]
fn removing_contact_frontmatter_demotes_node_back_to_file() {
    // If a user edits a contact note and strips the chan.kind
    // frontmatter, the next index pass should drop it from
    // Drive::contacts. We can't change the importer's output to
    // simulate this cleanly, so synthesize a file directly.
    let cfg = TempDir::new().unwrap();
    let drive_root = TempDir::new().unwrap();
    let lib = Library::open_at(cfg.path().join("config.toml")).unwrap();
    lib.register_drive(drive_root.path(), Some("Demote".into()))
        .unwrap();
    let drive = lib.open_drive(drive_root.path()).unwrap();

    drive
        .write_text("people/x.md", "---\nchan:\n  kind: contact\n---\n# X\n")
        .unwrap();
    drive.reindex(None).unwrap();
    assert_eq!(drive.contacts().unwrap().len(), 1);

    // Strip the contact tag.
    drive
        .write_text("people/x.md", "# X\n\nJust a note now.\n")
        .unwrap();
    drive.index_file("people/x.md").unwrap();
    assert_eq!(drive.contacts().unwrap().len(), 0);
}
