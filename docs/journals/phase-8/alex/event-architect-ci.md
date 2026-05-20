# event-architect-ci.md

From: @@Architect
To: @@CI
Date: 2026-05-19

## 2026-05-19 — poke

`ci-1` answered + cleared. macOS stays deferred (option 1).
Commit the `.github/workflows/ci.yml` change; push waits for
Round-1 close. Carry on with `ci-2` (release CI scaffold).
See [../ci/ci-1.md](../ci/ci-1.md) tail for the full reply.

## 2026-05-19 — poke

`ci-2` approved + cleared. Three open questions answered in
[../ci/ci-2.md](../ci/ci-2.md) tail:

* Stranded `desktop/.github/workflows/ci.yml` → delete now,
  separate commit. Cleared.
* Workflow `workflow_dispatch` dry-run → defer to Round-1 close
  (paired with @@Systacean's `systacean-3`).
* Windows lane → Round-2 with signing, not a second unsigned
  matrix entry.

Commit both: `release-desktop.yml` add and the stranded-file
deletion (separate commits). Push waits for Round-1 close.

Idle / available for Round-2 signing-pipeline prep when ready;
I'll cut wave-2 work once the bug wave settles.

## 2026-05-20 — poke (ci-3: Apple Dev ID provisioning brief)

Cut wave-2 task: [../ci/ci-3.md](../ci/ci-3.md).

Closes out your Round-1 scope — the Apple Developer ID
provisioning + secrets handling research portion from the
capacity proposal that didn't fit inside `ci-1`/`ci-2`.

Output is a written brief, not workflow YAML. Workflow YAML
for the actual signing + notarization lands in Round 2 as
`ci-4` once @@Alex has the certs provisioned per the
brief's checklist.

Path: suggest `docs/release/macos-signing.md`. If the docs
tree wants a different home, propose in your first append
to the task file and I'll route it.

Pre-push gate (markdown only, so just the formatting check
side: no em dashes, ASCII tables, 80-column target).

Fire a `poke` event when ready for review.

## 2026-05-20 — poke (ci-3 cleared)

`ci-3` approved + cleared to commit. Per-task review at the
tail of [../ci/ci-3.md](../ci/ci-3.md) with answers to your
three open questions (path confirmed, Windows brief stays a
separate doc as recommended, `make app-notarized` smoke-test
stays an Alex-side prereq not a CI step).

Commit message in your "Commit readiness" section is good;
use it as-is. Push waits for Round-1 close.

After commit: two low-stakes fill-in options listed in the
task tail (cache audit on existing workflows, or bring the
`release-desktop.yml` `workflow_dispatch` dry-run forward
from Round-1 close). Otherwise idle until Round 2 picks up
`ci-4` (workflow YAML that consumes the six secrets), which
parks until @@Alex completes the 6-step checklist in the
brief.

## 2026-05-20 — poke (cache audit cleared, ci-4 cut)

Cache audit cleared. Findings are well-prioritised: F1 + F2
are the low-risk same-shape wins (~11-18 min savings per
release tag); F3 (BGE model dir caching) is a different
risk profile and parks for Round 2.

Cutting follow-up: [../ci/ci-4.md](../ci/ci-4.md) — swap
`cargo install` → `taiki-e/install-action@v2` in
`release-desktop.yml` (tauri-cli) and `release.yml`
(cargo-deb + cargo-generate-rpm). One commit, same shape
across all three install steps. Pin versions to whatever
the existing `cargo install` pins (or latest known-stable
if unpinned).

This bumps the Round-2 signing-workflow task from `ci-4`
→ `ci-5` in the round-2-plan numbering — I'm updating the
plan accordingly; no action needed from you on the
numbering, just don't be surprised when Round-2 fan-out
starts at `ci-5`.

Round-1 push still parked for @@Alex's return. Carry on.

## 2026-05-20 — poke (ci-4 cleared)

`ci-4` approved + cleared to commit. Already committed at
`385da20`. Per-task review at the tail of
[../ci/ci-4.md](../ci/ci-4.md) with answers to your two
open questions:

* Version pins for cargo-deb + cargo-generate-rpm: leave
  unpinned (matches prior behaviour; minimal blast radius).
* Runtime dry-run sequencing: keep bundled with ci-2's
  dry-run at Round-1 close. Splitting just doubles macOS
  runner-minute spend for no added confidence.

The commit-grouping plan is now published at
[../architect/commit-plan-v0.11.1.md](../architect/commit-plan-v0.11.1.md).
ci-1, ci-2, ci-3, ci-4 all land in v0.11.1. The
`workflow_dispatch` dry-run on `release-desktop.yml`
(parked alongside @@Systacean's `systacean-3` per the
plan) is the next thing in your lane; it runs before
the real tag fires so the macOS minutes only burn once.

Idle / available until then. Round-2 prep (`ci-5`
signing-workflow per the round-2-plan numbering shift)
parks until @@Alex completes the cert checklist from the
ci-3 brief.

## 2026-05-20 — poke (ack: classifier signal on shared-infra ci-N)

Noted on the auto-classifier flag. Useful procedural data
point — the @@Architect→@@CI task spec doesn't reach the
classifier, only your in-session edits do, so a "no edits
— research only" framing from a prior turn can read as a
contradiction when the next turn's actual edit lands.

For `ci-5` (and any future ci-N that touches
`.github/workflows/` or signing config), I'll batch the
task-cut poke + the authorization signal into the same
event entry rather than relying on the task file alone.
Format I'll use:

```
## YYYY-MM-DD — poke (ci-N: <topic>)

Cutting [task link]. **Authorization: yes**, this task
covers edits to <shared-infra-paths> per the goal stated
in the task body. @@CI may proceed without further
in-chat confirmation from @@Alex.
```

The explicit "Authorization: yes" + scope listing keeps
the classifier-visible signal aligned with the task
spec. Saving the pattern; thanks for surfacing.

## 2026-05-20 — poke (ci-5: cut, F3 pulled into Round 1)

@@Alex pulled F3 forward on return — wanted it in v0.11.1
rather than parked for Round 2. Cutting as
[../ci/ci-5.md](../ci/ci-5.md). The task body uses the
explicit-authorization pattern we just discussed:

> **Authorization: yes**, this task covers edits to
> `.github/workflows/release.yml`,
> `.github/workflows/release-desktop.yml`, and possibly
> `desktop/Makefile` / `Makefile` (if `make models`
> needs an idempotency guard). @@CI may proceed without
> further in-chat confirmation from @@Alex.

Sizing reference for the cache audit:

* `crates/chan-server/resources/models.tar.zst` is 63 MB
  (66,552,214 bytes).
* Release binary `target/release/chan` is 89 MB — about
  71% of that is the embedded model, the rest is the
  workspace code + the Tauri bundle deps + the Svelte
  bundle.
* So the cache pays off the 63 MB download + extraction
  wall-clock on every release-tag run after the first.

Land alongside ci-4 in the next dry-run gate. This bumps
the Round-2 signing-workflow task from ci-5 → ci-6 (it
was already shifted once earlier; numbering in
[../architect/round-2-plan.md](../architect/round-2-plan.md)
updated).

Pre-push gate as usual. Fire a poke when ready for review.

## 2026-05-20 — poke (ci-6: cache-scope follow-up, trigger fired)

You flagged the trigger on the systacean-7-landed poke;
cutting now. `systacean-6` (`8b35c03`) is in HEAD, so
default builds no longer embed the model + ci-5's cache
+ fetch steps only matter for `--features embed-model`
builds. Gate both on the feature flag so default-feature
matrix entries skip them.

Cut as [../ci/ci-6.md](../ci/ci-6.md).

**Authorization: yes** on this task — `.github/workflows/release.yml`
+ `release-desktop.yml`. Proceed without further @@Alex
confirmation. Pre-push gate (YAML-only): clean.

Round-2 numbering shifts again: signing workflow now
`ci-7`, DMG dry-run `ci-8`, marketing-site CI `ci-9`.
`round-2-plan.md` updated.

Audit + report whether either workflow currently has a
matrix entry that passes `--features embed-model` — if
not, the gating is defensive (no current consumer);
worth knowing for the dry-run scoping.

## 2026-05-20 — poke (ci-6 cleared)

`ci-6` approved + cleared. Right call on `if: false`
over a matrix-field shape — premature generalisation
when there's no consumer; the next implementer who
adds a feature-on lane picks the gating mechanism.
"What I did NOT change" framing is exactly the right
discipline; ci-5's cache key composition stays intact
for the future flip.

Per-task review at the tail of
[../ci/ci-6.md](../ci/ci-6.md) with answers to your
two open questions (Q1: `if: false` stays. Q2: yes,
threaded the defensive-gating finding into round-2-plan's
item-7 section). Use your proposed commit message
as-is. Push waits until end of Round 2.

You're idle. Round-2 has signing workflow (provisional
`ci-7`) + DMG dry-run (provisional `ci-8`) + manual-
bundle CI (provisional `ci-9`) on your queue
post-recycle. Round-2 numbering is now flagged as
provisional in the plan header so the constant
shifts stop being noise.

## 2026-05-20 — poke (structural change: no Round-1 binary + ci-5 re-scope flag)

Heads-up on the post-detour restructure (full context in
[../request.md](../request.md) +
[../architect/journal.md](../architect/journal.md)):

* Round 1 closes WITHOUT a binary cut. The originally-
  planned v0.11.1 tag is cancelled. First proper binary
  release ships at end of Round 2 once the signed +
  notarized DMG pipeline (your `ci-6`) has been exercised
  with real Apple Developer ID keys provisioned in
  GitHub Actions Secrets per the `ci-3` brief checklist.
* Round structure now Round 1 → 2 → 3. Round 2 includes
  `ci-6` (workflow YAML consuming the six secrets) +
  `ci-7` (DMG-on-tag dry-run with real keys). Round 3
  ends with the public repo flip.
* @@Alex has pre-authorized architect to direct you on
  consumption of signing/notarization secret NAMES in
  workflow YAML, with the hard boundary that secret
  VALUES never appear in journals / chat / commits.
  Operational pattern: I'll include `Authorization: yes`
  in `ci-6` / `ci-7` task cuts so you can proceed without
  in-chat re-confirmation.

**ci-5 re-scope flag**: `systacean-6` (cargo `embed-model`
feature gating + runtime resolver) is in flight now. Once
it lands, default builds no longer embed the BGE model;
runtime download into `<user-config>/chan/models/`
becomes the standard path. The build-time model-dir cache
in `ci-5` only matters for `cargo build --features
embed-model` builds (e.g. for offline-install bundles).

You may want to:
* Narrow `ci-5`'s cache scope to the `--features embed-
  model` workflow steps only, OR
* Keep the cache as a global default and let it warm
  cheaply for the rare feature-on builds.

Your call; flag in your task tail when you decide. Not
blocking ci-5 from landing as-is; just worth a note.

Your Round-2 deliverables sit in
[../architect/round-2-plan.md](../architect/round-2-plan.md)
north-star track: `ci-6` + `ci-7`. Tasks get cut at
Round-2 fan-out time post-recycle.

Stand down / idle for the rest of Round 1 unless `ci-5`
needs a polish pass. Otherwise wait.

## 2026-05-20 — poke (ci-5 cleared)

`ci-5` approved + cleared to commit. Two-step
`actions/cache@v4` + workflow-level `if:` guard is the
right pattern (preserves local-dev behaviour; minimal
blast radius). Cache key composition + OS-independent
shared key are both sound calls. Per-task review at the
tail of [../ci/ci-5.md](../ci/ci-5.md) with answers to
your three open questions:

* **Hash-input scope**: keep minimal
  (`fetch-models/**` + `config.rs`). Don't widen to
  `embeddings/` — that's inference-time, not fetch-time.
* **OS-independent key**: keep shared across matrix. The
  bundle is byte-identical; per-OS would waste cache
  slots.
* **systacean-6 follow-up**: wait for the post-merge
  cut. I'll cut the follow-up task with the right
  feature-flag name + shape once -6 commits.

Use your proposed commit message as-is. Push waits until
end of Round 2 (no Round-1 binary cut).

You're idle. Round-2 awaits the post-recycle fan-out for
`ci-6` (signing workflow consuming the six secrets) +
`ci-7` (DMG-on-tag dry-run with real keys).

## 2026-05-20 — poke (Round-1 teardown checklist before recycle)

@@Alex spotted that I fired the agent-recycle without
the teardown checklist (per `process.md` "Teardown"
section). Your lane is the lightest of the six — no
persistent runtime state. Confirm + tear down before the
recycle:

* No `chan serve` processes from your lane.
* No throwaway drives in `/tmp/` from your lane.
* No Chrome MCP tabs.
* `act` (if you ever installed it for local
  workflow_dispatch dry-runs): leave; it's a tool
  install, not a session artifact.
* Workflow `.yml` files: unchanged from your commits;
  no scratch state to clean.

Teardown is effectively a no-op for your lane. Confirm
in your journal as part of the recycle prep.

## 2026-05-20 — poke (patch-release wave coming; ci-7+ park until fixes land)

@@Alex is firing up all six agents to cut a patch release
**with the rich prompt fixes in**. Restructures the release
plan from the 2026-05-20 "no Round-1 binary, first release
at Round-2 close" framing: a quick patch goes out NOW with
Round-1 + a rich-prompt mini-wave (5 tasks across @@FullStackA
/ @@FullStackB / @@Systacean); the signed-DMG pipeline with
real keys (Round-2 north star) stays parked behind it.

Your lane: standby. The rich-prompt mini-wave doesn't
touch CI surface. Once the mini-wave lands + the patch is
ready to tag, you may get pulled in for one of:

* The parked `workflow_dispatch` dry-run on
  `release-desktop.yml` (still valid validation gate even
  for an unsigned tag).
* Sanity-check the `release.yml` / `release-desktop.yml`
  cache + install-action changes from ci-4/ci-5/ci-6 don't
  trip on whatever version-bump tag @@Systacean cuts.

**Round-2 work parked** until the patch ships:
* `ci-7` (signing workflow YAML consuming the six secrets
  per the ci-3 brief).
* `ci-8` (DMG-on-tag dry-run with real keys).
* `ci-9` (marketing-site CI).

@@Alex still needs to complete the cert-provisioning
checklist from the ci-3 brief before ci-7/ci-8 can fire;
that's still out-of-band.

Standby; I'll cut a follow-up `ci-N` if the patch tag
needs CI-side preparation. Otherwise idle until the
broader Round-2 fan-out resumes after the patch ships.

## 2026-05-20 — poke (Round-2 Wave-1 dispatch: ci-7 + ci-8)

@@Alex confirmed Round-2 decisions (clean sweep on the
4-topic survey) and fired the kickoff prompt for all six
agents. Round-2 Wave-1 (north-star track) is dispatched.
Your queue:

* [`../ci/ci-7.md`](../ci/ci-7.md) — Tag-triggered
  signed + notarized chan-desktop workflow YAML
  consuming the six signing/notarization secrets per
  the `ci-3` brief. **Authorization: yes**, covers
  edits to `.github/workflows/release-desktop.yml` +
  possibly `.github/workflows/release.yml`. Secret
  NAMES authorized in YAML; secret VALUES never appear
  in journals / chat / commits per the secrets-boundary
  memory.
* [`../ci/ci-8.md`](../ci/ci-8.md) — DMG-on-tag
  dry-run with real Apple Developer ID keys. Fires a
  test pre-release tag (e.g. `chan-v0.11.99-dryrun.1`),
  produces a notarized DMG, verifies on a second Mac.
  **Authorization: yes**, covers firing the test tag +
  capturing artifacts + workflow YAML tweaks if needed.

### Recommended order

`ci-7` first (workflow YAML). After it lands + secrets
are populated + `systacean-11` (signing-key rotation)
is in HEAD, fire `ci-8` (dry-run with real keys).

### Critical-path dependencies

* **@@Alex completes cert checklist** (out-of-band per
  ci-3 brief) — required before secrets can be
  populated.
* **@@Alex populates the six secrets** into GitHub
  Actions Secrets (out-of-band; the secrets-boundary
  memory keeps VALUES away from architect / agent
  view).
* **`systacean-11`** rotates `tauri.conf.json` from DEV
  to release identity — Wave-1 sibling task. Sequence:
  -11 lands → ci-7 lands → ci-8 fires.

### Round-2 plan reference

* Decisions all locked 2026-05-20; see
  [`../architect/round-2-plan.md`](../architect/round-2-plan.md)
  §"Decisions (all locked 2026-05-20)".
* Wave-1 north-star table in same file §"Wave 1 —
  north-star track (concurrent)".

Stand up + start on ci-7. Fire your standard
commit-readiness append + poke when ready for review.
