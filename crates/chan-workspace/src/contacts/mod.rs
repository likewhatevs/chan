// Contacts: import third-party contact dumps as markdown notes.
//
// One file per contact, written into a caller-chosen directory in
// the workspace. Nested `chan: { kind: contact }` frontmatter classifies the file
// for downstream consumers (graph builder, editor `@` picker).
//
// The on-disk file is fully user-owned the moment it lands. chan
// does not re-edit imported contacts; re-running an import either
// skips existing files or overwrites them based on `ImportOpts`.
// There is no merge or two-way sync in v1.
//
// Module split:
//   provider.rs : ProviderKind enum (parser dispatch, not API client)
//   google.rs   : Google Contacts CSV parser
//   emit.rs     : Contact -> markdown (frontmatter + body)
//   slug.rs     : filename derivation, sanitization, collision suffix
//
// The orchestrator that actually writes files lives on `Workspace` so
// the import flow goes through the same path sandbox + atomic write
// as every other workspace op.

pub mod emit;
pub mod extract;
pub mod google;
pub mod import;
pub mod provider;
pub mod slug;

pub use extract::extract_emails;
pub use provider::ProviderKind;

use serde::{Deserialize, Serialize};

/// One contact, provider-agnostic. The on-disk markdown file
/// serializes a subset of these fields under the `contact:`
/// frontmatter key; full reconstruction from disk is not a goal
/// (the file is user-owned post-import).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Contact {
    pub provider: ProviderKind,
    /// Stable cross-rename anchor. `None` for CSV sources that don't
    /// expose one (Google's CSV export omits the People API resource
    /// name). Future providers that do expose a stable id should
    /// populate this.
    pub remote_id: Option<String>,
    pub display_name: String,
    pub given_name: Option<String>,
    pub family_name: Option<String>,
    pub emails: Vec<EmailAddress>,
    pub phones: Vec<PhoneNumber>,
    pub organizations: Vec<Organization>,
    pub notes: Option<String>,
    /// "Group Membership" in Google CSV; arbitrary strings.
    pub labels: Vec<String>,
    /// Alternate names that should resolve `@@<alias>` mentions to
    /// this contact. Lands in the contact note's frontmatter as a
    /// top-level `aliases:` array (Obsidian convention) so cross-tool
    /// interop stays cheap, and the indexer mirrors them into the
    /// graph node row alongside `emails` for the picker + the
    /// chan-server mention resolver. Empty list = "only the filename
    /// stem resolves to this contact" (the pre-phase-5 default).
    #[serde(default)]
    pub aliases: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailAddress {
    pub value: String,
    /// Free-form. Common values: "Home", "Work", "Other".
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhoneNumber {
    pub value: String,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Organization {
    pub name: String,
    pub title: Option<String>,
}

/// Knobs for the import orchestrator (`Workspace::import_contacts`).
#[derive(Debug, Clone, Default)]
pub struct ImportOpts {
    /// If true, replace files that already exist at the derived
    /// path. If false, the existing file is left alone and the
    /// outcome reports `Skipped { reason: "exists" }`.
    pub overwrite: bool,
}

/// Per-contact result of an import run.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum ImportOutcome {
    /// New file created.
    Wrote { path: String },
    /// Existing file replaced (only with `overwrite = true`).
    Overwrote { path: String },
    /// Existing file left alone.
    Skipped { path: String, reason: String },
    /// Could not write. `name` is the contact's display name (the
    /// path may not have been derivable).
    Failed { name: String, reason: String },
}

/// Aggregate of an import run; one entry per input contact.
#[derive(Debug, Clone, Default, Serialize)]
pub struct ImportSummary {
    pub outcomes: Vec<ImportOutcome>,
}

impl ImportSummary {
    pub fn counts(&self) -> ImportCounts {
        let mut c = ImportCounts::default();
        for o in &self.outcomes {
            match o {
                ImportOutcome::Wrote { .. } => c.wrote += 1,
                ImportOutcome::Overwrote { .. } => c.overwrote += 1,
                ImportOutcome::Skipped { .. } => c.skipped += 1,
                ImportOutcome::Failed { .. } => c.failed += 1,
            }
        }
        c
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ImportCounts {
    pub wrote: usize,
    pub overwrote: usize,
    pub skipped: usize,
    pub failed: usize,
}
