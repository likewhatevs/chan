# @@LaneB round-2 addenda - new asks (mid-round, @@Alex direct)

New asks @@Alex raised directly to @@LaneB after the round-2 bootstrap,
while watching the turn-1 browser smoke. Per
`feedback_inflight_task_amendments` these are NEW tasks, not amendments
to the original `lane-b-request-round-2.md` B-slices. Recorded here so
the round-2 scope stays auditable; detail + smoke in `lane-b/journal.md`.

## Addendum 1 - bullet marker glyph pick (folded into B-slice 1)

Two messages, the second supersedes the first:
1. "i want U+2022 Bullet for the bullet points" -> confirmed via
   AskUserQuestion as U+2022 (filled) top / U+25E6 (hollow) nested,
   matching image-1's filled/hollow style.
2. "change the bullet characters to U+25CF and U+25EF" -> final pick:
   - top level (filled): U+25CF ● BLACK CIRCLE
   - nested (hollow): U+25EF ◯ LARGE CIRCLE

Hyphen markers stay en-dash (U+2013) at all levels; ordered lists keep
the styled source number. Folded into B-slice 1 (`3eb7f4c4`); browser-
smoked - renders ● top / ◯ nested, correctly indented.

## Addendum 2 - Cmd+, flip must be strictly per-pane (bug)

Status: FIXED (`8c6f4a94`), gated + browser-smoked.

@@Alex repro: "if i hit cmd+, on an empty pane, it seems to record it,
and when i click on another pane that has a tab, that tab flips.. and
from there it gets really buggy, to the point that switch pane focus
will flip some tabs."

@@Alex spec (across follow-ups):
- "strictly tied to panes which have at least 1 tab"
- "the cycle of each flip should not impact other panes"
- "it's basically a per-pane state of whether it's flipped or not"
- "it should persist across window reloads"

Root causes (both leaking the per-pane flip across panes):
1. `splitPane` copied `showingBack`/`back` onto the new pane, which is
   born empty -> a flipped 0-tab pane the flip chord could not undo.
2. `setActivePane` (round-1 closing-2 B2c) cleared the previous pane's
   `showingBack` on focus-move -> "switching focus flips tabs".

Fix: `showingBack` is a strictly per-pane boolean only `flipHybrid`
writes (guarded to panes with >= 1 tab); `closeTab`/`closeTabsInPane`
clear it when the pane empties; `restoreLayout` restores it only when
the pane still has tabs (persists across reload, no stale flip on an
empty pane).

Behaviour change to flag: the round-1 "keep showingBack across a
last-tab close" (mid-config UX) is SUPERSEDED by "strictly >= 1 tab" -
closing the last tab now drops the flip to the empty front. If @@Alex
wanted the old mid-config behaviour kept, say so and it can be revisited.

## Addendum 3 - document new asks + commit the docs now

@@Alex: "dont forget to document these new asks, and to commit all the
docs we're producing here as part of your deliverable."

This deviates from `feedback_coordination_docs_commit_timing` (keep
phase-13 docs untracked until round close). Done now per the explicit
ask: this addenda file + the journal + channel updates are committed to
main as a `docs(phase-13)` in-flight commit. The round-close
`docs(phase-13): close round 2` commit will still capture the final
tree.
