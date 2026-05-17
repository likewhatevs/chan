# @@Systacean task 7: build Chan.app from this phase's HEAD and install in /Applications

Owner: @@Systacean
Status: REVIEW

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

* 2026-05-17 @@Systacean: picked up after the wrap chain landed the
  phase commits locally. Built the desktop bundle, installed it
  into `/Applications/Chan.app`, cleared quarantine, and performed
  shell-verifiable launch/package checks.

## Completion notes

* Build command: `make build` from `desktop/`. It rebuilt the web
  bundle, rebuilt the release `chan` binary, staged
  `desktop/src-tauri/binaries/chan-aarch64-apple-darwin`, ran
  `cargo tauri build`, and produced both:
  * `/Users/fiorix/dev/github.com/fiorix/chan/target/release/bundle/macos/Chan.app`
  * `/Users/fiorix/dev/github.com/fiorix/chan/target/release/bundle/dmg/Chan_0.8.1_aarch64.dmg`
* Build duration observed in the terminal: about 3 minutes end to
  end. Tauri signed the generated bundle with the local Developer
  ID identity but `codesign --verify --deep --strict` reported the
  generated signature invalid, so the installed app was re-signed
  ad hoc for this local maintainer install.
* Installed path: `/Applications/Chan.app` (98M). Quarantine is
  absent (`com.apple.quarantine` not present). Signature check:
  `codesign --verify --deep --strict --verbose=2
  /Applications/Chan.app` PASS after ad-hoc re-sign.
* Version metadata from `Info.plist`:
  `CFBundleShortVersionString=0.8.1`, `CFBundleVersion=0.8.1`,
  `CFBundleIdentifier=com.chanwriter.desktop`,
  `CFBundleExecutable=chan-desktop`.
* Embedded sidecar check: `/Applications/Chan.app/Contents/MacOS/chan
  --version` prints `chan 0.8.1`.
* Sanity launch: `open -a /Applications/Chan.app` launched process
  PID 28637 as `/Applications/Chan.app/Contents/MacOS/chan-desktop`;
  macOS Accessibility reports the visible window name
  `Chan Desktop`.
* GUI click-through items that require user interaction remain for
  Alex's manual pass: selecting/opening a registered drive, creating
  a terminal tab inside the webview, reloading that drive window to
  confirm terminal persistence, and checking `env | grep
  '^CHAN_MCP_'` in the app terminal. The installed app is in place
  for those checks.

## Superseded by 0.9.0 release

The notes above describe an interim 0.8.1 build that ad-hoc
re-signed because the local Developer ID signing failed during
that build attempt. That bundle was overwritten when Alex called
the "wrap" + the 0.9.0 release went through.

**What actually shipped:**

* Version bump committed at `97ad644 chore: bump version to 0.9.0`
  (Cargo workspace + `tauri.conf.json`).
* `make build-release` then `cd desktop && make app-notarized`
  produced:
  * `target/release/bundle/macos/Chan.app` — Developer ID signed
    (Alexandre Fiori, `W73XV5CK3N`), hardened runtime, notary
    ticket stapled. `xcrun stapler validate` PASS. `spctl -a -t
    open --context context:primary-signature -v` reports
    `accepted, source=Notarized Developer ID`.
  * `target/release/bundle/dmg/Chan_0.9.0_aarch64.dmg` — Developer
    ID signed, separately submitted to Apple's notary, ticket
    stapled. `xcrun stapler validate` PASS.
* Installed at `/Applications/Chan.app`, quarantine cleared.
  `CFBundleShortVersionString=0.9.0`, `CFBundleVersion=0.9.0`,
  `Contents/MacOS/chan --version` reports `chan 0.9.0`.
* Tag `v0.9.0` (annotated) created and pushed to origin alongside
  the phase-5 commits.

The DMG at `target/release/bundle/dmg/Chan_0.9.0_aarch64.dmg` is
the canonical distribution artifact for this release.
