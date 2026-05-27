# Phase 12 retrospective

Closed 2026-05-27 by @@Lead (orchestrator / @@Architect seat). Baseline at open
`fe6e126` (phase-11 close); main at close `4eb87901`. 56 commits over the round.

Roster: @@LaneA (graph + File Browser), @@LaneB (drive -> workspace rename),
@@LaneC (@@Alex ad-hoc bugs/cosmetics), @@LaneD (CI + release, added mid-round),
@@LaneE (cross-platform shortcuts, added mid-round). @@Alex human owner; @@Lead
orchestrated dispatch + serialized every merge + re-gated.

## Headline

The `drive -> workspace` rename is **100% complete** across code + docs + manual +
public marketing - a clean break (no migration), with only the `drive.chan.app`
hostname and the cloud-storage product names (Google/iCloud/OneDrive) preserved by
design. Alongside it: the graph overlay/scope-concept wipe, graph loading-state +
GI-10, the File Browser tab/dock independence, four+ ad-hoc bug fixes, the
cross-platform shortcut policy, and the CI/release machinery for the upcoming
v0.16.0 cut.

## Delivered (by lane)

- **@@LaneA**: overlay/scope-concept wipe W1-W7 (A1 scope-from-tab, A4 dock
  browserState, A3 hash retire, A5 dead-state + dead-kind deletion); GI-10 (drive
  node at bottom, spine up); graph loading-state slices 1+2 (indexing cue +
  ghost-node pullback); FB per-instance tree-expansion (addendum-2); addendum-3
  graph right-click-anywhere + Export-to-PDF->Inspector.
- **@@LaneB**: the entire rename - chunk 0 (free "workspace" / RichPromptSession),
  chunk 1 (crate + type rename), chunk-1 Tauri/perm fixups, chunk-1b consts, the
  /api/graph scope-variant hotfix, chunk 2 (atomic wire+frontend+CLI+desktop flip),
  chunk 2d (internal + full tunnel rename), chunk 3 (docs), + web-marketing.
- **@@LaneC**: addendum-1 (self-write race Bug3, Drafts-MCP Bug2, Bug4 closed,
  Bug1 terminal blur) + self-write follow-up; addendum-2 (editor no-reload +
  cursor/focus, terminal sleep/wake recovery, drag-drop image row-move); addendum-3
  terminal dot pulse + cmd+shift+i broadcast toggle.
- **@@LaneD**: corrected the CI framing (basic CI green; the v0.15.5 *release*
  workflow had failed); RPM staging-path fix; vitest re-added to the gate + the
  /api/drive flake fix; v0.16.0 release + clean-slate prep (cut in progress).
- **@@LaneE**: audited the shortcut policy (found it ~80% already implemented),
  shipped the gaps (web alt-nav, cmd+s search, splits, close-cascade window-tail,
  Linux ctrl+d-only close, infographics chord); found 2 chunk-1 bugs.

## Pending / carryover (next round)

- **v0.16.0 cut** (in progress, @@LaneD): version bump 0.15.5->0.16.0, CHANGELOG,
  release, delete old releases + repoint the upgrade channel, first origin push
  (fires the unrun `make ci-linux`/`ci-macos` + release.yml over the whole round).
- **@@Alex post-release verify list** (merged, empirically unverified by design):
  terminal Bug1 (focus-switch) + item2 (sleep/wake), the dot pulse, the cmd+shift+i
  broadcast, the find-triad.
- **Tunnel cloud redeploy** (@@Alex ops, cross-service): the wire changed
  (`drive`->`workspace`, `{workspace}` slug), so the `drive.chan.app` cloud tunnel
  server needs redeploy from the renamed `chan-tunnel-server` before tunnel mode
  works on 0.16.0. Gate-invisible. `drive.chan.app` hostname cleanup also deferred.
- **@@LaneC facet B** (chmod -w -> LOCKED tab) - parked (needs a backend writable
  signal); **graph** loading-state per-parent-dir pulse, dock reload-snapshot key
  timing, dead group-edge/SCOPE_HUB machinery (A5 follow-up), optional GI-11 tests.
- **Backlog carryover unchanged**: Linux desktop launch, macOS handoff window-paint
  check, GPU embedding fix, Linux inotify watch-count, manual/site streaming-copy.

## Highlights

- The rename landed clean and complete despite touching ~everything, with the
  hardest constraints respected (atomic wire+frontend flip, cloud/tunnel/hostname
  preservation, splittable sequencing around @@LaneA/@@LaneC quiescence).
- Every gate-blind defect was caught **before** any release - by the architect
  re-gate/re-smoke/leftover-audit, not the lane gate.
- Lanes self-coordinated well: @@LaneB's peer-to-peer quiescence checks + the c-e/
  b-e cross-lane declarations avoided collisions; @@LaneA/@@LaneC fresh-binary
  smokes caught real defects.
- The orchestration model (lanes cut tasks / report ready-to-merge; @@Lead
  serializes + re-gates; @@Alex rules scope) held up across 5 lanes + 56 commits.

## Lowlights

- **The gate-blind wire-rename class bit ~5 times**: stale Tauri perm names, the
  CLI->desktop handoff variant, the /api/graph scope variant, the
  `default_drive_root` frontend<->backend config mismatch + the chan-desktop compile
  break. `cargo` + `vitest` + `svelte-check` are ALL blind to serde/IPC/route/
  desktop-compile string drift - only runtime/in-browser/cross-service exercise it.
- A couple "complete" signals arrived ahead of the committed state (chunk 2), costing
  reject + re-do round-trips.
- A few late scope additions stretched the round's tail.

## Honest feedback

**To the agents**
- @@LaneA: model lane. Grounded every change against the real worktree, de-risked
  the destructive wipe into merge-safe slices, and its smokes caught cross-lane
  defects that weren't its own. Keep that fresh-binary discipline.
- @@LaneB: carried the round's hardest + largest work, with the standout insights
  (3 meanings of "drive", the wire-decoupling that made the codemod splittable) and
  honest post-mortems. But it shipped the gate-blind misses + a premature "complete".
  Next time: bake a wire-field + desktop-compile + cross-service audit + a runtime
  smoke into the per-chunk gate BEFORE reporting ready - "complete" must mean
  committed + smoked, not "bulk done".
- @@LaneC: honest and clean - solid static root-causes, explicit about what it
  couldn't verify, mapped its own merge guidance. Minor: stacking held commits (Bug1)
  forced cherry-picks; order held items last in the stack so dependents merge freely.
- @@LaneD: strong investigation (corrected the architect's CI framing; deterministic
  flake repro). Minor: committed the vitest gate before the flake fix - confirm a new
  gate is green on the target tree before wiring it.
- @@LaneE: efficient + audit-first (didn't over-build the ~80%-done policy), good
  keymap discipline, found 2 chunk-1 bugs, surfaced the WASD collision. Clean lane.

**To @@Alex**
- Decisive on the load-bearing calls (Option C, the clean break, ship-unverified
  pre-release, delete-old-releases) and good product instincts (caught the WASD
  collision and the tunnel scope the architect had mis-modeled).
- Constructive: the late-additions cadence (addenda 1-3, the broadcast shortcut, the
  Cmd+R nit, the tunnel + web-marketing follow-ups arriving across the tail) stretched
  the close and caused two round-trips - the Cmd+R "nit" was a false alarm from
  reviewing released v0.15.5 (which predated the feature), and one "B is complete"
  relay ran ahead of B's committed state. None hurt quality (the audit caught
  everything), but staging the full addenda set before round open (or batching
  mid-round) + sanity-checking nits against a fresh build would shorten the tail.

**To @@Lead (architect, self)**
- The win: the re-gate / re-smoke / leftover-audit + the atomic-chunk-2 rule caught
  every gate-blind defect before a release. That discipline is the round's safety net
  and must stay mandatory for any rename.
- The miss: I mis-modeled the tunnel as "preserve" from @@LaneB's conservative chunk-1
  framing and was heading toward escalating a correct change as a defect; @@Alex's
  "I explicitly wanted it removed" corrected me. I caught it at the audit (no false
  escalation reached the bus), but I should have re-derived scope from the ratified
  decision, not a lane's interpretation. Also: I hit repeated append-edit friction
  from line-wrap mismatches (fixed by reading exact tails first), and I caught the
  gate-blind class ad-hoc each time instead of codifying a wire-audit checklist after
  the first instance. Carry forward: a standing rename-audit checklist (wire strings,
  serde tags, routes, IPC/perms, desktop compile, cross-service wire, runtime smoke).

## Process note for the next rename / round

Pin wire strings (`#[serde(rename)]`) during the internal rename; flip producer +
consumer atomically in one verified pass; and SMOKE every renamed wire surface
(in-browser + desktop + any separately-deployed counterpart). The compile/unit gate
does not protect this class. See [[feedback-gate-blind-wire-renames]].
