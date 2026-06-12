#!/usr/bin/env bash
# One-time setup: create the `chan` notarytool Keychain profile so
# `xcrun notarytool log`, `xcrun notarytool history`, and
# `make app-notarized` (per .agents/desktop.md) all work locally
# without re-typing Apple ID / team ID / app-specific password.
#
# Sources the app-specific password from the `chan-notary` generic-
# password item already in the maintainer's Keychain (same item
# populate-apple-secrets.sh reads to push the APPLE_PASSWORD GitHub
# Actions secret). Apple ID + team ID match the constants in
# populate-apple-secrets.sh; update both files together if values
# change.
#
# After running this once, the standard notarytool flows from
# .agents/desktop.md ("Local notarization setup" + "Verifying the
# Keychain profile is in place") work as documented.
#
# Idempotent: if the profile already exists, notarytool's
# store-credentials overwrites it cleanly.

set -euo pipefail

APPLE_ID_VALUE='fiorix@gmail.com'
APPLE_TEAM_ID_VALUE='W73XV5CK3N'
KEYCHAIN_NOTARY_ITEM='chan-notary'
PROFILE_NAME='chan'

echo "==> Pre-flight: confirming Keychain has '$KEYCHAIN_NOTARY_ITEM'"
security find-generic-password -s "$KEYCHAIN_NOTARY_ITEM" >/dev/null 2>&1 || {
  echo "ERROR: Keychain item '$KEYCHAIN_NOTARY_ITEM' not found." >&2
  echo "Create via:" >&2
  echo "  security add-generic-password -a $APPLE_ID_VALUE -s $KEYCHAIN_NOTARY_ITEM -w <app-specific-password>" >&2
  exit 1
}

echo "==> Reading app-specific password from Keychain (never echoed)"
PASSWORD="$(security find-generic-password -s "$KEYCHAIN_NOTARY_ITEM" -w | tr -d '\n')"
trap 'unset PASSWORD' EXIT INT TERM

echo "==> Storing notarytool profile '$PROFILE_NAME'"
xcrun notarytool store-credentials "$PROFILE_NAME" \
  --apple-id "$APPLE_ID_VALUE" \
  --team-id "$APPLE_TEAM_ID_VALUE" \
  --password "$PASSWORD"

echo ""
echo "==> Verify the profile is in place"
security find-generic-password -s "com.apple.gke.notary.tool" -a "$PROFILE_NAME" >/dev/null \
  && echo "  Keychain item present."

echo ""
echo "==> Done. You can now run:"
echo "  xcrun notarytool log <submission-id> --keychain-profile $PROFILE_NAME"
echo "  xcrun notarytool history --keychain-profile $PROFILE_NAME"
echo "  make app-notarized   # in desktop/, uses the profile by default"
