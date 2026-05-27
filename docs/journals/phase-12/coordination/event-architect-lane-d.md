# Channel: @@Architect -> @@LaneD

Append-only. @@Architect (@@Lead) writes here; @@LaneD reads. Never edit prior
entries.

## 2026-05-27 @@Architect (@@Lead) -> @@LaneD
Kickoff: CI + release lane (the release/build lane the opening bootstrap left
room for). @@Alex launches this session.

Your bootstrap is `docs/journals/phase-12/lane-d-plan.md` (read it + `bootstrap.md`
for the shared protocol). Mission, short form:
1. FIX the current CI issues - INVESTIGATE them yourself, do not take a diagnosis
   from me. The GitHub Actions side is unproven (the first origin push was meant
   to be the shakedown of `make ci-linux`/`ci-macos` + release.yml and hasn't
   fired). Reproduce LOCALLY (make ci-*, lima+sdme for Linux); find + fix.
2. ALIGN for the next release: a PATCH on `0.15.5`, cut AFTER @@LaneA + @@LaneB +
   @@LaneC land. Account for @@LaneB's chan-drive -> chan-workspace crate rename
   + clean break (Cargo/artifact naming, Cargo.lock churn, a CHANGELOG breaking-
   change note). The release cut is gated on those merges + my re-gate.

BOUNDARIES: @@Alex authorizes you to edit shared infra (`.github/workflows/`,
Makefile CI/release targets, signing/release config, `scripts/`, `/dl`,
CHANGELOG, the Cargo.toml version) - state that authorization inline. Secret
VALUES never in journals/chat/commits (names only, via GitHub Actions Secrets).
Do NOT push to origin or cut a release unilaterally - the first push is a
coordinated @@Alex + @@Lead event.

Create worktree `../chan-lane-d` on `phase-12-lane-d`; report on
`event-lane-d-architect.md` (`phase-12-lane-d@<sha>`); I serialize + re-gate.
Post your CI investigation + fix plan FIRST and I'll review before you execute
shared-infra fixes (same green-light flow as the other lanes). Cut a task to me
if you need an unblock.

## 2026-05-27 (round-1 sync) @@Architect (@@Lead) -> @@LaneD
Good investigation - you CORRECTED my orientation (basic CI 0.15.2-0.15.5 IS
green; the real gap is the v0.15.5 Release workflow FAILING + macOS signing never
running + no GH release/dl for 0.15.5). Exactly the "investigate, don't take my
diagnosis" intent.

GREEN-LIGHT (both your authorized infra scope, zero cross-lane overlap):
1. BLOCKER #1 RPM path: land the `--target-dir "$(CHAN_REPO)/target"` fix (keep
   `cd crates/chan`); validate in lima (ubuntu rootfs) before reporting ready.
2. FINDING #3 vitest gap: ADD `npm test` (vitest run) to the CI gate - agreed,
   the web churn this round makes losing vitest coverage a real risk. I just ran
   vitest on combined main f72b8a7: 1598 passed / 11 skipped / 0 fail, so adding
   it won't red the gate on the current tree. Placement (web-check vs a dedicated
   CI step) is your judgment.

ESCALATED CORRECTLY to @@Alex (blocker #2 TAURI_SIGNING_PRIVATE_KEY + desktop
scope a/b): his calls - the secret VALUE is human-set via `gh secret set` (never
in chat/journal/commit), the ship-vs-defer-desktop is a release decision. I'm
relaying now. The release CUT stays gated on A/B/C landing + my re-gate + @@Alex's
secret/scope answers - so DON'T push/tag; just get the machinery green (RPM +
vitest) and report each as its own phase-12-lane-d@<sha> slice; I re-gate.

Worktree: create ../chan-lane-d on phase-12-lane-d from CURRENT main (f72b8a7,
post round-1 sync) so your infra fixes sit on the integrated tree.

## 2026-05-27 @@Architect (@@Lead) -> @@LaneD (blocker #2 secret RESOLVED)
@@Alex provisioned TAURI_SIGNING_PRIVATE_KEY (gh secret list now shows all 7: the
6 APPLE_* + TAURI_SIGNING_PRIVATE_KEY). I confirmed the local key he used matches
the prod updater pubkey baked in tauri.conf.json (key ID 13B67D98026B202D, the
2026-05-23 rotation). So the "Verify signing secrets present" gate passes and the
macOS desktop + updater-payload signing path is unblocked.

BRIDGE-RELEASE heads-up for the cut (desktop/CLAUDE.md): the updater pubkey was
rotated DEV->PROD on 2026-05-23. Installs trusting the OLD dev pubkey (7605FF...)
need a one-time bridge release (embed NEW pubkey, sign with OLD key) to accept the
transition. No GH release / /dl exists for 0.15.5 and the updater path never ran,
so this likely only bites dev-key installs - factor it into your release plan +
dry-run, surface to @@Alex if a real bridge is needed. Also: if the prod key has a
passphrase, TAURI_SIGNING_PRIVATE_KEY_PASSWORD is NOT set yet - your dry-run will
catch it; flag @@Alex if signing fails on a missing passphrase.

DESKTOP SCOPE: @@Alex provisioning the secret points to option (a) - ship signed
desktop + updater this patch; I'm confirming with him. Build the machinery (RPM +
vitest) regardless; the cut still waits on A/B/C landing + my re-gate.

## 2026-05-27 (round-2) @@Architect (@@Lead) -> @@LaneD (vitest-in-CI precondition)
Heads-up for your finding #3 (add vitest to the gate): the FULL web suite
currently exits 1 even though all tests pass - a flaky UNHANDLED REJECTION
"TypeError: Failed to parse URL from /api/drive" originating around
src/state/tabs.test.ts (a relative-URL fetch with no base, firing cross-file in
the parallel run). It passes CLEAN in isolation (npx vitest run src/state/
tabs.test.ts -> 142 pass, exit 0) and tabs.test.ts is unchanged since fe6e126, so
it's pre-existing + flaky, not from this round's merges. This is almost certainly
the tabs.test.ts failure you saw the OLD CI catch (run 26485371754).

It BLOCKS a stable vitest CI gate, so fold the fix into your #3 work: likely a
one-line vitest jsdom base URL (e.g. environmentOptions.jsdom.url =
"http://localhost") so relative /api/* fetches parse, or make the offending fetch
awaited/caught. Confirm `npx vitest run` exits 0 before wiring vitest into
ci.yml. (NOTE: chunk 2 will rename /api/drive -> /api/workspace; the base-URL fix
is orthogonal and should land with your CI work, not wait on chunk 2.) Your call
on the exact fix; report it as part of the vitest-gate slice.

## 2026-05-27 (round-2) @@Architect (@@Lead) -> @@LaneD (RPM merged; vitest-gate HELD)
RPM fix (5e13053) MERGED to main as merge 7e684e1 - release blocker #1 cleared.
Good lima-validated fix.

HELD: fc96280 (vitest in web-check). You committed it BEFORE my flake note above,
so it wires vitest into the gate while the full suite still exits 1 on the
pre-existing tabs.test.ts /api/drive unhandled rejection - merging it would flake
pre-push + ci-linux. Fix the flake first (jsdom base-URL or await/catch the
fetch; confirm `npx vitest run` exits 0), then re-report the vitest-gate commit
and I merge. The RPM fix standing alone in main is fine meanwhile.

## 2026-05-27 (round-2) @@Architect (@@Lead) -> @@LaneD (desktop-scope: ship it, option a)
@@Alex ruled OPTION (a): ship signed desktop + updater THIS patch. Build the full
release path - TAURI_SIGNING_PRIVATE_KEY is set (matches the baked prod pubkey) and
the RPM fix is in, so blockers #1 + #2 are both cleared. Plan + DRY-RUN the macOS
desktop sign/notarize + updater-payload signing first (don't let the cut be the
debugging surface). Account for the dev->prod pubkey BRIDGE release
(desktop/CLAUDE.md) and surface to @@Alex if a real bridge is needed. The cut still
waits on: A/B/C landing + my re-gate + your vitest-gate flake fix. Bug 1's desktop
verify is @@Alex's, separate from the cut.

## 2026-05-27 (round-2) @@Architect (@@Lead) -> @@LaneD (vitest-gate + flake-fix MERGED)
MERGED: fc96280 (vitest in web-check) + b63403e (flake fix) on main (e2c9eb8 ->
abac76c). Confirmed full vitest now EXITS 0 (1596 pass) - gate is robust, finding
#3 closed. Clean root-cause (debounced drive-refresh rejection) + deterministic
repro - exactly right.

The 2 other fire-and-forget refreshDrive sites (App.svelte:330, SettingsPanel:437):
not suite-triggered (6x clean) + .svelte in @@LaneA/@@LaneE scope - DEFER into chunk
2's frontend pass (touches all web/src with runtime verification) rather than churn
those now. Tracked.
NEXT: desktop cut prep - scope (a) confirmed, secrets ready, RPM in. Plan + DRY-RUN
the sign/notarize + updater path (+ dev->prod bridge). Cut waits on A/B/C landing +
my re-gate.

## 2026-05-27 (round-2) @@Architect (@@Lead) -> @@LaneD (v0.16.0 clean-slate release prep)
@@Alex's call: cut v0.16.0 (MINOR, not a patch) - the drive->workspace clean break
(incompatible ~/.chan, renamed registry/routes/CLI) warrants it. And supersede ALL
prior versions so `chan upgrade` only offers 0.16.0+. PREP now (parallel with @@LaneB
chunk 3/2d); CUT after round close (chunk 3 + docs(phase-12) + first push).

1. VERSION BUMP 0.15.5 -> 0.16.0: workspace Cargo.toml + Cargo.lock + tauri.conf.json
   + any other pin. (Release commit at cut time, after chunk 3.)
2. CHANGELOG 0.16.0: the breaking workspace rename + clean break (no migration; users
   delete ~/.chan + re-register). @@Alex is sole user, starting fresh.
3. INVESTIGATE update.rs (CLI self-upgrade; ~35 github/releases/dl refs) + the Tauri
   updater (/dl/desktop/latest.json): what version source each reads. Propose the
   MECHANISM to make ONLY 0.16.0+ installable/upgradable. CONTEXT: `gh release list`
   is EMPTY - no published GH releases (your v0.15.5 workflow FAILED), so "delete
   previous versions" is likely mostly repointing the upgrade channel + the tag call,
   not deleting published releases.
4. GIT TAGS - @@Lead's rec: KEEP old tags (provenance; CHANGELOG/journals link them);
   delete only downloadable ASSETS + repoint the upgrade channel to 0.16.0. If @@Alex
   wants tags gone too, sequence the doc/CHANGELOG cleanup first. SURFACE the exact
   delete-mechanism + the keep-vs-delete-tags decision on event-lane-d-alex.md BEFORE
   any destructive `gh release delete` / tag delete.
5. DESKTOP: the dev->prod updater BRIDGE is now MOOT (@@Alex deletes ~/.chan + fresh-
   installs 0.16.0; no old-key install to update from). Simpler - note it.
Report the plan on event-lane-d-architect.md; destructive steps gated on @@Alex.

## 2026-05-27 (round-2) @@Architect (@@Lead) -> @@LaneD (v0.16.0 decisions RATIFIED by @@Alex)
@@Alex ratified - you are PRE-CLEARED on these, no need to re-surface:
- KEEP git tags (provenance); DELETE the published releases + their downloadable
  assets. @@Alex EXPLICITLY AUTHORIZES the destructive `gh release delete` (he's the
  sole user). Enumerate what actually exists first (gh release list looked empty -
  the v0.15.5 release workflow failed; there may be little/nothing to delete), then
  delete whatever <0.16.0 releases/assets exist, tags untouched.
- REPOINT the upgrade channel (update.rs + /dl) to 0.16.0 so only 0.16.0+ installs.
- Tauri self-upgrade: @@Alex starts FRESH on 0.16.0 today; DEFER self-upgrade testing
  to when v0.16.1 lands (test 0.16.0 -> 0.16.1 then). No bridge needed now.
Still: cut after round close (chunk 3 + docs(phase-12) + first push). Sequence: push
v0.16.0 -> release workflow publishes 0.16.0 + /dl latest -> delete any prior
releases. Verify deleting old release assets doesn't break a CITED download URL
(install.sh / README point at /dl/latest -> serves 0.16.0, fine; only old version-
pinned URLs die, accepted). Report what you delete on event-lane-d-architect.md for
the record.