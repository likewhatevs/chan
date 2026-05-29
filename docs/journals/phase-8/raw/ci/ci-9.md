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

## 2026-05-21 — Patch landed (committing per pre-cleared authorization)

Architect's "Proceed without further architect ack — both
items unblocked" in
[`../alex/event-architect-ci.md`](../alex/event-architect-ci.md)
2026-05-21 "Items 2 + 3 routing" poke pre-cleared this
commit. Skipping the standard "readiness → wait for
clearance" loop; committing directly.

### Patch applied verbatim

Single file: `.github/workflows/release-desktop.yml`
(+13 / -7 net on the verify-step block, plus comment
expansion explaining the systacean-13 architectural shift):

* Drop `stapler validate "$APP"` (post-systacean-13 .app
  is unstapled).
* Swap `spctl -a -t open --context context:primary-signature
  -v "$APP"` → `spctl -a -t install -v "$DMG"` (install-time
  Gatekeeper check on the DMG = actual user flow).
* Keep `codesign -dv --verbose=2 "$APP"` (no staple needed
  to validate signing identity).
* Keep `stapler validate "$DMG"` (DMG is the stapled
  artifact in the new flow).

Comment header rewritten to name systacean-13 explicitly +
explain WHY DMG-only staple is canonical for DMG
distribution + why `stapler validate Chan.app` would fail.

### Validation

* YAML parses clean: `ruby -ryaml -e
  "YAML.load_file('.github/workflows/release-desktop.yml')"`
  → `YAML parses clean`.
* `git diff` confirmed scope: single file, verify-step
  block + its comment header only. No other YAML touched.

### Commit shape

Per the bootstrap-rule lesson learned on ci-7 (see
[`feedback-shared-worktree-commits`](file://~/.claude/projects/-Users-fiorix-dev-github-com-fiorix-chan/memory/feedback_shared_worktree_commits.md)
"Race-safe primitive"), using `git commit -m "..." --
<pathspec>` to commit only the named paths regardless of
what's staged. Pathspec: workflow YAML + this ci-9.md
task file.

Commit subject per architect's task spec (line 124):
```
ci: release-desktop verify step matches DMG-only staple flow (ci-9)
```
