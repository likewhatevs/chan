#!/usr/bin/env bash
# Populate the six Apple signing / notarization secrets into the chan
# repo's GitHub Actions Secrets, sourcing values from the local
# macOS Keychain.
#
# Per docs/release/macos-signing.md, release CI consumes
# these six secrets to build + sign + notarize the chan-desktop DMG on an
# approved vX.Y.Z release cut.
#
# This script is the one-shot setup helper the maintainer runs ONCE after
# their local Keychain has:
#  - the Developer ID Application cert + private key imported
#  - a generic Keychain item named `chan-notary` storing the
#    app-specific password for notarytool, with account = Apple ID
#    email
#
# Values pipe through stdin direct to `gh secret set`. Nothing
# sensitive is echoed to stdout / written to disk persistently. The
# .p12 temp file is removed at the end.
#
# Prereqs:
#  - `gh` CLI installed + authenticated (gh auth status -> ✓)
#  - macOS Keychain unlocked (default when logged in)
#  - The cert + chan-notary item already in the Keychain
#
# Macros below are CURRENT values from the maintainer's setup as of
# 2026-05-21. Update before running if any change.

set -euo pipefail

APPLE_SIGNING_IDENTITY_VALUE='Developer ID Application: Alexandre Fiori (W73XV5CK3N)'
APPLE_TEAM_ID_VALUE='W73XV5CK3N'
APPLE_ID_VALUE='fiorix@gmail.com'
KEYCHAIN_NOTARY_ITEM='chan-notary'

echo "==> Pre-flight checks"
gh auth status >/dev/null 2>&1 || {
  echo "ERROR: gh CLI not authenticated. Run: gh auth login" >&2
  exit 1
}
security find-generic-password -s "$KEYCHAIN_NOTARY_ITEM" >/dev/null 2>&1 || {
  echo "ERROR: Keychain item '$KEYCHAIN_NOTARY_ITEM' not found." >&2
  echo "Create via:" >&2
  echo "  security add-generic-password -a $APPLE_ID_VALUE -s $KEYCHAIN_NOTARY_ITEM -w <app-specific-password>" >&2
  exit 1
}
security find-identity -v -p codesigning | grep -q "$APPLE_TEAM_ID_VALUE" || {
  echo "ERROR: Developer ID Application cert with Team ID $APPLE_TEAM_ID_VALUE not in Keychain." >&2
  exit 1
}

echo "==> 1/6 APPLE_SIGNING_IDENTITY"
printf '%s' "$APPLE_SIGNING_IDENTITY_VALUE" | gh secret set APPLE_SIGNING_IDENTITY

echo "==> 2/6 APPLE_TEAM_ID"
printf '%s' "$APPLE_TEAM_ID_VALUE" | gh secret set APPLE_TEAM_ID

echo "==> 3/6 APPLE_ID"
printf '%s' "$APPLE_ID_VALUE" | gh secret set APPLE_ID

echo "==> 4/6 APPLE_PASSWORD (piped from Keychain '$KEYCHAIN_NOTARY_ITEM' item)"
security find-generic-password -s "$KEYCHAIN_NOTARY_ITEM" -w | tr -d '\n' | gh secret set APPLE_PASSWORD

echo "==> Generating fresh strong .p12 export passphrase (40 chars, alnum)"
PASSPHRASE="$(openssl rand -base64 32 | tr -d '\n=+/' | head -c 40)"
[ "${#PASSPHRASE}" -ge 32 ] || {
  echo "ERROR: passphrase generation produced fewer chars than expected." >&2
  exit 1
}

P12_PATH="$(mktemp -t chan-developerid.p12.XXXXXX)"
trap 'rm -f "$P12_PATH"; unset PASSPHRASE' EXIT INT TERM

echo "==> Exporting .p12 from login keychain (Keychain may prompt; click 'Always Allow' / 'Allow')"
security export -k "$HOME/Library/Keychains/login.keychain-db" -t identities -f pkcs12 \
  -P "$PASSPHRASE" -o "$P12_PATH"

echo "==> 5/6 APPLE_CERTIFICATE_PASSWORD"
printf '%s' "$PASSPHRASE" | gh secret set APPLE_CERTIFICATE_PASSWORD

echo "==> 6/6 APPLE_CERTIFICATE_BASE64"
base64 -i "$P12_PATH" | tr -d '\n' | gh secret set APPLE_CERTIFICATE_BASE64

echo ""
echo "==> Verify (NAMES only, values never logged)"
gh secret list

echo ""
echo "==> Done. Release CI macOS signing is now unblocked."
