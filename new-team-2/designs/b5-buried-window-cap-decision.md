# B5 decision note — buried windows and MAX_WINDOWS_PER_WORKSPACE

Author: @@Desktop. Authorized: task-Conductor-Desktop-17 (working
default by @@Conductor; on the round-close survey for @@Alex's cheap
veto). Carryover origin: phase-22.md lowlights.

## Old semantics

`ensure_window_capacity` (desktop/src-tauri/src/serve.rs) counted
every live webview whose label matched the workspace prefix —
visible AND buried. Ten total blocked the eleventh with "Workspace
already has 10 open windows; close one before opening another."
Because bury-on-close keeps the webview alive, a user who closed
(buried) 9 windows and kept 1 visible was told to "close one" while
seeing a single window — the error pointed at nothing on screen.

## New semantics (this round)

The cap counts VISIBLE windows only; buried windows are excluded
from the count. Rationale: the cap is an anti-runaway guard for
on-screen clutter, not a webview budget. Two consequences, accepted
deliberately:

1. **Unbury can exceed the cap.** Unburying shows an existing
   webview (no creation), so 10 visible + unbury = 11 visible. The
   cap re-engages for the next *new* window.
2. **Buried accumulation is no longer bounded by the cap.** Open 10,
   bury 10, open 10 more → 20 live webviews (the old semantics
   incidentally capped this at 10 total). Each buried webview is the
   result of an explicit user close, re-burying a label replaces its
   entry, and the Window menu now surfaces the count + cost, so the
   exposure is visible and user-driven. If real usage shows runaway
   buried memory, the follow-up is a buried-list cap
   (destroy-oldest past N), NOT reverting visible-cap semantics.

## Companion affordance

The Window menu's "Hidden Windows" header now reads
`Hidden Windows (N, kept warm in memory)` — count + cost hint, per
the no-gold-plating constraint (rebuild_window_menu, main.rs).

## One-commit revert path

Revert the single B5 commit. Code-wise that means: drop the two
`.filter(...)`-adjacent lines (the `state`/`buried` bindings and the
buried-exclusion filter) in `ensure_window_capacity`, restoring the
plain prefix count, and restore the static "Hidden Windows" header
string in `rebuild_window_menu`. No data, config, or wire formats
are touched — the change is purely in-process counting + menu text.

## Verification status

Gated (fmt + clippy -D warnings + full desktop test suite) in the
isolated worktree. Empirically UNVERIFIED on a live build: driving
bury (red-dot close) + 10-window fan-out needs interactive clicking
that the lane cannot automate right now; pre-release policy is to
merge gated-green and record that status (this note). Suggested
30-second human check at the next smoke: close a workspace window,
open the Window menu — header shows `(1, kept warm in memory)`;
open windows past 10 with one buried — the 11th opens.
