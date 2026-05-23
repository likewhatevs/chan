// End-to-end import: parse a Google CSV blob, write per-contact
// markdown files into a chosen drive directory, verify the on-disk
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
    lib.register_drive(drive_root.path()).unwrap();
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

    // Slim chan-block frontmatter + readable body bullets.
    let jane = drive.read_text("Contacts/Jane Doe.md").unwrap();
    assert!(jane.starts_with("---\n"));
    assert!(jane.contains("kind: contact"));
    assert!(jane.contains("provider: google"));
    // Body holds the contact data, NOT the frontmatter (Phase 0a:
    // the editor doesn't strip frontmatter, so we keep the chan
    // classifier slim and put the contact info where the user can
    // read it).
    assert!(!jane.contains("contact:"));
    assert!(!jane.contains("display_name:"));
    assert!(jane.contains("# Jane Doe"));
    assert!(jane.contains("- **Email**: jane@x.com (Home)"));
    assert!(jane.contains("- **Phone**: +1-555-0100 (Mobile)"));
    assert!(jane.contains("- **Org**: Acme - Engineer"));
    assert!(jane.contains("Met at FOSDEM"));
    // System "* myContacts" label dropped; user-set "Friends" kept.
    assert!(jane.contains("- **Labels**: Friends"));
    assert!(!jane.contains("myContacts"));
}

#[test]
fn re_import_default_skips_existing() {
    let cfg = TempDir::new().unwrap();
    let drive_root = TempDir::new().unwrap();
    let lib = Library::open_at(cfg.path().join("config.toml")).unwrap();
    lib.register_drive(drive_root.path()).unwrap();
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
    lib.register_drive(drive_root.path()).unwrap();
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
    lib.register_drive(drive_root.path()).unwrap();
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
    lib.register_drive(drive_root.path()).unwrap();
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
fn intra_batch_duplicate_name_skips_unrelated_existing_suffixed_file() {
    // Two contacts named "Jane Smith" land in one batch. An unrelated
    // user file already exists at "Contacts/Jane Smith (2).md". The
    // second contact must NOT clobber that file under overwrite, nor
    // get a misleading "skipped" outcome attached to it under skip.
    let cfg = TempDir::new().unwrap();
    let drive_root = TempDir::new().unwrap();
    let lib = Library::open_at(cfg.path().join("config.toml")).unwrap();
    lib.register_drive(drive_root.path()).unwrap();
    let drive = lib.open_drive(drive_root.path()).unwrap();

    drive.create_dir("Contacts").unwrap();
    drive
        .write_text(
            "Contacts/Jane Smith (2).md",
            "user-owned, unrelated to the import\n",
        )
        .unwrap();

    let dup = "\
Name,Given Name,Family Name,Notes,Group Membership,\
E-mail 1 - Type,E-mail 1 - Value
\"Jane Smith\",Jane,Smith,first,,Home,jane.a@x.com
\"Jane Smith\",Jane,Smith,second,,Home,jane.b@x.com
";
    let contacts = parse_google_csv(dup.as_bytes()).unwrap();
    assert_eq!(contacts.len(), 2);
    let summary = drive
        .import_contacts("Contacts", contacts, ImportOpts { overwrite: true })
        .unwrap();
    let counts = summary.counts();
    assert_eq!(counts.wrote, 2, "both contacts should land as new files");
    assert_eq!(counts.overwrote, 0);

    // The pre-existing user file is untouched.
    let preserved = drive.read_text("Contacts/Jane Smith (2).md").unwrap();
    assert_eq!(preserved, "user-owned, unrelated to the import\n");

    // The two import targets are "Jane Smith.md" and "Jane Smith (3).md".
    assert!(drive.exists("Contacts/Jane Smith.md"));
    assert!(drive.exists("Contacts/Jane Smith (3).md"));
}

#[test]
fn imported_contacts_are_reachable_by_email_substring_via_picker_filter() {
    // End-to-end: import a CSV with emails, reindex, then prove a
    // typed email fragment surfaces the matching contact through
    // the same code path the @ picker uses.
    let cfg = TempDir::new().unwrap();
    let drive_root = TempDir::new().unwrap();
    let lib = Library::open_at(cfg.path().join("config.toml")).unwrap();
    lib.register_drive(drive_root.path()).unwrap();
    let drive = lib.open_drive(drive_root.path()).unwrap();

    let contacts = parse_google_csv(CSV.as_bytes()).unwrap();
    drive
        .import_contacts("Contacts", contacts, ImportOpts::default())
        .unwrap();
    drive.reindex(None).unwrap();

    // Local-part match.
    let hits = drive.contacts_filtered(Some("jane@x"), 10).unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].rel_path, "Contacts/Jane Doe.md");
    assert!(hits[0].emails.iter().any(|e| e == "jane@x.com"));

    // Domain match.
    let hits = drive.contacts_filtered(Some("y.com"), 10).unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].rel_path, "Contacts/Bob Smith.md");

    // Case insensitivity.
    let hits = drive.contacts_filtered(Some("JANE@X.COM"), 10).unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].rel_path, "Contacts/Jane Doe.md");

    // The picker also surfaces emails on the row itself for the
    // secondary-line render.
    assert!(hits[0].emails.contains(&"jane@x.com".to_string()));
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
    lib.register_drive(drive_root.path()).unwrap();
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
