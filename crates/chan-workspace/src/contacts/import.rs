// Import orchestrator. The pure parser + emitter + slug live in
// sibling modules; this one wires them together with a `Workspace`.
//
// Flow:
//   1. Ensure the destination directory exists (create_dir_all).
//   2. Stamp `imported_at` once per batch so all files in the run
//      share a timestamp (useful when grepping later).
//   3. Pre-seed the collision set with the destination's existing
//      file names so we don't pick a path that already exists. The
//      caller's `overwrite` flag controls whether we'd then replace
//      such a file or skip it.
//   4. For each contact, derive the path, write or skip per opts,
//      and record the outcome.
//
// Errors from `Workspace::write_text` are captured per-contact rather
// than aborting the whole batch; one weird name shouldn't cost the
// user 200 imports. The orchestrator only returns `Err` for setup
// failures (the destination dir can't be created, etc).

use std::collections::HashSet;

use chrono::Utc;

use crate::error::Result;
use crate::workspace::Workspace;

use super::emit::{render_markdown, EmitContext};
use super::slug::slug_for;
use super::{Contact, ImportOpts, ImportOutcome, ImportSummary};

pub fn run(
    workspace: &Workspace,
    dir: &str,
    contacts: Vec<Contact>,
    opts: ImportOpts,
    progress: &dyn crate::progress::ProgressCallback,
) -> Result<ImportSummary> {
    use crate::progress::{ProgressEvent, ProgressStage};
    let dir = dir.trim_matches('/').to_string();
    if !dir.is_empty() {
        workspace.create_dir(&dir)?;
    }

    let ctx = EmitContext {
        imported_at: Utc::now(),
    };

    // Pre-seed: if a file already exists at the slugged path, the
    // slugger should NOT pick a different name for the *natural*
    // pick - we want to either overwrite (per opts) or report
    // skipped, not silently rename around it. So `taken` starts
    // empty and we let `slug_for` consult the disk only when it
    // falls into its " (N)" suffix loop (so two contacts with the
    // same display name in one batch don't accidentally clobber an
    // unrelated existing file at the suffixed path).
    let mut taken: HashSet<String> = HashSet::new();
    let mut unnamed = 0usize;
    let mut summary = ImportSummary::default();
    let on_disk = |p: &str| workspace.exists(p);

    let total = contacts.len() as u64;
    for (idx, c) in contacts.into_iter().enumerate() {
        let path = slug_for(&c, &dir, &mut taken, &mut unnamed, &on_disk);
        progress.on_progress(ProgressEvent {
            stage: ProgressStage::Import,
            current: idx as u64,
            total,
            label: Some(if c.display_name.is_empty() {
                path.clone()
            } else {
                c.display_name.clone()
            }),
            eta_secs: None,
        });
        let exists = workspace.exists(&path);

        if exists && !opts.overwrite {
            summary.outcomes.push(ImportOutcome::Skipped {
                path,
                reason: "exists".into(),
            });
            continue;
        }

        let body = render_markdown(&c, &ctx);
        match workspace.write_text(&path, &body) {
            Ok(()) => {
                if exists {
                    summary.outcomes.push(ImportOutcome::Overwrote { path });
                } else {
                    summary.outcomes.push(ImportOutcome::Wrote { path });
                }
            }
            Err(e) => summary.outcomes.push(ImportOutcome::Failed {
                name: c.display_name,
                reason: e.to_string(),
            }),
        }
    }

    Ok(summary)
}
