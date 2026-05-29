# Lane D journal: CI + release (phase 12)

@@LaneD = the CI + release lane, added mid-phase 2026-05-27. Append-only.
Worktree `../chan-lane-d` on `phase-12-lane-d` (from main @ `f72b8a7`). Channels
+ this journal edited in the MAIN checkout by absolute path.

---

## 2026-05-27 - kickoff + CI investigation

Identified as @@LaneD. Read `lane-d-plan.md`, `bootstrap.md`,
`coordination/README.md`, and the @@Architect kickoff on
`event-architect-lane-d.md`. Created worktree `../chan-lane-d` on
`phase-12-lane-d` from main `f72b8a7` (main has moved past the plan's stated
baseline `fe6e126`: phase-12 lanes A, B-chunk0, C are already merged).

Mission per plan: (1) NOW - investigate + fix current CI; (2) ALIGN the next
patch release on `0.15.5`, cut AFTER lanes A/B/C land, accounting for @@LaneB's
chan-drive -> chan-workspace rename.

### What I checked (surfaces, not a relayed diagnosis)

- `gh run list` - CI HAS run before (0.15.2-0.15.5), contradicting the plan's
  "GitHub Actions side is unproven" framing for the basic CI path. BUT the
  v0.15.5 **Release** workflow FAILED (run `26502279864`).
- `gh run view 26502279864` - failure isolated to `linux CLI packages` on BOTH
  arches (aarch64 job `78046765499`, x86_64 job `78046765531`), at the
  **"Stage Linux CLI artifacts"** step. The `make linux-deb`/`linux-rpm` steps
  PASSED; staging failed.
- `gh run view --job 78046765531 --log` - exact error:
  `##[error]missing .rpm for x86_64-unknown-linux-gnu`. The log shows the rpm
  WAS built, at
  `.../chan/crates/chan/target/x86_64-unknown-linux-gnu/generate-rpm/chan-0.15.5-1.x86_64.rpm`
  (a CRATE-local target dir).
- `gh release view v0.15.5` -> "release not found". The RPM staging failure
  aborted the publish chain: macOS jobs never ran, no GitHub release, no `/dl`
  metadata deployed for 0.15.5. The tag is on origin but "empty".
- `gh secret list` - 6 `APPLE_*` secrets present; **`TAURI_SIGNING_PRIVATE_KEY`
  is ABSENT**. Only the `github-pages` environment exists (no env-scoped TAURI
  secret).
- Read all four workflows (ci, pages, release, release-desktop) + both
  Makefiles. The current `ci.yml` (origin @ `9163404`) runs only
  `make ci-linux` + `make ci-macos` - no vitest job.

### Root cause #1 (BLOCKER): RPM staging path mismatch

`packaging/linux/Makefile` runs the two packagers from different CWDs:

- `deb` (line 48): `cd "$(CHAN_REPO)" && cargo deb -p chan --target <triple>`
  -> cargo-deb is workspace-aware, writes to `target/<triple>/debian/`.
- `rpm` (line 60): `cd "$(CHAN_REPO)/crates/chan" && cargo generate-rpm
  --target <triple>` -> cargo-generate-rpm 0.21.0 (CI) resolves its target dir
  relative to CWD, so output lands in `crates/chan/target/<triple>/generate-rpm/`.

The Makefile cd's into `crates/chan/` on purpose: the
`[package.metadata.generate-rpm]` asset `source` paths
(`crates/chan/Cargo.toml:107-119`) are resolved relative to the PACKAGE dir
(`../../target/release/chan`), unlike cargo-deb. Comment at Cargo.toml:110-114
documents this.

The Makefile's own verification (`find "$(CHAN_REPO)" ...`, line 61) searches
the WHOLE repo, so `make linux-rpm` PASSES wherever the rpm lands. But the
workflow's "Stage Linux CLI artifacts" step (`release.yml:184`):
`rpm=$(find target -path '*/generate-rpm/*.rpm' ...)` searches only the
workspace-root `target/`, never `crates/chan/target/`. => `missing .rpm`.

deb is unaffected (lands in workspace `target/`). Diagnosis is airtight from
CI log + source; will VALIDATE the fix empirically in lima (ubuntu rootfs)
before executing.

### Root cause #2 (BLOCKER, needs @@Alex): TAURI_SIGNING_PRIVATE_KEY absent

`release.yml` `macos-desktop-artifacts` runs on ANY tag push (and on dry-run
dispatch - the build jobs are not gated on publish; only publish-release/pages
are). Its "Verify signing secrets present" step (lines 360-383) HARD-REQUIRES
`TAURI_SIGNING_PRIVATE_KEY` and exits 1 if absent. It is not in `gh secret
list`. So even after the RPM fix, the next tag's Release workflow fails at the
macOS desktop job. Also: the macOS sign+notarize+staple path (lines 385-455)
has NEVER executed (it was gated behind the failed linux job in the v0.15.5
attempt) - it is unproven. Requires @@Alex to provision the secret (NAME only)
and a scope decision on whether signed desktop + updater payload ship in this
patch. -> escalated on `event-lane-d-alex.md`.

### Finding #3 (coverage gap, not a current red): vitest not gated

The OLD ci.yml (pre-`9163404`) ran a vitest "web tests" job; run `26485371754`
failed there on `web/src/state/tabs.test.ts` (`TypeError: Invalid URL
'/api/drive'`). The current `make ci-*` gate runs `web-check` (svelte-check +
build) but NOT `npm test`/vitest. So vitest is no longer gated by CI OR the
local pre-push gate. Latent risk given lanes A/C churned web/src heavily.
Flagging to @@Lead; treating as a separate decision, not part of the release
unblock.

### Fix plan (proposed; @@Lead reviews before I touch shared infra)

1. RPM path: in `packaging/linux/Makefile` rpm target, force deterministic
   output into the workspace target via `--target-dir "$(CHAN_REPO)/target"`
   (or `-o`), keeping `cd crates/chan` so asset resolution is untouched. Then
   the workflow `find target` finds it with NO workflow change. Validate in
   lima. (Authorized infra edit per plan.)
2. TAURI secret: escalate to @@Alex (provision NAME + scope decision).
3. vitest gap: flag to @@Lead as a separate decision.
4. @@LaneB rename accounting: deferred until @@LaneB's codemod lands (Cargo
   package names, Cargo.lock, artifact/install naming, CHANGELOG breaking note).

Posting findings + this plan on `event-lane-d-architect.md`; secret gate on
`event-lane-d-alex.md`. Will not execute infra fixes until @@Lead green-lights.

---

## 2026-05-27 - executed both green-lit fixes (slices 1 + 2)

@@Lead green-lit both (event-architect-lane-d.md): RPM `--target-dir` fix +
adding vitest to the gate. @@Alex provisioned `TAURI_SIGNING_PRIVATE_KEY` (all 7
secrets now present; key matches prod updater pubkey 13B67D98026B202D, the
2026-05-23 rotation) so blocker #2 is resolved for the workflow gate. @@Lead
flagged a BRIDGE-RELEASE consideration: the updater pubkey rotated dev->prod on
2026-05-23, so installs trusting the OLD dev pubkey (7605FF...) need a one-time
bridge release - factor into the cut + dry-run, surface to @@Alex if a real
bridge is needed. Logged here for the release plan.

### Empirical validation (RPM), local, CI-matching cargo-generate-rpm 0.21.0

Local macOS cargo-generate-rpm is 0.21.0 == the version CI used, so the flag's
path behavior is faithful without a full container rebuild.
- REPRO bug: `cd crates/chan && cargo generate-rpm --target X` -> rpm at
  `crates/chan/target/X/generate-rpm/chan-0.15.5-1.x86_64.rpm`; workflow's
  `find target` (workspace root) MISSES it. Matches the CI log exactly.
- FIX: add `--target-dir "$REPO/target"` -> rpm at
  `target/X/generate-rpm/...`; `find target` FINDS it; output is valid
  `RPM v3.0 bin` (asset resolution intact). `--auto-req no` was used ONLY to
  stub the test binary; the real Makefile change adds only `--target-dir`.

### Slice 1 - phase-12-lane-d@5e13053
`fix(release): emit linux rpm into the workspace target dir`
packaging/linux/Makefile: add `--target-dir "$(CHAN_REPO)/target"` to the rpm
target (keep `cd crates/chan`); narrow the verify `find` to `$(CHAN_REPO)/target`
so a future regression fails locally not just in CI. No workflow YAML change.

### Slice 2 - phase-12-lane-d@fc96280
`test(ci): gate vitest in web-check (pre-push / ci-linux)`
Makefile web-check: add `npm test` (== `vitest run`) between check and build.
Verified green on the integrated tree: 1598 passed / 11 skipped / 0 fail.

### Gate

Only the two Makefiles changed vs baseline f72b8a7 (zero .rs/.svelte/.ts):
- `cargo fmt --check`: OK.
- `make web-check` (svelte-check + vitest + build): green (build 4.92s; warnings
  are pre-existing chunk-size/dynamic-import notices).
- Rust clippy/test/build --no-default-features: identical to f72b8a7 by
  construction (Makefile-only diff); f72b8a7 is already merged with green CI.

Reported both slices ready on event-lane-d-architect.md. Did NOT push/tag (cut
gated on A/B/C + @@Lead re-gate + @@Alex scope answer). Next: @@LaneB rename
release accounting once the codemod lands, + a release dry-run for the macOS
sign/notarize + updater path (still never exercised) ahead of the cut.

---

## 2026-05-27 - vitest-gate flake fix (round-2; @@Lead held fc96280)

@@Lead merged the RPM fix (5e13053 -> main merge 7e684e1, blocker #1 cleared) but
HELD the vitest gate fc96280: the full suite exits 1 on a flaky UNHANDLED
REJECTION "Failed to parse URL from /api/drive" (pre-existing, clean in isolation,
unchanged since fe6e126 - the old CI caught it as run 26485371754). Wiring vitest
into the gate while that flake exists would flake pre-push + ci-linux. @@Lead
suggested a jsdom base URL or await/catch the fetch; my call on the exact fix.

### Investigation (I could NOT reproduce by chance - 9 full runs, exit 0)

- Probed the jsdom env: `window.location` is ALREADY `http://localhost:3000/`,
  yet node's undici `fetch("/api/drive")` STILL rejects ERR_INVALID_URL. So
  @@Lead's jsdom-base-URL idea would NOT work (undici ignores window.location);
  an absolute URL would just reject with a connection error instead - still
  unhandled. Ruled out the config-only base-URL fix with evidence.
- A never-settling global fetch stub killed the flake but TIMED OUT 3 tabs.test.ts
  restore tests - they AWAIT a fetch and handle its rejection, so they need fetch
  to reject fast. That proved the 1598 pass WITH undici rejecting; the only
  problem is ONE fire-and-forget call whose rejection is UNHANDLED. So the fix is
  to catch that leak, not to stub fetch globally. Reverted the stub.
- Found it by code inspection (error is specifically /api/drive == api.drive()):
  `scheduleDriveRefresh` (store.svelte.ts:1044) fires `void refreshDrive()` from a
  250ms setTimeout; `refreshDrive` does `await api.drive()` with no catch. Tests
  using fake timers + advanceTimersByTimeAsync past 250ms (e.g. the autosave test)
  fire it -> reject -> unhandled. Attributed to tabs.test.ts for that reason.
- DETERMINISTIC repro: a temp test calling scheduleDriveRefresh + advancing
  timers, with a process unhandledRejection logger, captured exactly "Failed to
  parse URL from /api/drive". Confirmed the source.

### Fix - phase-12-lane-d@b63403e (stacked on fc96280)

`fix(web): catch the debounced drive refresh's rejection`. One line in
store.svelte.ts: `void refreshDrive();` -> `refreshDrive().catch(() => {});`
(the file's existing fire-and-forget idiom). Also a latent real-app bug, not just
a test artifact. After the fix: repro clean, and the FULL suite ran 6x with the
unhandledRejection logger -> 0 nonzero exits, 0 unhandled rejections. Removed all
diagnostic scaffolding (probe/repro tests, temp setup file, vite.config setupFiles
reverted). FINAL diff for this fix = store.svelte.ts only (4 +/- lines).

NOTE: refreshDrive is fire-and-forget at two more sites (App.svelte:330,
SettingsPanel:437) - same latent pattern, NOT triggered by the suite (6x clean),
so not gate-blocking. Flagged to @@Lead for the owning lanes (.svelte files =
@@LaneA/@@LaneC territory; I stayed out to respect boundaries).

### Gate + merge-readiness

- `make web-check` (svelte-check 0 err/0 warn over 4110 files + 1598 vitest pass +
  build): green WITH the fix.
- The vitest-gate slice is now safe = fc96280 (Makefile) + b63403e (flake fix);
  the gate must not land without b63403e.
- Verified BOTH commits cherry-pick CLEANLY onto current main (4cb5ca8) - my
  branch is based on the older f72b8a7. store.svelte.ts edit is the drive-refresh
  path, orthogonal to lane-a's graph/scope work in the same file (no region
  overlap; clean cherry-pick proves it).
- The lone `git stash` entry in the worktree is webtest-a's, not mine (left
  untouched).

Re-reported the vitest-gate slice ready on event-lane-d-architect.md.
