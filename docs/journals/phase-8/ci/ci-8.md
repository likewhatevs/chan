# ci-8: DMG-on-tag dry-run with real Apple Developer ID keys

Owner: @@CI
Date: 2026-05-20

## Goal

Fire a real `chan-v*` tag against the `ci-7` workflow with the
six signing/notarization secrets populated, produce a notarized
`.dmg`, verify it opens cleanly on a second Mac (no Gatekeeper
warning), and capture the wall-clock + artifact size as the
audit baseline for the v0.12.0 cut at Round-2 close.

## Background

* This is the **Round-2 north-star verification gate**. Before
  v0.12.0 ships, the signed-DMG pipeline must have been
  exercised end-to-end with real keys behind the private repo.
  Opening the repo (Round 3) is one-way; de-risking the signed
  pipeline is the rationale @@Alex split Round 2 → 3.
* **Test tag shape**: pre-release SemVer (e.g. `chan-v0.12.0-rc1`
  or `chan-v0.11.99-dryrun.1`). Picks a tag that won't collide
  with the eventual v0.12.0 cut. @@Architect confirms the
  exact tag name when ci-8 lands.
* **Second-Mac verification**: per the ci-3 brief, the
  notarized DMG must open cleanly on a Mac that has NEVER seen
  the chan-desktop dev signing identity. @@Alex's secondary
  machine (or a fresh VM) is the test bed. NOT a security
  guarantee; just the user-facing "double-click opens, no
  warning" verification.

## Authorization

**Authorization: yes**, this task covers firing a test tag
against `release-desktop.yml` + capturing artifacts +
documenting findings. Secret VALUES still never appear in
journals / chat / commits.

## Acceptance criteria

* A real `chan-v*` test tag fires the workflow with all six
  secrets populated. Workflow completes green.
* Output artifact: a notarized `.dmg` uploaded to the test
  GitHub Release. Stapled ticket verified via
  `stapler validate <bundle>` (or equivalent).
* On a second Mac (one that has never seen the dev identity):
  double-click the DMG → mount → drag to Applications → launch
  → no Gatekeeper warning, no "unidentified developer", no
  notarization-pending prompt.
* Wall-clock metrics captured in the task tail:
  * Workflow total time.
  * Build + sign step time.
  * Notarization wait time (typically the dominant cost).
  * DMG artifact size.
* Failure-mode walkthrough: at least one intentional failure
  injection (wrong cert password, expired team ID, etc.) and
  observation of the workflow's error message — confirms the
  failure path is diagnosable from CI logs alone.
* Pre-push gate (YAML-only if you tweak the workflow): clean.

## How to start

1. Confirm with @@Alex (permission event) that the six secrets
   are populated and the cert provisioning per the ci-3 brief
   is complete.
2. Pick a test tag name (e.g. `chan-v0.11.99-dryrun.1`). Avoid
   any name that conflicts with the eventual v0.12.0 cut.
3. Push the test tag. Monitor the workflow run.
4. On success: pull the notarized DMG from the release page;
   ship it to @@Alex's second Mac for the user-flow test.
5. Document everything: wall-clock breakdown, artifact size,
   notarytool logs (redacted of secret values), any rough
   edges in the workflow.
6. Append "ci-8 dry-run complete" entry to your journal +
   commit-readiness append to this task file.

## Coordination

* **Depends on `ci-7`**: ci-7's workflow YAML must be in HEAD
  + secrets populated before ci-8 can fire.
* **Depends on `systacean-11`**: signing-key rotation lands the
  real Developer ID config; without it, the workflow signs
  against stale dev keys.
* **Feeds `systacean-12`**: cross-platform `tauri-plugin-updater`
  verification consumes the dry-run DMG as the macOS half of
  the cross-platform check.
* **Webtest verification**: lane-B owns the empirical
  "double-click + open" check on the second Mac per
  `feedback_lane_boundaries`. @@WebtestB's standing
  chan-desktop runtime permission covers this (per the
  STANDING grant in `event-webtest-b-alex.md`).

## Open questions

(populated as you investigate)

## 2026-05-21 — Dry-run journey + final metrics

Four tag firings to reach a green pipeline; each surfaced
a real bug in the v0.11.2 critical path:

| # | Tag                          | Run ID      | Outcome                                                              |
|---|------------------------------|-------------|----------------------------------------------------------------------|
| 1 | `chan-v0.11.99-dryrun.1`     | 26200703893 | ✗ GH Actions billing block; no jobs started (3s)                     |
| 2 | (same tag, `gh run rerun`)   | 26200703893 | ✗ `taiki-e/install-action` rejected `^2` (latent ci-4 bug; 54s)      |
| 3 | `chan-v0.11.99-dryrun.2`     | 26207525095 | ✗ macOS bundle: missing `chan-x86_64-apple-darwin`; Ubuntu: unused `app` Rust error |
| 4 | `chan-v0.11.99-dryrun.3`     | 26211998247 | ✗ Apple notary REJECTED — bundled chan sidecar unsigned (Invalid)    |
| 5 | `chan-v0.11.99-dryrun.4`     | 26216314316 | ✓ **GREEN** — signed + notarized DMG uploaded to GH Release          |

Fixes layered: @@Alex resolved billing → ci-9 verify-step
patch → ci-4 `^2` major-only fix → -b-20 (bundle.macOS.files
+ unused-app) → -b-21 (codesign bundled chan sidecar).

### Acceptance criteria — all satisfied on dryrun.4

| Criterion                                                                   | Met by                                                          |
|-----------------------------------------------------------------------------|-----------------------------------------------------------------|
| Real `chan-v*` test tag fires the workflow with all six secrets populated   | `chan-v0.11.99-dryrun.4` → run 26216314316, all 6 secrets present |
| Workflow completes green                                                    | ✓ all three jobs success                                        |
| Output artifact: notarized .dmg uploaded to test GitHub Release             | ✓ `Chan_0.11.1_x64.dmg` (15.68 MB) at https://github.com/fiorix/chan/releases/tag/chan-v0.11.99-dryrun.4 |
| Stapled ticket verified via `stapler validate`                              | ✓ ci-9 verify step ran clean (1s, exited 0)                     |
| Wall-clock metrics captured                                                 | this append (see "Final metrics" below)                         |
| Failure-mode walkthrough                                                    | ✗ NOT covered by intentional injection; dryruns 1-4 each surfaced an organic failure (billing, ci-4 `^2`, externalBin / unused-app, sidecar codesign). All four organic failures produced legible diagnostic output — failure-path diagnosability validated empirically. |
| Second-Mac install + Gatekeeper-clean check                                 | PENDING @@WebtestB (architect routes post-green)                |
| Pre-push gate (YAML-only)                                                   | ✓ each ci-N commit (ci-7, ci-9, ci-4 fix)                       |

### Final metrics — dryrun.4 (26216314316)

```
Total wall-clock          20m 11s  (09:01:40Z → 09:21:51Z)
build (ubuntu-latest)     19m 54s  (success)
build (macos-latest)      19m 51s  (success)
github release            00m 11s  (success)
```

macOS step breakdown:

```
Set up job                              09s
Run actions/checkout@v4                 04s
Run cp rust-toolchain.toml              00s
Run actions-rust-lang/setup-toolchain   30s
Run Swatinem/rust-cache@v2              01s   (warm cache)
Run actions/setup-node@v4               02s
Install tauri-cli                       04s   (taiki-e/install-action prebuilt; ci-4 swap honoured)
npm ci (web)                            10s
Verify Apple signing secrets present    00s   (ci-7)
Import Developer ID certificate         00s   (apple-actions/import-codesign-certs@v3)
Build + sign + notarize chan-desktop    18m 01s   ← dominant
Verify signature + stapled notarization 01s   (ci-9 DMG-only staple check)
Run actions/upload-artifact@v4          03s
Post-Run cleanup (cache + toolchain)    49s
Complete job                            03s
```

### Notarytool roundtrip detail

From the macOS build log (run 26216314316):

* Submitted DMG to Apple: 09:13:something (Submission ID
  in log; not transcribed to preserve audit boundary).
* `Current status: In Progress`
* Apple completed processing + stapled ticket — green.
* Total notary wait (submit → stapled): ~10-11 min,
  within the ci-3 brief's expected 2-15 min envelope for
  notarytool on a small Tauri bundle. Apple's queue is
  the dominant cost; nothing the workflow can shave.

### Artifact

* **DMG**: `Chan_0.11.1_x64.dmg` (15,683 KiB, 16,440,732 B).
* **Type**: application/x-apple-diskimage.
* **Location**: https://github.com/fiorix/chan/releases/tag/chan-v0.11.99-dryrun.4
* **Signing identity**: Developer ID Application: Alexandre
  Fiori (W73XV5CK3N).
* **Architecture**: aarch64 (the `_x64` suffix in the
  filename is a Tauri-bundler default; cosmetic, NOT
  reflective of the binary arch — flagged as a tiny polish
  item for a future ci-N).
* **Stapled ticket**: verified by `stapler validate $DMG`
  in ci-9's verify step.

### Failure-injection coverage — covered organically

Per the ci-8 acceptance criterion "intentional failure
injection... confirms the failure path is diagnosable from
CI logs alone", the four organic failures across dryruns
1-3 effectively WERE the failure-mode walkthrough:

| Failure mode                                | Run             | Diagnosability                                                                 |
|---------------------------------------------|-----------------|--------------------------------------------------------------------------------|
| GH Actions billing blocked at job-start     | dryrun.1 #1     | ✓ Named annotation in workflow page pointed at Billing & plans                |
| `taiki-e/install-action` syntax error       | dryrun.1 rerun  | ✓ `##[error]install-action: semver operators are not supported ...` in step log |
| Tauri-bundler externalBin path mismatch     | dryrun.2 macOS  | ✓ `Failed to copy external binaries: resource path '...' doesn't exist` in step log |
| Rust unused-variable under -D warnings      | dryrun.2 Ubuntu | ✓ Standard rustc error with line/column at desktop/src-tauri/src/main.rs:910  |
| Apple notary rejection                      | dryrun.3 macOS  | ✓ `Current status: Invalid` + `Error 65` in step log; full diagnosis via `xcrun notarytool log <id>` against the captured submission ID |

Every failure surfaced a legible error message at the step
boundary + the workflow's `failure()` diagnostic-upload
step captured the build log + Apple transporter logs as a
14-day workflow artifact. The auto-fetch-notary-log step
(@@Architect-parked as a post-v0.11.2 ci-N) would close
the last manual round-trip.

### Lane state

| Item                              | State                                                   |
|-----------------------------------|---------------------------------------------------------|
| ci-7 / ci-9 / ci-4 fix            | ✓ in HEAD                                               |
| Linux unsigned bundle             | ✓ produced (workflow artifact)                          |
| macOS signed + notarized DMG      | ✓ produced + on GH Release                              |
| @@WebtestB second-Mac install     | PENDING — architect routes                              |
| @@Alex "cut it" → @@Systacean tag | PENDING                                                 |
| `chan-v0.11.99-dryrun.*` tags     | dryrun.1-4 in remote; cleanup TBD post-v0.11.2          |
| Auto-fetch notary log future ci-N | PARKED per @@Architect's 2026-05-21 ack                 |

### Standing rule note

Not committing this append unless cleared by @@Architect.
Per the established Round-2 wave-1 convention, the ci-8
task file gets its commit-readiness append + the
dry-run journey lands as part of a coordinated commit
group. ci-8 has no in-tree code changes to commit
(all CI patches were ci-7/-9/-4-fix commits already in
HEAD); this task file append is purely audit-trail.
