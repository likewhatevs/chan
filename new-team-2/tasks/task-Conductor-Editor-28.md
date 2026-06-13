# task-Conductor-Editor-28 — cross-review: B5 buried-window cap (f198df7b)

From: @@Conductor. To: @@Editor. Cut: 2026-06-13.

## Why you

Original pairing (you review @@Desktop's lane) + you're the idle
lane. Small commit. This came to light via @@TeamFlow's report-data
sweep — B5 landed silently mid-lane (per their task's batched-
completion instruction, so no process fault) and had no review row.

## Scope

f198df7b — feat(chan-desktop): exclude buried windows from the
per-workspace window cap. desktop/src-tauri/src/main.rs (+6-1) +
serve.rs (+10). Verified on main. REQUIRED READING first:
designs/b5-buried-window-cap-decision.md (the authorized working
default + its two deliberate consequences) and
task-Conductor-Desktop-17 § B5 (my constraint).

## Targets

1. The filter logic: "visible windows only" — verify the buried
   detection in ensure_window_capacity is correct (label-prefix
   match unchanged, buried-state binding actually reflects bury
   state, no off-by-one vs the cap constant).
2. The two consequences are IN the code as designed: unbury can
   exceed the cap (unbury path does NOT call ensure_window_capacity
   — confirm) and buried accumulation is unbounded (no hidden cap
   added). These are deliberate — do NOT flag them as bugs; DO flag
   any THIRD consequence the note doesn't list.
3. Menu header: `Hidden Windows (N, kept warm in memory)` built in
   rebuild_window_menu — count source correct, no gold-plating
   beyond count+cost.
4. Revert-path claim: confirm the note's one-commit revert is real
   (changes confined to in-process counting + menu text; no
   config/wire/persisted state touched).
5. Rider walk (16 lines — quick) + the usual: no behavior beyond
   the note's scope.

## Note

Empirical verification is recorded as PENDING (interactive bury
clicking unautomatable in-lane; pre-release merge-gated-green
policy). The note's 30-second human check is being added to the
round-close hand-smoke checklist — your review is source-level.

## Completion

Findings (or clean pass) →
new-team-2/tasks/task-Editor-Conductor-<n>.md + 1-line poke.
