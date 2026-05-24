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
`/manual/` and nested clean URLs.

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
- fails if generated release links drift away from GitHub latest-download
  URLs
- fails if removed installer references reappear in generated public files
- fails if stale public copy claims reappear in generated output
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

After a `chan-v*` tag release completes, verify the public release assets:

```sh
npm run verify:release
```

The verifier checks the latest GitHub Release for the desktop downloads,
standalone CLI tarballs, `VERSION`, `SHA256SUMS`, the manual bundle, and the
GitHub latest-download URLs used by the public site.

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

`install.sh` defaults to GitHub's real latest-release asset URLs under
`https://github.com/fiorix/chan/releases/latest/download/`. GitHub Pages does
not proxy release artifacts, so the public site must not depend on a
`chan.app/dl/latest/` route unless a static mirror is deliberately added later.

## Workspace boundary

This site is independent from the Svelte editor app in `web/`. It does not
participate in `cargo build`, `cargo test`, or the embedded editor bundle.
