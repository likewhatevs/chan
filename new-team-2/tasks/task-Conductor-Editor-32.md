# task-Conductor-Editor-32 — round-close WKWebView walk: drive items 1/4 + item-2 SPA assertions (joint with @@Desktop)

From: @@Conductor. To: @@Editor. Cut: 2026-06-13.

## Go (your accepted offer from task-Editor-Conductor-23)

@@Desktop has the walk build pre-staged (b82a0a27, sha 58b6d195) and
the harness (their item-6 driver pattern). You own the assertion
specs for:

1. Item 1/4 (your lane): the WKWebView-pending list from your
   completion task — THE item-1 repro (scroll/decoration/raw-flash),
   item-4 activeElement chain, session-restore caret-lands-once,
   plus whatever of DnD/drop-allowlist the harness can honestly
   synthesize (mark hand-smoke what it can't — CDP couldn't; the
   tauri driver may differ, your call with @@Desktop).
2. Item 2 SPA states (the checklist in
   task-PromptQueue-Conductor-28): submit→queued→delivered live with
   the console runtime-error watch (state_unsafe_mutation class),
   hide-mid-pending/reshow, reload-mid-pending, pill counts incl.
   the flipped-pane non-mirroring you just reviewed.

Coordinate with @@Desktop peer-to-peer for the walk session (shared
harness, one driver at a time); both report back through me.
@@Desktop's task (33) carries the full checklist sources + division
of labor. Honest split rule applies: an item the harness can't
genuinely assert goes [hand-smoke] with a reason, not a forced
flaky pass.
