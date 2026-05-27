# web-marketing

Source for the public `chan.app` static site. The publishable artifact is
generated into `web-marketing/dist/`; do not publish the source tree directly.

## Layout

```text
web-marketing/
+-- package.json
+-- scripts/
|   `-- build.mjs          static generator and validator
+-- src/
|   +-- install.sh         public CLI installer source
|   +-- pages/             homepage and install page templates
|   +-- site.js
|   +-- styles.css
|   `-- templates/
|       `-- base.html
+-- assets/                public image assets copied to dist/assets/
+-- chan-mark.png
+-- favicon.ico
`-- qr-donate.png
```

Manual source lives in `docs/manual/`. The site build renders that tree to
`/manual/` and nested clean URLs. Manual navigation starts with the links in
`docs/manual/index.md`, then falls back to path order for pages not linked
from the manual landing page.

## Build

```sh
npm run build
```

Run the full local site gate:

```sh
npm run check
```

The build/check gate:

- renders `/`, `/install/`, `/manual/`, nested manual pages, and `/install.sh`
- packages the release manual bundle from generated manual pages
- writes `CNAME` for `chan.app`
- copies static assets into `dist/`
- fails on missing required inputs
- fails on broken local links
- fails if generated pages infer GitHub release asset URLs instead of using
  runtime release metadata hooks
- fails if removed installer references reappear in generated public files
- fails if stale public copy claims reappear in generated output
- dry-runs `/dl/**` release metadata generation from a local fixture
- serves `dist/` on loopback and smokes `/`, `/install/`, `/manual/`,
  `/manual/install/`, `/install.sh`, and `/install.ps1` absence

## Preview

Serve the generated artifact:

```sh
npm run build
python3 -m http.server 8080 -d dist
```

Then open `http://localhost:8080/`.

## Release verification

After a `v*` tag release completes, verify the public release assets:

```sh
npm run verify:release
```

The verifier checks the latest GitHub Release for the desktop downloads,
standalone CLI tarballs, and manual bundle. `VERSION` and `SHA256SUMS` are
checked when present, but `/dl/**` metadata is the source of truth for
downloads and updates.

Generate release metadata from an already verified asset manifest:

```sh
npm run generate:metadata -- \
  --manifest /tmp/chan-release-assets.json \
  --out dist/dl
```

The generator writes:

- `dist/dl/releases.json`
- `dist/dl/cli/latest.json`
- `dist/dl/cli/vX.Y.Z.json`
- `dist/dl/desktop/latest.json`
- `dist/dl/desktop/vX.Y.Z.json`

The manifest must list concrete GitHub Release asset URLs and SHA256 values.
It must not use GitHub `releases/latest/download` URLs.

Build the release manual bundle locally:

```sh
npm run build
npm run bundle:manual
```

While `github.com/fiorix/chan` is still private during pre-release work, use
`--skip-latest-download-heads` for asset-shape checks. The public launch
requires that flag to be absent so unauthenticated latest-download URLs are
checked.

## Install surface

`/install/` is desktop-first. The shell installer is CLI-only and supports
only the active standalone CLI release targets:

- Linux x86_64
- Linux aarch64
- macOS aarch64

Desktop packages are downloaded directly as release artifacts. They are not
installed by `install.sh`.

`install.sh` defaults to `https://chan.app/dl/cli/latest.json`. Download links
on the site read `/dl/releases.json` at runtime and fall back to the GitHub
Releases page if metadata is unavailable.

## Workspace boundary

This site is independent from the Svelte editor app in `web/`. It does not
participate in `cargo build`, `cargo test`, or the embedded editor bundle.
