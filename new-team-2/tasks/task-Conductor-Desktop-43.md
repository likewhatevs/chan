# task-Conductor-Desktop-43 — WKWebView walk: graph keep-alive (3fdd4bfe), joint with @@Editor

From: @@Conductor. To: @@Desktop. Cut: 2026-06-13.

## Go

@@TeamFlow review CLEAN PASS (task-TeamFlow-Conductor-42, 7/7 +
2 mutation bite-tests); HEAD settled at 3fdd4bfe. Build the WKWebView
gate now: sync your worktree forward to 3fdd4bfe, recreate the harness,
strip-clean binary, provenance-verify the served binary before any
assertion (record the sha).

## Division (same as the round-1 walk)

- YOU own: build, harness, the dir/tag-scoped fixture with a real
  scope BOUNDARY (subdir graph + files OUTSIDE it — your pre-plan,
  required for #5), provenance, teardown.
- @@EDITOR drives the assertion specs (they offered; coordinate
  peer-to-peer, one driver at a time on the shared harness). They know
  the load()-instrumentation pattern from the Chrome smoke.

## Checklist (graph keep-alive walk = items 1/6/7 + console, plus 2
additive)

1. **The @@Alex-visible symptom**: switch onto a graph tab → NO redraw/
   reload flash (the whole point); pan/zoom preserved across switches.
2. **Reload menu item**: right-click → Depth → Reload → Copy link
   order; Reload fetches once, menu closes.
3. **Lazy restore**: session with multiple graph tabs → only the active
   one fetches on restore.
4. **Hidden→dirty→one-on-reactivation**: in-scope on-disk edit while
   hidden → no reload hidden, exactly one on reactivation.
5. **#5 out-of-scope (the new empirical gap)**: on the dir/tag-scoped
   fixture, edit an OUT-of-scope file while the graph is hidden →
   ZERO reload on reactivation. (@@Editor couldn't exercise this in
   Chrome — workspace-scoped test graph.)
6. **Resize-while-hidden**: resize the pane while the graph tab is
   hidden → reactivate → resume resize() refits the backing store
   WITHOUT jumping the transform.
7. **Console sweep**: no state_unsafe_mutation / errors across the walk
   on the real engine.

Honest-split rule: anything the harness can't genuinely assert →
[hand-smoke] with a reason for @@Alex's list; never a forced pass.

## Report

One completion task from YOU (Editor co-signs): per-item
PASS/HAND-SMOKE table, provenance line (served sha), console result,
teardown confirm. Findings → tasks via me.
