# macOS code signing + notarization

Reference for the credentials behind chan-desktop's signed +
notarized releases: what the maintainer obtains from Apple, how the
signing material is exported safely, how it is stored in GitHub
Actions Secrets, and the notarization flow
`.github/workflows/release-desktop.yml` consumes. The workflow's
missing-secret error points here; this is also the runbook for
rotating or re-provisioning any of the credentials.

## Scope

In scope:

* macOS Developer ID signing + notarization for the
  chan-desktop `.app` and `.dmg` produced by
  `desktop/Makefile`'s `app-notarized` target.
* The GitHub Actions secret names the release workflow reads.
* The chronological checklist for provisioning (or
  re-provisioning) the credentials.

Out of scope (separate briefs):

* Apple updater bundle signing (the Tauri `minisign` key). The
  key-rotation procedure lives in `.agents/desktop.md`; CI
  reads `TAURI_SIGNING_PRIVATE_KEY` once the bridge release
  ships.
* Windows Authenticode. Mentioned at the bottom as a pointer;
  a dedicated `docs/release/windows-signing.md` lands when we
  open the Windows lane.
* Linux package signing (.deb / .rpm GPG). Separate brief when
  that lane opens.

## Background

An unsigned bundle cannot be installed on a default-configured
macOS without right-click -> Open or `xattr` shell incantations:
Gatekeeper rejects anything not signed by a trusted Developer ID
and notarized by Apple.

`desktop/Makefile` implements the local signed + notarized path via
the `app-notarized` target. It expects four env vars
(`APPLE_SIGNING_IDENTITY`, `APPLE_TEAM_ID`, `APPLE_ID`,
`APPLE_PASSWORD`) and assumes the Developer ID cert is already
imported into the local Keychain. The CI workflow reproduces that
environment on a clean GitHub-hosted macOS runner: it imports the
cert into a temp keychain, populates the env, and calls
`make app-notarized`.

## Apple Developer Program enrollment

Two enrollment shapes exist. Pick one before anything else;
the cert is issued against whichever account you enrolled.

| Type         | Annual cost | Team ID owner    | Notes                       |
|--------------|-------------|------------------|-----------------------------|
| Individual   | USD 99      | The enrollee     | Recommended for chan today  |
| Organization | USD 99      | Legal entity     | Requires D-U-N-S number     |

Recommendation: enroll as an Individual under the maintainer's
Apple ID. chan is solo-maintained; the organization path adds
D-U-N-S provisioning and legal-name verification that adds weeks
for no functional benefit. Future migration from Individual to
Organization is supported by Apple but requires a fresh cert and a
Team ID change; that only matters if chan ever incorporates.

Enrollment URL: `https://developer.apple.com/programs/`. The
review takes 24-48 hours typically. Apple emails the
confirmation; the Apple Developer Account page then shows the
10-character Team ID.

## Developer ID Application certificate generation

This certificate type signs apps for distribution outside the
Mac App Store. Mac App Store distribution uses a different
cert type (Mac App Distribution) and is not what we want.

Generation flow:

1. On a macOS machine, open Keychain Access. From the menu:
   `Certificate Assistant -> Request a Certificate From a
   Certificate Authority`.
2. Fill in: user email (Apple ID email), common name (e.g.,
   `Alexandre Fiori`), CA email blank. Select `Saved to disk`.
   Click Continue. Keychain Access writes a
   `CertificateSigningRequest.certSigningRequest` file.
3. Sign in to `https://developer.apple.com/account/resources/certificates/list`.
   Click `+` to add a certificate. Pick
   `Developer ID Application` (not `Mac Installer`).
4. Upload the CSR from step 2. Apple issues a
   `developerID_application.cer` file; download it.
5. Double-click the `.cer` to import into Keychain Access.
   The private key generated in step 2 binds to the imported
   cert and the pair shows as
   `Developer ID Application: <Name> (<TEAMID>)` under My
   Certificates.
6. Verify locally:

   ```
   security find-identity -v -p codesigning
   ```

   The output must include the `Developer ID Application: ...`
   line with the 10-char Team ID in parentheses.
7. Export the pair to a `.p12` file for CI:
   * In Keychain Access, expand the cert in My Certificates,
     select both the cert and its private key.
   * Right-click -> `Export 2 items...`.
   * Pick the `Personal Information Exchange (.p12)` format.
   * Set a strong export password and store it in a password
     manager. This password protects the `.p12` blob at rest;
     CI will need it to import the cert on the runner.

The `.p12` file is the credential CI consumes. The private key
inside it is the one Apple-trusted root chains back to;
treat the file as if it were the Apple Developer account
password.

## App-specific password for notarytool

`notarytool` (Apple's replacement for the long-deprecated
`altool`) authenticates against the developer account. Use an
app-specific password, not the developer account's iCloud
password.

Generation:

1. Sign in to `https://account.apple.com/`.
2. Navigate to `Sign-In and Security -> App-Specific
   Passwords -> Generate an app-specific password`.
3. Label the password (e.g., `chan-notary-ci`) so the audit
   trail in Account is legible.
4. Apple shows the 19-character password (format
   `xxxx-xxxx-xxxx-xxxx`) exactly once. Copy it to the
   password manager. There is no recovery; lost passwords are
   regenerated.

Why app-specific and not the account password: two-factor auth
prevents the account password from working unattended in CI,
and an app-specific password can be revoked from the Account
page without rotating the account password. Apple's
`notarytool` documentation calls this out explicitly.

## Notarization team ID + bundle ID

Notarization metadata that CI submits with each upload:

| Field      | Value                       | Source                       |
|------------|-----------------------------|------------------------------|
| Team ID    | (from enrollment)           | Account page / cert name     |
| Bundle ID  | `app.chan.desktop`          | `src-tauri/tauri.conf.json`  |
| Profile    | `developer-id`              | `notarytool submit` flag     |

The Team ID is also embedded at the end of the signing
identity string (`Developer ID Application: Alexandre Fiori
(ABCD123456)`), so the Makefile auto-derives it via `sed`. The
bundle ID is fixed in `tauri.conf.json` and CI does not need
to touch it.

## GitHub Actions Secrets shape

Names below match what `desktop/Makefile`'s `app-notarized`
target reads, plus the two extra secrets the CI workflow needs
to import the cert into the runner's keychain
(`APPLE_CERTIFICATE_BASE64` and `APPLE_CERTIFICATE_PASSWORD`,
which the local Makefile path does not need).

| Secret                       | Holds                                         |
|------------------------------|-----------------------------------------------|
| APPLE_CERTIFICATE_BASE64     | Developer ID Application .p12, base64-encoded |
| APPLE_CERTIFICATE_PASSWORD   | Export password set when the .p12 was created |
| APPLE_SIGNING_IDENTITY       | Full cert name, e.g.                          |
|                              | `Developer ID Application: Name (ABCD123456)` |
| APPLE_TEAM_ID                | 10-char Team ID (auto-derived from identity)  |
| APPLE_ID                     | Apple developer account email                 |
| APPLE_PASSWORD               | App-specific password for notarytool          |

Notes on each:

* `APPLE_CERTIFICATE_BASE64` is the .p12 file produced by the
  Keychain export step, base64-encoded so it survives the
  GitHub Secrets text-only constraint. Generate with
  `base64 -i developerID_application.p12 -o cert.b64` on
  macOS and paste the file contents into the secret value.
* `APPLE_CERTIFICATE_PASSWORD` is the export passphrase from
  the .p12 export. CI uses it once during keychain import and
  never logs it.
* `APPLE_SIGNING_IDENTITY` is optional when only one Developer
  ID Application cert is in the runner's keychain (the
  Makefile auto-detects in that case), but setting it
  explicitly avoids ambiguity if multiple certs ever get
  imported. CI sets it explicitly.
* `APPLE_TEAM_ID` is auto-derived from
  `APPLE_SIGNING_IDENTITY` by the Makefile. Set it explicitly
  in CI anyway so a renamed cert does not break notarization
  silently.
* `APPLE_ID` and `APPLE_PASSWORD` together feed
  `notarytool`'s `--apple-id` and `--password` flags via
  Tauri's CLI bundler.

No secret values land in this repo. Secret population is a
manual one-time step in the GitHub repo's
`Settings -> Secrets and variables -> Actions` page. Use
`Repository secrets`, not `Environment secrets`, until we have
a reason to gate releases on a protected environment.

## Recommended cert-import packaging

The clean-runner cert-import step has two reasonable shapes:

* `apple-actions/import-codesign-certs@v7` (third-party action,
  widely used, ~200 LoC of shell under the hood).
* A hand-rolled `security` block inside the workflow.

Recommendation: `apple-actions/import-codesign-certs@v7`. It is
the de-facto standard for this step, handles the temp-keychain
creation + unlock + cert import + keychain add-to-search-list
in one place, and reduces the workflow YAML to a single step.
The cost of the third-party dep is small (one pinned action
version) and the benefit is not maintaining shell that handles
edge cases like keychain locking on macos-14 runners.

The workflow step shape:

```yaml
- name: Import Developer ID certificate
  uses: apple-actions/import-codesign-certs@v7
  with:
    p12-file-base64: ${{ secrets.APPLE_CERTIFICATE_BASE64 }}
    p12-password: ${{ secrets.APPLE_CERTIFICATE_PASSWORD }}

- name: Build + notarize chan-desktop
  working-directory: chan/desktop
  env:
    APPLE_SIGNING_IDENTITY: ${{ secrets.APPLE_SIGNING_IDENTITY }}
    APPLE_TEAM_ID: ${{ secrets.APPLE_TEAM_ID }}
    APPLE_ID: ${{ secrets.APPLE_ID }}
    APPLE_PASSWORD: ${{ secrets.APPLE_PASSWORD }}
  run: make app-notarized
```

The `make app-notarized` call reuses the existing local path
verbatim; CI's only responsibility is to set up the keychain
state the Makefile assumes.

The fallback (`security` shell block) is documented below for
the record, in case the third-party action ever becomes
abandoned or gets rate-limited:

```bash
KEYCHAIN_PATH="$RUNNER_TEMP/build.keychain-db"
KEYCHAIN_PASSWORD=$(openssl rand -base64 24)
security create-keychain -p "$KEYCHAIN_PASSWORD" "$KEYCHAIN_PATH"
security set-keychain-settings -lut 21600 "$KEYCHAIN_PATH"
security unlock-keychain -p "$KEYCHAIN_PASSWORD" "$KEYCHAIN_PATH"
echo "$APPLE_CERTIFICATE_BASE64" | base64 --decode > cert.p12
security import cert.p12 -k "$KEYCHAIN_PATH" \
    -P "$APPLE_CERTIFICATE_PASSWORD" \
    -T /usr/bin/codesign -T /usr/bin/security
security list-keychain -d user -s "$KEYCHAIN_PATH"
security set-key-partition-list -S apple-tool:,apple: \
    -k "$KEYCHAIN_PASSWORD" "$KEYCHAIN_PATH"
rm cert.p12
```

If the fallback path ever gets adopted, add `KEYCHAIN_PASSWORD`
to the secret list above (currently it is generated per-run
inside the workflow, so it does not need a stored secret).

## Provisioning checklist

Chronological. Items 1-3 are one-time per developer identity;
items 4-6 are one-time per repository.

1. Enroll in the Apple Developer Program as Individual at
   `https://developer.apple.com/programs/`. Wait for Apple's
   email confirmation (24-48 hours typical). Record the 10-char
   Team ID from the Account page.
2. Generate the Developer ID Application certificate per the
   `Developer ID Application certificate generation` section
   above. Import it into Keychain Access on the workstation.
3. Export the cert + private key as a `.p12` file with a strong
   passphrase. Store the passphrase in a password manager.
4. Generate an app-specific password at
   `https://account.apple.com/` (label `chan-notary-ci`).
   Store it in the password manager.
5. base64-encode the `.p12`:

   ```
   base64 -i developerID_application.p12 -o cert.b64
   ```

   Open `cert.b64`, copy its contents.
6. Populate GitHub repo secrets under
   `Settings -> Secrets and variables -> Actions ->
   Repository secrets`. Add the six secrets from the
   `GitHub Actions Secrets shape` table. Delete the local
   `cert.b64` and the `.p12` from the working directory once
   the secrets are in place; the only canonical copies should
   live in the password manager and GitHub.
7. Optional but recommended: verify the local Makefile path
   works end-to-end on the workstation before CI runs.
   Export the four env vars in a shell, run
   `cd desktop && make app-notarized`, confirm the resulting
   `.dmg` opens cleanly on a second Mac with no Gatekeeper
   warning. This catches enrollment / cert / password issues
   before CI ever touches them.

The release workflow consumes the secrets by name, so once step 6
is complete the next `vX.Y.Z` release cut produces a notarized
`.dmg` ready for GitHub Release upload. The same release workflow also checks
for the Tauri updater signing secret names documented in
`.agents/desktop.md`; those keys are separate from Apple
Developer ID signing.

## Parallel Windows signing path (pointer only)

A separate brief lands when the Windows lane opens
(`docs/release/windows-signing.md` is the suggested path).
Two cert shapes worth noting for sequencing:

| Cert type | SmartScreen reputation              | Cost / year |
|-----------|-------------------------------------|-------------|
| OV        | Builds reputation over time         | ~USD 200    |
| EV        | Trusted immediately, hardware token | ~USD 400    |

Recommendation when we cross that bridge: OV is sufficient for
chan-desktop's distribution model (single signed installer
hosted on GitHub Releases). EV's instant-reputation benefit
matters most for high-volume enterprise distribution. The
hardware-token requirement also makes EV painful for CI
(token attestation does not run on GitHub-hosted runners
without custom hardware).

The release workflow signs macOS only today; Windows signing lands
together with a Windows release lane.

## References

* Apple Developer Program enrollment:
  `https://developer.apple.com/programs/`.
* Apple ID app-specific passwords:
  `https://account.apple.com/`.
* `notarytool` manual:
  `man notarytool` (shipped with Xcode 13+).
* `tauri-apps/tauri-action` for the alternative
  fully-managed CI shape we did not pick (chan-desktop's
  Makefile path is the source of truth):
  `https://github.com/tauri-apps/tauri-action`.
* `apple-actions/import-codesign-certs`:
  `https://github.com/apple-actions/import-codesign-certs`.
* `.agents/desktop.md` for the orthogonal Tauri updater key
  rotation that has to ship before the first signed release.
* `desktop/Makefile` `app-notarized` target as the local
  reference implementation of the CI path.
