# event-architect-alex.md

From: @@Architect
To: @@Alex
Date: 2026-05-20

## 2026-05-20 — poke (Round-1 test-server URL hand-off)

@@WebtestA cleared the lane-A walkthrough and parked a live
test server for you to click around. URL with bearer token:

```
http://127.0.0.1:8787/?t=Am6NjQ7pSNeH2ibHCyaftLu8m8MuNntm
```

Drive: `/tmp/chan-test-phase8-wa/` (chan repo seeded as the
drive). Watcher attached to `watcher-events/` with sample
survey events + a reply file in place if you want to inspect
the bubble overlay path.

Coverage already verified by @@WebtestA: bugs 1, 2, 4, 5, 7,
9, 10, 18, 19, 21 hold on HEAD. Active repros that still need
fixes: bug 8 (graph false-missing, 8/1102 nodes), bug 11
(image-insert pushes cursor + view doesn't roll). Partials:
bug 6 (Cmd+T blocked on web; native viable), bug 20 (re-open
with bubble present focuses prompt input). Could-not-
reproduce: bug 14 (watcher first-try hang).

Full sweep summary at the tail of
[../webtest-a/webtest-a-1.md](../webtest-a/webtest-a-1.md)
under "Round-1 bug-sweep summary (curated)". @@WebtestB has a
parallel lane-B server up at `127.0.0.1:8820` against
`/tmp/chan-test-phase8-wb` if you want to A/B against a
different drive seed.

Webtests will keep their servers up through Round-1 close. No
action gated on this; surfacing per the test-server URL hand-
off step in `process.md`.

## 2026-05-20 — poke (WebtestB blocker — needs your call)

@@WebtestB cleared every bug in their coverage cluster
*except* `fullstack-b-1` (chan-desktop window-config LRU).
They source-verified the change (17 chan-desktop tests pass,
source matches the plan) but the runtime walkthrough needs
`Chan.app` launched on real macOS — and that sits outside
their standing webtest permission (terminal exec scoped to
`chan serve` + Chrome MCP; not Tauri bundle launch).

Two options. I recommend (a):

1. **(a) Extend @@WebtestB's standing permission for the rest
   of Round 1** to include `npm run tauri dev` / `Chan.app`
   launch against a throwaway drive. Minimal disruption — both
   webtest lanes are already standing through Round 1; this
   adds one runtime command + window-launch to lane B's
   existing scope. Lane B picks up the runtime walkthrough
   on `fullstack-b-1` directly.
2. **(b) Route the runtime walkthrough to a code lane with
   existing Tauri launch capability** (@@FullStackB built the
   feature, so they could self-verify). This breaks the
   "webtests own audit-trail walkthroughs" lane boundary — a
   code lane doesn't normally produce verdict appends.

If (a), the approval can be transcribed straight into
[event-webtest-b-alex.md](event-webtest-b-alex.md) with the
existing standing-approval format ("approved (transcribed by
@@Architect)") — scope add-on: `npm run tauri dev` /
`Chan.app` runtime launch against a throwaway drive, through
Round-1 close.

## 2026-05-20 — agent-recycle (@@FullStackA)

@@Alex called Round 1 closed. Recycle @@FullStackA.

Handover anchor: most-recent entry in
[../fullstack-a/journal.md](../fullstack-a/journal.md);
queue empty after `-27` clearance. Last commit in HEAD
is up through `-26` (Hybrid editor toolbar parity);
`-27` (Hybrid hamburger polish) sits uncommitted in
working tree (Pane.svelte + perHybridTheme.test.ts).
Fresh session should pick up the `git status` working
tree + commit -27 first, then standby until Round-2
fan-out.

### Amendment 2026-05-20 — hold recycle until -27 commits

@@Alex poked the current @@FullStackA session; they
picked up the `-27` commit themselves. Hold the fresh-
session spawn until the current session lands the
commit, otherwise two sessions would both try to
commit the same files (Pane.svelte + perHybridTheme
.test.ts).

Recycle proceeds AFTER `-27` is in HEAD. The current
session's bootstrap context is already correct +
queue-empty; they commit + standby. When @@Alex is
ready (or when a context-window pressure surfaces),
spawn the fresh session against an already-clean
working tree.

## 2026-05-20 — agent-recycle (@@FullStackB)

@@Alex called Round 1 closed. Recycle @@FullStackB.

Handover anchor: most-recent entry in
[../fullstack-b/journal.md](../fullstack-b/journal.md);
queue empty after `-12` clearance. All 12 tasks
committed in HEAD up through `-12` (terminal Source
Code Pro + cursor parity). Standby for Round-2
signing pipeline + bundled-chan-binary work.

## 2026-05-20 — agent-recycle (@@Systacean)

@@Alex called Round 1 closed. Recycle @@Systacean.

Handover anchor: most-recent entry in
[../systacean/journal.md](../systacean/journal.md);
queue empty after `-9` clearance. All Round-1 tasks
committed (`-1`/`-2`/`-4`/`-5`/`-6`/`-7`/`-8`/`-9` +
Makefile fill-in). `-3` cancelled per the no-Round-1-
binary restructure. Standby for Round-2 signing-key
rotation + chan-drive pre-flight + `chan reports
enable/disable` CLI.

## 2026-05-20 — agent-recycle (@@CI)

@@Alex called Round 1 closed. Recycle @@CI.

Handover anchor: most-recent entry in
[../ci/journal.md](../ci/journal.md); queue empty after
`ci-6` clearance. Six commits across five task IDs
(`ci-1` / `ci-2` ×2 / `ci-3` / `ci-4` / `ci-5` / `ci-6`).
Standby for Round-2 signing workflow (provisional
`ci-7`) + DMG dry-run with real keys (provisional
`ci-8`). Cert provisioning per the `ci-3` brief is the
prerequisite @@Alex is dogfooding in parallel.

## 2026-05-20 — agent-recycle (@@WebtestA)

@@Alex called Round 1 closed. Recycle @@WebtestA.

Handover anchor: most-recent entry in
[../webtest-a/journal.md](../webtest-a/journal.md);
last activity is the Round-1 sweep + per-fix verdicts
at the tail of `webtest-a-1.md`. Wave-2/-3 verification
cadence partial — @@Alex's call to roll the remaining
verifications into Round 2's BOOT + pre-flight work
since those changes rebuild the binary. Standby for
Round-2 walkthroughs against the new signed-DMG
artifacts.

## 2026-05-20 — agent-recycle (@@WebtestB)

@@Alex called Round 1 closed. Recycle @@WebtestB.

Handover anchor: most-recent entry in
[../webtest-b/journal.md](../webtest-b/journal.md);
last activity is the wave-1 verifications + proactive
walks on `systacean-7` (caught `systacean-8` + `-9`).
Tauri-launch permission extension granted +
transcribed in
[event-webtest-b-alex.md](event-webtest-b-alex.md);
fresh session inherits the extension. Standby for
Round-2 walkthroughs against the new signed-DMG +
chan-desktop work.

## 2026-05-20 — poke (rich-prompt session evolution — planning artifact)

You dropped a 5-item ask for the rich prompt + spawn-agent
surface to become chan's multi-agent session conductor:
history backlog with `.md` transcript + cwd preflight +
shell-vs-agent submit-mode toggle + spawn-agent eyeball
preflight with identity broadcast + multi-row spawn form
with "launch in the back".

Drafted [`../architect/rich-prompt-session-evolution.md`](../architect/rich-prompt-session-evolution.md)
as a planning artifact (NOT dispatched). Grounded the
decomposition against the current code via an Explore
agent — surfaced one important reframing on item C:
the rich prompt today sends the buffer verbatim with no
explicit trailing newline append (the "Enter to the
shell" effect comes from whatever newline the editor
leaves at end-of-buffer). The agent-submit-chord
encoding (xterm modifier-other-keys vs literal CR) is
flagged as a per-agent investigation at task-cut time.

Survey at the tail of the artifact (4 topics × 2-3
options each) on history storage scope, cwd preflight
timing, submit-mode toggle surface, and multi-agent
spawn surface. Recommendations included; mostly biased
toward the "rich prompt becomes the single session
conductor" framing.

Sequencing recommendation: items A/B/C ride in
Round-2 wave 2 (after the chord migration lands the
Cmd+P binding); items D/E pair as Round-2 wave 3 (D
consumes E's output). No tasks cut yet; awaiting your
survey answers.

Also committed the round-1 teardown housekeeping
(architect-side checklist pokes + four agent
teardown-complete confirmations) as `ecfceec` so the
working tree is clean. Push still parked for end of
Round 2.

## 2026-05-20 — poke (rich-prompt evolution decisions locked)

Clean sweep on the 4 survey topics — you picked the
architect-recommended option on each: history → on-disk
`.md`; cwd preflight → always-visible header; mode
toggle → per-prompt toolbar icon; team surface →
inside the rich prompt.

Decisions table + interlocking implementation notes
appended to [`../architect/rich-prompt-session-evolution.md`](../architect/rich-prompt-session-evolution.md);
canonical decision record mirrored to the decisions log
in [`../architect/journal.md`](../architect/journal.md).
Five Round-2 tasks shaped (3 wave-2, 2 wave-3 paired)
ready for fan-out alongside the rest of Round-2 once
sequencing is confirmed.

## 2026-05-20 — poke (rich-prompt mini-wave dispatched + patch release re-activated)

Live broadcast smoke test with you surfaced four bugs in
the rich-prompt bubble overlay + survey-reply path
(flicker, non-survey replies don't dismiss, no explicit
close affordance, survey-reply echoes `poke<Enter>` when
agents need `poke<Cmd+Enter>`) plus the prior page-width
and collapse-dead-space findings. You called the play:
cut a patch release **with the rich-prompt fixes in**
before the broader Round 2 resumes.

Five tasks fanned out:

| Task             | Owner        | Scope                                                              |
|------------------|--------------|--------------------------------------------------------------------|
| `fullstack-a-28` | @@FullStackA | BubbleOverlay regression: filter generalization + explicit dismiss + refresh diff-merge |
| `fullstack-a-29` | @@FullStackA | Collapse chevron dead-space: terminal-host margin recompute on the -a-24 collapse transition |
| `fullstack-a-30` | @@FullStackA | Per-prompt page-width + slider in textbox right-click menu        |
| `fullstack-b-13` | @@FullStackB | Shell/agent submit-mode toggle + survey-reply echo consumer (PTY chord encoding research front-loaded) |
| `systacean-10`   | @@Systacean  | Event-watcher fsnotify path: silent-skip non-matching filenames + module-doc + process.md convention note |

Dispatch pokes fired to each agent's inbound channel +
to @@WebtestA / @@WebtestB for verification queues. @@CI
on standby (no signing-pipeline work this wave).

Patch-release re-activation: the originally-cancelled
v0.11.1 tag effectively comes back as the patch-release
target (or v0.11.x depending on what @@Systacean cuts).
Signed-DMG north star with real keys stays parked
behind it.

**Commit-grouping plan TBD**: I'll publish
`commit-plan-v0.11.x.md` (final version TBD) once the
mini-wave produces commit-ready batches. The Round-1
work already in HEAD + this mini-wave + @@Systacean's
version-bump + tag form the patch-release set.

Permission asks that stayed parked at recycle (the
`fullstack-b-7` runtime click + `fullstack-b-1`
empirical LRU walk) carry forward to this cycle — they
still need your interactive participation; not blocking
the patch tag if you decide to ship without them.

## 2026-05-20 — agent-recycle (@@FullStackA, mid-wave context-fill)

@@Alex picked path (a) — recycle @@FullStackA, fresh
session lands the remaining mini-wave queue
(`-32/-33/-34/-35`), then @@Systacean cuts the patch tag.

This is a **mid-wave recycle**, not a round-boundary
recycle. The current @@FullStackA session correctly hit
the context-fill flag I offered when the queue widened
to 8 tasks; their last poke at the tail of
[../alex/event-fullstack-a-architect.md](../alex/event-fullstack-a-architect.md)
confirms stand-down + clean working tree.

### Handover anchors for the fresh @@FullStackA

The fresh session bootstraps per
[`../../../agents/bootstrap.md`](../../../agents/bootstrap.md)
and inherits a populated queue:

* Mini-wave commits landed in HEAD by the prior session:
  `-28` (`1a83050`), `-29` (`3d708a2`), `-30` (`20ece30`),
  `-31` (`18811e0`).
* Remaining queue (dispatched, task files in place):
  `-32` chord migration + context-aware spawn semantics,
  `-33` graph-from-here default + ancestor breadcrumbs
  (hard-pair: land before `-32`), `-34` Wysiwyg paste
  escape fix, `-35` file rename UX.
* Latest dispatch poke at the tail of
  [`event-architect-fullstack-a.md`](event-architect-fullstack-a.md)
  carries the recommended order (`-33 → -32 → -34 → -35`)
  + handover details (no uncommitted code in working tree;
  lane-A test server torn down at recycle; fresh
  `/tmp/chan-test-...` path for the new session).

### After the fresh session lands the queue

* I publish `commit-plan-v0.11.x.md` (final version TBD;
  `v0.11.1` re-activated per the patch framing) with the
  push order across the full mini-wave commit set.
* @@Systacean reads the plan + cuts the tag via the
  re-activated `systacean-3`.
* @@WebtestA / @@WebtestB run patch-release smoke tests
  against the cut binary.

### Status

Standing by for you to spawn the fresh @@FullStackA
session using the bootstrap prompt. Other agents
unaffected by this recycle:

* @@FullStackB — queue-empty for the mini-wave; standby.
* @@Systacean — queue-empty; awaiting `systacean-3`
  re-activation post-mini-wave.
* @@CI — standby.
* @@WebtestA / @@WebtestB — verification queues queued;
  @@WebtestB can start incremental verification on the
  rebuilt binary now if they have bandwidth (B's
  -b-13 server + SPA + -b-14 are landed + exercisable).

## 2026-05-20 — poke (Round-2 decisions locked; fan-out unblocked)

Clean sweep on the 4-topic survey covering the 5 open
Round-2 decisions. All architect-recommended options
approved. Decision table mirrored to the decisions log
in [`../architect/journal.md`](../architect/journal.md);
plan head at [`../architect/round-2-plan.md`](../architect/round-2-plan.md)
updated from "Decisions @@Alex needs to confirm" →
"Decisions (all locked 2026-05-20)" with each item
carrying its lock rationale.

| # | Decision                  | Locked                                              |
|---|---------------------------|-----------------------------------------------------|
| 1 | Sequencing                | 7+ci-7 → 6 → 1+4 → 2 → 3 → 5                        |
| 2 | Item-6 hosting            | GitHub Pages with custom domain                     |
| 3 | Item-7 bundled-chan layout| PATH-first w/ bundled fallback + version match     |
| 4 | Item-3 PIN hash           | SHA-256 + per-install salt                          |
| 5 | Manual home               | `docs/manual/`                                      |
| 6 | First-release version     | v0.12.0 (locked earlier 2026-05-20)                 |

### What's next on my end

When you spawn fresh Round-2 sessions for the six
working agents (bootstrap prompt at
[`../../../agents/bootstrap.md`](../../../agents/bootstrap.md)),
I cut Wave-1 task files for:

* @@CI — workflow YAML consuming the six secrets;
  DMG-on-tag dry-run with real keys.
* @@Systacean — chan-desktop signing-key rotation;
  tauri-plugin-updater cross-platform verification.
* @@FullStackB — bundled chan binary in chan-desktop
  resources; launch-time version probe (PATH-first per
  decision 3).

Plus the parallel rich-prompt session-evolution stack
(history backlog, cwd preflight, team conductor) per
[`rich-prompt-session-evolution.md`](../architect/rich-prompt-session-evolution.md)
slots into Wave-2 alongside items 1+4.

### What stays out-of-band on your end

* Apple Developer ID cert checklist from `ci-3` brief.
* Six signing secrets populated into GitHub Actions
  Secrets (NAMES directed by architect; VALUES
  populated by you per the secrets-boundary memory).

### Sequencing recommendation: fan out now in parallel
### with webtest walkthroughs

@@WebtestA + @@WebtestB are walking the v0.11.1 cut
binary against their respective verification queues.
Recommend NOT blocking Round-2 fan-out on those
verdicts — they're independent of Round-2 code work,
and any v0.11.1 regression they surface either folds
into v0.11.2 (separate cut) or into Round-2's same
surface (cheap to absorb). Spawn fresh sessions for
the six working agents at your convenience; I'm ready
to cut Wave-1 task files the moment they bootstrap.

## 2026-05-20 — poke (Round-2 Wave-1 fanned out)

All six agents spawned per your kickoff prompt
(identity-confirmation beat included). Wave-1
north-star track dispatched. Standby agents
acknowledged + read in.

### Wave-1 task files cut

| Task            | Owner        | Scope                                                              |
|-----------------|--------------|--------------------------------------------------------------------|
| [`ci-7`](../ci/ci-7.md)                          | @@CI         | Tag-triggered signed + notarized chan-desktop workflow YAML        |
| [`ci-8`](../ci/ci-8.md)                          | @@CI         | DMG-on-tag dry-run with real Apple Developer ID keys               |
| [`systacean-11`](../systacean/systacean-11.md)   | @@Systacean  | chan-desktop signing-key rotation (DEV → release identity)    |
| [`systacean-12`](../systacean/systacean-12.md)   | @@Systacean  | Verify `tauri-plugin-updater` on macOS + Linux + Windows           |
| [`fullstack-b-15`](../fullstack-b/fullstack-b-15.md) | @@FullStackB | Bundled chan binary inside chan-desktop app resources          |
| [`fullstack-b-16`](../fullstack-b/fullstack-b-16.md) | @@FullStackB | Launch-time PATH-first probe + binary selection (decision 3)   |

Each carries `Authorization: yes` framing for shared
infra (workflow YAML, `tauri.conf.json`,
`desktop/Makefile`, `desktop/CLAUDE.md`). Secret VALUES
stay behind the boundary per the secrets-boundary
memory.

### Critical-path sequence

```
@@Alex completes ci-3 cert checklist (out-of-band)
                  ↓
@@Alex populates 6 secrets in GH Actions Secrets (out-of-band)
                  ↓
systacean-11 (rotate tauri.conf.json)  ci-7 (workflow YAML)  fullstack-b-15 (bundle chan)
                  ↓ (parallel-able)                                                ↓
                  ci-8 (dry-run with real keys)              fullstack-b-16 (probe logic)
                  ↓
              First notarized DMG → second-Mac verification (@@WebtestB)
```

`systacean-12` (tauri-plugin-updater verify) runs
parallel to the critical path; needed before v0.12.0
ships but not on the same dependency chain.

### Standby lanes

* **@@FullStackA**: Wave-2 work queued (carousel +
  Infographics + manual UX + rich-prompt session
  evolution). No immediate task. Reading-in on the
  locked decisions + the session-evolution artifact.
* **@@WebtestA**: v0.11.1 lane-A walkthrough is the
  immediate queue (carry-over from the GO poke earlier
  in event-architect-webtest-a.md).
* **@@WebtestB**: v0.11.1 lane-B walkthrough is the
  immediate queue (including `-b-13` end-to-end with
  Claude Code in a chan terminal, which exercises the
  patch-release north star).

### What's owed to you

* **Apple Developer ID cert checklist** (ci-3 brief):
  6 steps. Outputs the six secrets that `ci-7`
  consumes.
* **GitHub Actions Secrets population**: NAMES are
  authorized in workflow YAML by architect; you
  populate VALUES manually.
* **Optional**: green-light a test pre-release tag
  name when `ci-8` is ready to fire (recommend
  `chan-v0.11.99-dryrun.1` or similar to avoid
  colliding with the eventual v0.12.0 cut).

### My state

Standing by for inbound pokes from the six agents +
any new bugs / scope shifts you flag. Will route
commit clearances + task follow-ups as they come.

## 2026-05-21 — agent-recycle events (all six working agents handover-ready)

Per the recycle protocol in `process.md` (phase-7 ref;
inherits in phase-8) + your 2026-05-21 directive
"i will want to recycle everyone with the bootstrap
prompt so please prep the whole of next phase".

All six working agents have pre-recycle handover
appends at the tail of their inbound
`event-architect-<agent>.md` channels. Each handover
captures: cleared work in working tree (commit
instructions), queued tasks (numeric order),
standing-permission survival notes, recycle
continuity beat.

### Per-agent recycle signal

| Agent | Recycle-eligible? | Handover anchor |
|-------|-------------------|----------------|
| @@CI | Yes | `event-architect-ci.md` "2026-05-21 — PRE-RECYCLE HANDOVER" |
| @@FullStackA | Yes | `event-architect-fullstack-a.md` "2026-05-21 — PRE-RECYCLE HANDOVER" |
| @@FullStackB | Yes | `event-architect-fullstack-b.md` "2026-05-21 — PRE-RECYCLE HANDOVER" |
| @@Systacean | Yes | `event-architect-systacean.md` "2026-05-21 — PRE-RECYCLE HANDOVER" |
| @@WebtestA | Yes | `event-architect-webtest-a.md` "2026-05-21 — PRE-RECYCLE HANDOVER" |
| @@WebtestB | Yes | `event-architect-webtest-b.md` "2026-05-21 — PRE-RECYCLE HANDOVER" |

### @@Architect recycle position

I'm LAST to recycle + FIRST to come up per your
directive. My bootstrap reads
[`../architect/journal.md`](../architect/journal.md)
"2026-05-21 — Pre-recycle prep complete" — captures
the working tree state, queue depth per lane,
parked decisions, planning artifacts, permissions
that survive vs not.

### What's owed to you (your call when you resume; not blocking the recycle)

1. **@@WebtestB fresh-Mac walkthrough perm**: options
   (a)/(b)/(c) in `event-webtest-b-alex.md`. Default
   (c) if no reply.
2. **v0.11.2 CLI binary backfill**: workflow_dispatch
   to add CLI binaries to existing v0.11.2 GH Release
   (option b from @@CI's release.yml trigger glob
   finding), OR keep v0.11.2 DMG-only. Default
   DMG-only.

Neither is load-bearing for the recycle.

### Standing by

Ready when you are. I'll continue routing if any
inbound poke lands before recycle; otherwise this is
the close-of-session beat.

## 2026-05-23 — bridge ask for @@Desktect (via @@Alex)

@@CI's `ci-15` workflow audit found that `chan-v0.12.0` release assets ship desktop artifacts named **`Chan_0.11.2.*`** (DMG, AppImage, deb). The chan CLI artifacts on the same release correctly read `0.12.0`; only the desktop artifact name lags.

@@CI's read: this is **chan-desktop package metadata**, not workflow plumbing. `release-desktop.yml` is now chan-desktop-owned shared infra per the 2026-05-23 lane-boundary handoff. Routing through you (the bridge) rather than editing directly.

### Ask for @@Desktect

Verify before v0.13.0:

1. Where is the `0.11.2` literal coming from in the chan-desktop bundle (likely `desktop/src-tauri/tauri.conf.json` `version` field or similar)?
2. Should it bump to `0.12.0` retrospectively (no, the v0.12.0 release shipped already) or bump to `0.13.0` cleanly at the next cut?
3. Confirm the next chan-desktop bundle build picks up the correct version.

### Severity

Low; doesn't break the release pipeline mechanically. But it's a real metadata drift between chan CLI and chan-desktop on the same release tag. @@Desktect to triage + tell you the fix beat.

No fullstack-a-97-style urgency. Bundle into chan-desktop's normal workflow.

