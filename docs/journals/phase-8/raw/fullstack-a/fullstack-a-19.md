# fullstack-a-19: Hybrid NAV chord-table documentation drift cleanup

Owner: @@FullStackA
Date: 2026-05-20

## Goal

Sync `PaneModeHelp.svelte`'s Hybrid NAV chord table + the
auto-generated portion of `crates/chan/src/main.rs::SERVE_LONG_ABOUT`
with the actual runtime chord set. Three known drifts to
address; absorb any others you spot while there.

## Background

@@FullStackB flagged the drift in `fullstack-b-9`:

> The rest of the Hybrid NAV section drift (still says
> "Pane Mode (Cmd+K)", lists `s` for Search and `k` for
> kill-pane both of which moved in earlier tasks) untouched
> to stay in scope; that's a separate cleanup pass and
> belongs alongside whoever owns the next chord update.

Filed in [`../phase-8-bugs.md`](../phase-8-bugs.md). Last
side-observation in the Round-1 list; closes out the
chord-table audit-trail consistency.

## Known drifts

* **Section header**: "Pane Mode (Cmd+K)" → renamed to
  "Hybrid NAV (Cmd+.)" in `fullstack-a-7`.
* **Search chord**: lists `s` → moved to `f` in phase-7
  `fullstack-74`.
* **Kill-pane chord**: lists `k` → moved to Cmd+K Backspace
  in phase-7 `fullstack-77`.

While you're in the file, audit the rest of the table for
any additional drift (it's been a few phases since the
last full sync). Any other stale entries get fixed in the
same pass.

## Acceptance criteria

* `PaneModeHelp.svelte` Hybrid NAV section header reads
  "Hybrid NAV (Cmd+.)".
* Search chord row reads `f` (not `s`).
* Kill-pane row reads "Cmd+K Backspace" (or whatever
  shorthand the cheatsheet uses for chord sequences).
* `SERVE_LONG_ABOUT`'s Hybrid NAV section in
  `crates/chan/src/main.rs` matches the cheatsheet (chord
  set + section header).
* No regression on the structural tests
  (`paneModeKeymap.test.ts`, `paneModeHelpClickable.test.ts`)
  — update their literal expectations to match the new copy
  if needed.
* Visual verification on the lane-A server: enter Hybrid
  NAV via Cmd+. (the new binding), press H, the help
  overlay matches the runtime chord set with no stale
  entries.

## How to start

1. Open `web/src/components/PaneModeHelp.svelte`. Compare
   each row against the current runtime in
   `web/src/state/shortcuts.ts` / `web/src/App.svelte`'s
   `handlePaneModeKey`. Fix every mismatch you find.
2. Open `crates/chan/src/main.rs::SERVE_LONG_ABOUT`. The
   Hybrid NAV section is in the auto-generated portion
   (look for the section header pattern). Re-sync against
   the corrected cheatsheet.
3. Update the structural tests' literal expectations to
   match. The tests currently pin specific row labels;
   the new labels need to land in the regex / string
   match.
4. Pre-push gate before commit.

## Coordination

* @@WebtestA verifies on lane-A drive once landed.
* No other agents touching these files in flight.

## 2026-05-20 — implementation note

Audited the cheatsheet against the runtime in
`App.svelte::handlePaneModeKey` (the dispatch source of truth).
Three known drifts plus one stale comment in PaneModeHelp; six
items of drift in `SERVE_LONG_ABOUT`.

### PaneModeHelp.svelte

Title row was `<div class="title">Hybrid NAV</div>`. Extended to
`Hybrid NAV (Cmd+.)` per the task spec (the chord pins the entry
binding so the user can see how to open the overlay it documents).

The WASD swap-down cap was `{ label: "S", key: "S" }` with a
comment justifying the uppercase key as a Search-overlay
collision workaround. That collision is gone post-`fullstack-74`
(Search moved to `f`; both `s` / `S` now route to swap-down via
`case "s": case "S":`). Switched the cap to
`{ label: "S", key: "s" }` so it matches the W / A / D
lowercase-key pattern, and rewrote the comment to record why the
case-split is no longer needed.

The remaining body of the cheatsheet (Spawn, Split, Dock, Close,
Resize, Commit) already matches the runtime — those rows were
synced piecemeal by `fullstack-a-3`, `fullstack-a-7`,
`fullstack-a-9`, `fullstack-a-16`, `fullstack-69`,
`fullstack-77`, `fullstack-b-9`. Nothing else to fix in the
cheatsheet.

### crates/chan/src/main.rs::SERVE_LONG_ABOUT

The CLI help block had drifted further than the SPA cheatsheet
because it didn't get updated when individual chord moves
landed. Six edits in one pass:

* Section header: `Pane Mode (Cmd+K)` → `Hybrid NAV (Cmd+.)`
  (matches the runtime entry chord from `fullstack-a-7`).
* Search row: `s` → `f` (moved in phase-7 `fullstack-74`).
* Kill-pane row: `k` → `Backspace` (moved in phase-7
  `fullstack-77`).
* Added `p` → Rich prompt (fullstack-50; was missing).
* Added `< / >` → Dock toggles (fullstack-69; was missing).
* Added `Tab` → Flip Hybrid (fullstack-48 phase C; was only
  documented as the outer-keymap `Cmd+. Tab`).

### paneModeHelpClickable.test.ts + hybridNavRename.test.ts

Two test updates:

* `paneModeHelpClickable.test.ts` line 33 comment said
  "Spawn keys (1-4 + p / s)". Updated to "(1-4 + p + f)" so the
  audit-trail copy tracks the runtime chord set. No assertion
  change needed — the explicit `key:` checks the test enforces
  don't include `s` or `f`.
* `hybridNavRename.test.ts` line 49 asserted
  `>Hybrid NAV<`. With the chord suffix added to the title,
  this exact substring no longer appears. Updated to
  `>Hybrid NAV (Cmd+.)<` and added a leading comment recording
  why the chord suffix is now part of the pinned copy.

Files touched:

* `web/src/components/PaneModeHelp.svelte` — title + WASD cap +
  comments.
* `crates/chan/src/main.rs` — `SERVE_LONG_ABOUT` Hybrid NAV
  block re-synced.
* `web/src/components/paneModeHelpClickable.test.ts` — comment
  refreshed.
* `web/src/components/hybridNavRename.test.ts` — title assertion
  updated to match the new chord-suffixed copy.

Pre-push gate: vitest 480/480 green; `npm run check` 0 / 0;
`npm run build` clean; `cargo fmt --check` clean;
`cargo clippy -p chan --all-targets -- -D warnings` clean;
`cargo test -p chan` 58/58; `cargo build -p chan` re-embeds
the new bundle clean.

To verify on the lane-A server (post-restart): enter Hybrid NAV
via Cmd+. (note the entry chord pinned in the title), press H
for the cheatsheet — header reads `Hybrid NAV (Cmd+.)`, every
row's chord matches what actually fires. For the CLI side,
`chan serve --help` shows the updated Hybrid NAV block.

## 2026-05-20 — @@Architect: approved + commit clearance

Reviewer: @@Architect.

Comprehensive audit. The SPA-side cheatsheet was already
in good shape thanks to the piecemeal updates over phase-7
(-69, -77) and phase-8 (-a-3, -a-7, -a-9, -a-16, -b-9);
adding the entry-chord suffix to the title closes the
last cosmetic gap. The S→s WASD cap switch is the right
post-fullstack-74 cleanup; the comment refresh records
why the case-split is gone.

The `SERVE_LONG_ABOUT` block was the bigger fish. Six
edits in one pass (header + search row + kill-pane row +
three additions for missing chords) — that's a clean
sweep against the runtime. CLI help text now matches the
SPA cheatsheet against the same dispatch source-of-truth.

Two test updates handled cleanly:
* `paneModeHelpClickable.test.ts` comment-only refresh
  (no assertion change since the explicit `key:` checks
  don't include `s` or `f`).
* `hybridNavRename.test.ts` assertion updated to track
  the new chord-suffixed title; leading comment records
  why.

Cross-stack gate clean: vitest 480/480, check 0/0, npm
build clean, cargo fmt + clippy + test (-p chan 58/58),
cargo build re-embeds the new bundle.

**Commit clearance**: approved. Suggested commit subject:

```
Hybrid NAV: chord-table doc drift cleanup across PaneModeHelp + SERVE_LONG_ABOUT (fullstack-a-19)
```

Push waits until end of Round 2 (no Round-1 binary cut).

Queue clear for Round 1 once -19 commits. Wave-3 commits
in working tree:
* -15 (md.md double-append fix)
* -16 (Stage: → Spawn)
* -17 (TerminalTab focus gate)
* -18 (Wysiwyg onSubmit threading)
* -19 (this one)
* -20 (defaultPrevented guard hotfix)

Plus three new detour tasks in queue:
* -21 (Settings UI for semantic search) — wait for
  systacean-7 API contract.
* -22 (Hybrid pane flip animation) — independent.
* New chord migration task ("fullstack-a-NN") for Round 2,
  drafted in round-2-plan.md.

Stand down on -21 / -22 if you're done for the session;
they're queue items, not urgent. Confirm what you want to
pick up next via the next inbound event.