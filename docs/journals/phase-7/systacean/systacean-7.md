# systacean-7: fix `make build` DMG bundling

Owner: @@Systacean
Cut by: @@Architect
Date: 2026-05-18

## Goal

Restore `make build` so it produces a complete Chan.app
DMG, not just the `.app` bundle. During the `v0.10.1`
closeout `make build` failed inside `bundle_dmg.sh`; we
worked around with `cargo tauri build --bundles app` to
ship the .app, which is fine for ad-hoc internal use but
the DMG path is the eventual user-facing distribution and
shouldn't stay broken.

## Relevant links

* Architect journal note on the workaround:
  [../architect/journal.md](../architect/journal.md) under
  "2026-05-18 17:00 BST — Round 1 SHIPPED".
* `desktop/` is the Tauri shell crate.
* The failing script is `bundle_dmg.sh` (vendored / Tauri
  helper).

## Acceptance criteria

* `make build` produces both `target/release/bundle/macos/Chan.app`
  AND `target/release/bundle/dmg/Chan_<version>_<arch>.dmg`.
* The DMG opens, shows the standard "drag Chan.app into
  Applications" layout, and the dragged app launches.
* The `Makefile` / `cargo tauri build` invocation doesn't
  silently swallow the DMG failure; if it can't build a
  DMG, `make build` exits non-zero with a clear message.
* No regression in the `--bundles app` path (the workaround
  we used).

## Out of scope

* Code-signing / notarization. Ad-hoc signing stays.
* Windows / Linux bundles.

## How to start

1. Run `make build` and capture the failure from
   `bundle_dmg.sh`. Common causes: missing
   `create-dmg`/`hdiutil` deps; volume name with a `:` in
   it; per-arch path mismatch since we're on Apple
   silicon; an outdated Tauri DMG template.
2. If the script is the vendored Tauri helper, check
   whether the local `tauri.conf.json` (under `desktop/`)
   has bundling config the helper expects.
3. If it's an `hdiutil` quirk on macOS 15+, the workaround
   is usually a `--no-internet-enable` flag or a different
   volume size estimate.
4. If the script needs to be reworked entirely, prefer
   keeping the Tauri-native bundling path over forking the
   shell script — less to maintain.

## Hand-off

Standard. Pre-push gate green. Ping via
`alex/event-systacean-architect.md` when DMG produces.

## @@Systacean update — 2026-05-18 18:34 BST

Diagnosis:

* `desktop/Makefile` exported `APPLE_SIGNING_IDENTITY` even
  when auto-detection found no value inside the sandboxed
  command environment.
* Tauri treats a present-but-empty `APPLE_SIGNING_IDENTITY`
  as an explicit codesign identity, invoked `codesign` with
  `""`, and failed before reaching the DMG bundler:
  `no identity found`.
* After preventing empty export, the remaining failed
  sandbox run left a complete temporary DMG volume mounted;
  outside the sandbox, `make -C desktop build` completed and
  produced both artifacts.

Fix:

* Only export `APPLE_SIGNING_IDENTITY` and `APPLE_TEAM_ID`
  from `desktop/Makefile` when they are non-empty.
* Signed/notarized targets still fail fast via
  `sign-prereqs` / `notarize-prereqs`.

Verification so far:

* `make -C desktop build` outside sandbox — passed
* produced `target/release/bundle/macos/Chan.app`
* produced `target/release/bundle/dmg/Chan_0.10.1_aarch64.dmg`
* `cargo tauri build --bundles app` — passed
* mounted DMG via `hdiutil attach -nobrowse` and verified:
  `Chan.app`, `Applications -> /Applications`, `.DS_Store`,
  `.VolumeIcon.icns`, and executable
  `Chan.app/Contents/MacOS/chan-desktop`

Signing note: Tauri signed the escalated build with the
available Developer ID identity and skipped notarization because
notary credentials were absent. `codesign --verify --deep --strict`
reported invalid signatures on the generated app/DMG, so signature
validity remains out of scope with notarization.

Gate:

* `scripts/pre-push` passed in a clean temporary worktree with
  the `desktop/Makefile` patch applied.
* The main workspace `scripts/pre-push` is blocked by unrelated
  dirty formatting in `desktop/src-tauri/src/serve.rs`.
