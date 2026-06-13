# task-Editor-Conductor-29 — B5 review (f198df7b): CLEAN PASS

From: @@Editor. To: @@Conductor. Cut: 2026-06-13.
Closes: task-Conductor-Editor-28.md. Source-level review per the
task (empirical bury-click check stays on the round-close
hand-smoke list, as recorded).

Read first: b5-buried-window-cap-decision.md +
task-Conductor-Desktop-17 § B5. Findings against the five targets:

1. **Filter logic — correct.** Prefix match unchanged
   (`starts_with(prefix)` retained, exclusion added as a second
   filter with exact-label equality). The buried binding truly
   reflects bury state: entries are created ONLY by the close→hide
   handler (serve.rs ~742-743, hide then bury_window), removed on
   unbury BEFORE the show (main.rs:2208 precedes 2209-2216) and in
   the single Destroyed cleanup (serve.rs:761) — so no stale entry
   can describe a visible or dead window. Re-bury replaces
   (retain+push, main.rs:256-264), so neither the exclusion nor the
   header count can double-count. `count >= MAX_WINDOWS_PER_WORKSPACE`
   comparison untouched — no off-by-one. Swept every `.show()` in
   the crate: besides unbury_window, only the "main" launcher window
   is shown (show_window callers all pass "main"; main never enters
   the buried registry) — no show-without-remove path exists.
   Locking: the buried mutex is taken and dropped inside
   ensure_window_capacity; no caller holds it (most_recent_buried
   clones out before unbury_or_restore reaches the cap call) — no
   re-entrancy.
2. **Both consequences in code as designed.** Unbury never consults
   the cap: the spawn preamble short-circuits on
   unbury_instead_of_spawn BEFORE ensure_window_capacity
   (serve.rs:486-489), and the menu/keyboard unbury paths
   (main.rs:1907, 2297-2327) call unbury_window directly, which only
   removes+shows. No hidden buried-list bound was added (bury_window
   is retain+push, unbounded). **Third-consequence scan: none
   found.** One unlisted IMPROVEMENT, not a hazard: the cap's error
   text "close one before opening another" becomes factually correct
   under the new semantics (closing buries → frees a visible slot;
   it used to point at nothing). Also checked: the header's count is
   all-workspaces while the cap is per-family — that's the
   pre-existing app-global Window-menu scope, not a semantics change.
3. **Menu header — conforms.** Count source is buried_snapshot(),
   the same list the menu rows render from (main.rs:2002 → header at
   the diff site), so count always matches the rows shown. Count +
   cost hint only; no gold-plating.
4. **Revert path — real.** 2 files, +15/-1: bindings + one filter
   line + doc comment (serve.rs), header format! + comment
   (main.rs). No config, wire, or persisted state; the note's
   described revert matches the diff exactly.
5. **Rider walk — clean.** All 16 lines in scope; the new doc
   comment on ensure_window_capacity accurately documents
   consequence 1 in-place.

No findings. Nothing to route to @@Desktop.
