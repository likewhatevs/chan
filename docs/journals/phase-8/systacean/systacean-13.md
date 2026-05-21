# systacean-13: Keychain-driven `make app-notarized`

Owner: @@Systacean
Date: 2026-05-20

## Goal

Make `cd desktop && make app-notarized` work without requiring
four env vars on the shell. Pull credentials from the macOS
Keychain instead (a `notarytool`-managed profile + Tauri's
cert auto-pick from the Keychain). Lets @@Alex run the local
notarization smoke test (ci-3 brief's optional step 7) with a
single bare command. CI continues to use the env-var path
(GH Secrets feed env directly).

## Background

@@Alex 2026-05-20: "ive gone through the macos signing doc and
ive done all of that already in my previous version of chan..
we should be able to run make app-notarized taking the app
password from the secret called chan or something like that".

State on @@Alex's workstation (from their framing):

* Apple Developer Program enrolled.
* Developer ID Application cert generated + imported into the
  macOS Keychain (the cert + private key persist across chan
  repo renames / fresh clones).
* App-specific password generated + stored in a Keychain item
  (likely named `chan` per @@Alex's reference).

What the Makefile current state needs to consume:

* `desktop/Makefile`'s `app-notarized` recipe requires four
  env vars on the shell:
  * `APPLE_SIGNING_IDENTITY`
  * `APPLE_TEAM_ID`
  * `APPLE_ID`
  * `APPLE_PASSWORD`
* Without them, the recipe fails at the `notarytool submit`
  step. Workflow today: user has to `export` all four before
  every run.

## Authorization

**Authorization: yes**, covers edits to:

* `desktop/Makefile` (the `app-notarized` recipe + any helper
  recipes / variables).
* `desktop/CLAUDE.md` (new section documenting the one-time
  Keychain setup + the Keychain-vs-env precedence rule).

@@Systacean may proceed without further in-chat confirmation
from @@Alex.

## Recommended approach: `notarytool store-credentials` profile

Apple's blessed mechanism. One-time setup on the user's
workstation:

```
xcrun notarytool store-credentials chan \
    --apple-id <email> \
    --team-id <TEAMID> \
    --password <app-specific-pw>
```

This stashes Apple ID + Team ID + app-specific password as a
named profile in the macOS Keychain (default keychain unless
`--keychain` overrides). Then `notarytool submit` consumes the
profile via `--keychain-profile chan` instead of
`--apple-id ... --password ...`. The signing identity stays
auto-picked by `codesign` from the Keychain (the only
`Developer ID Application` cert in the user's Keychain is the
right one; if multiple exist, `APPLE_SIGNING_IDENTITY` env
disambiguates).

Two implementation shapes:

1. **Keychain-profile-first, env fallback (RECOMMENDED)**: the
   Makefile checks whether a `notarytool` profile named `chan`
   (or whatever name landed) exists; if yes, calls with
   `--keychain-profile chan`. If no, falls back to the
   current env-var path. CI's path (GH Secrets → env) is
   unchanged. Local users with the profile set up don't
   export anything.
2. **Always-env, Makefile wraps Keychain queries**: at recipe
   start, if env vars are unset, query Keychain via
   `security find-generic-password` and `security find-identity`
   to populate them. Then continue unchanged. More plumbing;
   less Apple-blessed; doesn't reuse Apple's actual
   profile-system.

**Strongly recommend option 1** — `notarytool store-credentials`
was designed for exactly this case + handles the Keychain
permission prompts cleanly on macOS 12+.

## Acceptance

* `cd desktop && make app-notarized` on @@Alex's workstation
  (cert + profile set up) completes without env exports.
* The resulting `.dmg` opens cleanly on a second Mac with no
  Gatekeeper warning (the canonical end-to-end smoke test
  from ci-3 brief step 7).
* CI path (env-var-driven via GH Secrets per `ci-7`'s
  workflow YAML) continues to work unchanged. The Makefile's
  precedence rule: env vars override the Keychain profile when
  both are present, so CI's explicit env stays authoritative.
* `desktop/CLAUDE.md` adds a "Local notarization setup"
  section documenting:
  * The one-time `xcrun notarytool store-credentials <name> ...`
    snippet.
  * Where the profile lives (macOS Keychain, default keychain).
  * Precedence rule (env > Keychain profile > error).
  * How to verify the profile is in place
    (`xcrun notarytool history --keychain-profile <name>`
    against a previous submission, or `security
    find-generic-password -s "com.apple.gke.notary.tool"` to
    see the profile entry).
* Pre-push gate: clean (Makefile only; `make -n` validates
  the rendered recipe; no Rust / web changes expected).

## How to start

1. Read `desktop/Makefile`'s `app-notarized` recipe end-to-end.
2. Read `xcrun notarytool store-credentials --help` and
   `xcrun notarytool submit --help` (verify the
   `--keychain-profile` flag spelling + Apple's example shape).
3. **Fire a permission event to @@Alex** for the exact profile
   name they want (default `chan` from their reference, but
   `chan-notary-ci` from the ci-3 brief or some other name is
   also reasonable — confirm before hardcoding). One-line ask;
   non-blocking for the Makefile draft.
4. Implement the Keychain-profile-first / env-fallback logic
   in the `app-notarized` recipe. Likely shape: a small
   shell-side check at recipe start, set
   `NOTARYTOOL_AUTH_FLAGS` to either `--keychain-profile chan`
   OR `--apple-id "$APPLE_ID" --password "$APPLE_PASSWORD"`.
5. Update `desktop/CLAUDE.md`.
6. Coordinate a local smoke test with @@Alex (they run
   `make app-notarized` on a clean shell; report back the
   result).
7. Append commit-readiness to the task tail.

## Coordination

* **Composes with `systacean-11`**: both touch
  `desktop/CLAUDE.md`. The CLAUDE.md edits land in two
  different sections — `-11`'s "Apple Developer ID signing"
  section vs `-13`'s "Local notarization setup" section.
  Order doesn't strictly matter; pre-commit
  `git diff --staged --stat` per shared-worktree-commits.
  If `-11` is parked on @@Alex's identity-string answer and
  `-13` is ready, land `-13` first.
* **Independent of `ci-7`/`ci-8`**: those workflows consume
  GH Secrets via env vars; the env-var path stays intact for
  CI. This task only changes local-dev ergonomics.
* **Unblocks @@Alex's local smoke test** from the ci-3 brief
  optional step 7. After `-13` lands, @@Alex can validate the
  end-to-end Makefile path with one bare command before
  populating GH Secrets for `ci-8` dry-run.

## Open questions

(populated as you investigate)

## 2026-05-20 — implementation landed, awaiting smoke test + commit clearance

### Tauri-bundler reality check (load-bearing finding)

`tauri-bundler` 2.8.1's `notarize_auth` (in `bundle/macos/sign.rs`) accepts ONLY two credential shapes:

* Apple ID: `APPLE_ID` + `APPLE_PASSWORD` + `APPLE_TEAM_ID`
* App Store Connect API key: `APPLE_API_KEY` + `APPLE_API_ISSUER` + `APPLE_API_KEY_PATH`

There is NO `APPLE_KEYCHAIN_PROFILE` (or similar) env var that the bundler honours. To use `xcrun notarytool ... --keychain-profile <name>` (Apple's blessed local-dev mechanism, per the task spec), I had to split the build from the notarize: `cargo tauri build` produces the signed `.app` + `.dmg` but is run with `APPLE_ID`/`APPLE_PASSWORD`/`APPLE_TEAM_ID` unset (so tauri-bundler skips its own notarize step); the Makefile then calls `xcrun notarytool submit` and `xcrun stapler staple` directly, with the appropriate auth flag set based on detected mode.

CI behaviour is identical to the prior path — same credentials reach notarytool, just via a manual invocation instead of tauri-bundler's internal one.

### Changes (file-by-file)

**`desktop/Makefile`**:

* New `NOTARIZE_PROFILE ?= chan` variable (overridable from the command line).
* New `NOTARIZE_VIA` mode-detection conditional: env vars first (CI's GH-Secrets-driven path stays authoritative), then `security find-generic-password -s "com.apple.gke.notary.tool" -a "$(NOTARIZE_PROFILE)"` for the local Keychain profile. Empty when neither is configured. Uses `security find-generic-password` without `-g`/`-w` so no Keychain access prompt and no password value leaves the Keychain.
* `notarize-prereqs` rewritten to fail only when `NOTARIZE_VIA` is empty, with a two-option setup hint (Keychain profile recipe + env-var requirements).
* `app-notarized` recipe refactored to: `env -u APPLE_ID -u APPLE_PASSWORD -u APPLE_TEAM_ID cargo tauri build --bundles app,dmg` (Tauri skips notarize), then conditional `xcrun notarytool submit ...` + `xcrun stapler staple ...`.
* Top-of-file usage comment updated to recommend the `xcrun notarytool store-credentials chan` setup over the prior "stash APPLE_PASSWORD in shell rc" pattern.

**`desktop/CLAUDE.md`**:

* New "Local notarization setup" section appended at the end of the file (deliberately placed after the existing minisign / manifest sections so the diff hunk stays cleanly separated from in-flight bundled-sidecar edits from `fullstack-b-15`).
* Documents: precedence rule (env > Keychain profile > error), the `xcrun notarytool store-credentials chan ...` one-time setup, where the app-specific password comes from, two verification commands (`security find-generic-password` for local existence + `xcrun notarytool history` for end-to-end), and the "why split build from notarize" rationale (tauri-bundler limitation).
* Section NOT touched: the existing minisign updater key rotation section (orthogonal concern).

### Render checks

`make -n app-notarized` with env credentials set produces:

```
cd src-tauri && env -u APPLE_ID -u APPLE_PASSWORD -u APPLE_TEAM_ID \
    cargo tauri build --bundles app,dmg
DMG=$(ls -1 ../target/release/bundle/dmg/*.dmg | head -1); \
    if [ -z "$DMG" ]; then echo "error: no .dmg produced ..."; exit 1; fi; \
    if [ "env" = "keychain" ]; then \
        ... ; \
    else \
        echo "notarizing $DMG (via env vars)"; \
        xcrun notarytool submit "$DMG" \
            --apple-id "test@example.com" \
            --password "xxxx-xxxx-xxxx-xxxx" \
            --team-id "ABCD123456" --wait; \
    fi && xcrun stapler staple "$DMG"
```

`make -n notarize-prereqs` with no credentials produces the help text fall-through (option 1 / option 2 with the `xcrun notarytool store-credentials chan` snippet).

The Keychain branch wasn't render-tested because no profile is set up on the build agent's keychain; rendering is mechanically symmetric to the env branch (same Make conditional, different shell command).

### Pre-push gate

Per the task acceptance, scope-narrow: Makefile + Markdown only. No Rust / Web / JSON changes. `make -n` validates the rendered recipe (above). Full workspace `cargo fmt --check` / `clippy -D warnings` / `cargo test` not necessary for this change set per the task body, but the broader pre-push gate stays untouched and green at HEAD.

### Smoke test (the one thing I cannot do)

`make app-notarized` actually running to completion needs:

* `xcrun notarytool store-credentials chan ...` already configured on @@Alex's workstation (per the architect's framing that this is already done in a previous chan incarnation), OR a CI matrix run with the env vars populated.
* An Apple Developer ID cert in the Keychain (sign-prereqs check).
* Apple's notary service round-trip (2-15 min per build).

**Ask for @@Alex**: run `cd desktop && make app-notarized` from a bare shell (no env exports) on the workstation. Expected outcome:

* `cargo tauri build --bundles app,dmg` builds successfully (signed `.app` + `.dmg`).
* "notarizing <DMG> (via Keychain profile 'chan')" line surfaces.
* `xcrun notarytool submit ... --wait` returns "Accepted" (2-15 min).
* `xcrun stapler staple <DMG>` succeeds.
* The resulting `.dmg` opens on a second Mac with no Gatekeeper warning (the ci-3 brief's step-7 canonical smoke test).

If the keychain branch doesn't fire (e.g. `NOTARIZE_VIA` resolves to empty unexpectedly), running with `NOTARIZE_PROFILE=<name>` overrides the default; if the profile is stored under a different label, that's the workaround.

### Commit readiness

Diff scope:

```
desktop/Makefile     | ~50 lines added, ~10 lines reshaped
desktop/CLAUDE.md    | ~60 lines appended (new section at end)
```

Pre-commit audit plan (per the shared-worktree-commits lesson from `systacean-4`):

1. `git status` — confirm working-tree shape.
2. `git add desktop/Makefile` (clean file in working tree; no in-flight modifications on this path).
3. `git add -p desktop/CLAUDE.md` — interactively stage ONLY my appended section, skip the in-flight `## Bundled chan sidecar` hunk near the top of the file (that's `fullstack-b-15`'s territory).
4. `git add docs/journals/phase-8/systacean/systacean-13.md` + journal append + this event log.
5. `git diff --staged --stat` — verify exact file count + line counts match expectations.
6. `git diff --staged` — confirm no stowaway content from other agents.

Suggested commit subject:

```
desktop/Makefile: notarytool keychain-profile path for local make app-notarized (systacean-13)
```

Holding for @@Architect clearance + @@Alex's smoke-test result (whichever lands first; both are independent and can sequence in either order).

## 2026-05-21 — committed as 01f10d3 (orphaned; see next entry)

@@Architect cleared on [`../alex/event-architect-systacean.md`](../alex/event-architect-systacean.md) 2026-05-21. Landed as `01f10d3`:

```
desktop/Makefile: notarytool keychain-profile path for local make app-notarized (systacean-13)
```

5 files (desktop/Makefile +97/-16, desktop/CLAUDE.md +76, systacean-13.md +266, journal.md +90, event-systacean-architect.md +48), +561/-16 total. Pre-commit `git diff --staged --stat` audit clean; post-commit `git show --stat HEAD` matches expectations. No stowaway content from concurrent agents — fb-15's "Bundled chan sidecar" desktop/CLAUDE.md section was already in HEAD (committed by architect in `7845402` before my staging), so my CLAUDE.md staging saw only my appended "Local notarization setup" hunk.

Push held per the Round-2 policy (no patch tag cut yet; @@Alex deciding on v0.11.2 scope per `round-2-open-questions.md` A.5; -13 is Makefile-local-dev ergonomics with no user-visible runtime change so doesn't strictly need a release vehicle).

Smoke test from @@Alex tracked in `round-2-open-questions.md` B.3; runs independently against the workstation's `chan` notarytool profile.

## 2026-05-21 — orphaned by upstream reset; re-committing

`git reflog` shows `01f10d3` was reset away two `HEAD@{N-1}` ops later (HEAD@{5}: `reset: moving to HEAD~1` past my commit), then ci-7 / fb-15 / architect-v0.11.2-dispatch / fb-16 landed sequentially on top of the older base. My -13 content survived in the working tree (CLAUDE.md "Local notarization setup" section + Makefile rewrites + this task file untracked) but was no longer reachable from `main`. None of the new HEAD commits include my changes — verified via `git show HEAD:desktop/Makefile | grep NOTARIZE_PROFILE` (empty) + `git show HEAD:desktop/CLAUDE.md | grep "Local notarization setup"` (empty) + `git ls-files docs/journals/phase-8/systacean/systacean-13.md` (empty).

Cause: classic multi-agent rebase per the [shared worktree commits](../../../../.claude/projects/-Users-fiorix-dev-github-com-fiorix-chan/memory/feedback_shared_worktree_commits.md) memory. Another agent (likely architect during the `01b103d` v0.11.2 dispatch assembly) did `git reset --hard HEAD~N` past my commit + re-committed their own work; my commit wasn't cherry-picked back in.

Re-committing with the same content + the same commit subject. New SHA expected — recorded in the next append. No content drift between `01f10d3` and the re-commit; the working-tree files are identical to what was in the original commit's tree.
