# Phase 14 - Gateway monorepo migration, then a frontend review and pristine cleanup

Status: closed (three rounds; round 1 the gateway migration, round 2 the
frontend review and pristine cleanup, round 3 paced hot paths plus the
pre-flight relocation)
Span: 2026-05-29 to 2026-05-30 (estimate; see Duration)

Tags: #refactor #docs #performance #graph #ci #desktop

## Initial asks

Three rounds, each with its own source request from @@Alex.

Round 1 (roadmap-round-1.md): bring the chan.app gateway (the account,
sign-in, and reverse-proxy surface) into this monorepo. Port the existing
`chan-writer/chan-gateway` workspace (`profile`, `identity`, `drive-proxy`,
`admin`, `gateway-common`) in, adapt it to the monorepo's hardened crates
and conventions, re-home it onto the in-repo `chan-tunnel-*` crates, apply
the `drive` to `workspace` rename end to end, and get it building, tested,
packaged, and documented as a nested Cargo workspace that is NOT a member
of the root workspace (so the single-binary, no-runtime-deps core build
stays free of Postgres / sqlx / oauth2). Round 1 was already done when the
later rounds opened.

Round 2 (roadmap-round-2.md): a deep code review and pristine cleanup of
the frontend trees (`web/`, `gateway/crates/identity/web`,
`gateway/web-common`, `web-marketing/`), now that the gateway's identity
SPA and shared web package sit alongside the editor SPA. Framed around
the first public release, so there is nothing to be backwards compatible
with: be ruthless about back-compat shims, aliases, deprecation paths,
dead transitional code, and changelog-style comments. The published code
must read as if written today from scratch. Quality only, not a feature
round: keep the working outcomes identical. Driven by `/webdev` (review
and cleanup of code) and `/architect` (comments, documentation, and
user-facing copy).

Round 3 (roadmap-round-3.md): two hardening-and-relocation themes, both
preserving today's outcomes. Theme 1, pace and stream the hot paths so the
frontend never blocks on large data: on a large workspace (the Linux
kernel source at `/tmp/linux`) the dashboard indexing graph and the graph
tab pull enough at once to hog the API bus and freeze the UI. The frontend
must stay responsive at all times, the backend transmitting small bounded
amounts the frontend appends gradually, with backpressure. A revised graph
gesture model rode along: double-click on a directory node expands or
collapses it in place with no graph reload, the old double-click "graph
from here" rescope is dropped (rescope stays in the inspector), and the
expanded set persists across a window reload (File Browser parity). Theme
2, move the new-workspace pre-flight out of chan-desktop into chan-server's
first boot so local and remote workspaces share one flow, presented on a
locked OverlayShell (no close button, ESC ignored) until completion.

An addendum (addendum-1.md), filed by @@LaneB at @@Alex's direction,
folded five phase-13 round-2 carryovers into the lanes: a circular
`/dl` preserve-release-metadata guard, flaky tests that gate releases, a
pending WKWebView desktop walk, the Team Work notification bubbles slated
to return later (an intentionally retained blueprint, not to be cleaned),
and the vestigial `team-work-N` draft convention.

## Team, profiles, and coordination

Lanes are positional phase-14 handles, not the named roster. @@LaneC ran
the `/architect` seat (the journals second-brain reorg plus the round-2
frontend comments/docs/copy pass), which resolves to a card via
[../../agents/README.md](../../agents/README.md).

```
handle    role this phase                              card
--------  -----------------------------------------    ----------------
@@Alex    human owner; planner; sole push authority;   (human owner)
          authored the three roadmaps and addendum-1
@@LaneA   backend hot paths (paced graph delivery) +   (no card; nearest
          new-workspace pre-flight (chan-server) +      is fullstack-a.md)
          chan-desktop launch; Rust only
@@LaneB   all frontend: incremental graph render +     (no card; nearest
          pre-flight overlay lock, then the round-2     is fullstack-b.md)
          pristine cleanup; no Rust
@@LaneC   the /architect: C2 docs/journals reorg       architect.md
          (concurrent) + C1 frontend comments/docs/
          copy pass (closing wave)
```

@@LaneA and @@LaneB were each allowed 1-2 in-session subagents (@@LaneB
ran its B2 cleanup as parallel edit-only subagents over disjoint
per-file partitions, after a first whole-directory attempt over-ran).

Coordination scheme: per-lane git worktrees (`../chan-p14-lane-{a,b,c}`)
hold source code only; all coordination docs (plans, journals, contracts,
event channels) are edited in the main checkout by absolute path, so the
bus stays one shared, append-only, conflict-free log. The cross-lane
seams between @@LaneA (backend) and @@LaneB (frontend) live in
`coordination/contracts.md`: @@LaneA proposes, @@LaneB confirms, and the
agreed shape is PINNED there before either side builds against it. Two
contracts were pinned this round, incremental graph delivery (section 1)
and the new-workspace pre-flight state machine (section 2). Directional
append-only inboxes (`event-<from>-<to>.md`) carry the announcements and
flags between every pair. @@LaneA and @@LaneB ran concurrently sharing
only those seams; @@LaneC's C2 (journals only) collided with nobody and
ran concurrently too, while C1 ran last because it edits the same
frontend code @@LaneA and @@LaneB were rewriting. The whole doc tree
stayed untracked as the live bus through the round.

## Duration

Estimate: 2026-05-29 into 2026-05-30. Basis: git author dates. The round-1
gateway migration commits run 2026-05-29 00:08 to 03:46; the round 2 and 3
lane work (the journal-dir commits) runs 2026-05-29 10:16 to 2026-05-30
08:55, with the dated journal headers landing on 2026-05-29 and
2026-05-30. The coordination tree stayed untracked during the round, so
git gives the bookends rather than continuous wall-clock; session hours
are not recoverable.

## Highlights and lowlights

Highlights:
- The gateway migration landed as an isolated nested workspace: zero
  gateway crates show in `cargo metadata` at the root, so the core's
  single-binary, no-runtime-deps line held while the gateway gained its
  own CI gate, release `.deb` flow, and single-source domain config. The
  prod packaging path was validated end to end in a systemd sdme
  container.
- @@LaneA's first cut of the paced graph delivery (commit `cd1d625`)
  matched the eventually-pinned contract section 1 exactly, so @@LaneB had
  zero rework on the graph seam. The whole-scope path (no params) stayed
  byte-identical, so the existing depth-cap probe and CLI did not move.
- The contract-first split paid off at integration: against the live
  endpoint on `/tmp/linux`, the paged `/api/fs-graph` walked the full
  cursor chain (373 batches, 47,734 nodes at depth 4) and terminated
  correctly, and the pre-flight reached `ready` within roughly 3 seconds
  so the lock was brief rather than a multi-minute block.
- @@LaneA caught a THIRD indexer-flake offender
  (`writes_to_drafts_subtree...`) that the addendum had not named, only
  because it ran the full parallel `cargo test --all-targets` gate (the
  load profile that flakes), and fixed it without losing coverage.
- @@LaneB's pristine sweep cut history-narration comments from 174 files
  to about 5, keeping only WHY-snapshot comments and legitimate
  test-data `@@handle` mentions.

Lowlights:
- @@LaneA over-committed to an SSH-forward diagnosis for a "blank remote
  window" report; the actual fix was a one-line flag-name typo
  (`--tunnel-workspace` should be `--tunnel-workspace-name`) in the
  desktop snippet, which a 30-second grep of the desktop's `chan serve`
  command construction would have surfaced first. The same wrong flag
  then turned up in three more places (gateway UI plus docs), handed to
  @@LaneC.
- Three empirical checks could not be run by an agent and were deferred:
  the WKWebView desktop walk (Chrome/Blink cannot reproduce WKWebView),
  the graph-fills-in-gradually browser visual, and the locked-overlay
  render (the index completes fast and the embedding model is bundled, so
  the `model` decision rarely blocks long enough to observe the lock).
- @@LaneB's first B2 cleanup attempt over whole directories over-ran
  badly (one subagent ran roughly 5 hours before a socket timeout); the
  second pass needed tight per-file single-pass scopes with an explicit
  anti-loop instruction.

## Constructive feedback

- @@LaneA: for "desktop launches X and X is broken," diff the exact
  command and flags the desktop emits against the CLI's clap definitions
  before theorizing about transport. And make de-flake work always
  validate against the full parallel `--all-targets` run, since that is
  the load profile that flakes; scoping A5 to the two named offenders
  missed the third until the full gate ran.
- @@LaneB: keep subagent scopes tight (per-file, single-pass, with an
  anti-loop guard) from the start; the over-run came from whole-directory
  scopes. The session-identity bug class recurred twice this round (the
  draft buffer with no session marker, and a graph-tab persistence key
  bound to a regenerating tab id), so watch for state keyed on an
  ephemeral id.
- @@Alex: the contract-first split (propose, confirm, pin before
  building) is the right shape for a concurrent backend/frontend round and
  it produced zero seam rework; worth keeping. The model-prompt policy and
  the desktop pre-flight relocation are product calls that landed as
  @@LaneA-flagged carryovers rather than ratified decisions; stating the
  intended default up front would let the lane finish them in-round.
- Architect (@@LaneC): the C2 spec-first approach (sign off the reorg
  shape before writing any report) kept the large journals migration
  coherent. Flagging product/validation inconsistencies on the bus (the
  identity charset mismatch, the tunnel domain drift) rather than
  rewriting copy blindly was the right call.

## What shipped, tried, and undone

Round 1 (gateway migration, versioned in lockstep at 0.18.0): the gateway
moved into `gateway/` as its own Cargo workspace, re-homed onto the in-repo
tunnel crates by path; `drive-proxy` became `workspace-proxy` with the
`drive` to `workspace` rename applied suite-wide (cookie `workspace_gate`,
host `workspace.chan.app`, routes `/api/workspaces/*`, env `WORKSPACE_*`,
tables `workspaces` / `workspace_grants`, migrations edited in place since
pre-release); single-source domain config (`gateway_common::domain::Domains`
derives every host from `CHAN_DOMAIN` plus `PUBLIC_SCHEME`); a new
`gateway-ci.yml` gate and the four gateway `.deb`s wired into `release.yml`;
and packaging/docs reconciled to the current env contract (a pre-existing
`configure.sh` `install /dev/stdin` bug fixed along the way).

Round 2 and 3 lane outcomes:
- @@LaneA: cursor-paged `/api/fs-graph` delivery (opt-in via `limit`,
  resumed via opaque `cursor`, bounded DFS batches of at most 256 nodes /
  64 KiB, idempotent cursors, scope-bound rejection with 400); the
  depth-slider "next degree, not a re-walk" primitive; the new-workspace
  pre-flight endpoints (`GET /api/preflight` poll plus
  `POST /api/preflight/decision`), derived from live indexer state rather
  than stored, with `locked = phase != ready`; the backend half of the
  draft-banner stress test; de-flaking of three indexer/PTY tests by
  intent (capability-gated skips, not bigger timeouts); and dropping the
  vestigial `team-work-N` convention.
- @@LaneB: the false "unsaved changes from a previous session" banner fix
  (a per-page-load `SESSION_ID` plus an mtime-stale guard, so own-session
  edits never raise the banner while a genuine crashed session still
  recovers); the Cmd+, panes-flip desync fix (a `paneChordBlocked()`
  guard so the flip never fires while an overlay or modal owns the
  keyboard); the `/dl` preserve-release-metadata rewrite to regenerate
  from the latest GitHub Release instead of self-fetching the live site
  (closing the circular 404 guard); graph directory expand/collapse with
  reload-persistence; cursor-paged graph consumption (chase `cursor`
  until `done`, yielding to a frame between batches); the locked
  pre-flight `PreflightOverlay.svelte`; and the pristine comment sweep
  (174 files to about 5).
- @@LaneC: C2 reorganized `docs/journals/` into per-phase second-brain
  reports (the front door) with the raw material archived per phase and
  all images replaced by text descriptions, committed as `741aa787`. C1
  swept ASCII typography across all four frontend trees (em/en dashes,
  ellipsis, middle dot, and the corresponding HTML entities normalized to
  `-` and `...`, about 100 files in `web/`), made grounded factual
  corrections (a botched rename codemod that turned the verb
  "Drives/Controls" into "Workspaces", stale pre-Phase-5 `chan-core`
  references corrected to the real post-split crates, an outdated "RP"
  welcome-tile comment), and rewrote `web/src/editor/design.md` as a
  present-state snapshot (the tiptap-to-CM6 migration history preserved in
  the journal and CHANGELOG, not in code).

Tried, deferred, or left as carryovers:
- A3 desktop relocation (`default_workspace.rs` / `serve.rs` / `main.rs`)
  was NOT done: the server pre-flight is additive and the desktop already
  launches `chan serve`, so the rewiring is best done once @@LaneB's
  locked OverlayShell exists and the flow can be verified in WKWebView
  (couples to the A5d walk).
- @@LaneA made a unilateral product call on the model-prompt policy
  (prompt only when semantic search is enabled but the model is missing,
  not on every fresh workspace) and flagged it loudly for @@Alex to
  ratify; factory-reset was kept desktop-side for v1 to avoid coupling
  chan-server to the desktop's default-Chan-root concept.
- The "blank remote window" investigation spent effort proving the SPA
  loads fine over loopback before the real cause (the flag-name typo) was
  found; the one-line desktop fix was later landed directly on main by
  @@Alex as `f2eb32a9` (and the gateway-copy and gateway-docs occurrences
  swept), so the lane's own snippet commit went redundant on rebase.
- The tunnel hostname `drive.chan.app` was renamed to `workspace.chan.app`
  across the code, the chan client tunnel default, the `chan-tunnel-*`
  crates, chan-server, desktop, the manual, and marketing copy, after
  @@Alex chose it as the canonical host (the 75 `drive.chan.app` mentions
  under `docs/journals` are history and were left as written).
- The identity charset mismatch (the SPA advertises `._-` for a workspace
  name while `--tunnel-workspace-name` accepts only `[a-z0-9-]`) was left
  as an open carryover for @@Alex to sort out after close; the depth-slider
  paging optimization (frontier-only single-dir expands rather than a
  paged 0..N refetch) was flagged for a follow-up after the responsiveness
  baseline.

Verification gate carried out of the round: the WKWebView desktop walk
(`make macos-chan-dmg`, then Cmd+Shift+N, Cmd+I no longer Dashboard,
Cmd+P Team Work, self-upgrade 0.17.0 to 0.18.0 from `/dl`) is a human
verify only, since Chrome/Blink cannot reproduce WKWebView.

## Raw material

Raw working material (per-author journals, task/request/roadmap files,
the coordination contracts and event channels) is preserved in git
history under this phase's flat working tree; it was removed from the
working tree in the phase-15 docs cleanup.
