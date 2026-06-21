# Windows code signing (Authenticode)

Reference for signing chan-desktop's Windows installer + binary for public
distribution: what to buy (and the 2023 rule that changed it), how the signing
material is stored in GitHub Actions Secrets, and how the signing step plugs into
`.github/workflows/release-desktop.yml` and `release.yml`. This is the Windows
counterpart to `docs/release/macos-signing.md`.

> **Status today: UNSIGNED, not a public asset.** The manual `release-desktop.yml`
> dry-run builds an unsigned NSIS installer as an Actions artifact only
> (`release-desktop.yml:122-172`). The tag-triggered `release.yml` ships **no**
> Windows asset at all — `publish-release` needs Linux + macOS jobs only
> (`release.yml:616-622`). `desktop/src-tauri/tauri.conf.json:65-70` carries only
> NSIS install settings — no signing fields. This doc is the plan to change that.

## Why sign

An unsigned Windows installer triggers a full-screen **Microsoft Defender
SmartScreen** warning ("Windows protected your PC") on download/run, with "More
info -> Run anyway" hidden behind a click. Most users abandon at that screen. An
Authenticode signature (a) replaces "Unknown publisher" with the verified
publisher name in the UAC + SmartScreen prompts, and (b) accrues SmartScreen
*reputation* over downloads so the warning eventually disappears entirely.

## TL;DR — what to buy

**The plain ~$200/yr OV cert that downloads as a `.pfx` no longer exists.** Since
**2023-06-01** the CA/Browser Forum Code Signing Baseline Requirements mandate
that every code-signing private key (OV **and** EV) be generated and stored in a
FIPS 140-2 Level 2+ hardware token or a cloud HSM. There is no downloadable key
file for a new certificate anymore — which kills the "base64 a `.pfx` into a CI
secret" model that `macos-signing.md` sketched for Windows.

Realistic options, cheapest first:

| Option | ~Cost/yr | Key storage | CI on GH-hosted runners | SmartScreen |
|--------|----------|-------------|-------------------------|-------------|
| **Azure Trusted Signing** (recommended) | ~USD 120 (USD 9.99/mo) | Microsoft cloud HSM | Yes — native `azure/trusted-signing-action` | OV-level; builds over time |
| OV cert + cloud signing (SSL.com eSigner, DigiCert KeyLocker) | ~USD 250-400 | CA cloud HSM | Yes — vendor CLI/API | OV-level; builds over time |
| OV/EV cert on a USB hardware token | ~USD 200-600 | physical token | **No** — token can't attest on a hosted runner | EV = instant |

**Recommendation: Azure Trusted Signing.** Cheapest, no hardware, first-class
GitHub Actions support, and it issues a validated-identity (OV-equivalent)
certificate. The trade-offs: it needs an Azure subscription and an identity
validation step, and it adds an Azure dependency the repo otherwise has none of.
If Trusted Signing onboarding doesn't fit, fall back to an OV cert from a CA that
offers **cloud signing** (SSL.com eSigner is the usual cheap pick). **Do not buy
a hardware-token cert** — it cannot sign on GitHub-hosted runners (you'd need a
self-hosted Windows runner with the token plugged in).

The rest of this doc details the recommended Azure path, then the OV+eSigner
fallback, then the shared CI wiring (identical regardless of which you pick,
because both end at a `signtool`-compatible command).

## Scope

In scope: Authenticode signing of the chan-desktop `.exe` and its NSIS installer,
the GitHub Actions secrets, and the `tauri.conf.json` + workflow changes that make
a signed installer a public release asset.

Out of scope (separate briefs): macOS signing/notarization (`macos-signing.md`,
already shipping), the Tauri updater `minisign` key (`.agents/desktop.md`), Linux
package signing, and a Windows *updater feed* (the auto-update channel; signing
the installer is the prerequisite, the feed is a later step).

---

## Path A (recommended): Azure Trusted Signing

Azure Trusted Signing is Microsoft's managed signing service: the cert + key live
in Microsoft's cloud HSM, you authenticate from CI with an Azure service
principal, and a GitHub Action signs your files. No `.pfx`, no token.

### A.1 Purchase + identity validation (do this first)

1. **Azure subscription.** Sign in at `https://portal.azure.com`. A pay-as-you-go
   subscription is fine; Trusted Signing billing is ~USD 9.99/month for the Basic
   tier (covers chan's volume many times over).
2. **Create a Trusted Signing account.** In the portal, create a *Trusted Signing
   Account* resource (pick a region; note the account name).
3. **Create an Identity Validation.** This is the gate that takes time. Microsoft
   validates either:
   - an **Organization** (legal entity; needs business records — fastest if chan
     is ever incorporated), or
   - an **Individual** (validates a person's legal identity; eligibility for
     individuals has been expanding — confirm current availability in your region
     when you start). The validated name becomes the certificate subject /
     publisher string users see.
   Budget a few business days; Microsoft may request documents. **Start this
   before wiring CI** — everything downstream blocks on an Approved validation.
4. **Create a Certificate Profile** under the account, bound to the approved
   identity (this is what issues the short-lived signing certs). Note the
   *account name* and *profile name*.

### A.2 Service principal for CI

CI authenticates as an Azure AD app, not your portal login:

1. Create an App Registration (Azure AD) -> note `AZURE_TENANT_ID`,
   `AZURE_CLIENT_ID`; create a client secret -> `AZURE_CLIENT_SECRET`.
2. Grant that principal the **Trusted Signing Certificate Profile Signer** role on
   the Trusted Signing account (IAM -> Role assignment).

### A.3 GitHub Actions secrets (Azure path)

`Settings -> Secrets and variables -> Actions -> Repository secrets`. Values live
only in GitHub; never in the repo, journals, or chat.

| Secret | Holds |
|--------|-------|
| `AZURE_TENANT_ID` | Azure AD tenant GUID |
| `AZURE_CLIENT_ID` | service-principal app (client) ID |
| `AZURE_CLIENT_SECRET` | service-principal client secret |
| `WINDOWS_SIGNING_ACCOUNT` | Trusted Signing account name |
| `WINDOWS_SIGNING_PROFILE` | certificate profile name |

(The account endpoint region — e.g. `https://eus.codesigning.azure.net` — is not
secret; keep it as a workflow `env`.)

---

## Path B (fallback): OV cert + cloud signing

If Trusted Signing doesn't fit, buy a standard **OV code-signing certificate** from
a CA that bundles a **cloud signing** add-on so CI can sign without a physical
token. SSL.com's "OV Code Signing + eSigner" is the common cheap pick
(~USD 250-300/yr incl. eSigner); DigiCert KeyLocker is the pricier enterprise
equivalent.

1. Purchase the OV cert; complete the CA's organization/individual validation
   (similar identity checks to Azure; days, not minutes).
2. Enroll the cert into the vendor's **cloud signing** service (eSigner /
   KeyLocker) — the key is generated in their HSM; you get API credentials + a TOTP
   secret, not a key file.
3. Secrets for the eSigner path (names illustrative — match the vendor's CLI):
   `WINDOWS_CSC_USERNAME`, `WINDOWS_CSC_PASSWORD`, `WINDOWS_CSC_CREDENTIAL_ID`,
   `WINDOWS_CSC_TOTP_SECRET`. The vendor's `CodeSignTool`/eSigner CLI consumes
   these and exposes a `signtool`-compatible sign step.

Both paths converge on "a command that signs a file"; the CI wiring below is
written so only that one step differs.

---

## Wiring it into the build (after you have signing access)

### W.1 Sign during the Tauri bundle (`tauri.conf.json`)

The NSIS installer embeds the `chan-desktop.exe`, so the inner binary must be
signed **before** the installer is built, then the installer itself signed. Tauri
2's `bundle.windows.signCommand` does both — Tauri invokes it once per artifact,
substituting the file path for `%1`. Extend `desktop/src-tauri/tauri.conf.json`'s
existing `bundle.windows` block (`tauri.conf.json:65-70`):

```jsonc
"windows": {
  "nsis": { "installMode": "currentUser", "languages": ["English"] },
  "signCommand": {
    "cmd": "pwsh",
    "args": ["-File", "scripts/windows/sign.ps1", "%1"]
  }
}
```

Keep the actual signing logic in `scripts/windows/sign.ps1` (new file) rather than
a brittle inline string. That script:
- **Azure path:** `Invoke-TrustedSigning` (the Trusted Signing PowerShell module)
  or `signtool sign` with the Trusted Signing dlib, using the `AZURE_*` +
  `WINDOWS_SIGNING_ACCOUNT/PROFILE` env, `/fd sha256`, and an RFC-3161
  `/tr <timestamp-url> /td sha256` timestamp.
- **eSigner path:** the vendor `CodeSignTool sign` invocation with the
  `WINDOWS_CSC_*` env.

`signCommand` runs only when the env is present, so local unsigned dev builds
still work.

> Timestamping is mandatory: `/tr <RFC-3161 TSA> /td sha256`. Without it,
> signatures become invalid the day the cert expires; with it, they remain valid
> after expiry. Use the TSA your signing provider documents (Azure Trusted
> Signing and the CA cloud services each publish one).

### W.2 The dry-run job (`release-desktop.yml`)

Mirror the macOS "verify secrets present" guard (`release-desktop.yml:76-99`) for
Windows, then make the existing `windows-latest` build step
(`release-desktop.yml:122-133`) sign. For the Azure path, add
`azure/trusted-signing-action` (or set the `AZURE_*` env so `sign.ps1` runs inside
the bundle). Shape:

```yaml
- if: matrix.os == 'windows-latest'
  name: Verify Windows signing secrets present
  shell: bash
  env:
    AZURE_TENANT_ID: ${{ secrets.AZURE_TENANT_ID }}
    AZURE_CLIENT_ID: ${{ secrets.AZURE_CLIENT_ID }}
    AZURE_CLIENT_SECRET: ${{ secrets.AZURE_CLIENT_SECRET }}
    WINDOWS_SIGNING_ACCOUNT: ${{ secrets.WINDOWS_SIGNING_ACCOUNT }}
    WINDOWS_SIGNING_PROFILE: ${{ secrets.WINDOWS_SIGNING_PROFILE }}
  run: |
    set -e
    missing=()
    for v in AZURE_TENANT_ID AZURE_CLIENT_ID AZURE_CLIENT_SECRET \
             WINDOWS_SIGNING_ACCOUNT WINDOWS_SIGNING_PROFILE; do
      [ -n "${!v}" ] || missing+=("$v")
    done
    if [ ${#missing[@]} -gt 0 ]; then
      echo "::error::Missing Windows signing secrets: ${missing[*]}"
      echo "::error::Populate per docs/release/windows-signing.md."
      exit 1
    fi
```

The build step at `:122-133` already runs `cargo tauri build --bundles nsis`; once
`signCommand` is set (W.1) and the `AZURE_*` env is exported to that step, the
bundle comes out signed. Keep the headless `/api/health` smoke (`:135-166`) as-is.

### W.3 Make it a public asset (`release.yml`)

The tag-triggered release ships no Windows asset today. To change that:
1. Add a `windows-desktop-artifacts` job to `release.yml` modeled on the macOS
   desktop job (`release.yml:~460-611`): checkout, toolchain, node, tauri-cli,
   verify-secrets, build+sign (W.1/W.2), then a **verify-signature** step
   (`signtool verify /pa /v <installer>` — the Windows analogue of the macOS
   `codesign --verify` + `spctl` at `release.yml:539-546`), stage into
   `release-artifacts/`, and `upload-artifact` as `release-windows-desktop`.
2. Add `windows-desktop-artifacts` to the `publish-release` `needs:` list
   (`release.yml:616-622`) so the GitHub Release waits for it. The
   `release-*` download pattern (`release.yml:638`) already picks it up.
3. Teach the `/dl` metadata generators about the Windows asset — the Pages job
   runs `collect-release-assets.mjs` + `generate-release-metadata.mjs`
   (`release.yml:687-688`); the CLI self-upgrade + desktop updater feeds read that
   metadata, so the Windows installer must appear there for download links to work.

### W.4 Single-source the secret guard (optional, matches repo norm)

`release.yml`'s combined "Verify signing secrets present" (`release.yml:497-520`)
checks all macOS + updater secrets in one step. Add the Windows secrets to that
same guard in the Windows job rather than duplicating logic, so a renamed secret
fails fast with a pointer here.

---

## Verifying a signature

On any Windows box (or the CI runner):

```powershell
signtool verify /pa /v path\to\Chan_x.y.z_x64-setup.exe
# or:
Get-AuthenticodeSignature path\to\Chan_x.y.z_x64-setup.exe | Format-List
```

A good result shows `Status: Valid`, the verified publisher (your validated
name), `SHA256` digest, and a countersignature/timestamp. Then sanity-check the
*human* experience: download the installer through a browser on a clean Windows VM
and confirm the SmartScreen/UAC prompt names the publisher (reputation still
builds over the first downloads even with a valid signature).

## Maintenance

- **Renewal:** Azure Trusted Signing renews with the subscription (no cert
  rotation in CI). A CA OV cert renews yearly — re-validate + refresh the cloud
  signing enrollment; the GitHub secrets change only if the credential IDs do.
- **Timestamps** keep already-released installers valid past cert expiry (W.1).
- **Reputation** is per signing-identity. Keep the same identity across releases;
  switching certs resets SmartScreen reputation.

## Provisioning checklist

One-time, in order:

1. (Azure path) Create the Azure subscription, Trusted Signing account, and
   **Identity Validation** — submit early; it gates everything. Then a Certificate
   Profile. (Fallback: buy an OV cert + cloud-signing add-on and complete CA
   validation.)
2. Create the CI service principal (Azure) / API credentials (eSigner) and grant
   the signer role.
3. Add the secrets from A.3 (or B.3) under repo `Settings -> Secrets and variables
   -> Actions -> Repository secrets`.
4. Add `scripts/windows/sign.ps1` + the `signCommand` block to `tauri.conf.json`
   (W.1).
5. Wire the dry-run job (W.2); run `release-desktop.yml` via `workflow_dispatch`
   and confirm the artifact is signed (`signtool verify`).
6. Add the public `windows-desktop-artifacts` job + `publish-release` need + `/dl`
   metadata (W.3); the next `vX.Y.Z` tag then ships a signed installer.
7. Update `macos-signing.md`'s "Parallel Windows signing path" pointer
   (`macos-signing.md:183-194`) to reference this doc as the live procedure.

## References

- CA/Browser Forum Code Signing Baseline Requirements (2023 hardware-key
  mandate): `https://cabforum.org/working-groups/code-signing/`.
- Azure Trusted Signing: `https://learn.microsoft.com/azure/trusted-signing/` and
  the `azure/trusted-signing-action` GitHub Action.
- Tauri 2 Windows signing (`bundle.windows.signCommand`, Azure Trusted Signing):
  `https://v2.tauri.app/distribute/sign/windows/`.
- SSL.com eSigner / DigiCert KeyLocker (cloud-signing fallback CAs).
- `docs/release/macos-signing.md` — the macOS counterpart this mirrors.
- `desktop/src-tauri/tauri.conf.json:65-70` — the `bundle.windows` block to extend.
- `.github/workflows/release-desktop.yml:122-172` — the current unsigned Windows job.
- `.github/workflows/release.yml:497-520,616-622,687-688` — the macOS secret guard,
  the `publish-release` needs, and the `/dl` metadata generators to extend.
