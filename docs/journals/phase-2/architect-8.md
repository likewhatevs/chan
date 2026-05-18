# architect-8: webtest report-finding triage + scratch fixture path

Owner: @@Architect. Status: REVIEW.

Routes the two open items from
[[phase-2/webtest-2.md]]:

1. Code report shows only Markdown on the shared drive even though
   `.rs`, `.ts`, `.svelte` files exist under
   `/tmp/chan-dev/Source/chan-workspace-copy`.
2. Destructive browser smoke (delete-while-open, live-add) needs a
   scratch path so it does not mutate shared fixture content.

## Finding 1: code report is Markdown-only on the shared drive

### Root cause (verified in code)

* `chan_drive::Drive::watch` wraps the user callback in a
  `ReportFanOut` so watcher events fan out into chan-report. The
  fan-out IS active in chan-server (`lib.rs:296` calls
  `drive.watch(bridge)`). So the wire is in place.
* `chan-report` persists its index to `.chan/report.jsonl` and
  `ReportState::open()` loads from there on boot. If the JSONL
  was written before the source copy AND chan-serve was restarted
  after the copy, the loaded index does NOT include the new
  files. The watcher only fires on subsequent FS events, so the
  pre-existing-on-disk source tree never gets ingested.
* Bulk recursive `cp -r` on macOS FSEvents tends to coalesce
  events at the directory level. `ReportState::on_event` only
  handles per-file create/modify/remove/rename. A coarse
  directory-create event will not recursively walk and reconcile.

Both gaps point at the same shape: there is no
"reconcile-on-load / reconcile-on-coarse-event" path in
chan-report.

### Phase-2 disposition

Not a phase-2 blocker. The user request does not include
chan-report reconcile work, and the @@Webtest finding does not
block any phase-2 work-item smoke once we point smoke at a fresh
drive. Filed as a backend follow-up:
[[phase-2/backend-5.md]].

### Immediate workaround for @@Webtest

Either:

(a) Stop chan-serve, `rm /tmp/chan-dev/.chan/report.jsonl`,
restart. The first call to `report()` after restart pays the
full-scan cost and picks up every on-disk file.

(b) Switch the shared service to a different drive whose state
matches the fixture you want to test (e.g. a fresh
`/tmp/chan-dev-phase2/` with the workspace copy in place from
the start).

Pick whichever is faster. Document the chosen workaround in
[[phase-2/webtest-1.md]] alongside the rebuilt
service log.

## Finding 2: scratch fixture path

Use `/tmp/chan-dev/Scratch/phase2-smoke/` for any browser smoke
that needs to create, delete, or rename files while the overlay
is open. Rules:

* Confine destructive operations to that directory subtree only.
* Pre-create the directory before the smoke run; clean it after.
* Sample-file basenames should namespace by probe (e.g.
  `ghost-probe-<unix-ts>.md`,
  `live-add-probe-<unix-ts>.md`) so concurrent smoke runs do not
  collide.
* The directory is permanent (a regular folder, not a tmpfs);
  cleanup is webtest's responsibility.

This is enough for the two open smoke gaps (G1a ghost, G4 live
add). The depth-cap probe (G3 / frontend-9) does not need
destructive writes; it operates on whatever the loaded scope
returns.

## Done means

* `architect-8` flips to DONE when:
  * webtest-2 picks up the workaround for finding 1 (status note
    appended to [[phase-2/webtest-1.md]] or
    [[phase-2/webtest-2.md]]).
  * webtest-2 records the scratch path in its smoke matrix.
  * backend-5 is created and tagged non-blocker.

Status: REVIEW until both webtest pickups and backend-5 filing
are recorded in the journal.
