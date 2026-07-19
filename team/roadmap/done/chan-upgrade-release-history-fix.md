# Keep the Last 5 CLI Upgrade Versions Available

> Status: shipped in [v0.71.0](../../release/release-v0.71.0.md).

## Summary

`chan upgrade --version X.Y.Z` already resolves a concrete metadata URL:

```text
https://chan.app/dl/cli/vX.Y.Z.json
```

The bug is on the publishing side. Each release rebuilds the entire Pages artifact and regenerates `/dl` from only the just-published release, so older `cli/vX.Y.Z.json` files disappear from the published site. Preserve the latest 5 GA releases in generated `/dl` metadata so the latest version and 4 previous GA versions remain upgradeable by explicit version.

Do not change the Rust CLI protocol for this fix. Keep `latest.json` as the default update channel, and keep prereleases out of the Pages `/dl` metadata.

## Current Contract

- `crates/chan/src/update.rs`
  - `chan upgrade` without `--version` fetches `/dl/cli/latest.json`.
  - `chan upgrade --version 0.14.0` fetches `/dl/cli/v0.14.0.json`.
  - The client validates the returned metadata's `version`, `tag`, HTTPS URLs, target list, and SHA256 values.
- `web/packages/marketing/scripts/generate-release-metadata.mjs`
  - Consumes one release asset manifest.
  - Writes `releases.json`, `cli/latest.json`, `cli/vX.Y.Z.json`, `desktop/latest.json`, and `desktop/vX.Y.Z.json`.
  - Currently puts exactly one release in `releases.json.releases`.
- `.github/workflows/release.yml`
  - The GA-only Pages job runs `collect-release-assets --tag "$TAG"` then `generate-release-metadata --out dist/dl`.
  - The deploy replaces the full published site, so omitted older `/dl` files are removed.
- `web/packages/marketing/scripts/preserve-release-metadata.mjs`
  - Rebuilds `/dl` for manual marketing-only Pages deploys.
  - It also regenerates from the latest GitHub Release only today.

## Implementation Plan

### 1. Teach asset collection to collect release history

Update `web/packages/marketing/scripts/collect-release-assets.mjs`:

- Add an option such as `--latest-count N`.
- Default remains the existing single-release behavior unless this option is passed.
- When `--latest-count 5 --tag vX.Y.Z` is used:
  - Fetch the requested tag first.
  - Fetch the GitHub releases list.
  - Filter to GA tags only: `vX.Y.Z` with no prerelease suffix and not marked prerelease.
  - Sort newest first using GitHub release order, with the requested tag forced to the front because it is the release being published.
  - Deduplicate by tag.
  - Collect up to 5 releases.
- When `--latest-count 5` is used without `--tag`:
  - Fetch the GitHub releases list.
  - Filter to GA releases.
  - Collect the newest 5.
- Preserve the current fixture path:
  - `--release-json` plus `--asset-dir` should still collect exactly that one fixture release unless the test explicitly passes a new fixture-history mode.

Use release asset checksums efficiently:

- If the GitHub Release asset includes a `digest` string in `sha256:<hex>` form, use it as the SHA256.
- Fall back to downloading and hashing asset bytes for local fixtures or old API responses that do not include `digest`.
- Keep detached updater `.sig` assets downloaded/read because the desktop metadata needs the signature body.

Output shape:

- For one release, keep the current manifest object shape for compatibility.
- For history mode, write a JSON array of manifest objects newest first.

### 2. Teach metadata generation to emit all retained version files

Update `web/packages/marketing/scripts/generate-release-metadata.mjs`:

- Accept either:
  - the current single manifest object, or
  - an array of manifest objects.
- Normalize and validate every manifest with the existing rules.
- Treat the first manifest as latest.
- Build metadata for every manifest.
- Write:
  - `dist/dl/cli/latest.json` from the first manifest.
  - `dist/dl/desktop/latest.json` from the first manifest.
  - `dist/dl/cli/vX.Y.Z.json` for each retained manifest.
  - `dist/dl/desktop/vX.Y.Z.json` for each retained manifest.
  - `dist/dl/releases.json` containing all retained release entries, newest first.
- Keep `releases.json.latest` and `releases.json.latest_tag` pointed at the first manifest.
- Keep each release entry's `cli` and `desktop` fields pointing at `/dl/cli/vX.Y.Z.json` and `/dl/desktop/vX.Y.Z.json`.

Validation rules:

- Reject duplicate versions or duplicate tags in a history manifest.
- Require the first manifest to be the newest/latest release for the generated metadata.
- Keep the existing concrete URL checks: no GitHub `releases/latest/download` URLs and HTTPS only.

### 3. Wire release and Pages workflows to history mode

Update `.github/workflows/release.yml` in the GA-only `pages-artifact` job:

```sh
node scripts/collect-release-assets.mjs \
  --tag "$TAG" \
  --latest-count 5 \
  --out /var/tmp/chan-release-assets.json
node scripts/generate-release-metadata.mjs \
  --manifest /var/tmp/chan-release-assets.json \
  --out dist/dl
```

Update `web/packages/marketing/scripts/preserve-release-metadata.mjs`:

- Use `collect-release-assets.mjs --latest-count 5`.
- Preserve the existing optional `--tag` passthrough.
- Keep `--allow-missing-release` behavior for pre-first-release site builds.

Update help text for both scripts so operators know that `/dl` retains latest plus 4 previous GA releases.

### 4. Keep the Rust CLI unchanged except optional diagnostics

No required Rust change:

- The client already asks for the concrete retained URL.
- A retained version succeeds.
- A version outside the retained window still fails with the current fetch/HTTP context.

Optional small improvement:

- If desired, improve the `--version` failure message to say the requested version may be outside the retained release window. Do this only if it can be added without masking HTTP status or metadata validation errors.

## Tests

Add focused marketing script coverage:

- Extend `smoke-release-metadata.mjs` to generate 6 synthetic release manifests from the existing `v0.15.4` fixture.
- Pass an array of those manifests to `generate-release-metadata.mjs`.
- Assert:
  - only the newest 5 versions are emitted.
  - `cli/latest.json` equals the newest retained `cli/vX.Y.Z.json`.
  - the oldest excluded version has no `cli/vX.Y.Z.json` file.
  - `desktop/vX.Y.Z.json` files exist for retained versions.
  - `releases.json.releases` has 5 entries, newest first.
  - `releases.json.latest` and `latest_tag` match the newest entry.
  - install-page metadata selection remains compatible because `site.js` already chooses `metadata.latest`.

Add collector coverage:

- Extend `smoke-release-assets-manifest.mjs` with an asset carrying `digest: "sha256:<hex>"`.
- Assert the manifest uses that digest value without requiring the asset bytes for that checksum path.
- Keep an existing fixture asset without `digest` to prove fallback hashing still works.

Run:

```sh
cd web/packages/marketing
npm run check
```

If Rust diagnostics are changed, also run:

```sh
cargo test -p chan update
```

## Assumptions and Defaults

- "Last 5 versions" means latest plus 4 previous GA releases.
- RC/prerelease metadata stays out of Pages `/dl`; prerelease GitHub Release assets can exist, but they must not drive self-upgrade or desktop-updater channels.
- Retention is count-based, not time-based.
- The published site remains static. The release workflow computes and writes all retained files at deploy time.
- No new runtime dependencies are introduced.
