---
name: release
description: >-
  Cut a chan release: bump every version pin, run the full gate, tag,
  and let CI build and publish. Covers the dry-run and the macOS signing path.
when_to_use: >-
  The user asks to cut a release, bump the version, tag a new vX.Y.Z,
  or ship a build to chan.app.
---

# Cut a release

A release is a single annotated tag `vX.Y.Z`. Pushing the tag fires `.github/workflows/release.yml`, which builds the CLI and desktop artifacts, signs and notarizes the macOS desktop build, and publishes the download metadata to chan.app (GitHub Pages).

## Version pins (bump together)

Every pin moves to the same `X.Y.Z` in one commit. Missing one breaks the release at tag time:

- `Cargo.toml` `[workspace.package]` `version` (the workspace source).
- `desktop/src-tauri/tauri.conf.json` `version` (the `.app` bundle).
- `Cargo.lock` (refreshed by a `cargo build` after the bump).
- Any other `version` that ships in the release surface (the marketing site reads the workspace version at build time, so it does not need a separate bump, but confirm nothing else has drifted).

The desktop Rust package version is inherited from the workspace, so the `.app` version and the workspace stay aligned once `tauri.conf.json` matches.

## Self-upgrade is data-driven

Self-upgrade reads the latest manifest from `/dl` on chan.app. Cutting a release auto-supersedes prior versions; there is no `update.rs` edit required. The desktop updater probes the static manifest at `https://chan.app/dl/desktop/latest.json`, generated at release time by `web-marketing/scripts/generate-release-metadata.mjs`.

## Procedure

1. Run the full gate green first (see [gate](../gate/SKILL.md)). The release gate must cover EVERY workspace CI ships, including the separate gateway Cargo workspace. A green core gate that skips gateway can still die at tag time.
2. Bump the version pins above in one commit.
3. Dry-run the release workflow before tagging: trigger `release.yml` via `workflow_dispatch` with `publish=false`. macOS sign/notarize only runs on Actions, so this is the only way to validate that path off a workstation.
4. Push the `vX.Y.Z` tag. The pre-push hook gates every push, including tags. A backgrounded gated push can SIGPIPE (exit 141) and silently fail to update the remote, so push in the foreground, redirect output to a file, and verify with `git ls-remote` before relying on the tag.
5. Watch the workflow. macOS signing secrets are validated up front; a missing secret fails fast with a pointer to `docs/release/macos-signing.md` and the desktop updater notes in `.agents/desktop.md`.

## Signing notes

- macOS Developer ID signing + notarization material lives in GitHub Actions Secrets; the per-secret table is in `docs/release/macos-signing.md`.
- The Tauri updater minisign key is separate from the Apple Developer ID cert. Rotation procedures for both live in `.agents/desktop.md`.
- Secret VALUES never appear in journals, chat, or commits. Only the secret NAMES are referenced in workflow YAML and docs.
