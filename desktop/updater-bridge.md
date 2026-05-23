# Updater Bridge Release Runbook

This runbook covers the one-time updater key bridge after the
production updater pubkey was embedded in
`src-tauri/tauri.conf.json`.

The updater key is separate from Apple Developer ID signing and
notarization. Apple signing controls whether macOS trusts the app
bundle. The updater minisign key controls whether an installed Chan
app accepts a downloaded update payload.

## Bridge Purpose

Existing installs before `3c1435b` trust the old phase-8 DEV updater
pubkey. Current builds embed the production updater pubkey.

The bridge release must therefore have two properties:

1. The app binary contains the production updater pubkey from
   `src-tauri/tauri.conf.json`.
2. The updater payload is signed with the old DEV updater private key.

After users install that bridge release, their local app trusts the
production updater pubkey. Every later updater payload must be signed
with the production updater private key.

Do not sign the bridge updater payload with the production key and
expect old installs to accept it. They will verify against the old DEV
pubkey and reject the update.

## Key And Secret Rules

Private key values and passwords must not be printed, committed, or
pasted into logs.

Public key material is not secret. To inspect the currently embedded
updater pubkey without printing the full key:

```sh
cd desktop
node -e 'const c=require("./src-tauri/tauri.conf.json"); const crypto=require("crypto"); const k=c.plugins.updater.pubkey; console.log(`updater pubkey length=${k.length} sha256=${crypto.createHash("sha256").update(k).digest("hex")}`)'
```

To confirm local key files by presence only:

```sh
test -f ~/.tauri/chan-desktop.key && echo old-dev-key-file-present
test -f ~/.tauri/chan-desktop-prod.key && echo prod-key-file-present
test -f ~/.tauri/chan-desktop-prod.key.pub && echo prod-pubkey-file-present
```

## Local Bridge Signing

Build the bridge app with the current config, which already embeds the
production updater pubkey:

```sh
cd desktop
make app-notarized
```

For a signed but non-notarized app bundle:

```sh
cd desktop
make app-signed
```

The direct Tauri build command shape used by the Makefile is:

```sh
cd desktop/src-tauri
cargo tauri build --bundles app,dmg
```

The local `desktop/Makefile` produces app and DMG bundles. It does not
currently create or publish updater feed files such as `latest.json`,
per-platform payload URLs, or detached signature files. If the release
publisher does not already assemble those artifacts, route CI/publisher
work through @@Desktect and @@Alex rather than editing workflows here.

Sign the bridge updater payload with the old DEV key:

```sh
cd desktop/src-tauri
TAURI_SIGNING_PRIVATE_KEY_PATH="$HOME/.tauri/chan-desktop.key" \
TAURI_SIGNING_PRIVATE_KEY_PASSWORD="" \
cargo tauri signer sign <UPDATE_PAYLOAD_FILE>
```

For a password-protected old DEV key, set
`TAURI_SIGNING_PRIVATE_KEY_PASSWORD` to the password instead of the
empty string. For a no-password key, keep the explicit empty variable;
the Tauri signer can otherwise prompt or fail in non-interactive runs.

Publish the resulting signature in the updater manifest for the bridge
version. A manifest entry has this shape:

```json
{
  "version": "<bridge-version>",
  "notes": "Bridge release carrying the production updater pubkey.",
  "pub_date": "2026-05-23T00:00:00Z",
  "platforms": {
    "darwin-aarch64": {
      "signature": "<old-dev-key-signature>",
      "url": "https://chan.app/dl/desktop/darwin-aarch64/Chan_<bridge-version>_aarch64.app.tar.gz"
    }
  }
}
```

Use the actual bridge version, target, payload URL, and signature
emitted by the release process. The bridge version must be greater
than every installed version that should discover it through the
updater.

## Production Signing After The Bridge

After the bridge has shipped and had enough time to update existing
installs, sign future updater payloads with the production key:

```sh
cd desktop/src-tauri
TAURI_SIGNING_PRIVATE_KEY_PATH="$HOME/.tauri/chan-desktop-prod.key" \
TAURI_SIGNING_PRIVATE_KEY_PASSWORD="$TAURI_SIGNING_PRIVATE_KEY_PASSWORD" \
cargo tauri signer sign <UPDATE_PAYLOAD_FILE>
```

CI may use key contents instead of a filesystem path:

```sh
TAURI_SIGNING_PRIVATE_KEY="<secret key contents>" \
TAURI_SIGNING_PRIVATE_KEY_PASSWORD="<secret password or empty>" \
cargo tauri signer sign <UPDATE_PAYLOAD_FILE>
```

Only the env var names belong in docs and workflow comments. Values
belong in the release owner's local secret store or CI secrets.

## CI Env Vars

Bridge release updater signing:

- `TAURI_SIGNING_PRIVATE_KEY` or `TAURI_SIGNING_PRIVATE_KEY_PATH`:
  old DEV updater private key.
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`: password for the old DEV key,
  or an explicit empty value for a no-password key.

Post-bridge updater signing:

- `TAURI_SIGNING_PRIVATE_KEY` or `TAURI_SIGNING_PRIVATE_KEY_PATH`:
  production updater private key.
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`: password for the production
  key, or an explicit empty value for a no-password key.

macOS packaging and notarization also need the Apple signing variables
documented in `CLAUDE.md`, but those are not updater signature keys.

## Identifying Which Key Signed A Payload

Given an updater payload and its signature, verify against the old DEV
public key and the production public key. The key that verifies the
signature is the signing key.

The production public key is the current
`plugins.updater.pubkey` value in `src-tauri/tauri.conf.json`.

The old DEV public key is the value from the parent of the production
pubkey rotation commit:

```sh
git show 3c1435b^:desktop/src-tauri/tauri.conf.json \
  | node -e 'let s=""; process.stdin.on("data", d => s += d); process.stdin.on("end", () => console.log(JSON.parse(s).plugins.updater.pubkey))'
```

With `minisign` available, write the public key and detached signature
to files, then verify:

```sh
minisign -Vm <UPDATE_PAYLOAD_FILE> -P '<PUBLIC_KEY>' -x <SIGNATURE_FILE>
```

Expected results:

- Bridge payload: verifies with the old DEV public key and fails with
  the production public key.
- Post-bridge payload: verifies with the production public key and
  fails with the old DEV public key.

If the updater manifest stores the signature inline, copy only that
signature string into a temporary signature file for verification.

## Failure Modes

If the old DEV private key is unavailable or its password is unknown,
there is no auto-update bridge from old installs to production-key
installs. Ship the production-key build as a manual DMG update and
expect old installs to reject production-key-signed updater payloads.

If the bridge manifest points to a payload signed with the production
key, old installs reject it.

If the bridge version is not greater than the installed version, old
installs will not request it.

If update-feed generation or publishing needs GitHub Actions changes,
stop and route the change through @@Desktect so @@Alex can coordinate
with chan-core.
