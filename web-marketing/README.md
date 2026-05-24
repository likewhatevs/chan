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

The build:

- renders `/`, `/install/`, `/manual/`, nested manual pages, and `/install.sh`
- writes `CNAME` for `chan.app`
- copies static assets into `dist/`
- fails on missing required inputs
- fails on broken local links
- fails if removed installer references reappear in generated public files

## Preview

Serve the generated artifact:

```sh
npm run build
python3 -m http.server 8080 -d dist
```

Then open `http://localhost:8080/`.

## Install surface

`/install/` is desktop-first. The shell installer is CLI-only and supports
only the active standalone CLI release targets:

- Linux x86_64
- Linux aarch64
- macOS aarch64

Desktop packages are downloaded directly as release artifacts. They are not
installed by `install.sh`.

## Workspace boundary

This site is independent from the Svelte editor app in `web/`. It does not
participate in `cargo build`, `cargo test`, or the embedded editor bundle.
