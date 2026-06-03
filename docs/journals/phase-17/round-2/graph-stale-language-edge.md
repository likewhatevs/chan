# Task: graph language edge goes stale for a recently-modified file

For @@LaneC (you re-verified the earlier lang=X graph fix, so you have the
context). Identify from `$CHAN_TAB_NAME`.

## Symptom (@@Alex, ~/Documents/Chan, graph from root, max depth)

A markdown file (`new-team-1/journals/journal-Lead.md`, modified ~19m before
the screenshot) renders FLOATING: connected to its parent `journals/` dir but
MISSING its edge to the `Markdown` language node. Other .md files have it.

## What I (@@LaneA) already verified

- It is NOT a `merge_language_layer` logic bug. On a FRESH serve of a COPY of
  that workspace, `/api/graph` DOES include the edge:
  `language:Markdown -> new-team-1/journals/journal-Lead.md` (confirmed by
  exact-edge extraction), and `/api/report/file` classifies it
  `"language":"Markdown"` (code:0, comments:12 - markdown prose, not empty).
- So the file NODE is fresh (the unified tree layer picked up the edit) but the
  language EDGE, which `merge_language_layer` reads from `workspace.report()`,
  was stale in @@Alex's LIVE running workspace. => a report/index incremental-
  refresh gap, surfaced only on a live long-running server, not a fresh index.

## Where to look

- `crates/chan-workspace/src/report.rs:227` `self.report.on_event(&event)` and
  `workspace.rs:2692` `if self.reports_enabled()...` / `report_state()` /
  `self.report.get()/set()` (workspace.rs ~3311-3379). Does a file CREATE /
  MODIFY watcher event refresh the per-file language in the report snapshot the
  graph reads, or only on a full rebuild? Is `reports_enabled()` possibly off
  in @@Alex's workspace (would make `report()` empty/stale)?
- Repro the STALE state (a fresh index hides it): serve a workspace, load
  `/api/graph`, then CREATE or MODIFY a .md file, and re-curl `/api/graph` -
  does the new/edited file get its language edge without a restart/reindex?

## Acceptance

- A file created/modified in a live workspace gets (or refreshes) its language
  edge in `/api/graph` promptly (after the watcher debounce), no restart.
- Own-gate: fmt/clippy -Dwarnings/test for the touched crates. Report to
  @@LaneA; @@LaneA owns the full-tree gate. Do NOT push.
