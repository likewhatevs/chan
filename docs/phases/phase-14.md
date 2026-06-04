# Phase 14 - Gateway monorepo migration, then a frontend review and pristine cleanup

Status: closed (three rounds; round 1 the gateway migration, round 2 the
frontend review and pristine cleanup, round 3 paced hot paths plus the
new-workspace pre-flight relocation)
Span: 2026-05-29 to 2026-05-30 (estimate; basis: git author dates)
Versions: v0.18.0
Tags: #refactor #docs #performance #graph #ci #desktop

## Roadmap (the asks)

Round 1: bring the chan.app gateway (account, sign-in, and reverse-proxy
surface) into this monorepo. Port the existing chan-gateway workspace
(`profile`, `identity`, `drive-proxy`, `admin`, `gateway-common`) in,
adapt it to the monorepo's hardened crates and conventions, re-home it
onto the in-repo `chan-tunnel-*` crates, apply the `drive` to `workspace`
rename end to end, and get it building, tested, packaged, and documented
as a nested Cargo workspace that is NOT a member of the root workspace
(so the single-binary, no-runtime-deps core build stays free of Postgres /
sqlx / oauth2). Round 1 was already complete when the later rounds opened.

Round 2: a deep code review and pristine cleanup of the frontend trees
(`web/`, `gateway/crates/identity/web`, `gateway/web-common`,
`web-marketing/`), framed around the first public release. Nothing to be
backwards compatible with: be ruthless about back-compat shims, aliases,
deprecation paths, dead transitional code, and changelog-style comments.
The published code must read as if written today from scratch. Quality
only, not a feature round. Driven by `/webdev` (review and cleanup) and
`/architect` (comments, documentation, user-facing copy).

Round 3: two themes, both preserving existing outcomes. Theme 1 (hot
paths): pace and stream the graph delivery so the frontend never blocks on
large workspaces. On the Linux kernel source at `/tmp/linux`, the dashboard
indexing graph and the graph tab pull enough at once to hog the API bus and
freeze the UI. The backend must transmit small bounded batches that the
frontend appends with backpressure. A revised graph gesture model rode
along: double-click on a directory node expands or collapses it in place
(no reload), with the expanded set persisting across a window reload for
parity with the File Browser. The old double-click "graph from here"
rescope is dropped (rescope stays in the inspector). Theme 2 (pre-flight):
move the new-workspace pre-flight out of chan-desktop into chan-server's
first boot so local and remote workspaces share one flow, presented on a
locked OverlayShell (no close button, ESC ignored) until completion.

An addendum folded five phase-13 round-2 carryovers into the lanes: a
circular `/dl` preserve-release-metadata guard, flaky tests blocking
releases, a pending WKWebView desktop walk, the Team Work notification
bubbles (intentionally retained blueprint, not to be cleaned), and the
vestigial `team-work-N` draft convention.

## Rounds and waves

Round 1 ran before rounds 2 and 3 opened. The gateway migration is a
single coherent body of work; git author dates place those commits from
2026-05-29 00:08 to 03:46.

Round 2 and 3 ran concurrently across three lanes. @@LaneA (backend Rust)
and @@LaneB (frontend only) started in parallel, gated on pinned contracts
for the two cross-lane seams. @@LaneC ran C2 (journals reorg, no source-
code overlap) concurrently and C1 (frontend comments/docs/copy) last,
after @@LaneA and @@LaneB had settled the code. The coordination tree
stayed untracked as the live bus throughout; the dated journal headers
land on 2026-05-29 and 2026-05-30.

Lane work from rounds 2 and 3:
- @@LaneA: cursor-paged `/api/fs-graph` (A1), depth-slider next-degree
  primitive (A2), pre-flight backend endpoints (A4), draft-banner stress
  test backend half (B5 assist), de-flake of three indexer/PTY tests (A5),
  drop `team-work-N` convention. Desktop relocation (A3) was staged but not
  completed (see Tried/Deferred).
- @@LaneB: false unsaved-changes banner fix (B1), Cmd+, panes-flip desync
  fix (B2), `/dl` circular 404 guard rewrite (B3), graph directory
  expand/collapse with reload-persistence (B4), cursor-paged graph
  consumption (B4), locked pre-flight `PreflightOverlay.svelte` (B4), and
  the pristine comment sweep (B2 cleanup subagents).
- @@LaneC C2: reorganized `docs/journals/` into per-phase second-brain
  reports (committed as `741aa787`). C1: ASCII typography sweep across all
  four frontend trees, grounded factual corrections (botched rename codemod,
  stale pre-Phase-5 `chan-core` references, outdated welcome-tile comment),
  and rewrote `web/src/editor/design.md` as a present-state snapshot.

## Team and coordination

Lanes are positional phase-14 handles. The agent roster is in
../agents/README.md.

```
handle    role this phase
--------  --------------------------------------------------
@@Alex    human owner; authored the three roadmaps and
          addendum-1; sole push authority
@@LaneA   backend hot paths + pre-flight server; Rust only
@@LaneB   all frontend; no Rust; ran B2 cleanup as parallel
          edit-only subagents over per-file partitions
@@LaneC   /architect: C2 journals reorg (concurrent) then
          C1 frontend comments/docs/copy (closing wave)
```

Coordination scheme: per-lane git worktrees (`../chan-p14-lane-{a,b,c}`)
hold source code; all coordination docs (plans, journals, contracts, event
channels) are edited in the main checkout by absolute path, keeping the
bus one shared, append-only, conflict-free log. The cross-lane seams
between @@LaneA and @@LaneB lived in `coordination/contracts.md`: @@LaneA
proposed the contract shape, @@LaneB confirmed it, and the agreed API was
PINNED before either side built against it. Two contracts were pinned this
round: incremental graph delivery (section 1) and the new-workspace
pre-flight state machine (section 2). Directional append-only inboxes
(`event-<from>-<to>.md`) carried announcements and flags between every pair.

## What shipped, tried, and undone

**Shipped (round 1, v0.18.0)**

- Gateway moved into `gateway/` as its own Cargo workspace, re-homed onto
  the in-repo tunnel crates by path.
- `drive-proxy` renamed to `workspace-proxy`; the `drive` to `workspace`
  rename applied suite-wide: cookie `workspace_gate`, host
  `workspace.chan.app`, routes `/api/workspaces/*`, env `WORKSPACE_*`,
  tables `workspaces` / `workspace_grants`, migrations edited in place
  (pre-release, no migration history to preserve).
- Single-source domain config: `gateway_common::domain::Domains` derives
  every host from `CHAN_DOMAIN` plus `PUBLIC_SCHEME`.
- New `gateway-ci.yml` gate and four gateway `.deb` packages wired into
  `release.yml`.
- Packaging and docs reconciled to the current env contract; a
  pre-existing `configure.sh` `install /dev/stdin` bug fixed along the way.
- Tunnel hostname renamed from `drive.chan.app` to `workspace.chan.app`
  across the chan client tunnel default, `chan-tunnel-*` crates, chan-server,
  desktop, the manual, and marketing copy.

**Shipped (rounds 2 and 3)**

- Cursor-paged `/api/fs-graph`: opt-in via `limit`, resumed via opaque
  `cursor`, bounded DFS batches of at most 256 nodes / 64 KiB, idempotent
  cursors, scope-bound 400 rejection. Whole-scope path (no params) stays
  byte-identical; existing depth-cap probe and CLI did not move.
- Depth-slider "next degree, not a re-walk" primitive.
- Pre-flight backend: `GET /api/preflight` poll plus
  `POST /api/preflight/decision`, derived from live indexer state, with
  `locked = phase != ready`.
- False unsaved-changes banner fix: per-page-load `SESSION_ID` plus an
  mtime-stale guard so own-session edits never raise the banner while a
  genuine crashed session still recovers.
- Cmd+, panes-flip desync fix: `paneChordBlocked()` guard so the flip
  never fires while an overlay or modal owns the keyboard.
- `/dl` circular 404 rewrite: regenerate from the latest GitHub Release
  instead of self-fetching the live site.
- Graph directory expand/collapse in place (double-click), with the
  expanded set persisting across a window reload. Old "graph from here"
  double-click rescope dropped; rescope stays in the inspector.
- Cursor-paged graph consumption in the frontend (chase `cursor` until
  `done`, yielding to a frame between batches).
- Locked `PreflightOverlay.svelte` (no close button, ESC ignored).
- Pristine comment sweep: history-narration comments cut from 174 files
  to about 5; only WHY-snapshot comments and legitimate test-data
  `@@handle` mentions kept.
- ASCII typography normalized across all four frontend trees (em/en dashes,
  ellipsis, middle dot, and corresponding HTML entities normalized to `-`
  and `...`; about 100 files in `web/`).
- `web/src/editor/design.md` rewritten as a present-state snapshot.
- Vestigial `team-work-N` draft convention dropped.
- De-flake of three indexer/PTY tests (capability-gated skips, not bigger
  timeouts).

**Tried then corrected**

- The "blank remote window" investigation spent time proving the SPA loads
  fine over loopback before the real cause was found: a one-line flag-name
  typo (`--tunnel-workspace` should be `--tunnel-workspace-name`) in the
  desktop snippet. @@Alex landed the fix directly on main as `f2eb32a9`
  and swept the gateway-copy and gateway-docs occurrences; the lane's own
  snippet commit went redundant on rebase.
- @@LaneB's first B2 cleanup attempt over whole directories over-ran badly
  (one subagent ran roughly 5 hours before a socket timeout). The second
  pass used tight per-file single-pass scopes with an explicit anti-loop
  instruction.

**Deliberately deferred**

- A3 desktop relocation (`default_workspace.rs` / `serve.rs` / `main.rs`):
  best done once the locked OverlayShell exists and the flow can be verified
  in WKWebView. Coupled to the A5d empirical walk, which agents cannot
  automate (Chrome/Blink cannot reproduce WKWebView render).
- Model-prompt policy: @@LaneA made a unilateral call (prompt only when
  semantic search is enabled but the model is missing, not on every fresh
  workspace) and flagged it for @@Alex to ratify; left as a carryover.
- Identity charset mismatch: the SPA advertises `._-` for a workspace name
  while `--tunnel-workspace-name` accepts only `[a-z0-9-]`. Left open for
  @@Alex to resolve.
- Depth-slider paging optimization (frontier-only single-dir expands rather
  than a paged 0..N refetch): flagged for follow-up after the
  responsiveness baseline settles.
- WKWebView desktop walk: a human-only verify (`make macos-chan-dmg`, then
  Cmd+Shift+N, Cmd+I, Cmd+P, self-upgrade 0.17.0 to 0.18.0 from `/dl`).

## Retrospective

**Highlights**

The gateway nested-workspace structure held the line: zero gateway crates
appear in `cargo metadata` at the root, so the single-binary, no-runtime-
deps core stayed clean while the gateway gained its own CI gate, release
`.deb` flow, and single-source domain config. The prod packaging path was
validated end to end in a systemd sdme container.

The contract-first coordination model proved its value. @@LaneA's first
cut of the paged graph delivery matched the eventually-pinned contract
exactly, so @@LaneB had zero rework on the graph seam. Against the live
endpoint on `/tmp/linux`, the paged `/api/fs-graph` walked the full cursor
chain (373 batches, 47,734 nodes at depth 4) and terminated correctly, and
the pre-flight reached `ready` within roughly 3 seconds.

@@LaneA caught a third indexer-flake offender
(`writes_to_drafts_subtree...`) not listed in the addendum, because it ran
the full parallel `cargo test --all-targets` gate (the load profile that
flakes) rather than a scoped subset. This is worth repeating: de-flake work
should always validate against the full parallel run.

**Lowlights**

@@LaneA over-committed to an SSH-forward diagnosis for the blank-remote-
window report. The actual cause was a one-line flag-name typo that a 30-
second grep of the desktop's `chan serve` command construction would have
surfaced immediately. The pattern of ruling out transport before checking
the exact flags emitted is backwards for desktop/CLI divergence bugs.

The B2 whole-directory subagent scope caused a 5-hour over-run. Tight per-
file scopes with an explicit anti-loop guard are the right shape for
multi-agent cleanup passes from the start, not as a recovery measure.

Three empirical checks could not be automated and were deferred: WKWebView
desktop walk, graph-fills-in-gradually browser visual, and the locked-
overlay render (the index completes fast and the embedding model is
bundled, so the lock is rarely observable). Planning for the WKWebView
dependency earlier would have let @@Alex schedule the manual verify without
it becoming a carryover.

**Lessons**

- For "desktop launches X and X is broken": diff the exact command and
  flags the desktop emits against the CLI's clap definitions BEFORE
  theorizing about transport. Flag-name typos are cheaper to find than
  SSH-forward diagnostics.
- Keep subagent scopes tight (per-file, single-pass, anti-loop guard) from
  the start of a cleanup wave, not as a recovery measure after an over-run.
- Watch for state keyed on an ephemeral id: the draft buffer with no
  session marker and the graph-tab persistence key bound to a regenerating
  tab id are two instances of the same class of bug that recurred this round.
- Contract-first splits (propose, confirm, pin before building) pay off at
  integration in concurrent backend/frontend rounds; the shape produced zero
  seam rework here and is worth keeping as the default for that pairing.
- Product calls that are not ratified up front (model-prompt policy, desktop
  pre-flight relocation) tend to land as flagged carryovers. Stating the
  intended default at roadmap time lets the lane finish cleanly.
- Spec-first reorgs (sign off the shape before writing any report) keep
  large doc migrations coherent; @@LaneC's C2 approach is the right model
  for journals work.

## Notes

Terminology drift active in this phase:
- `drive` to `workspace` rename: applied in round 1 end to end. `drive-proxy`
  became `workspace-proxy`; the cookie, host, routes, env vars, and database
  tables all follow. The 75 `drive.chan.app` mentions under
  `docs/journals/phase-14/` are historical and were left as written.
- `chan-drive` is the old name for `chan-workspace` (renamed in phase 12).
  Phase-14 source uniformly uses `chan-workspace`.
- "Rich Prompt" is the old name for "Team Work." Phase-14 source uses
  "Team Work."

The raw working material (per-author journals, task/request/roadmap files,
coordination contracts, event channels) is preserved in git history under
`docs/journals/phase-14/`; that tree was removed from the working tree in
the phase-15 docs cleanup.
