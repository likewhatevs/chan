# @@Systacean task 7: build Chan.app from this phase's HEAD and install in /Applications

Owner: @@Systacean
Status: BLOCKED — fires after [architect-3](./architect-3.md)
rewrites the chan-term commit messages and [architect-2](./architect-2.md)
lands the wave-1 + wave-2 commits. Alex confirmed sequencing
("ok to do this after the commit").

## Goal

Produce a `Chan.app` that contains all Phase 5 changes and install
it at `/Applications/Chan.app` so Alex can click through the new
behaviour while reading [summary.md](./summary.md). Pair with
[webtest-3](./webtest-3.md) which keeps a dev server up for
API-level pokes alongside the real app.

## Pre-requisites

* HEAD has the Phase 5 cleanup + enhancement + bug-fix + terminal
  persistence commits landed locally (via [architect-2](./architect-2.md)
  groupings). Push to origin is not required for this lane.
* Pre-push gate green on HEAD: `cargo fmt --check`,
  `cargo clippy --all-targets -- -D warnings`,
  `cargo build --no-default-features`, `cargo test`,
  `npm --prefix web run check`, `npm --prefix web test -- --run`,
  `npm --prefix web run build`. Architect can re-run the gate
  pre-flight; current round-6 baseline was green.

## Steps

1. **Repo-root release build.**

   ```
   make build-release
   ```

   This runs `make models` (pre-fetches the BGE-small embedding
   model bundle into `crates/chan-server/resources/`), `make web`
   (vite build → `web/dist/`), and `make build` (the release
   `chan` binary at `target/release/chan`). The frontend bundle
   is embedded by rust-embed at compile time.

2. **Stage the chan binary for Tauri.**

   The `desktop/Makefile` `chan-bin` step (a dependency of `build`)
   copies the release binary into `desktop/src-tauri/binaries/chan-<TARGET_TRIPLE>`
   under the architecture-qualified name Tauri's sidecar mechanism
   expects.

3. **Tauri release build (unsigned).**

   ```
   cd desktop && make build
   ```

   Output: `desktop/src-tauri/target/release/bundle/macos/Chan.app`.
   No code-signing or notarization is required for Alex's local
   install; we're not redistributing. The `app-signed` /
   `app-notarized` targets exist for the distribution case (require
   `APPLE_SIGNING_IDENTITY` + `APPLE_TEAM_ID` + `APPLE_ID` +
   `APPLE_PASSWORD`), but skip them here.

4. **Install into /Applications.**

   Replace any prior `/Applications/Chan.app` cleanly:

   ```
   rm -rf /Applications/Chan.app
   cp -R desktop/src-tauri/target/release/bundle/macos/Chan.app \
         /Applications/Chan.app
   xattr -dr com.apple.quarantine /Applications/Chan.app
   ```

   The `xattr` clear removes Gatekeeper's quarantine flag so
   unsigned builds open without the "downloaded from the internet"
   dialog on first launch. Necessary because this is an unsigned
   dev build for the maintainer's own use.

5. **Sanity launch.**

   `open -a /Applications/Chan.app`. Expect:
   * Drive Manager window shows the registered drives (including
     `chan-test-phase5` if it's still in the registry).
   * Opening a drive spawns a webview with the editor bundle baked
     in.
   * Terminal tabs persist across a window reload (the headline
     feature added by [systacean-5](./systacean-5.md) +
     [frontend-4](./frontend-4.md)).
   * MCP env discovery vars are visible inside a terminal tab
     (`env | grep '^CHAN_MCP_'`).

   Report the chan version stamp from `Chan.app` -> About (or from
   `Chan.app/Contents/Info.plist` if the About panel isn't wired).

## Acceptance criteria

* `Chan.app` is at `/Applications/Chan.app` with the Phase 5 HEAD
  baked into the embedded `chan` binary.
* Quarantine flag is cleared.
* The five sanity-launch checks above all PASS.
* Build artifacts elsewhere in the tree are not left in a state
  that confuses future `cargo build`s (no need to clean; just
  document the new bundle path).

## Reporting

* Note the build duration, the produced bundle path, and the
  installed bundle's `CFBundleShortVersionString` /
  `CFBundleVersion` from `Info.plist`.
* If any of the sanity-launch checks fail, leave the install in
  place but file the issue back to the relevant owner via a new
  task and link it here.

## Hardening expectations

* Confirm the embedded chan binary inside the .app is the release
  build we just produced (the Tauri sidecar resolves
  `chan-<target-triple>`; mis-staged binaries are silent failures).
* Confirm the embedded web bundle is the post-Phase-5 one (sanity:
  open a terminal tab in the running .app, the new
  "persistent terminal" close-confirm dialog + reattach behaviour
  should both work).

## Progress

(populated by @@Systacean once the commit gate opens)

## Completion notes

(populated by @@Systacean; include the build duration, bundle
path, version string, and the five-check report)
