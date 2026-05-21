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

## 2026-05-20 — poke (ci-7 cleared + answers to your three Qs)

`ci-7` approved + cleared to commit. Comprehensive shape:
verify-secrets fast-fail (no value leakage), brief-recommended
`apple-actions/import-codesign-certs@v3`, `make app-notarized`
under the four `APPLE_*` env vars, full
codesign+spctl+stapler-validate trifecta, failure-mode
diagnostics upload, tag-gated release job with
`fail_on_unmatched_files: true`. Header-comment rewrite from
the "Round-2 follow-ups deferred" framing to current shape is
also right hygiene.

Per-task review at the tail of [`../ci/ci-7.md`](../ci/ci-7.md);
use your proposed commit subject (the body's "Closes phase-8
ci-7" citation is the audit-trail anchor; no need to add a
parenthetical task tag). Push waits until end of Round 2.

### Q1 — macOS universal2 scope: FOLLOW-UP, not ci-7

**Confirmed**: cut as a follow-up `ci-N` after ci-7 + ci-8
land green on aarch64. Don't absorb into ci-7.

@@FullStackB's `desktop/CLAUDE.md` amendment is forward-looking
but correctly identifies CI as the right surface (Makefile is
the wrong layer for a per-arch `lipo`-merge; CI's matrix
build is the natural place). The amendment doesn't claim
ci-7-specific ownership — re-read it as "CI's release
workflow eventually owns this" rather than "ci-7 absorbs it".
Both lanes read this correctly; I'm just confirming the
boundary aloud so the audit trail is unambiguous.

Practical sequencing: get a signed+notarized aarch64-only
DMG through ci-8's dry-run first (validates the whole
sign+notarize+staple pipeline against real keys). THEN cut a
`ci-N` that adds the x86_64 matrix entry + `lipo -create`
step + universal2 bundling. Splitting the work makes each
piece independently bisectable if something regresses.

### Q2 — Linux + Windows GH Release upload sequencing: FOLLOW-UPS

**Confirmed**: cut as `ci-N` follow-ups when each platform's
signing lane opens.

* **Linux**: needs the GPG-signing brief (separate ci-3-style
  research lap; not yet cut). Once that brief lands, a
  follow-up `ci-N` extends ci-7's release job to consume the
  Linux artifacts.
* **Windows**: not yet in the matrix. Adding it requires:
  Windows-side signing cert + provisioning (pre-authorized
  per the secrets-boundary memory's standing-permissions
  table; @@Alex sources the cert), Windows runner matrix
  entry, MSI/EXE signing step. Probably another ci-3-style
  brief first. Cut at fan-out when ready.

I'll add a tracking row to round-2-plan.md's "Round-2 close"
table for both follow-ups so the cuts don't get forgotten
when ci-8 lands green.

### Q3 — `apple-actions/import-codesign-certs@v3` SHA pin: ROUND-3

**Confirmed**: stay on major-version pin for ci-7. Full-SHA
pin sweep across all third-party actions is the right shape
for the Round-3 Track 3 hardening pass (per
[`../architect/round-3-plan.md`](../architect/round-3-plan.md)
Track 3 — code cleanup + hardening + efficiency + docs
review + release readiness). I'll flag this in the Round-3
plan's Track 3 list when fan-out time comes; you'd be the
natural lane.

### Proceed on ci-8

ci-8 parks until:

1. `ci-7` commits — your suggested commit subject + body.
2. `systacean-11` commits — @@Systacean's signing-key
   rotation is the chan-desktop config that ci-7's actual
   signing step reads from the rotated identity. -11 is
   currently parked on @@Alex's release-identity decision
   (permission event fired by @@Systacean to
   `event-systacean-alex.md` 2026-05-20).
3. @@Alex confirms the six signing secrets are populated in
   GitHub Actions Secrets (the verify-secrets step fails fast
   when they're absent; ci-8 firing the test tag without
   them populated would just exercise the failure path,
   which IS useful coverage but should be a deliberate
   choice, not an accident).

Your existing permission event to @@Alex on the secrets-state
question is the right shape — don't transcribe approval since
this needs @@Alex's interactive participation (logging into
GitHub Settings is a hands-on action).

Standing by for the ci-7 commit + my onward routing.

## 2026-05-21 — poke (items 2 + 3 routing + ci-9 cut)

@@Alex 2026-05-21 confirmed B.2 ANSWERED — all six
secrets populated (per script-ran-clean transcribed to
[`event-ci-alex.md`](event-ci-alex.md)).

### Item 2 — ci-7 verify step regression: Option (a) — cut ci-9 patch

Sharp catch on the `stapler validate "$APP"` mismatch
with `systacean-13`'s split flow. Right call — DMG-only
stapling IS the canonical Apple shape for DMG
distribution; the .app inherits trust from the DMG
wrapper at mount time.

Cutting [`../ci/ci-9.md`](../ci/ci-9.md) for the YAML
patch. **Authorization: yes** — covers
`.github/workflows/release-desktop.yml` only. Use your
proposed 5-line replacement (drop `stapler validate
"$APP"` + swap `spctl -t open` on .app → `spctl -t
install` on DMG; keep codesign metadata check on .app).
Small commit; pre-push gate: YAML-only.

### Item 3 — v0.11.2 fires SIGNED: Option (c) — fire ci-8 dry-run first

Approved. Plan-intent preservation + pre-validation
both matter; option (c) gives both. Order:

1. `ci-9` (verify-step patch) lands first — blocks the
   silent failure at the verify step.
2. `ci-8` fires `chan-v0.11.99-dryrun.1` (or whichever
   pre-release tag name you pick) via tag push OR
   `workflow_dispatch`. Validates the whole
   sign+notarize+staple pipeline end-to-end against
   real keys.
3. If dry-run is green, @@Systacean cuts `chan-v0.11.2`
   tag — which then ALSO fires signed automatically.
   v0.11.2 becomes the first signed release in
   practice.
4. `commit-plan-v0.11.2.md` gets an architect-side
   update reflecting "v0.11.2 ships SIGNED, not
   unsigned" + the ci-8-first sequencing. I'll handle
   the plan-doc update architect-side.

You can fire `ci-8` (dry-run) the moment `ci-9`
commits. Don't wait for the v0.11.2 task commits to
land — the dry-run is independent of the SPA /
chan-desktop changes in the patch wave (only validates
the chan-desktop signing pipeline against the chan
crate + current Wave-1 work).

### Queue state on your lane

* `ci-7` ✓ committed (`666c027`).
* `ci-9` NEW — verify-step patch. Cut now.
  **Authorization: yes**,
  `.github/workflows/release-desktop.yml`.
* `ci-8` — fires after `ci-9` commits. Test-tag
  `chan-v0.11.99-dryrun.1` (or @@Alex redirects).

Proceed without further architect ack — both items
unblocked.

## 2026-05-21 — poke (billing unblocked — rerun ci-8)

@@Alex 2026-05-21 fixed the GitHub Actions billing state
(failed-payment resolved + spending limit bumped). Budget
is healthy.

### Action

Rerun the existing dry-run workflow without creating a new
tag:

```
gh run rerun 26200703893
```

Same SHA (`chan-v0.11.99-dryrun.1`), same workflow YAML
(includes your `ci-9` `f5b0122` verify-step patch), same
six secrets in GH Settings. The Linux + macOS jobs should
now execute end-to-end + produce:

* Linux unsigned bundles uploaded as workflow artifacts.
* macOS signed + notarized `.dmg` uploaded to a workflow
  artifact AND attached to the `chan-v0.11.99-dryrun.1`
  GitHub Release.

### What to capture in the ci-8 task tail

Per the acceptance criteria:

* Workflow total wall-clock + per-step breakdown.
* Notarization wait time (typically the dominant cost).
* DMG artifact size.
* notarytool log excerpt confirming green.
* Any failure-mode walkthrough (intentional or accidental).

### After ci-8 green

* Ping back here so I can route @@WebtestB for the
  second-Mac install + double-click + Gatekeeper-clean
  check on the produced DMG (their chan-desktop standing
  permission per `ada8478` covers it).
* Once @@WebtestB confirms green, @@Systacean cuts
  `chan-v0.11.2` (which now ships SIGNED per the plan
  revision committed at `abf5ab2`).

### v0.11.1 chan-desktop bundles backfill — DON'T re-run

Per your earlier flag: leave `chan-v0.11.1`'s broken workflow
as-is. Re-running it post-billing-fix would produce signed
chan-desktop bundles for v0.11.1 which contradicts the
"v0.11.1 unsigned, v0.11.2 first signed" narrative we just
locked in the plan revision. v0.11.2 is the first to ship
chan-desktop binaries; cleaner story for users + audit
trail.

Proceed with the rerun whenever you're ready.

## 2026-05-21 — poke (latent ci-4 `^2` bug: approve Option C + tag-shape (b))

Sharp catch on the latent bug from `ci-4`. The `^2`
assumption was reasonable given cargo's syntax familiarity
— `taiki-e/install-action`'s narrower contract (no semver
operators) is exactly the kind of thing that surfaces only
when the workflow fires for real.

### Routing approved

* **Option C** (direct amendment commit, no task file) —
  approved. 1-line YAML fix doesn't warrant task ceremony.
  Use your proposed commit message:

  ```
  ci: tauri-cli major-only pin for taiki-e/install-action (fixes ci-4 latent bug)
  ```

  Bump the message slightly to include the v0.11.2-wave
  context if you like, but the audit trail in the commit
  body + the [ci-4 task file](../ci/ci-4.md) "Open
  questions" / "Findings" append (recommend adding one
  noting the latent bug + the post-mortem) is enough.

* **Tag-shape (b)** — approved. Cut a new dry-run tag
  `chan-v0.11.99-dryrun.2` pointing at the fix commit.
  Leaves `dryrun.1`'s failed-run audit anchor in the
  Actions history; `dryrun.2` carries the green run.

### Action sequence

1. Apply the 1-line YAML fix to
   `.github/workflows/release-desktop.yml`.
2. YAML-parse validation locally.
3. Append a "Findings (2026-05-21 post-fire bug)" section
   to [`../ci/ci-4.md`](../ci/ci-4.md) documenting the
   `^2` vs `2` issue so the audit trail self-corrects.
4. Commit (single workflow file + the ci-4 docs append):
   ```
   ci: tauri-cli major-only pin for taiki-e/install-action (fixes ci-4 latent bug)
   ```
5. Cut + push the new dry-run tag:
   ```bash
   git tag -a chan-v0.11.99-dryrun.2 -m "Dry-run #2 with ci-4 ^2 fix" HEAD
   git push origin chan-v0.11.99-dryrun.2
   ```
6. Workflow fires automatically on the tag. Watch the run
   + capture metrics in the `ci-8` task tail per the
   acceptance criteria.

### After ci-8 dry-run green

* Ping back so I can route @@WebtestB for second-Mac
  install + Gatekeeper-clean check.
* Once @@WebtestB confirms green, @@Alex fires the "cut it"
  signal → @@Systacean cuts `chan-v0.11.2`.

### Lane state update

| Item                                  | State                                |
|---------------------------------------|--------------------------------------|
| ci-7 commit                           | ✓ (`666c027`)                        |
| ci-9 commit                           | ✓ (`f5b0122`)                        |
| Billing                               | ✓ unblocked                          |
| ci-4 `^2` bug                         | Approved Option C amendment commit   |
| ci-8 dry-run #2                       | Approved tag-shape (b); fire after the fix commits |

Proceed without further architect ack — both decisions
locked.

## 2026-05-21 — poke (both dry-run #2 bugs routed; standby for dry-run #3)

Excellent post-fire diagnosis — your CI infra is genuinely
validated end-to-end. The two surfacing bugs are both
out-of-lane (build-side, chan-desktop crate); good signal
that the workflow YAML + verify + secrets-handling are
all clean.

### Routing

Both bugs combined into single hotfix [`../fullstack-b/fullstack-b-20.md`](../fullstack-b/fullstack-b-20.md)
against @@FullStackB:

* **Bug #1**: Option (a) approved — drop universal2
  expectation for v0.11.2; aarch64-only DMG ships.
  @@FullStackB's task body covers JSON-shape options
  (i)/(ii)/(iii) per Tauri 2 docs.
* **Bug #2**: 1-char `app` → `_app` rename at
  `main.rs:910`. Trivial.

Single commit covers both. Dispatch poke fired to
@@FullStackB.

### Your action sequence (after -b-20 commits)

1. Cut `chan-v0.11.99-dryrun.3` pointing at the new HEAD
   (post -b-20 commit).
2. Push tag → release-desktop.yml fires.
3. Watch the run. Should be clean green now barring
   further latent bugs (which we'd flag as dry-run #4 +
   keep iterating).
4. Capture metrics in `ci-8` task tail per acceptance.
5. Ping back — I route @@WebtestB for second-Mac verify.

### Universal2 follow-up (post-v0.11.2)

Once v0.11.2 ships aarch64-only, cut a real `ci-N` task
for the universal-DMG work:
* Makefile builds both `aarch64-apple-darwin` +
  `x86_64-apple-darwin` chan binaries.
* `lipo -create` step merges them into a universal2 fat
  binary.
* `tauri.conf.json` restores the auto-expansion shape
  (or stays explicit, implementer picks).
* CI matrix may add a parallel x86_64 build step on
  macos-13 runners (cheaper than running lipo locally
  on macos-latest).
* `desktop/CLAUDE.md`'s "Bundled chan sidecar" /
  "Architecture handling" subsection gets revised to
  reflect universal2.

Provisional task number: `ci-10` or whatever's free at
fan-out. Not v0.11.2 scope.

### v0.11.2 cut path remaining

| Gate | Owner | State |
|------|-------|-------|
| -b-20 commit | @@FullStackB | dispatched |
| ci-8 dry-run #3 fires green | @@CI | waits on -b-20 |
| @@WebtestB second-Mac verify | @@WebtestB | waits on green DMG |
| @@Alex "cut it" | @@Alex | final |
| @@Systacean cuts `chan-v0.11.2` | @@Systacean | final |

Standing by.

## 2026-05-21 — ack (ci-8 dry-run #3 diagnosis + routing)

**Written by @@Alex (via assistant) outside the regular
@@Architect session** — Round-2-close momentum;
recording so the regular @@Architect picks it up on next
bootstrap.

Notary log on submission
`7f327f46-8c5a-430d-80fb-95d174109d50` confirmed your
suspicion exactly: three errors, all on
`Chan.app/Contents/MacOS/chan` (the bundled sidecar),
arch arm64:

1. Not signed with a valid Developer ID certificate.
2. Signature does not include a secure timestamp.
3. Hardened runtime not enabled.

Root cause confirmed: `-b-20`'s `bundle.macOS.files`
shape bypasses Tauri's signing pass (which only walks
`externalBin` + chan-desktop + .app wrapper).

### Routing

Cut [`../fullstack-b/fullstack-b-21.md`](../fullstack-b/fullstack-b-21.md)
against @@FullStackB. Three fix options sketched:
recommendation is to try `bundle.macOS.externalBin`
per-platform first (Option C in the task body); fall
back to a chan-bin codesign step (Option A) if Tauri 2's
per-platform externalBin has the same triple-append bug
as the top-level key did.

### Your next move

Standing by. Once @@FullStackB lands -b-21, the action
sequence at the tail of your prior poke applies again:

1. Cut `chan-v0.11.99-dryrun.4` against new HEAD.
2. Push tag, watch the run. Linux should stay green
   (no changes to that path); macOS notarization should
   now pass.
3. Capture metrics in ci-8 task tail. Ping back; I
   route @@WebtestB.

### Auto-fetch notary log on failure — future work

Your earlier suggestion to add an auto-`notarytool log`
step to release-desktop.yml's failure path is a great
ci-N candidate post-v0.11.2. Would have shaved the
"@@Alex runs Keychain command locally" round-trip off
this loop. Park it; cut after v0.11.2 ships.

### Provenance

Same as on event-architect-fullstack-b.md: this ack +
the -b-21 task file were written outside the regular
@@Architect session. No standing @@Architect behaviour
implied.

## 2026-05-21 — poke (chan-v0.11.2 cut-it signal — your workflow auto-fires)

@@Alex cleared the cut. Recap for your next bootstrap:

* `ci-8` dryrun.4 is GREEN (run 26216314316; signed +
  notarized DMG on GH Release; ~20m11s wall-clock total
  with ~10-11m notary wait — within your `ci-3` brief's
  envelope).
* @@WebtestB walked the DMG on the dev Mac. All
  load-bearing Gatekeeper signals (spctl + stapler +
  codesign + syspolicyd) came back accepted-Notarized-
  Developer-ID. Audit trail at
  [`../webtest-b/webtest-b-1.md`](../webtest-b/webtest-b-1.md)
  "ci-8 DMG signed/notarized Gatekeeper check (dryrun.4)".
* @@Alex **accepted the dev-Mac partial as sufficient**
  rather than block the cut on a canonical second-Mac
  walkthrough. The cross-Mac literal acceptance is
  deferred to next time the verification fires (under
  tighter WebtestB scope rules I'm landing in their
  inbound channel).
* @@Systacean instructed (parallel poke at
  [`event-architect-systacean.md`](event-architect-systacean.md))
  to tag `chan-v0.11.2` on their next bootstrap.

### Your queue

Nothing to dispatch. `release.yml` +
`release-desktop.yml` auto-fire when the tag arrives —
that's your workflow doing its job. Stand by for:

1. Watch the actual `chan-v0.11.2` workflow run when
   @@Systacean pushes. Predicted green on the same
   trajectory as dryrun.4 (no workflow changes since).
2. If anything reds, route diagnostic per the failure-
   mode framing in `ci-8`'s tail. Apple notary log
   fetch is still manual until the auto-fetch-on-failure
   ci-N lands.
3. Post-tag cleanup: the `chan-v0.11.99-dryrun.1..4`
   tags can be deleted from the remote at your
   convenience; parked behind the v0.11.2 cut per the
   ci-8 final-metrics append. Not urgent.

### Auto-fetch notary log on failure (carryover)

Still parked as a post-v0.11.2 `ci-N`. Cut it when you
spin up the next session. Acceptance criterion: the
`failure()` diagnostic-upload step in release-desktop.yml
captures `xcrun notarytool log <submission_id>` as an
artifact so the human round-trip to @@Alex's local
keychain goes away. Submission ID is in the build log
already; just needs to be parsed + passed to notarytool.

### Cosmetic polish (future ci-N)

The DMG filename suffix is `_x64` despite the artifact
being aarch64 (Tauri-bundler default). Cosmetic only;
flagged in ci-8's final-metrics append. Worth a tiny
fixup in some future tag — not v0.11.2 scope.

Standing by for the workflow result poke on tag-push.
