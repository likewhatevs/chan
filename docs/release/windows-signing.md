# Windows code signing (Authenticode)

Reference for signing chan-desktop's Windows installer + binary for public distribution. **Decision (2026-06-22): SSL.com "Personal ID Code Signing" certificate (IV+OV validation), signed from CI via SSL.com's eSigner cloud HSM.** This is the Windows counterpart to `docs/release/macos-signing.md`.

> **Status today: UNSIGNED, not a public asset.** The *Personal ID Code Signing* order (placed 2026-06-21, registrant **Alexandre Fiori**) is purchased and the personal identity is validated, **but the certificate order still shows `pending validation` / not yet issued** — its `perform validation` step + issuance, then eSigner enrollment + the CI wiring below, are still pending. The manual `release-desktop.yml` dry-run builds an unsigned NSIS installer as an Actions artifact only (`release-desktop.yml:122-172`). The tag-triggered `release.yml` ships **no** Windows asset at all — `publish-release` needs Linux + macOS jobs only (`release.yml:616-622`). `desktop/src-tauri/tauri.conf.json:65-70` carries only NSIS install settings — no signing fields. This doc is the plan to change that.

## Decision record — why SSL.com IV (not Azure)

chan ships as an **individual**, not an incorporated org, so the validated publisher users see should be a person's legal name, via **Individual Validation (IV)**.

- **Azure Trusted Signing** was the first pick (cheapest, native GitHub Action). But its identity validation for a person issues an **OV-equivalent** ("public-individual") identity, not a true IV product. After back-and-forth with Azure we concluded it didn't give us the IV path we wanted.
- **SSL.com** sells a **Personal ID Code Signing** certificate (validates at **IV+OV** — the individual's legal name as publisher) and supports cloud signing from CI via **eSigner** — its FIPS-140-2 Level 3 cloud HSM — with no USB token and full GitHub-hosted-runner support. Personal identity validated; the certificate order is **pending order-level validation / issuance** as of 2026-06-22.

A USB hardware token was never viable: a physical token can't attest on a hosted runner (it needs a self-hosted Windows runner with the token plugged in).

## Why sign

An unsigned Windows installer triggers a full-screen **Microsoft Defender SmartScreen** warning ("Windows protected your PC") on download/run, with "More info -> Run anyway" hidden behind a click. Most users abandon at that screen. An Authenticode signature (a) replaces "Unknown publisher" with the verified publisher name (for IV, the individual's validated legal name) in the UAC + SmartScreen prompts, and (b) accrues SmartScreen *reputation* over downloads so the warning eventually disappears.

> IV (like OV) builds SmartScreen reputation **over time** — only **EV** clears the warning instantly. The first downloads of a freshly-signed IV installer may still show SmartScreen until reputation accrues for this signing identity.

## The 2023 key-storage rule (why eSigner, not a `.pfx`)

Since **2023-06-01** the CA/Browser Forum Code Signing Baseline Requirements mandate that every code-signing private key (IV, OV **and** EV) be generated and stored in a FIPS 140-2 Level 2+ hardware token or a cloud HSM. There is no downloadable key file for a new certificate anymore — which kills the "base64 a `.pfx` into a CI secret" model. SSL.com issues IV/OV certs **either** on a FIPS USB token **or** into **eSigner** (cloud HSM). We use eSigner so CI signs without hardware.

## Scope

In scope: Authenticode signing of the chan-desktop `.exe` and its NSIS installer, the GitHub Actions secrets, and the `tauri.conf.json` + workflow changes that make a signed installer a public release asset.

Out of scope (separate briefs): macOS signing/notarization (`macos-signing.md`, already shipping), the Tauri updater `minisign` key (`.agents/desktop.md`), Linux package signing, and a Windows *updater feed* (the auto-update channel; signing the installer is the prerequisite, the feed is a later step).

---

## Provisioning the SSL.com / eSigner path

The cert + key live in SSL.com's cloud HSM; CI authenticates with account credentials + a TOTP secret and runs **CodeSignTool** (SSL.com's signing CLI) or the official GitHub Action. No `.pfx`, no token.

### P.1 Certificate + identity validation — IN PROGRESS

1. **Buy the cert.** Ordered 2026-06-21: *Personal ID Code Signing*, **IV+OV** validation, 1yr, registrant **Alexandre Fiori**. *(done)*
2. **Validate + issue.** The personal identity is validated, but the certificate order still shows **`pending validation`** (`issued on: pending`) — click **`perform validation`** on the order to complete order-level validation and trigger issuance. The validated name (**Alexandre Fiori**) becomes the certificate subject / publisher string users see. *(next — blocks everything downstream)*

### P.2 Enroll the cert in eSigner (do this next)

1. In the SSL.com dashboard, **enroll the IV cert into eSigner** (cloud signing). The key is generated in SSL.com's HSM — you never download it.
2. During enrollment eSigner shows a **QR code** for TOTP-based automation. **Copy and save the TOTP secret string** shown alongside it — this is `ES_TOTP_SECRET`, and it lets CodeSignTool compute the one-time code non-interactively. (Without it CodeSignTool prompts for a manual OTP — unusable in CI.)
3. Note the **`credential_id`** for the cert (the eSigner signing-credential identifier; visible in the dashboard / via CodeSignTool's `get_credential_ids`).

### P.3 GitHub Actions secrets (eSigner path)

`Settings -> Secrets and variables -> Actions -> Repository secrets`. Values live only in GitHub; never in the repo, journals, or chat. These are the names SSL.com's CodeSignTool / `SSLcom/esigner-codesign` Action expect:

| Secret | Holds |
|--------|-------|
| `ES_USERNAME` | SSL.com account username |
| `ES_PASSWORD` | SSL.com account password |
| `CREDENTIAL_ID` | eSigner signing-credential ID for the IV cert |
| `ES_TOTP_SECRET` | the eSigner TOTP secret saved in P.2 |

---

## Wiring it into the build (after eSigner enrollment)

### W.1 Sign during the Tauri bundle (`tauri.conf.json`)

The NSIS installer embeds `chan-desktop.exe`, so the inner binary must be signed **before** the installer is built, then the installer itself signed. Tauri 2's `bundle.windows.signCommand` does both — Tauri invokes it once per artifact, substituting the file path for `%1`. Extend `desktop/src-tauri/tauri.conf.json`'s existing `bundle.windows` block (`tauri.conf.json:65-70`):

```jsonc
"windows": {
  "nsis": { "installMode": "currentUser", "languages": ["English"] },
  "signCommand": {
    "cmd": "pwsh",
    "args": ["-File", "scripts/windows/sign.ps1", "%1"]
  }
}
```

Keep the actual signing logic in `scripts/windows/sign.ps1` (new file) rather than a brittle inline string. That script invokes **CodeSignTool** against eSigner:

```text
CodeSignTool sign \
  -username=$env:ES_USERNAME -password=$env:ES_PASSWORD \
  -credential_id=$env:CREDENTIAL_ID -totp_secret=$env:ES_TOTP_SECRET \
  -input_file_path="%1" -override
```

- **`-override` is mandatory.** CodeSignTool otherwise writes the signed file to an output dir; Tauri expects `%1` signed *in place*, so `-override` overwrites it.
- **Timestamping is automatic.** CodeSignTool "sign and timestamp" uses SSL.com's own TSA — no manual `/tr <url> /td sha256` needed (SSL.com only operates a TSP server; don't point it at a third-party TSA).
- **CodeSignTool needs Java** on the runner, and eSigner runs a **malware scan** before signing (it can refuse to sign a binary it flags). Budget for both.
- `signCommand` runs only when the `ES_*` env is present, so local unsigned dev builds still work.

> Alternative: SSL.com's **eSigner CKA** installs a virtual cert into the Windows cert store so plain `signtool` works — cleaner if we ever want Tauri's built-in `certificateThumbprint` path instead of `signCommand`. CodeSignTool is the documented CI route, so lead with it.

### W.2 The dry-run job (`release-desktop.yml`)

Mirror the macOS "verify secrets present" guard (`release-desktop.yml:76-99`) for Windows, then make the existing `windows-latest` build step (`release-desktop.yml:122-133`) sign by exporting the `ES_*` env to it (so `sign.ps1` runs inside the bundle). Shape:

```yaml
- if: matrix.os == 'windows-latest'
  name: Verify Windows signing secrets present
  shell: bash
  env:
    ES_USERNAME: ${{ secrets.ES_USERNAME }}
    ES_PASSWORD: ${{ secrets.ES_PASSWORD }}
    CREDENTIAL_ID: ${{ secrets.CREDENTIAL_ID }}
    ES_TOTP_SECRET: ${{ secrets.ES_TOTP_SECRET }}
  run: |
    set -e
    missing=()
    for v in ES_USERNAME ES_PASSWORD CREDENTIAL_ID ES_TOTP_SECRET; do
      [ -n "${!v}" ] || missing+=("$v")
    done
    if [ ${#missing[@]} -gt 0 ]; then
      echo "::error::Missing Windows signing secrets: ${missing[*]}"
      echo "::error::Populate per docs/release/windows-signing.md."
      exit 1
    fi
```

The build step at `:122-133` already runs `cargo tauri build --bundles nsis`; once `signCommand` is set (W.1) and the `ES_*` env is exported to that step (plus Java available for CodeSignTool), the bundle comes out signed. Keep the headless `/api/health` smoke (`:135-166`) as-is.

### W.3 Make it a public asset (`release.yml`)

The tag-triggered release ships no Windows asset today. To change that:
1. Add a `windows-desktop-artifacts` job to `release.yml` modeled on the macOS desktop job (`release.yml:~460-611`): checkout, toolchain, node, tauri-cli, Java setup, verify-secrets, build+sign (W.1/W.2), then a **verify-signature** step (`signtool verify /pa /v <installer>` — the Windows analogue of the macOS `codesign --verify` + `spctl` at `release.yml:539-546`), stage into `release-artifacts/`, and `upload-artifact` as `release-windows-desktop`.
2. Add `windows-desktop-artifacts` to the `publish-release` `needs:` list (`release.yml:616-622`) so the GitHub Release waits for it. The `release-*` download pattern (`release.yml:638`) already picks it up.
3. Teach the `/dl` metadata generators about the Windows asset — the Pages job runs `collect-release-assets.mjs` + `generate-release-metadata.mjs` (`release.yml:687-688`); the CLI self-upgrade + desktop updater feeds read that metadata, so the Windows installer must appear there for download links to work.

### W.4 Single-source the secret guard (optional, matches repo norm)

`release.yml`'s combined "Verify signing secrets present" (`release.yml:497-520`) checks all macOS + updater secrets in one step. Add the `ES_*` secrets to that same guard in the Windows job rather than duplicating logic, so a renamed secret fails fast with a pointer here.

---

## Verifying a signature

On any Windows box (or the CI runner):

```powershell
signtool verify /pa /v path\to\Chan_x.y.z_x64-setup.exe
# or:
Get-AuthenticodeSignature path\to\Chan_x.y.z_x64-setup.exe | Format-List
```

A good result shows `Status: Valid`, the verified publisher (your validated IV name), `SHA256` digest, and a countersignature/timestamp. Then sanity-check the *human* experience: download the installer through a browser on a clean Windows VM and confirm the SmartScreen/UAC prompt names the publisher (reputation still builds over the first downloads even with a valid signature).

## Maintenance

- **Renewal:** an SSL.com IV cert renews yearly — re-validate identity + refresh the eSigner enrollment; the GitHub secrets change only if `CREDENTIAL_ID` / the TOTP secret rotate.
- **Timestamps** (automatic via SSL.com's TSA) keep already-released installers valid past cert expiry.
- **Reputation** is per signing-identity. Keep the same IV identity across releases; switching certs resets SmartScreen reputation.

## Provisioning checklist

One-time, in order:

1. Buy the cert (*Personal ID Code Signing*, IV+OV) + complete validation. *(ordered 2026-06-21, identity validated; order still `pending validation` — click `perform validation` to issue)*
2. **Enroll the cert in eSigner**; save the **TOTP secret** and note `credential_id` (P.2). *(next)*
3. Add `ES_USERNAME`, `ES_PASSWORD`, `CREDENTIAL_ID`, `ES_TOTP_SECRET` under repo `Settings -> Secrets and variables -> Actions -> Repository secrets` (P.3).
4. Add `scripts/windows/sign.ps1` (CodeSignTool + `-override`) + the `signCommand` block to `tauri.conf.json` (W.1).
5. Wire the dry-run job + Java setup (W.2); run `release-desktop.yml` via `workflow_dispatch` and confirm the artifact is signed (`signtool verify`).
6. Add the public `windows-desktop-artifacts` job + `publish-release` need + `/dl` metadata (W.3); the next `vX.Y.Z` tag then ships a signed installer.
7. Update `macos-signing.md`'s "Parallel Windows signing path" pointer (`macos-signing.md:183-194`) to reference this doc as the live procedure.

## References

- SSL.com **IV Code Signing**: `https://www.ssl.com/products/software-integrity/code-signing/iv/`.
- SSL.com **eSigner** cloud signing service: `https://www.ssl.com/products/software-integrity/signing-service/`.
- eSigner + **GitHub Actions** how-to: `https://www.ssl.com/how-to/cloud-code-signing-integration-with-github-actions/` and the `SSLcom/esigner-codesign` Action (`uses: SSLcom/esigner-codesign@develop`).
- **CodeSignTool** command guide (flags incl. `-override`, `-totp_secret`, `get_credential_ids`): `https://www.ssl.com/guide/esigner-codesigntool-command-guide/`.
- Tauri 2 Windows signing (`bundle.windows.signCommand`): `https://v2.tauri.app/distribute/sign/windows/`; SSL.com + Tauri worked example: `https://gist.github.com/thewh1teagle/a89d1bc44353c9da1d1198b265da8806`.
- CA/Browser Forum Code Signing Baseline Requirements (2023 hardware-key mandate): `https://cabforum.org/working-groups/code-signing/`.
- Azure Trusted Signing (considered, rejected — OV-equivalent for individuals): `https://learn.microsoft.com/azure/trusted-signing/`.
- `docs/release/macos-signing.md` — the macOS counterpart this mirrors.
- `desktop/src-tauri/tauri.conf.json:65-70` — the `bundle.windows` block to extend.
- `.github/workflows/release-desktop.yml:122-172` — the current unsigned Windows job.
- `.github/workflows/release.yml:497-520,616-622,687-688` — the macOS secret guard, the `publish-release` needs, and the `/dl` metadata generators to extend.
