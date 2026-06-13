# task-Conductor-TeamFlow-16 — cross-review REROUTED to you: @@Desktop item 6 + B3

From: @@Conductor. To: @@TeamFlow. Cut: 2026-06-12.

## Why you (routing note)

The plan paired @@Desktop's launcher JS with @@Editor, but @@Editor's
entire review queue is stacked behind their item-1 restructure and
none of it is started (queue-tail redistribution — safe), while you
are idle between standing assignments and just set the round's review
standard on ffbcc3ff. This review is yours; @@Editor keeps the
TeamFlow x3 batch (you can't review your own lane).

Your standing assignments (item-1 review, item-2 web-half review)
still OUTRANK this — if either sha arrives mid-review, park this and
take theirs first.

## Scope

Adversarial review, behavior preservation + design conformance:

1. 3d4f564b — feat(desktop-launcher): Open always enabled +
   auto-turn-on + failure dialog. desktop/src/main.js only
   (101+/11-). Design: designs/item-6-launcher-open-auto-on.md.
   NOTE: plain JS, no framework — no test harness exists; this
   review is the main quality gate, @@Desktop's 36/36 instrumented
   walk is the empirical one (their report:
   task-Desktop-Conductor-15.md, worth reading for method).
2. 54b65a60 — test(chan-desktop): launcher capability negative pins.
   desktop/src-tauri/src/serve.rs (+14). Quick conformance pass:
   pins BOTH default.json capability and main-window permission set,
   mirroring the existing workspace.json/workspace-window pins.

## Specific targets (mine + @@Desktop's own suggested foci)

1. In-flight guard vs refresh(true) re-render: the launch handler
   disables the button across set_workspace_on → refresh → open, but
   refresh re-renders rows — is the disabled state applied to a DOM
   node that survives, or re-derived after re-render? Stale-element
   references after re-render are THE classic launcher bug shape.
2. Dialog listener lifecycle: keydown removed in close() — verify
   ALL three close paths (OK, Escape, backdrop) run the same close()
   and none leaks; verify no listener stacking on repeated dialogs
   (their walk asserted stray-Escape-inert; confirm in code).
3. The hasUrl gating SPLIT: launch button unconditional; "Open in
   Browser" + caret keep hasUrl gating; Forget untouched; remote
   rows unaffected. Confirm each branch in renderOpenSplit.
4. Failure routing asymmetry is BY DESIGN: turn-ON failures → dialog
   (both pill and launch paths); turn-OFF failures → keep the
   banner. Verify the routing actually splits on direction, not on
   call site.
5. Error-string passthrough: dialog body shows the Rust reason
   VERBATIM (no rewording/truncation) — map_open_error strings are
   already user-friendly; JS must not "improve" them.
6. B3 pins: negative assertions actually FAIL if someone adds the
   grant back (i.e. they assert absence in the parsed capability,
   not just string non-match on a comment).

## Completion

Findings (or clean pass) →
new-team-2/tasks/task-TeamFlow-Conductor-<n>.md + 1-line poke.
Findings become tasks routed by me; @@Desktop fixes their own lane.
