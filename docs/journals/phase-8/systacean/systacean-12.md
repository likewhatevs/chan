# systacean-12: Verify tauri-plugin-updater works on all three platforms

Owner: @@Systacean
Date: 2026-05-20

## Goal

Confirm `tauri-plugin-updater` (chan-desktop's self-update
mechanism) functions correctly on macOS + Linux + Windows.
Round-2 ships v0.12.0 as the first proper release; users
launching v0.12.0 should be able to receive future v0.12.x /
v0.13.0 / v1.0 updates without re-downloading the installer
manually. This task validates that the plumbing is intact.

## Background

* **Round-2 plan**: [`../architect/round-2-plan.md`](../architect/round-2-plan.md)
  §"North-star through-line" lists this as the item 7 prereq.
* **chan-desktop today**: includes `tauri-plugin-updater`
  in the dependency tree (per phase-7 work). The plugin needs:
  * A signing key (the **minisign** updater key, distinct from
    the macOS Developer ID cert) — `@@Alex` pre-authorized
    rotation per the secrets memory.
  * An update-feed endpoint (JSON describing latest version +
    download URLs) — likely hosted at `chan.app/updates/latest`
    or a GitHub Releases-derived path; the exact shape is
    Round-2 work.
  * Per-platform installer formats compatible with the
    plugin's auto-apply: `.app.tar.gz` (macOS), `.AppImage` /
    `.deb` (Linux), `.msi` / `.exe` (Windows).
* **What this task is NOT**: it's NOT setting up the update
  feed (that's a later Round-2 task once chan.app is hosted)
  + it's NOT shipping v0.11.x → v0.12.0 updates (this task is
  pre-flight verification only). It's: does the plugin's
  check-for-updates + download + verify-signature + apply
  pathway work on each platform, end-to-end, against a mock
  update feed.

## Authorization

**Authorization: yes**, this task covers edits to
`desktop/src-tauri/tauri.conf.json` (updater config block),
`desktop/src-tauri/Cargo.toml` (if plugin version needs a
bump), a new mock update feed (likely a static file or a tiny
local server for testing), and any `desktop/CLAUDE.md`
documentation updates capturing the verification procedure.
The minisign updater key is a Round-2 concern (rotation TBD);
this task uses a test key for the dry-run. @@Systacean may
proceed without further in-chat confirmation from @@Alex.

## Acceptance criteria

* Mock update feed published (static JSON, served via `python3
  -m http.server` or equivalent — local-only).
* chan-desktop launches against the mock feed; updater plugin
  detects "newer version available"; downloads; verifies
  signature against the test minisign public key; applies the
  update; relaunches successfully.
* Verified on macOS (lane @@Alex's primary). Verified on Linux
  + Windows via VM or @@Alex's secondary machines (coordinate
  on which environments are available).
* Failure modes exercised: invalid signature (rejected with
  named error), corrupted download (rejected), version
  downgrade attempt (rejected per the plugin's defaults).
* Documentation updated in `desktop/CLAUDE.md` (or a new
  `desktop/UPDATER.md`) capturing: how to test against a mock
  feed, the production-feed shape that Round-2 will land,
  rotation procedure for the minisign key.
* Pre-push gate: clean.

## How to start

1. Read the `tauri-plugin-updater` upstream docs:
   https://v2.tauri.app/plugin/updater/ (verify the URL
   resolves; if outdated, the plugin's crate-doc on docs.rs is
   authoritative).
2. Inspect chan-desktop's current updater config in
   `tauri.conf.json` + `Cargo.toml`. Identify gaps.
3. Generate a test minisign keypair (don't commit either
   half; the test private key stays in your local working
   dir + the public key gets temporarily baked into the test
   chan-desktop build).
4. Build a tiny mock-feed JSON describing a fake v0.99.0
   release.
5. Build + launch chan-desktop pointed at the mock feed.
6. Walk the update flow. Capture findings.
7. Iterate per-platform.

## Coordination

* **Independent of `systacean-11`**: signing-key rotation is a
  different key (Apple Developer ID for the macOS installer;
  minisign for the updater itself). Both rotations can land
  in either order.
* **Feeds Round-2 follow-up tasks**: once the update feed
  endpoint is decided (likely after item 6 lands chan.app
  hosting), a subsequent task wires the real feed URL into
  chan-desktop's release config.
* **Cross-platform verification**: if Linux + Windows
  verification needs hands-on time on physical machines or
  VMs that @@Alex owns, fire a permission event to
  coordinate the test session.

## Open questions

(populated as you investigate)
