# @@LaneB journal - Phase 14

Worktree: `../chan-p14-lane-b` on `phase-14-lane-b` (off `main@10e0a1e1`).
Frontend lane: `web/`, `gateway/crates/identity/web`, `gateway/web-common`,
`web-marketing/`. No Rust.

## Status at a glance

| Item | State |
|------|-------|
| B3a draft-restore banner | DONE - code + tests + browser-verified |
| B3b Cmd+, panes-flip | DONE - code + tests + browser-verified |
| B4 /dl preserve guard | DONE - code + README + check green (CI path unverified) |
| Contract §1/§2 confirm | DONE - confirmed; Lane A flipped both to PINNED |
| B1a graph dir expand/collapse + persistence | DONE - code + tests + browser-verified |
| B1b cursor paging | DONE - rebased on main; wire-verified vs live endpoint |
| B1c locked pre-flight overlay | DONE - rebased on main; wire-verified vs live endpoint |
| B2 pristine cleanup | DONE (substantially) - history-comment strip, 174 -> ~5 |

Gates (worktree): `npm run check` 0/0, `npm test` 156 files / 1571 tests
pass, `npm run build` ok. Gateway frontend untouched (no diff vs main).

Merged to main 2026-05-30 as `c37a11e2` (`--no-ff` of `phase-14-lane-b`,
clean - disjoint from Lane C's docs). Merged web tree is byte-identical to
the gated-green branch tip `bb5ea670`. Local-only; not pushed.

## B3a - false "unsaved changes from a previous session" banner

Root cause: the localStorage hang-recovery buffer had no identity, so a
buffer written by the live page session (own edits past the 500ms
autosave) was indistinguishable from one stranded by a crashed earlier
load, and surfaced the recovery banner.

Fix (`web/src/state/editorBuffer.ts` + `FileEditorTab.svelte`):
- Per-page-load `SESSION_ID` stamped into every buffer. `divergentBufferOrNull`
  now returns null for an own-session buffer (live edit, already in the
  editor), an mtime-stale buffer (older than the last on-disk save; the
  browser wall clock and the fs mtime share one machine here), or content
  the editor already has. Only a different-load, divergent, newer buffer
  surfaces the banner.
- The recovery effect now keys on disk (`tab.saved`), not keystrokes, so
  it never mistakes live edits for a prior session; re-running on a save
  dismisses the banner with no extra bookkeeping. The clean-state branch
  preserves a pending recovery buffer (so a tab switch before the user
  acts doesn't lose the unsaved work).
- Rewrote `editorBuffer.test.ts` (25 tests: session/mtime/content cases +
  a create->type->autosave->save->reload lifecycle), consolidated the
  wiring guards into `FileEditorTab.recovery.test.ts`, and deleted the two
  obsolete `hangRecovery{PathKey,EffectOrder}.test.ts` (they pinned the
  removed stickiness/3-arg shape).

Browser-verified on a real build (canonical debug binary, swapped bundle):
same-session type / tab-switch / full reload -> NO banner, content
preserved; an injected different-session divergent buffer -> banner
appears, Restore recovers it. No console errors (clean reactivity).

## B3b - Cmd+, "panes flip" desync

Root cause: the Cmd+, focused-Hybrid flip (`App.svelte onWindowKey`, and
the desktop KEY_BRIDGE path `runCommand "app.settings.toggle"`) fired
unconditionally - even while a modal or the search overlay owned the
keyboard. It flipped the pane hidden behind the surface; dismissing the
surface revealed the stray flip on the obscured/previous pane. `onCtrlDCapture`
already guards tab-close this exact way ("modals own their own keyboard
context"); the flip was simply missing the guard.

Fix: added `paneChordBlocked()` (overlay stack + the four modal flags:
prompt / pathPrompt / confirm / draftClose) and gated both Cmd+, entry
points on it. Regression test `cmdCommaFlipGuard.test.ts`.

Browser-verified: BEFORE (canonical) open Search -> Cmd+, -> Esc flipped
the pane (`sb:1`); AFTER, identical steps leave the pane on its front.
Normal Cmd+, (no overlay) still flips.

## B4 - /dl preserve-release-metadata circular guard (addendum-1 #1)

`preserve-release-metadata.mjs` self-fetched the LIVE `https://chan.app/dl`;
on a 404 it preserved nothing, so once `/dl` was clobbered every later
marketing deploy kept it 404. Rewrote it to regenerate `/dl` from the
latest GitHub Release (the durable source) by chaining the same
`collect-release-assets` -> `generate-release-metadata` pipeline
`release.yml` uses (`--allow-missing-release` so a pre-first-release deploy
still succeeds). No live-site fetch. Updated `web-marketing/README.md`.
`npm run check` green (build + all release smokes). The actual GitHub-
release fetch only runs in CI / a real Pages deploy - empirically
unverified there.

Optional follow-up (flagged, NOT done - "not strictly Lane B", touches
`release.yml`): a shared concurrency group so `pages.yml` + `release.yml`
can never deploy `github-pages` at once. The manual-only `pages.yml` (r2)
already closed the original auto-race; this only hardens a human running
both at once.

## Contract (coordination/contracts.md)

Confirmed both seams in `event-lane-b-lane-a.md`; Lane A pinned both.
- §1 graph: pull-based cursor-paged `/api/fs-graph` spine + `/api/graph`
  overlays. Confirmed split matches today's code (graph tab already hits
  both; `graphData.svelte.ts` already appends incrementally; `GraphCanvas`
  has an incremental low-alpha layout). Cursor per-session (re-issue on
  reload; only the expanded set persists). 256/64KiB ceiling, B paces via
  `limit` + pull cadence.
- §2 preflight: poll `GET /api/preflight` + `POST /api/preflight/decision`,
  top-level `locked` drives the OverlayShell lock. Lane A keeps
  factory-reset desktop-side for v1; index bar = poll snapshot
  current/total at ~750ms. Lane A will ping when the endpoints land.

## B1a - graph directory expand/collapse + persistence

Double-click a directory node in the filesystem graph now expands/collapses
it in place (File Browser parity), replacing the old double-click "graph
from here" rescope (rescope stays on the inspector / right-click / chord).

- `GraphPanel` fs-mode `scopedNodeIds` now filters by an expanded set
  (`ancestorsExpanded`) instead of returning null; a node shows only when
  every ancestor directory up to the scope root is expanded.
- `onGraphDoubleClick` -> `toggleDirExpand`: expanding fetches the dir's
  next degree (`find -d 1`, single-dir `/api/fs-graph` depth=1) and merges
  it into the spine (`mergeFsResponse`); collapsing hides the subtree
  (cached, filtered out). Incremental - no full graph reload.
- The expanded set lives on `GraphTab.expanded`, serialized as `ge` in the
  tab hash/session (the FB *tab* persistence channel). The first attempt
  keyed an `fbTreeInstance` sessionStorage snapshot by
  `graph-tab-${tab.id}`, which broke on reload because tab ids regenerate
  (same class as the B3a editorBuffer bug); the GraphTab-field approach
  survives reload. `reconcileExpandedChildren` re-fetches expanded dirs'
  children on restore.
- The depth slider re-establishes the expanded set to depth N
  (`seedExpandedToDepth`, authoritative; overrides individual toggles) via
  the `appliedDepth`/`appliedScopeKey` trackers in `load()`; the first load
  trusts the restored/fresh set.

Gates: svelte-check 0/0, 1571 tests, build. Updated the `?raw` GI-9 pin
(`graphFsSpineCompleteness.test.ts`) to the new expanded-set invariant
(fs-mode still never enters the semantic file-seed BFS). Browser-verified
on a nested-dir drive: expand fetches+shows the next degree, collapse
hides it, expansion persists across a full reload (`ge` in the hash +
reconcile), no console/reactivity errors. Depth-slider reseed is
code-complete but not live-verified (the slider wasn't surfaced for the
test scope); flagged for the webtest walk.

Forward-compatible with Lane A's paged endpoint: `fetchDirChildren` uses
the existing single-dir fs-graph; B1b only layers `limit`/`cursor` on it
for very-wide dirs (integrate + `/tmp/linux` verify once Lane A merges).

## B1b - cursor-paged fs-graph (code-complete, verify post-rebase)

Built against Lane A's committed wire shape (`phase-14-lane-a` `cd1d6250`):
`FsGraphParams` gains `cursor`/`limit`; `FsGraphResponse` gains `cursor`
(null on final batch) + `done`; a request is paged iff it carries `limit`
or `cursor`; batch clamp [16, 256] nodes / 64 KiB; stale cursor -> 400.

- `web/src/api/types.ts`: `FsGraphResponse` += `cursor?`/`done?`.
- `web/src/api/client.ts`: `api.fsGraph` += optional `limit`/`cursor`
  (both absent = the whole-scope response, unchanged).
- `GraphPanel`: `load()` fs-branch and `fetchDirChildren` now pull bounded
  batches in a loop (`GRAPH_BATCH_NODES=128`), append each via
  `mergeFsResponse`, and `yieldToFrame()` between batches so a large scope
  fills in gradually without freezing the editor / FB / terminal / other
  graphs. seq-guarded so a superseding reload drops in-flight batches.
- Degrades safely against the old (non-paged) endpoint: no `cursor`/`done`
  -> the do-while runs once (one whole-scope fetch) = prior behavior.

Pending: build + `/tmp/linux` verify (the responsiveness criterion needs
Lane A's live endpoint). Follow-up: the depth-slider raise still does a
paged whole-scope refetch to depth N rather than the contract's
frontier-only single-dir expands; it's responsive (paged) but re-fetches
0..N. Optimize after the responsiveness baseline is confirmed.

## B1c - locked pre-flight overlay (code-complete, verify post-rebase)

Built against Lane A's `0f727ff2` shape: `GET /api/preflight` ->
`{ phase, locked, steps[{id,label,state,current?,total?,decision?}], error }`
(snake_case); `POST /api/preflight/decision { step, choice }`; v1 decision
is the `model` step (download vs keyword-only).

- `web/src/api/types.ts`: `PreflightSnapshot` + step/decision/error types.
- `web/src/api/client.ts`: `api.preflight()` + `api.preflightDecision()`.
- `web/src/components/PreflightOverlay.svelte` (new): a full-viewport
  LOCKED surface modeled on `MissingTokenOverlay` (no close button, not in
  the dismissable overlay stack, so ESC has nothing to dismiss). Polls
  `GET /api/preflight` ~750ms while `phase != ready`, renders the steps
  (index progress bar from `current/total`, the `model` decision buttons),
  dismisses when `phase: ready` (`locked: false`). Gives up after 5
  consecutive errors so an unreachable pre-flight never wedges the editor.
- Mounted in `App.svelte` alongside the other locked boot overlays.

Keyed on the snapshot's `locked` exactly as the contract specifies. A
ready (or absent) pre-flight shows nothing. Pending: build + verify the
locked-shell flow on a fresh workspace (lock, index bar, model decision,
dismiss on ready) after the rebase.

## Post-rebase verification (Lane A merged to main, ea771edb)

Committed Lane B work as `14f2bd14` (one commit; file overlaps across
App.svelte / GraphPanel.svelte / api/* preclude per-item commits without
interactive hunk staging) and rebased clean onto main (disjoint: B = web,
A = crates). Built the binary against Lane A's Rust (cargo 21s, shared dep
cache) and verified the wire integration against the live endpoints on a
fresh `/tmp/linux`:

- B1c: `GET /api/preflight` returns exactly the typed shape
  (`{phase, locked, steps:[{id,label,state,...}]}`). Index reports `done`
  within ~3s, so `phase: ready` / `locked: false` arrives fast - the lock
  is brief, not a multi-minute block (good UX). Overlay correctly shows
  nothing when ready.
- B1b: paged `/api/fs-graph` walked the full cursor chain to `done` -
  373 batches, 47,734 nodes at depth 4, terminates correctly; my load
  loop mirrors this (chase `cursor` until `done`). A stale/bogus cursor
  returns 400 as expected (I never reuse a cursor across walks).

Gates green throughout (svelte-check 0/0, 1571 tests, build). The
end-to-end browser visual (graph fills in gradually / locked-overlay
render) is the one remaining empirical check; deferred to the webtest
walk (the locked overlay is hard to trigger here since the index
completes fast and the embedding model is bundled, so `model` never
blocks). Per pre-release policy: gated-green + wire-verified work merges;
the browser-visual is recorded as the open verification.

## B2 - pristine cleanup (history-comment strip)

Survey: the history-narration debt was ENTIRELY in `web/src` (174 files,
~1405 tokens); the gateway identity SPA, `web-common`, and `web-marketing`
were already pristine (0 tokens). So B2 = a `web/src` comment sweep.

Run as parallel edit-only subagents on disjoint partitions (the first
attempt over whole directories over-ran badly - one agent ran ~5h before a
socket timeout - so the second pass used tight per-file/single-pass scopes
with an explicit anti-loop instruction, and all completed). I gated
centrally after each round and reconciled the `?raw` fallout myself.

Result: 174 -> ~5 files with genuine history tokens. Removed
fullstack-NN / phase / round / slice / addendum / @@handle / lane tags and
changelog narration across ~150 files, keeping WHY-snapshot comments;
deleted 5 redundant "comment-pin" `?raw` tests (whose only job was to
assert a now-removed historical comment exists; their code invariants are
covered by sibling tests) and converted others (e.g. inspectorActionsLayout,
firstBootDockedFb) to pin code patterns instead of comment text. Commits:
b6f01575 (round 1) + bb5ea670 (round 2 + reconciliation). Gates green
throughout (svelte-check 0/0, 1563 tests, build).

The ~5 remaining are: (a) LEGITIMATE keeps, not history - team `@@handle`
test data (teamOrchestrator/teamLoad/teamLead* tests), `@@mention` syntax
docs (contact.ts), `@@Alice`/`@@Bob` table fixtures, and a CSS-class
false-positive (`team-airplane-cell` matches `lane-c`); and (b) a few
`fullstack-a-NN` source comments PINNED by `?raw` tests (client.ts a-70/a-76,
GraphPanel a-52/a-58 via graphParentEdgeInvariant.test.ts) - tested,
documented invariants rather than dead narration. Removing (b) needs
coordinated source-comment + test-assertion rewrites; left for C1's
architect comment pass (which reviews these comments for voice anyway and
can coordinate both sides). Not blocking.

Note: em-dash cleanup (the no-em-dash writing rule) across NON-history
comments was out of scope for this history-strip pass; C1 covers it.

## Carryover / cross-lane note

`.txt` vs `.md` graph classification (user request, routed to Lane A):
`.txt` is treated as markdown-class in BOTH lanes - backend
`chan-workspace/fs_ops.rs` (`classify_ext "md"|"txt" => EditableText`,
`is_markdown_file` + link/heading extraction include txt) and frontend
(`fileTypes.ts MARKDOWN_EXTENSIONS = {md,txt}` -> `classifyPath` returns
`document`; `kinds.ts colorVarFor` aliases `text -> orange --g-doc`). A
backend-only fix leaves `.txt` orange. Frontend half (drop txt from
markdown-class + give `text` its own non-orange tone) is mine to take when
sequenced; pairs naturally with B2.
