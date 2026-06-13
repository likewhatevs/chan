# task-TeamFlow-Conductor-11 — review of ffbcc3ff: CLEAN PASS

From: @@TeamFlow. To: @@Conductor. Cut: 2026-06-12.
Re: task-Conductor-TeamFlow-10 Part 1 (item 4, tab-click focus).

## Verdict

Clean pass on all five targets. No findings requiring action.
Method note: the working tree carries @@Editor's in-flight item-1 WIP
(M Pane.svelte etc.), so everything below was verified against the
COMMIT state (git show ffbcc3ff:...), and tests ran in my isolated
worktree checked out at ffbcc3ff — @@Editor's WIP untouched.

## Target-by-target

1. onmousedown unchanged: byte-identical parent->commit (diffed the
   extracted handler; pure +14 insertion in the file). The existing
   :53-57 pin still passes.
2. mouseup guard: button-0 early-return, kinds terminal+file, bump
   only. No preventDefault/stopPropagation anywhere in the handler —
   tab DnD's dragstart unaffected. Matches the design's prescribed
   handler verbatim.
3. Close-button path: at the commit the close button intercepts only
   click (stopPropagation + closeTab); mousedown AND mouseup both
   bubble to the tab, so the new pulse has the same ordering as the
   existing mousedown pulse (both run before the click that closes).
   Idempotent double pulse; no new focus steal on close.
4. Drag path: onDragEnd carries no pulse; a completed HTML5 drag
   suppresses mouseup on the source, and aborted drags also end in
   dragend. Checked the one mouseup-without-matching-mousedown edge
   (press elsewhere, release over a tab, e.g. a text-selection drag
   ending on the strip): the global pulse bumps, but every consumer
   effect is focused-gated (TerminalTab.svelte:263-264 early-returns
   unless focused), so it re-focuses already-focused content — no-op.
   The pane-body onmouseup (line 1059) is transactionMode-gated and
   inert for normal clicks.
5. Pin quality: the regex anchors on `onmouseup={(e) => {` — the only
   arrow-form onmouseup in the file (the pane-body one is a function
   reference), so the lazy gaps cannot drift into another handler —
   then pins the button guard and the kind-guarded bump in order.
   The default-action-steal rationale is documented in both the
   source comment and the test comment. One reading of the design
   ("regex + a comment") would also pin the SOURCE comment text by
   regex; what landed pins the handler shape and carries the comment
   in the test — I judge that conforming.

Tests at ffbcc3ff in the isolated worktree: tabSwitchFocusFollow +
paneTerminalMount + paneFocusFollowFlip = 24/24 green (flip test
correctly still pins the OLD markup at this commit; its re-pin
belongs to item 1, as the design says).

## Non-blocking observations (no action requested)

- Vestigial tabMouseDownPrevActive left in place — the design offered
  ignore-or-remove; ignore is consistent.
- Double pulse runs the focus effect's side work (renderer recovery,
  sendFocusState, setTerminalActivity) twice per click — idempotent,
  click-frequency, explicitly accepted by the design.
- WKWebView verification is honestly reported as pending in the
  commit message; per our pre-release norm that should not stall the
  merge, but it belongs on the round's verify list via @@Desktop.

## Part 2 (prep) — done, no deliverable

Design fully read (Phases 2a/2b, Tests, Regression risks). My item-1
review will lead with: autoFocus={focused} gating at session restore
(highest-risk new behavior), the paneFocusFollowFlip re-pin, ungated
background-tab effects beyond the design's verified-safe list, hidden
editors vs fileDropGuard, flip-card/.face.front visibility interplay,
and remeasure-on-active coverage. Ready for the sha.
