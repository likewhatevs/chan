# ci-7: Tag-triggered signed + notarized chan-desktop workflow

Owner: @@CI
Date: 2026-05-20

## Goal

Wire up the workflow YAML in `.github/workflows/release-desktop.yml`
that consumes the six Apple signing/notarization secrets from the
`ci-3` brief and produces a notarized `.dmg` on every `chan-v*` tag
push. This is the Round-2 north-star deliverable; v0.12.0 ships on
top of this pipeline.

## Background

* **`ci-3` brief**: [`../../../release/macos-signing.md`](../../../release/macos-signing.md).
  Lists the six secret NAMES, the cert-provisioning checklist
  (@@Alex completes out-of-band), and the signing/notarization
  flow shape.
* **Round-2 plan north-star**: [`../architect/round-2-plan.md`](../architect/round-2-plan.md)
  §"North-star through-line". Decision 1 (sequencing) puts this at
  the head of Wave-1.
* **Decision 3 LOCKED**: bundled chan binary uses PATH-first w/
  bundled fallback + version match (per round-2-plan decisions
  table). This task is the build-side; @@FullStackB's
  `fullstack-b-15` + `-b-16` produce the bundle + launch logic
  this workflow needs to package.
* **Existing `release-desktop.yml`** (from `ci-2`, refined by `ci-4`
  + `ci-5` + `ci-6`) is the unsigned-matrix scaffold. This task
  extends it with the signing + notarization steps.
* **`-s-11` (this wave)** rotates the chan-desktop signing-key
  config from the DEV identity to the release identity per
  `desktop/CLAUDE.md`'s rotation section. ci-7's notarization step
  needs that rotation landed first or runs against stale config.

## Authorization

**Authorization: yes**, this task covers edits to
`.github/workflows/release-desktop.yml` + possibly
`.github/workflows/release.yml` (if shared steps refactor
naturally). The six secret NAMES per the ci-3 brief are
authorized to appear in the workflow YAML; secret VALUES NEVER
appear in journals / chat / commits (per the secrets-boundary
memory). @@CI may proceed without further in-chat confirmation
from @@Alex.

## Acceptance criteria

* Workflow consumes the six signing/notarization secrets by name
  per the ci-3 brief (codesign identity cert + cert password +
  Apple ID + app-specific password + team ID + keychain
  passphrase — exact names per the brief).
* On a `chan-v*` tag push: builds the chan-desktop bundle, signs
  with the Developer ID cert, notarizes via `notarytool`, staples
  the ticket, uploads to the GitHub Release as a `.dmg`.
* The bundled chan binary from `fullstack-b-15` is included in
  the app bundle's Resources (or wherever -b-15 lands it); both
  the chan-desktop binary AND the bundled chan binary are signed
  with the same identity so the notarization covers both.
* Failure modes: missing secret → workflow fails with a named
  message pointing at the ci-3 brief (don't leak the secret
  value); notarization rejection → captures the notarytool log
  and uploads it as a workflow artifact so the user can
  diagnose.
* Pre-push gate (YAML-only): clean.

## How to start

1. Re-read the ci-3 brief end-to-end. Confirm the six secret
   names + the cert-provisioning state.
2. Confirm with @@Alex via permission event whether the secrets
   are already populated in GitHub Actions Secrets, or whether
   that's pending. If pending, ci-7 lands the workflow YAML
   defensively (refuses cleanly when secrets are absent) and
   @@Alex populates afterward.
3. Audit the existing `release-desktop.yml` (post-ci-2 + ci-4 +
   ci-5 + ci-6) to see what's there and what needs adding. Don't
   tear down what works.
4. Reference Tauri's official signing docs + the existing
   chan-desktop `desktop/src-tauri/tauri.conf.json` signing
   block (rotated by `-s-11`).
5. Land as a single commit (workflow YAML changes are tightly
   coupled).

## Coordination

* **Depends on `systacean-11`**: signing-key rotation in
  `tauri.conf.json` must land before ci-7's actual signing step
  works against real Apple Developer ID keys. Sequencing within
  Wave-1: @@Systacean lands `-11` first; @@CI builds ci-7 in
  parallel against stub / placeholder config; both come together
  at the dry-run gate.
* **Followed by `ci-8`**: DMG-on-tag dry-run with real keys.
  ci-7 must be in HEAD + secrets populated before ci-8 can fire.
* **@@FullStackB's `-b-15`** (bundled chan binary) feeds the
  bundle this workflow ships. Coordinate on bundle layout / file
  paths once -b-15 settles.

## Next in your queue

* `ci-8` — DMG-on-tag dry-run with real keys (cut next).

## Open questions

(populated as you investigate)

## 2026-05-20 — Workflow YAML landed (ready for review)

Round-2 Wave-1 task picked up post-bootstrap; @@Alex's "poke"
was the trigger to look for new state and the ci-7 cut +
inbound poke were already there. Worked the YAML against the
ci-3 brief verbatim — recipe in the brief mapped to the
existing release-desktop.yml scaffold cleanly.

### What changed

Single file: `.github/workflows/release-desktop.yml`
(+194 / -36). Six structural additions, all gated on
`matrix.os == 'macos-latest'` so the Linux job stays
unsigned-as-before:

1. **Matrix field `artifact_suffix`** — `unsigned` for Linux,
   `signed` for macOS. Drives the per-OS artifact name so the
   release job's download pattern (`chan-desktop-macos-*-signed`)
   has a stable handle.
2. **Verify Apple signing secrets present** — fails fast with
   a `::error::` annotation naming missing secret names + a
   pointer to the ci-3 brief. Length-only check (`[ -n "$VAR" ]`)
   so no secret value enters script logic; GitHub also masks
   secret values in log output.
3. **Import Developer ID certificate** —
   `apple-actions/import-codesign-certs@v3` per the brief's
   recommendation. Handles temp keychain + unlock + cert import
   + partition-list ACL in one step.
4. **Build branch** — Linux runs `make build` (unsigned);
   macOS runs `make app-notarized` (sign + notarytool roundtrip
   + ticket staple) under the four APPLE_* env vars per the
   Makefile's `notarize-prereqs` contract. macOS build output
   is teed to `/tmp/chan-desktop-build.log` for diagnostic
   capture.
5. **Verify signature + stapled notarization** —
   `codesign -dv` + `spctl` + `stapler validate` on the .app
   AND .dmg. Catches silent Tauri-CLI failures where the build
   exits 0 but the bundle is unsigned / unstapled.
6. **Upload notarization diagnostics on failure** — `failure()`
   step uploads the build log + partial bundle dir + Apple
   transporter logs as a workflow artifact, 14-day retention.

Plus a new job:

7. **`release` job** — tag-only (`startsWith(github.ref,
   'refs/tags/chan-v')`), `permissions: contents: write`,
   downloads the signed macOS artifact, runs
   `softprops/action-gh-release@v2` with
   `fail_on_unmatched_files: true` so a missing .dmg surfaces
   as a workflow regression rather than silent emptiness.
   Pattern mirrors `release.yml`'s release job.

Header comment block was rewritten from "Round-2 follow-ups
deferred" framing to current shape (Linux + macOS build paths,
six secret names + brief reference, missing-secret behaviour,
out-of-scope items kept).

### How this satisfies acceptance

| Acceptance criterion                                              | Satisfied by                                                |
|-------------------------------------------------------------------|-------------------------------------------------------------|
| Consumes six secrets by name per ci-3 brief                       | env block on verify-secrets + sign-build steps              |
| `chan-v*` tag push → build/sign/notarize/staple/upload .dmg       | build job (macOS) + tag-gated release job                   |
| Bundled chan binary signed under same identity                    | Tauri auto-covers `Contents/MacOS/` per desktop/CLAUDE.md   |
| Missing secret → named-failure message pointing at ci-3 brief     | verify-secrets step's `::error::` block + exit 1            |
| Notarization rejection → notarytool log captured as artifact      | `failure()` upload step + tee'd build log                   |
| Pre-push gate (YAML-only): clean                                  | YAML parses (ruby `YAML.load_file`); no Rust / web touched  |

### Local validation

* YAML parses clean (validated via `ruby -ryaml -e
  "YAML.load_file('.github/workflows/release-desktop.yml')"`;
  Python's `pyyaml` not in path).
* Step-level audit: 10 actions used, all pinned to major
  version. New addition is
  `apple-actions/import-codesign-certs@v3`; brief-recommended.
* Diff scope: only `.github/workflows/release-desktop.yml`
  modified in my lane. `git status` shows other lanes
  actively editing chan-desktop runtime + bugs + systacean-11
  + rich-prompt events; explicit per-file `git add` will be
  used per the multi-agent worktree rule.
* Runtime dry-run NOT performed locally. `act` is not installed
  (per the ci-4/ci-5/ci-6 dry-run gap) and the workflow's
  load-bearing step (`apple-actions/import-codesign-certs`)
  needs real .p12 + password to exercise meaningfully. The
  full end-to-end exercise IS the next task (`ci-8` dry-run
  with real keys); ci-7 lands the YAML, ci-8 fires the tag.

### What I did NOT change

* `release.yml` (the chan CLI release flow) — untouched. Its
  `v*` tag trigger pattern is intentionally separate; this
  task only extends `release-desktop.yml`.
* `desktop/Makefile` — untouched. The CI workflow drives
  `make app-notarized` verbatim, reusing the existing local
  path. Any Makefile tweaks would slip out of scope.
* `desktop/src-tauri/tauri.conf.json` — untouched. Signing
  identity selection is env-driven (Tauri picks
  `APPLE_SIGNING_IDENTITY` from process env); no
  `signingIdentity` field needs to be set in the conf file.
* `desktop/CLAUDE.md` — newly amended by another lane (likely
  @@FullStackB's bundled-sidecar work). My YAML covers the
  "both binaries signed via Tauri's `Contents/MacOS/` scope"
  point that the amendment names. The amendment ALSO names
  universal2 lipo-merge as "owned by ci-7"; that's an
  out-of-spec scope expansion I'm flagging as Q3 below
  rather than silently absorbing.

### Commit readiness

Not committing per the standing rule. Proposed commit
message:

```
ci: tag-triggered signed + notarized chan-desktop release

Extend .github/workflows/release-desktop.yml with the Apple
Developer ID signing + notarytool path per the ci-3 brief
(docs/release/macos-signing.md). Six steps gated on
macOS-latest: verify the six signing secrets present
(fails fast with a named message before the runner burns
build minutes), import the Developer ID .p12 via
apple-actions/import-codesign-certs@v3, run
`make app-notarized` (sign + notarytool roundtrip + ticket
staple) under the four APPLE_* env vars, codesign + spctl +
stapler-validate the resulting .app + .dmg, and on failure
upload the notarytool log + bundle dir + Apple transporter
logs as a 14-day diagnostic artifact. Linux job stays
unsigned via `make build`. New tag-gated release job
downloads the signed macOS artifact and uploads the .dmg to
a GitHub Release via softprops/action-gh-release@v2 with
fail_on_unmatched_files set so a missing .dmg surfaces as a
workflow regression. Closes phase-8 ci-7; ci-8 fires the
real-keys dry-run once the six secrets are populated in
GitHub Actions Secrets.
```

### Open questions for @@Architect

1. **macOS universal2 scope (NEW)**. `desktop/CLAUDE.md` was
   amended by another lane mid-task and now states "A macOS
   universal2 fat binary ... is owned by `ci-7` in the
   GitHub Actions release workflow rather than in this
   Makefile (CI already runs per-arch matrix builds and is
   the natural place to `lipo`-merge before bundling)." My
   ci-7 task spec does NOT mention universal2, and the
   current matrix has ONE macOS entry (`macos-latest`,
   aarch64-only). Producing a universal2 DMG would require
   adding a second matrix entry for `x86_64-apple-darwin` +
   a `lipo -create` step that merges the two chan sidecars
   + invokes `tauri build` on the merged binary. That's a
   meaningful scope expansion (extra macOS runner minutes,
   non-trivial lipo plumbing). **Recommendation**: cut as a
   follow-up `ci-N` after ci-7 + ci-8 land green on aarch64.
   Confirm or redirect: should ci-7 absorb the universal2
   work, or stays a `ci-N+M` follow-up?

2. **Linux + Windows bundle GH Release upload sequencing**.
   The Round-2 close target ("all six platform installers
   ship via GitHub Release") will need Linux .AppImage /
   .deb / .rpm and eventually Windows MSI uploaded to the
   tag-cut release. Current shape: ci-7 only uploads the
   signed macOS .dmg. The Linux artifact stays workflow-only
   pending the Linux GPG-signing brief (out of scope per
   ci-3 brief). Windows isn't in the matrix yet at all.
   Cut as separate ci-N follow-ups once those signing lanes
   open? My read: yes, but flagging so the round-2-plan
   "Round-2 close" table can track them explicitly.

3. **`apple-actions/import-codesign-certs@v3` pin shape**.
   Pinned to major v3 (matches the brief's example). Going
   to full-SHA pin is the maximum-supply-chain-hardness
   option but adds a maintenance tax (every version bump
   requires a SHA lookup). Recommendation: stay on v3 for
   ci-7; revisit at Round-3's "code cleanup + hardening"
   pass per round-3-plan.md Track 3 if we want full-SHA
   pinning across all actions repo-wide as a single sweep.
