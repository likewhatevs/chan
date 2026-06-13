# task-TeamFlow-Conductor-17 — review of 3d4f564b + 54b65a60: CLEAN PASS

From: @@TeamFlow. To: @@Conductor. Cut: 2026-06-12.
Re: task-Conductor-TeamFlow-16 (rerouted launcher review, @@Desktop
item 6 + B3).

## Verdict

Clean pass on both commits, all six targets. Three non-blocking
observations, no findings requiring action. Method: static review at
commit state (git show blobs; the launcher has no test harness) +
helper/file verification for the B3 pins; @@Desktop's 36/36
instrumented WKWebView walk (task-Desktop-Conductor-15.md) is the
empirical leg and its claims (stray-Escape-inert, double-click guard,
pill consistency) match what the code says.

## Target-by-target

1. In-flight guard vs re-render: `btn = e.currentTarget` is disabled
   on the LIVE node, and within the handler's own flow no re-render
   happens until AFTER the awaited set_workspace_on (the only refresh
   follows the await), so the guard holds across the whole vulnerable
   window. Failure path restores disabled=false on the still-live
   node BEFORE refresh replaces the row; success path never restores,
   but render() rebuilds main.innerHTML wholesale and re-binds, so
   the stale node is discarded and the new button derives from true
   serve state. No stale-element bug. Post-refresh the handler only
   uses the captured `path` string, never the detached DOM.
2. Dialog lifecycle: OK click, Escape, and backdrop click all funnel
   into the one close(), which removes the document keydown,
   removes the overlay, and resolves. Listeners are per-dialog
   closures: keydown is explicitly removed (the design's named trap,
   avoided), ok/overlay listeners die with the node. No stacking on
   repeated dialogs — code-confirmed and corroborated by the walk's
   stray-Escape-inert check. Backdrop close is correctly gated on
   e.target === overlay so in-dialog clicks don't dismiss.
3. hasUrl split in renderOpenSplit: launch unconditional; Open in
   Browser keeps the hasUrl gate (renamed browserDisabled — rename is
   faithful); the caret-disabled expression is byte-unchanged; Forget
   untouched. Remote rows keep their own launch handlers
   (open_tunneled_workspace / open_outbound_workspace) which guard on
   their own ids — the local turn-on path is unreachable from them
   (it binds only under tr[data-path]).
4. Failure routing splits on DIRECTION: the pill catch branches on
   toggle.checked (the requested direction; stable mid-flight because
   the toggle disables itself for the transition), and the launch
   path is inherently turn-on. Turn-on → dialog from BOTH call sites;
   turn-off → banner. Not call-site-keyed. Conforms.
5. Verbatim error: string | .message | String(reason) →
   body.textContent. No rewording, no truncation, and textContent
   keeps it injection-safe. map_open_error strings arrive untouched.
6. B3 pins are real: capability_permissions() serde-parses
   include_str!("../capabilities/default.json") (the shipped file)
   and asserts absence in the parsed permissions array;
   app_permission_set("main-window") PANICS on a missing set id (no
   vacuous pass) and panics on non-string entries (an object-form
   grant still fails the test). Re-adding the grant to either surface
   fails the test. Greps of the shipped files confirm the grant lives
   only in local-drop.json today. Mirrors the existing
   workspace.json/workspace-window pins exactly.

## Non-blocking observations

- O1 (parity, pre-existing class): an EXTERNAL re-render mid-flight
  (registry-changed → refresh() with a JSON delta from another row,
  main.js:1256) would replace the row and discard the disabled state,
  re-arming the button while the turn-on is still in flight. The pill
  toggle's disabled guard has the identical hazard today; worst case
  is a second set_workspace_on losing the race and landing in the new
  failure dialog. Narrow, self-healing, not a regression — noting so
  the shape is on record if a dedupe-by-path guard is ever wanted.
- O2 (cosmetic): two overlapping failure dialogs (pill + launch
  failing near-simultaneously) each add a document keydown; one
  Escape closes both at once. Each removes its own listener — no
  leak — just a both-at-once dismiss of two same-class messages.
- O3 (method note): I did not build the chan-desktop workspace to
  re-run the B3 test (test-only +14 in the separate heavy workspace);
  verification was by reading the parse helpers + grepping the
  shipped capability/permission files, which the assertions consume
  directly. @@Desktop's lane gate ran it.

## Status

Standing assignments unchanged and still outrank: primed for item-1
restructure (sha pending) and item-2 web-half (sha pending). Holding.
