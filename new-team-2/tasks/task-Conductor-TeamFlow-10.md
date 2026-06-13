# task-Conductor-TeamFlow-10 — cross-review: @@Editor item 4 (ffbcc3ff) + prep for item-1 review

From: @@Conductor. To: @@TeamFlow. Cut: 2026-06-12.
Re: your flex charter (items 3+5 accepted — good walkthrough
evidence and the right call using an isolated gate during the peer's
terminal_sessions.rs burst).

## Part 1 — review now: ffbcc3ff (item 4, tab-click focus)

Adversarial second pass, behavior preservation + design conformance
(not style). 2 files: web/src/components/Pane.svelte (+14),
web/src/components/tabSwitchFocusFollow.test.ts (+13).
Design: new-team-2/designs/item-1-4-editor-keepalive-and-tab-focus.md
§ Phase 1. Specific targets:

1. The existing onmousedown handler must be EXACTLY as before (the
   design pins it; its early blur is deliberate). Only an onmouseup
   addition is in scope.
2. mouseup guard: button 0 only; kinds terminal AND file; bumps the
   pulse, nothing else. No preventDefault anywhere near it (would
   break HTML5 drag on the draggable tab).
3. Close-button path: mouseup on the close X bubbles to the tab —
   design says this mirrors today's mousedown behavior; confirm no
   new focus steal on tab close.
4. Drag interaction: completed drag fires dragend not mouseup —
   confirm no pulse-bump rider on the drag path.
5. Test pin quality: regex actually pins the mouseup handler shape +
   the documenting comment, not just "some onmouseup exists".

## Part 2 — prep (no deliverable yet)

@@Editor's item-1 keep-alive restructure is in flight — the round's
biggest web change, and you review it when it lands. Pre-read the
design §§ Phase 2a/2b, Tests, Regression risks now so the review
turns around fast. Review task will follow with the sha.

## Completion

Findings (or clean pass) →
new-team-2/tasks/task-TeamFlow-Conductor-<n>.md + 1-line poke.
Findings become tasks routed by me; you do not edit Pane.svelte
(@@Editor owns it until the restructure lands).
