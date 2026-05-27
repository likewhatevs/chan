# chan-desktop notes

## Local serving and self-contained runtime

chan-desktop is fully self-contained. It links `chan-workspace` and
`chan-server` directly and embeds the web bundle (`web/dist`) via
rust-embed at build time. There is no `chan` binary at runtime and
none is shipped in the app bundle.

Local workspaces open through the embedded chan-server `WorkspaceHost`,
which owns a single `chan_workspace::Library`. Every registry mutation
(add / remove / default-workspace reconciliation) and feature toggle
(semantic search, reports) runs in-process against that same
`Library`, or against the live `Arc<Workspace>` the host already holds
for a mounted workspace. Routing through one shared registry is what
keeps a freshly-added workspace openable immediately: a subprocess
`chan add` used to mutate only the on-disk registry, leaving the
host's boot-time in-memory snapshot stale, which surfaced as a
spurious "workspace not registered" error on first open.

Blocking chan-workspace calls (`register_drive`, `unregister_drive`,
`open_drive`, `boot`, the feature setters) run via
`tokio::task::spawn_blocking` so a slow initial scan never blocks
the async executor. `remove_drive` runs `unregister_drive` in a
bounded retry loop: `serve::stop` drops the host's handle
synchronously, but a background indexer or in-flight request may
hold the workspace's flock for a moment, surfacing as
`WorkspaceAlreadyOpen` / `WorkspaceLocked` until it releases.

External `chan serve` processes are still supported as explicit
remote attachments, but they are a separate transport, not a local
serving dependency. When a standalone `chan serve` already holds a
workspace's lock, opening that workspace in chan-desktop maps the
`WorkspaceLocked` / `WorkspaceAlreadyOpen` error to a friendly "open in
another chan process" message and reverts the row's On toggle
rather than surfacing a raw error.

### No bundled binary

There is no `chan-bin` Makefile step, no `bundle.macOS.files` entry,
and no `bundle.externalBin`. `make dev` / `make build` /
`make app-signed` / `make app-notarized` depend only on the `web`
bundle (the rust-embed input) and the tauri CLI. The single
codesigned + notarized artifact is the chan-desktop `.app` itself;
Tauri's signing pass covers it directly, with no second binary to
sign. This removed the v0.11.2-era universal2 / externalBin
machinery entirely; multi-arch distribution is now purely a matter
of how the `.app` itself is built and merged.

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
version for `chan-desktop` is inherited from the workspace, so the
`.app` version and the workspace stay aligned when chan bumps the
workspace for the release.

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
