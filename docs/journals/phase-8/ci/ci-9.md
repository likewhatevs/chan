# ci-9: ci-7 verify-step patch (DMG-only staple per systacean-13's split flow)

Owner: @@CI
Date: 2026-05-21

## Goal

Patch `.github/workflows/release-desktop.yml`'s verify step
to match the canonical DMG-only-stapling Apple shape that
`systacean-13` adopted. The current verify step (committed
at `666c027`) runs `stapler validate "$APP"` which would
exit non-zero against the new flow's output (`.app` is
signed but not stapled; only the `$DMG` carries the notary
ticket).

Tiny patch (~5 lines diff). Lands before `ci-8` fires so
the dry-run doesn't burn macOS minutes hitting a known
verify-step failure.

## Background

* **ci-7 commit**: `666c027`. Workflow YAML for
  tag-triggered signed + notarized chan-desktop build.
* **systacean-13 commit**: `2fb3f12`. Split the
  `app-notarized` Makefile recipe — tauri-bundler 2.x
  doesn't consume notarytool Keychain profiles, so the
  new shape unsets `APPLE_ID` / `APPLE_PASSWORD` /
  `APPLE_TEAM_ID` during `cargo tauri build` (forces
  tauri-bundler to skip its own notarize step) and runs
  `xcrun notarytool submit` + `xcrun stapler staple`
  manually against the DMG ONLY.
* **Why DMG-only is canonical**: Apple's distribution
  shape for DMG installers is to staple the notary ticket
  to the DMG wrapper; the `.app` inside inherits trust via
  Gatekeeper's "verify-when-mounted" check. Stapling both
  is allowed but redundant.

## Authorization

**Authorization: yes**, covers
`.github/workflows/release-desktop.yml` only. @@CI may
proceed without further @@Alex confirmation.

## The patch

Replace the existing verify step:

```yaml
- name: Verify signature + stapled notarization
  run: |
    set -e
    APP="chan/target/release/bundle/macos/Chan.app"
    DMG=$(ls chan/target/release/bundle/dmg/*.dmg | head -1)
    codesign -dv --verbose=2 "$APP" 2>&1 | head -30
    spctl -a -t open --context context:primary-signature -v "$APP"
    stapler validate "$APP"     # <-- DROP: post-systacean-13 .app is unstapled
    stapler validate "$DMG"
```

With:

```yaml
- name: Verify signature + stapled notarization
  run: |
    set -e
    APP="chan/target/release/bundle/macos/Chan.app"
    DMG=$(ls chan/target/release/bundle/dmg/*.dmg | head -1)
    echo "=== codesign -dv on Chan.app (no staple required) ==="
    codesign -dv --verbose=2 "$APP" 2>&1 | head -30
    echo "=== stapler validate $DMG ==="
    stapler validate "$DMG"
    echo "=== spctl assessment on $DMG (install context) ==="
    spctl -a -t install -v "$DMG"
```

Changes (3 deltas):

* Drops `stapler validate "$APP"` — `.app` is unstapled
  per systacean-13's new flow; staple check would fail.
* Swaps `spctl -t open` on `.app` → `spctl -t install`
  on `$DMG` — `install` is the actual install-time
  Gatekeeper assessment users hit when they double-click
  the DMG.
* Keeps codesign metadata check on `.app` (works on
  signed-but-unstapled bundles; validates the signing
  identity matches).

Adds `echo` headers to the workflow log for diagnosability.

## Acceptance criteria

* Verify step in `release-desktop.yml` matches the diff
  above.
* YAML parses clean (ruby `YAML.load_file` or python's
  pyyaml).
* `actions/cache@v4` + `apple-actions/import-codesign-certs@v3`
  + other prior-pinned action references untouched.
* Pre-push gate (YAML-only): clean.

## How to start

1. Read the current verify step in
   `.github/workflows/release-desktop.yml` (post-ci-7
   commit `666c027`).
2. Apply the patch.
3. YAML-parse validation locally
   (`ruby -ryaml -e 'YAML.load_file(".github/workflows/release-desktop.yml")'`).
4. Append commit-readiness.

## Coordination

* **Blocks `ci-8`**: dry-run can't fire usefully until
  the verify step's known regression is fixed. ci-9 →
  ci-8 sequencing is the v0.11.2-tag-prep critical path.
* **Plan revision**: `commit-plan-v0.11.2.md` gets an
  architect-side append noting v0.11.2 will ship SIGNED
  (not unsigned as the original plan stated) — this is
  consequence of ci-7 + secrets being in HEAD before the
  tag cuts. Plan update is architect's; not your task.

## Suggested commit subject

```
ci: release-desktop verify step matches DMG-only staple flow (ci-9)
```

## Open questions

(populated as you investigate)
