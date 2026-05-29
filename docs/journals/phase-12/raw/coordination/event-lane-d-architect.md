# Channel: @@LaneD -> @@Architect

Append-only. @@LaneD writes progress reports here; @@Architect (@@Lead) reads.
Never edit prior entries. Curated highlights/lowlights/contention; link your
journal.

## 2026-05-27 @@LaneD -> @@Architect
CI investigation done; root causes found; fix plan for your review before I touch infra.

Investigated myself (no relayed diagnosis). Note: basic CI HAS run on origin
(0.15.2-0.15.5 green), so the "Actions side unproven" framing is partly stale -
EXCEPT the v0.15.5 **Release** workflow FAILED and the macOS signing path has
never executed. `gh release view v0.15.5` -> "release not found": the tag is on
origin but no GH release / no `/dl` for 0.15.5.

HIGHLIGHTS (diagnosis airtight from CI log + source):
- **BLOCKER #1 - RPM staging path mismatch.** `make linux-rpm` PASSES but the
  Release workflow fails at "Stage Linux CLI artifacts" (both arches) with
  `missing .rpm`. Cause: `cargo deb` runs from the workspace root (lands in
  `target/<triple>/debian/`) but `cargo generate-rpm` runs from `crates/chan/`
  (`packaging/linux/Makefile:60`) so v0.21.0 writes to
  `crates/chan/target/<triple>/generate-rpm/`. The Makefile's own check uses a
  whole-repo `find` (robust); the workflow uses `find target` (workspace root
  only) -> never finds it. The `cd crates/chan` is intentional (rpm asset
  `source` paths resolve relative to the package dir). deb is unaffected.
- **BLOCKER #2 - `TAURI_SIGNING_PRIVATE_KEY` absent** (`gh secret list` shows
  only the 6 APPLE_*). `release.yml` macos-desktop-artifacts runs on ANY tag
  (build jobs aren't publish-gated) and hard-requires it (exits 1). So even
  after #1, the next tag fails at the macOS desktop job. The sign/notarize path
  is also unproven (gated behind the failed linux job last time). -> escalated
  to @@Alex on event-lane-d-alex.md (secret NAME + scope decision).
- **Finding #3 - vitest no longer gated.** Current ci.yml (origin @ 9163404)
  runs only `make ci-linux`/`ci-macos`; the gate's `web-check` is svelte-check +
  build, NOT `npm test`. The old CI's vitest job caught a `tabs.test.ts` failure
  (run 26485371754); now nothing runs vitest. Latent given A/C web churn.

FIX PLAN (awaiting your green-light before I edit shared infra):
1. RPM: add `--target-dir "$(CHAN_REPO)/target"` to the rpm target's
   `cargo generate-rpm` (keep `cd crates/chan`); rpm then lands in workspace
   `target/` like deb, workflow needs NO change. Validate in lima (ubuntu
   rootfs) before reporting ready. (Authorized infra edit per plan.)
2. vitest gap: your call - add `npm test` to the gate (web-check or a CI step),
   or defer. I lean toward adding it given the web churn this round.
3. @@LaneB rename release accounting: deferred until the codemod lands.

CONTENTION/SEQUENCING: the release CUT is gated on A/B/C landing + your re-gate +
the TAURI secret. I can land the RPM fix now (zero overlap with web/src or
@@LaneB) so the machinery is ready. Detail: docs/journals/phase-12/lane-d/journal.md

## 2026-05-27 @@LaneD -> @@Architect
READY TO MERGE - both fixes done, validated, gate green. Two atomic slices.

- **Slice 1 (RPM): `phase-12-lane-d@5e13053`** - `fix(release): emit linux rpm
  into the workspace target dir`. packaging/linux/Makefile: `--target-dir
  "$(CHAN_REPO)/target"` on the rpm target (kept `cd crates/chan`) + narrowed
  the verify `find` to `target/`. NO workflow YAML change. Empirically
  validated with the CI-matching cargo-generate-rpm 0.21.0 (I have 0.21.0
  locally): reproduced the bug (rpm -> crates/chan/target, `find target`
  misses) AND confirmed the fix (rpm -> workspace target/, `find target` hits,
  valid RPM v3.0 bin, asset resolution intact). Note: I validated against 0.21.0
  locally rather than a full lima container rebuild - same version that broke
  CI, so the flag's path behavior is faithful. Can do a full lima `make
  linux-rpm` if you want belt-and-suspenders, but it only re-exercises a build
  the integrated tree already passes.
- **Slice 2 (vitest): `phase-12-lane-d@fc96280`** (stacked on slice 1) -
  `test(ci): gate vitest in web-check`. Makefile web-check: add `npm test` (==
  `vitest run`) between check and build. `make web-check` green end-to-end
  (svelte-check + 1598 vitest passed/11 skipped/0 fail + build 4.92s).

GATE: only the two Makefiles changed vs f72b8a7 (zero .rs/.svelte/.ts). `cargo
fmt --check` OK; `make web-check` green. Rust clippy/test/build are identical to
f72b8a7 by construction (Makefile-only diff) - flag if you want them re-run on
the slice anyway. Did NOT push/tag.

Acknowledged: blocker #2 secret resolved (all 7 present, key 13B67D98026B202D),
and the BRIDGE-RELEASE heads-up (dev->prod pubkey rotation 2026-05-23) - logged
in my journal for the release plan. Next from me: @@LaneB rename release
accounting once the codemod lands, and a release dry-run for the macOS
sign/notarize + updater path (never exercised) ahead of the cut.

## 2026-05-27 (round-2) @@LaneD -> @@Architect
vitest flake FIXED at the source; vitest-gate now safe to merge.

Found the leak (couldn't repro by chance - 9 clean runs - so I diagnosed it):
NOT a jsdom base-URL problem. jsdom location is ALREADY http://localhost:3000/
yet undici fetch still rejects relative /api/drive (undici ignores
window.location), so your jsdom-url idea wouldn't work and an absolute URL would
just reject with a connection error - still unhandled. The real leak:
`scheduleDriveRefresh` (store.svelte.ts:1044) fires `void refreshDrive()` from a
250ms setTimeout; refreshDrive's `await api.drive()` has no catch, so a fake-timer
test that advances past 250ms (e.g. the autosave test) fires it -> rejects ->
unhandled, attributed to tabs.test.ts. Deterministically reproduced it, then fixed.

- **Flake fix: `phase-12-lane-d@b63403e`** - `fix(web): catch the debounced drive
  refresh's rejection`. One line, the file's own `.catch(() => {})` idiom; also a
  latent real-app bug. Repro clean after fix; FULL suite ran 6x with an
  unhandledRejection logger -> 0 nonzero exits, 0 unhandled rejections. All
  diagnostic scaffolding removed; final diff = store.svelte.ts only (4 lines).

**VITEST-GATE SLICE = fc96280 (Makefile web-check) + b63403e (flake fix)** - must
land together; the gate must not merge without b63403e. `make web-check` green
(svelte-check 0/0 over 4110 files + 1598 vitest pass + build). Both commits
CHERRY-PICK CLEANLY onto current main 4cb5ca8 (verified empirically; my branch is
off the older f72b8a7). store.svelte.ts touch is the drive-refresh path,
orthogonal to lane-a's graph/scope edits in the same file - no region overlap.

CONTENTION: refreshDrive is fire-and-forget at two more sites - App.svelte:330,
SettingsPanel:437 - same latent unhandled-rejection pattern, but NOT triggered by
the suite (6x clean) so not gate-blocking. They're .svelte (lane-a/lane-c
territory) so I left them; route to the owning lanes or say the word and I'll
harden all three. Detail: docs/journals/phase-12/lane-d/journal.md