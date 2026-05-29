# Phase 12 - drive-to-workspace rename and graph/FB carryover

Status: closed (the v0.16.0 cut was handed to a lane as a post-close
carryover)
Span: 2026-05-27, one day (estimate; see Duration)

## Initial asks

Source: the carryover backlog
[raw/phase-12-backlog.md](raw/phase-12-backlog.md), assembled from phases
10 and 11, plus three ad-hoc request bundles. Three primary tracks:

- Graph and File Browser (lane A): the overlay / scope-concept wipe ("the
  big one") that makes scope equal to a filesystem directory and deletes
  the load-bearing overlay state, plus GI-10 (drive node pinned to the
  bottom, the spine growing upward), a graph loading-state, and File
  Browser per-instance expansion.
- Drive-to-workspace rename (lane B, scope first): "Rename the
  `chan-drive` crate to `chan-workspace` and all 'drive' terminology to
  'workspace' across code, comments, and documentation," with lane B as a
  scoping architect producing a scope doc and surfacing the big decisions
  (the "team workspace" collision, user-facing CLI and config, the
  `drive.chan.app` tunnel domain, uniffi bindings, sequencing) before any
  codemod.
- Frontend cosmetics and keyboard shortcuts (lane C, @@Alex-driven), fed
  by three ad-hoc bundles covering Rich Prompt and Drafts bugs, editor and
  terminal issues, and the full cross-platform shortcut policy.

A release/build carryover (the v0.16.0 cut and the first origin push) had
no lane at open and went to lane D mid-round.

## Team, profiles, and coordination

Lanes are positional phase-12 handles, not the named roster; only
@@Architect resolves to a card via
[../../agents/README.md](../../agents/README.md). The @@Architect seat was
operated by a handle signed "@@Architect (@@Lead)".

```
handle       role this phase                           card
-----------  ---------------------------------------   ----------------
@@Architect  orchestrate, serialize merges, re-gate,   architect.md
             own the codemod-window decision
@@LaneA      graph + File Browser carryover            (no card)
@@LaneB      scoping architect for the rename          (no card)
@@LaneC      @@Alex ad-hoc frontend/cosmetics/shortcuts (no card)
@@LaneD      CI + release (added mid-round)            (no card)
@@LaneE      cross-platform shortcuts (added mid-round) (no card)
@@Alex       human owner; drives lane C; rules scope   (human owner)
```

Coordination scheme: per-lane git worktrees hold source code only; all
coordination docs (plans, journals, channels) are edited in the main
checkout by absolute path, so the bus stays one shared, conflict-free,
append-only log. Channels are directional `event-<from>-<to>.md` files
under `raw/coordination/`, created on first use, including cross-lane
seams. Lanes never merge to main; they report "ready to merge:
branch@sha" and the architect serializes every merge, re-gates the
combined tree, and owns the codemod-window decision. The dominant
coordination problem was that all frontend lanes touch `web/src` while
lane B's codemod touches nearly everything, so lane B scoped first and the
codemod landed in a sequenced freeze window gated on the other lanes'
`web/src` quiescence. The whole doc tree stayed untracked as the live bus
and was committed once at close.

## Duration

Estimate: a single day, 2026-05-27. Basis: the only two git commits (open
and close) are both 2026-05-27, and every dated header reads the same.
The live bus stayed untracked all round, so git gives only the bookends.

## Highlights and lowlights

Highlights:
- The drive-to-workspace rename landed clean and complete across code,
  docs, manual, and marketing as a clean break (no migration), respecting
  the hard constraints (an atomic wire-plus-frontend flip, and preserving
  the cloud, tunnel, and hostname surfaces per @@Alex).
- Every gate-blind defect was caught before any release, by the architect
  re-gate, re-smoke, and leftover audit rather than the lane gate.
- Lanes self-coordinated via peer-to-peer quiescence checks; the model
  held across five lanes and 56 commits.
- Lane E found the shortcut policy was about 80 percent already
  implemented, so it audited first and shipped only the gaps.

Lowlights:
- The gate-blind wire-rename class bit about five times (stale Tauri
  permission names, a handoff serde variant, the `/api/graph` scope
  variant, a config-key mismatch, a desktop compile break). The cargo,
  vitest, and svelte-check gates are all blind to serde, IPC, route, and
  desktop-compile string drift.
- A couple of "complete" signals arrived ahead of the committed state,
  costing reject-and-redo round-trips.
- Several late scope additions stretched the round's tail.

## Constructive feedback

- Lane B carried the hardest work with standout insights (the three
  meanings of "drive"; the wire-decoupling that made the codemod
  splittable) but shipped the gate-blind misses and a premature
  "complete"; bake a wire-field, desktop-compile, cross-service, and
  runtime-smoke audit into the per-chunk gate before reporting ready, and
  let "complete" mean committed and smoked.
- @@Alex was decisive on the load-bearing calls and caught real product
  issues (a WASD collision, the tunnel scope the architect had
  mis-modeled); staging the full addenda set before open, and
  sanity-checking nits against a fresh build, would shorten the tail.
- Architect self-note: the re-gate / re-smoke / leftover-audit caught
  every gate-blind defect; codify a standing rename-audit checklist (wire
  strings, serde tags, routes, IPC and permissions, desktop compile,
  cross-service wire, runtime smoke) rather than catching the class
  ad-hoc each time.

## What shipped, tried, and undone

Shipped: the overlay/scope-concept wipe across merge slices; GI-10 and
the graph loading-state; File Browser per-instance expansion; the entire
rename in sequenced chunks (a free "workspace" reservation, the crate and
type rename with wire literals held, then an atomic wire-plus-frontend
flip, the full tunnel rename, a docs sweep, and web-marketing); the lane C
ad-hoc bug fixes; lane D's CI framing correction and release prep; and
lane E's shortcut-gap fixes. End state: zero code drive-residue, with only
the `drive.chan.app` hostname and the third-party cloud product names
preserved by design.

Tried, undone, or deferred:
- Lane A's authorized subagents were not used; the wipe slices were too
  tightly coupled on shared files, so lane A ran in-session sub-slices.
- The first chunk-2 squash reported gate-and-smoke green but was rejected
  on a re-audit (a frontend config mismatch and a desktop compile break);
  re-squashed with both fixed, then merged.
- The `/api/graph` scope variant was renamed before the client caught up,
  breaking the whole-workspace graph; hotfixed by pinning the variant
  with a serde rename, with the real flip riding the atomic chunk.
- The architect mis-modeled the tunnel as "preserve"; @@Alex corrected it
  ("I explicitly wanted it removed") and the full tunnel rename shipped.
- Two terminal fixes were merged unverified by design (pre-release, a
  WKWebView-only render glitch @@Alex skipped verifying).

Carryover: the v0.16.0 cut (version bump, changelog, the first origin
push that fires the unrun CI and release workflows, and repointing the
upgrade channel), an @@Alex post-release verify list, a tunnel cloud
redeploy (the wire changed), and the remaining graph and Linux-desktop
backlog.

## Raw material

- The opening backlog: [raw/phase-12-backlog.md](raw/phase-12-backlog.md)
- The opening contract (roster, codemod sequencing, merge protocol):
  [raw/bootstrap.md](raw/bootstrap.md)
- The close-out retrospective:
  [raw/retrospective.md](raw/retrospective.md)
- The rename scope doc:
  [raw/workspace-rename-spec.md](raw/workspace-rename-spec.md)
- The orchestration log, per-lane journals, the ad-hoc request bundles,
  and the `raw/coordination/` channels live alongside them in
  [raw/](raw/).

The lane C ad-hoc request bundles originally embedded four screenshots
(the Rich Prompt bootstrap and three terminal-font-after-sleep shots);
per the journals-wide image removal each is now a short text note in
[raw/lane-c/addendum-1/request.md](raw/lane-c/addendum-1/request.md) and
[raw/lane-c/addendum-2/request.md](raw/lane-c/addendum-2/request.md).
