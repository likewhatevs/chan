# chan-desktop notes

## Auto-upgrade signing (tauri-plugin-updater)

The desktop app verifies update bundles with a minisign signature.
Pubkey is embedded in `src-tauri/tauri.conf.json` under
`plugins.updater.pubkey`. Matching private key lives outside the
repo at `~/.tauri/chan-desktop.key`.

### Current key is a DEV key

Generated with `cargo tauri signer generate --ci ...` on
2026-05-11. No password. Unencrypted on disk. Anyone with read
access to the dev box's `~/.tauri/` can sign a "valid" update.

### Rotate before any public release

1. On a secure machine, run:
   `cargo tauri signer generate -w <newkey>`
   Set a strong password when prompted.
2. Replace `plugins.updater.pubkey` in `tauri.conf.json` with the
   contents of `<newkey>.pub` and ship a "bridge" release still
   signed with the OLD key (so existing installs accept it). The
   bridge release embeds the NEW pubkey in the binary.
3. Every release after the bridge is signed with the NEW key.
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
