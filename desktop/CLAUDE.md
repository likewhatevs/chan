# chan-desktop notes

## Local serving and bundled chan helper

Normal local drives do not spawn `chan serve`. chan-desktop links
`chan-drive` and `chan-server` and opens local drives through the
embedded multi-drive host. External `chan serve` processes are still
supported, but desktop treats them as explicit remote attachments.

chan-desktop still ships with a copy of the `chan` binary for
CLI-owned surfaces that have not moved behind desktop-native APIs:
registry mutations, version/status probing, feature toggles, and
hidden MCP proxy plumbing. `desktop/Makefile`'s `chan-bin` recipe
stages `target/release/chan` to
`src-tauri/binaries/chan-<target-triple>` before every build; the
macOS bundle copy is configured through Tauri's `bundle.macOS.files`
map in `src-tauri/tauri.conf.json`.

### Helper binary layout

| Build              | Helper path                                  |
|--------------------|----------------------------------------------|
| `make run`         | staged under `src-tauri/binaries/` first     |
| Packaged macOS     | `Chan.app/Contents/MacOS/chan`               |
| Packaged Linux     | not bundled in v0.11.2 (see below)           |
| Packaged Windows   | not bundled in v0.11.2 (see below)           |

`Contents/MacOS/chan` is the recommended location on macOS because
Tauri's code-signing step automatically covers everything under
`Contents/MacOS/`. No custom `codesign --deep` invocation is needed:
both binaries get a Developer ID signature from the same identity in
one pass, and `ci-7`'s notarization roundtrip covers both for free.

### v0.11.2 hotfix: aarch64-only DMG, no externalBin

Tauri 2's `bundle.externalBin` field auto-expands the configured
path with the target triple AND, on macOS, looks for BOTH
`-aarch64-apple-darwin` and `-x86_64-apple-darwin` (universal2
expectation). `desktop/Makefile`'s `chan-bin` recipe only stages
the host triple, so the x86_64 lookup fails on the `macos-latest`
CI runner. `ci-8` dry-run #2 surfaced this as a hard
"resource path doesn't exist" bundle error.

`fullstack-b-20` (the v0.11.2 hotfix) routes around the universal2
expectation by:

* Dropping `bundle.externalBin` entirely.
* Using Tauri 2's `bundle.macOS.files` map (destination paths are
  relative to `Chan.app/Contents/`, NOT the bundle root) to copy
  the host-triple binary into `MacOS/chan` directly. This bypasses
  the triple-expansion logic since `bundle.macOS.files` is a
  literal source-to-destination map. End result on disk:
  `Chan.app/Contents/MacOS/chan`.

`fullstack-b-21` (notarization fix for the same v0.11.2 hotfix
path) adds a codesign step to the `chan-bin` Makefile recipe.
Reason: `bundle.macOS.files` is a "copy verbatim" primitive.
Tauri's signing pass walks the binaries it knows about
(`chan-desktop`, `externalBin` entries, the `.app` wrapper) but
does NOT route the `files` map through that pass. The
cargo-built `target/release/chan` carries an ad-hoc rustc/macOS
signature that satisfies `codesign --verify` locally but Apple's
notary correctly rejects (no Developer ID, no hardened runtime,
no secure timestamp). Tested option C
(`bundle.macOS.externalBin`) first; Tauri 2's per-platform
`macOS` config does NOT support an `externalBin` field (only
`frameworks`, `files`, `signing-identity`, etc.). Falling back
to Option A: Makefile codesign before staging:

```make
@if [ -n "$$APPLE_SIGNING_IDENTITY" ]; then \
    codesign --force --options=runtime --timestamp \
        --sign "$$APPLE_SIGNING_IDENTITY" $(CHAN_BIN); \
fi
```

Skipped when `APPLE_SIGNING_IDENTITY` is empty (local dev /
unsigned-build path). After this, Tauri's bundler copies the
already-signed chan binary into `Chan.app/Contents/MacOS/chan`
verbatim, preserving the signature. Verified locally:
`codesign -dv --verbose=2` shows
`flags=0x10000(runtime)` + `Authority=Developer ID Application:
... (TEAMID)` + `Timestamp=<date>` on the bundled chan;
`codesign --verify --strict --deep` exits 0 on `Chan.app`.

Two known regressions from the externalBin drop:

1. **Dev-mode auto-copy gone**: `cargo tauri dev` previously
   relied on Tauri's externalBin auto-copy to drop `chan` into
   `target/debug/` so `bundled_chan_path()` resolves. With
   externalBin removed, the bundled sidecar doesn't exist in dev.
   The `-b-16` PATH-first resolver picks up
   `cargo install --path crates/chan` chan automatically, so dev
   mode WORKS for any contributor who has installed the matching
   chan version to PATH. Without a PATH install, dev mode reports
   `BinStatus::missing` and disables spawn paths. Documented
   trade-off until the Makefile gains a manual `target/debug/`
   copy step (post-v0.11.2 follow-up).
2. **Linux + Windows bundling no longer ships chan**:
   `bundle.macOS.files` is macOS-only. On Linux, the .deb / .appimage
   produced by `make build` no longer includes chan. Users on Linux
   need to install chan separately via `cargo install --path
   crates/chan` and rely on the `-b-16` PATH resolver. Windows
   distribution is not currently exercised; same caveat applies.

Both regressions are scoped trade-offs for v0.11.2's signed-macOS
DMG ship. The full multi-platform externalBin restoration is a
post-v0.11.2 `ci-N` task that pairs with the universal2 work.

### Resolution helpers

`crates/chan-desktop/src/serve.rs` exposes three helpers used by
every CLI spawn site. These helpers are not part of local serving:
embedded local drives use chan-server's in-process host.

* `resolve_chan_binary() -> Result<PathBuf, String>`: the
  PATH-first picker. Walks `PATH` for a `chan` (or `chan.exe`)
  binary; if found, probes its `--version` and accepts only an
  EXACT semver match against chan-desktop's own
  `env!("CARGO_PKG_VERSION")`. Any failure (no chan on PATH,
  spawn error, --version error, version mismatch) falls through
  to `bundled_chan_path()`. Result: `add_drive`, `remove_drive`,
  `set_drive_on`, and the boot-time preflight all delegate to
  this helper so a power user who runs `cargo install --path
  crates/chan` against the matching version gets to use their
  own build through chan-desktop without rebuilding chan-desktop.
* `bundled_chan_path() -> Result<PathBuf, String>`: pure path
  math over `current_exe()`. Returns the expected sidecar path
  without checking that the file exists. Cheap, infallible at
  runtime in practice (only fails if `current_exe()` itself
  fails).
* `probe_chan_version(bin: &Path) -> Result<(), String>`: runs
  `<bin> --version` and asserts an EXACT semver match against
  `env!("CARGO_PKG_VERSION")`. Used by both the boot-time
  preflight (validating whatever `resolve_chan_binary()` picks)
  AND `resolve_chan_binary()` itself (validating the PATH
  candidate before accepting it). Exact match (not a `>=` floor)
  is the locked Round-2 decision-3 contract: bundled chan is
  built from the same workspace checkout as chan-desktop, and
  any PATH chan that doesn't match the same version is rejected
  in favour of the bundled fallback.

The boot-time preflight in `main.rs::compute_bin_status` runs the
resolver + existence check + version check exactly once, stores
the verdict in `AppState::bin_status`, and every IPC that spawns
chan gates on `require_bin()` before calling
`resolve_chan_binary()`.

#### Resolution algorithm (user-facing behaviour)

| State                                          | Picked binary            |
|------------------------------------------------|--------------------------|
| `chan vN` on PATH (N = chan-desktop's version) | PATH chan                |
| `chan vM` on PATH, M ≠ N                       | bundled (PATH rejected)  |
| `chan` on PATH errors on `--version`           | bundled                  |
| no `chan` on PATH                              | bundled                  |
| both unavailable                               | error → `BinStatus::missing` |

Fresh installs Just Work: chan-desktop ships the matching `chan`
inside the app bundle. Power users who want chan-desktop to drive
their own chan build install it to PATH at the matching version
(`cargo install --path crates/chan` from the same checkout), and
chan-desktop picks it up automatically on next launch. A
mismatched PATH install (older or newer than chan-desktop) is
ignored cleanly; the app keeps working via the bundled binary.

### Architecture handling

`desktop/Makefile`'s `chan-bin` recipe currently builds for the
host's target triple only (`$(shell rustc -vV | sed -n 's/host: //p')`),
and `bundle.macOS.files` in `tauri.conf.json` hardcodes
`binaries/chan-aarch64-apple-darwin` as the source path. The
v0.11.2 macOS DMG is therefore aarch64-only.

A macOS universal2 fat binary (`aarch64-apple-darwin` +
`x86_64-apple-darwin` merged via `lipo -create`) is the next
post-v0.11.2 step for distributing a single DMG that runs natively
on both Apple Silicon and Intel Macs. That work is owned by a
`ci-N` follow-up in the GitHub Actions release workflow (CI
already runs per-arch matrix builds and is the natural place to
`lipo`-merge before bundling); the same task can restore
multi-platform `bundle.<linux|windows>.files` for Linux/Windows
bundling once the matrix supports it.

## Apple Developer ID signing

The chan-desktop `.app` (and the bundled `.dmg`) are codesigned with
an Apple Developer ID Application certificate so Gatekeeper accepts
the install on default-configured macOS. The signing identity is
named in `src-tauri/tauri.conf.json` under
`bundle.macOS.signingIdentity`:

```json
"bundle": {
  "macOS": {
    "signingIdentity": "Developer ID Application: <Name> (<TEAMID>)"
  }
}
```

The identity NAME is a public identifier (the same string that
`security find-identity -v -p codesigning` prints), safe to land in
the repo. The matching private key + cert blob stay outside the repo
entirely: in the developer's macOS Keychain for local builds; in
`secrets.APPLE_CERTIFICATE_BASE64` + `secrets.APPLE_CERTIFICATE_PASSWORD`
for CI (imported into a temp keychain by
`apple-actions/import-codesign-certs@v3` in `release-desktop.yml`).
See the [macOS signing brief](../docs/release/macos-signing.md) for
the full per-secret table.

`bundle.macOS.providerShortName` is omitted because chan-desktop's
Apple Developer account is Individual enrollment with a single ASC
team. Populate the field only if the account is associated with
multiple teams.

### Local-build vs CI-build behaviour

* **Local build with the cert in your Keychain**: `make app-signed`
  and `make app-notarized` codesign the .app with the named identity.
  Works out of the box.
* **Local build WITHOUT the cert**: `make app-signed` /
  `make app-notarized` fail at the codesign step with
  `identity '...' not in keychain` (the `sign-prereqs` check in
  `desktop/Makefile`). That is expected; it just means you cannot
  produce a signed bundle on a workstation that does not hold the
  signing key. `make build` (the unsigned path) still works for
  general development.
* **CI build**: `release-desktop.yml` imports the cert into a temp
  keychain via secrets, then runs `make app-notarized` with the env
  vars populated. Same identity, just supplied differently.

### Rotation (if the Developer ID identity ever changes)

Developer ID Application certs expire every 5 years; rotation also
applies if the cert is revoked or replaced. Steps:

1. Generate the new cert per the [macOS signing brief](../docs/release/macos-signing.md)
   "Developer ID Application certificate generation".
2. Update `bundle.macOS.signingIdentity` in `tauri.conf.json` to the
   new identity string. Single-field swap.
3. Refresh `APPLE_CERTIFICATE_BASE64` + `APPLE_CERTIFICATE_PASSWORD`
   + `APPLE_SIGNING_IDENTITY` + `APPLE_TEAM_ID` in GitHub Actions
   Secrets via `docs/release/populate-apple-secrets.sh` (re-run the
   relevant steps).
4. Refresh the local Keychain: import the new `.p12`; remove the
   old cert (`security delete-certificate -c "Developer ID
   Application: <Old Name> ..."`) so the `sign-prereqs`
   `grep -c "Developer ID Application:"` detection picks the new
   one unambiguously.

No bridge release is needed for Developer ID cert rotation (unlike
the minisign updater key rotation below). Gatekeeper trusts any
Developer ID Application cert chained to Apple's root, regardless
of which one signed the previous release, so the next signed bundle
under the new identity is accepted by clients that had the old one.

## Auto-upgrade signing (tauri-plugin-updater)

The desktop app verifies update bundles with a minisign signature.
The production pubkey is embedded in `src-tauri/tauri.conf.json`
under `plugins.updater.pubkey`. Matching private key material lives
outside the repo in the release owner's secret store.

Runbook: [`updater-bridge.md`](updater-bridge.md).

### Bridge release required after key rotation

The configured pubkey was rotated from the phase-8 DEV updater key
to the production updater key on 2026-05-23. Existing installs that
already trust the old DEV pubkey need a bridge release:

1. Ship one bridge release that embeds the NEW production pubkey but
   signs the update bundle with the OLD DEV private key, so existing
   installs accept it.
2. Sign every release after the bridge with the NEW production
   private key.
3. If the OLD DEV private key is unavailable, existing installs
   cannot auto-update across this key rotation. Users need a manual
   DMG install for the first production-key release.
4. Old installs that never picked up the bridge release will fail
   to verify NEW-key-signed bundles and stall on their last good
   version until the user manually reinstalls. Plan the bridge
   window for how long you're willing to support that tail.
5. The signing identity used at build time is selected via
   `TAURI_SIGNING_PRIVATE_KEY` (key contents) or
   `TAURI_SIGNING_PRIVATE_KEY_PATH` (file path), with optional
   `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`. CI should pull these from
   a secrets store, never from the repo.

### Manifest endpoint

Client probes:
`https://chan.app/dl/desktop/{{target}}/{{current_version}}/latest.json`

Server-side publishing of that manifest is owned by chan-prod-setup.

## Release package version metadata

Tauri bundle artifact names are derived from
`src-tauri/tauri.conf.json`:

* `productName` supplies the `Chan` prefix.
* `version` supplies the semantic version in bundle names. For the
  next cut, generated desktop artifacts should start with
  `Chan_0.14.0.` or `Chan_0.14.0_`, depending on the bundle type and
  platform suffix.

Before pushing a `chan-vX.Y.Z` desktop release tag, update
`src-tauri/tauri.conf.json` `version` to `X.Y.Z`. The Rust package
version for `chan-desktop` is inherited from the workspace so the
bundled `chan` sidecar and desktop version probe stay aligned when
chan-core bumps the workspace for the release.

## Local notarization setup

`make app-notarized` (in `desktop/Makefile`) accepts notarization
credentials from two sources, in this precedence order:

1. **Environment variables** (`APPLE_ID` + `APPLE_PASSWORD` +
   `APPLE_TEAM_ID`). What CI uses; populated from GitHub Actions
   Secrets in `.github/workflows/release-desktop.yml`. Wins when
   present, so a one-off `APPLE_ID=... APPLE_PASSWORD=...
   APPLE_TEAM_ID=... make app-notarized` shadows whatever Keychain
   profile is configured locally.
2. **`notarytool` Keychain profile** (Apple's blessed mechanism for
   local dev). One-time setup on the workstation:

   ```
   xcrun notarytool store-credentials chan \
       --apple-id <your-apple-id-email> \
       --team-id <10-char-team-id> \
       --password <app-specific-password>
   ```

   Stashes the Apple ID + Team ID + app-specific password in the
   default macOS Keychain under a profile named `chan`. After this,
   plain `make app-notarized` runs from a bare shell with no env
   exports; the Makefile detects the profile and passes
   `--keychain-profile chan` to `xcrun notarytool submit`. Override
   the profile name via `NOTARIZE_PROFILE=othername make
   app-notarized` if you store it under a different label.

The app-specific password (option 2 above) is generated at
<https://account.apple.com/> -> Sign-In and Security ->
App-Specific Passwords -> Generate. It is NOT the iCloud account
password; the iCloud password will not work for `notarytool`
because of two-factor auth.

### Verifying the Keychain profile is in place

```
security find-generic-password -s "com.apple.gke.notary.tool" -a chan
```

Returns the keychain item attributes if the profile exists, exits
non-zero otherwise. The actual password value never leaves the
Keychain (no `-g`/`-w` flag).

`xcrun notarytool history --keychain-profile chan` is the
network-dependent variant: lists Apple-side notary submissions and
fails on a bad / non-existent profile. Useful as an end-to-end
"is this profile actually valid against Apple" check.

### Why the Makefile splits build from notarize

`cargo tauri build --bundles app,dmg` signs the `.app` (via
`codesign`, driven by `APPLE_SIGNING_IDENTITY`) and packages the
`.dmg`. Tauri's bundler can ALSO notarize as part of that single
command, but only when `APPLE_ID` + `APPLE_PASSWORD` +
`APPLE_TEAM_ID` are present in the env. `tauri-bundler` 2.x does
not consume `notarytool` Keychain profiles directly. To support
the local Keychain-profile path AND the CI env-var path under one
recipe, the Makefile unsets the three notarize env vars during the
`cargo tauri build` call (so tauri-bundler skips its own notarize
step) and then runs `xcrun notarytool submit` + `xcrun stapler
staple` itself with the appropriate flag set.

CI behaviour is identical to the prior path: the credentials reach
notarytool the same way, just via a manual invocation instead of
tauri-bundler's internal one.

### CI does not need the Keychain profile

`release-desktop.yml` reads `APPLE_ID` + `APPLE_PASSWORD` +
`APPLE_TEAM_ID` straight from `secrets.APPLE_*` (per the
`docs/release/macos-signing.md` brief). The env-var path runs
unchanged on the runner; no `xcrun notarytool store-credentials`
step is needed in CI.
