# @chan/marketing

Source for the public `chan.app` static site, a member of the `./web`
npm-workspaces monorepo. The publishable artifact is generated into `dist/`; do
not publish the source tree directly.

## Layout

```text
web/packages/marketing/
+-- package.json
+-- scripts/
|   `-- build.mjs          static generator and validator
+-- src/
|   +-- install.sh              public CLI installer source
|   +-- pages/                  homepage and install page templates
|   +-- site.js
|   +-- styles.css
|   +-- launcher-demo.ts        eager entry: mounts the launcher demo widget
|   +-- workspace-demo.ts       lazy loader for the workspace demo overlay
|   +-- WorkspaceDemoOverlay.svelte  hosts @chan/workspace-app/demo
|   `-- templates/
|       `-- base.html
+-- assets/                     public image assets copied to dist/assets/
+-- chan-favicon.png
`-- chan-mark.png
```

## Build

```sh
npm run build
```

Run the full local site gate:

```sh
npm run check
```

The build/check gate:

- renders `/`, `/install/`, and `/install.sh`
- writes `CNAME` for `chan.app`
- copies static assets into `dist/`
- fails on missing required inputs
- fails on broken local links
- fails if generated pages infer GitHub release asset URLs instead of using runtime release metadata hooks
- fails if removed installer references reappear in generated public files
- fails if stale public copy claims reappear in generated output
- dry-runs `/dl/**` release metadata generation from a local fixture
- dry-runs collection of uploaded release assets into the metadata manifest
- builds the embedded demos (see below) and snapshots the repo into `dist/assets/demo-workspace.json`
- serves `dist/` on loopback and smokes `/`, `/install/`, `/install.sh`, and `/install.ps1` absence

## Preview

Serve the generated artifact:

```sh
npm run build
python3 -m http.server 8080 -d dist
```

Then open `http://localhost:8080/`.

## Embedded demos

The site runs both chan SPAs frontend-only, with no backend, so the landing page
is a live product tour. See the frontend design doc
(`web/packages/workspace-app/src/design.md`, "Frontend-only demo") for the
architecture; the wiring here is:

- The landing page eager-loads `launcher-demo.ts`, which mounts the real
  launcher (`@chan/launcher/demo`) as the hero widget.
- Clicking any window tile fires the launcher demo's `onOpenWindow` hook, which
  dynamic-imports `workspace-demo.ts` and opens the real workspace app
  (`@chan/workspace-app/demo`) in a near-fullscreen overlay. The whole
  workspace-app bundle is a lazy chunk, so it never loads until first click.

`scripts/build.mjs` produces, under `dist/assets/`:

- `launcher-demo.{js,css}` -- the eager launcher entry.
- `workspace-demo.{js,css}` plus split vendor chunks -- the lazy workspace app.
- `demo-workspace.json` -- an in-memory workspace snapshotted from this repo by
  `web/packages/workspace-app/scripts/snapshot-workspace.mjs` (the demo's file
  tree, contents, graph, and search all derive from it).

The build serves the demo chunks from `base: "/assets/"` and scopes each demo
bundle's global CSS to its own frame (`.launcher-demo-frame` /
`.workspace-demo-frame`) so a demo can never restyle the marketing page.

## Release verification

After a `v*` tag release completes, verify the public release assets:

```sh
npm run verify:release
```

The verifier checks the latest GitHub Release for the desktop downloads and standalone CLI tarballs. `VERSION` and `SHA256SUMS` are checked when present, but `/dl/**` metadata is the source of truth for downloads and updates.

Generate release metadata from an already verified asset manifest:

```sh
npm run collect:release -- \
  --tag vX.Y.Z \
  --out /tmp/chan-release-assets.json
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

The manifest must list concrete GitHub Release asset URLs and SHA256 values. It must not use GitHub `releases/latest/download` URLs. The collector builds that manifest from uploaded GitHub Release assets and detached updater signature assets.

A manual Pages deploy (`gh workflow run pages.yml`) ships marketing-only updates between releases. It rebuilds `/dl/**` from the latest GitHub Release rather than reading the live site, so the download page and update-check metadata survive the deploy. The release workflow regenerates `/dl/**` for each new tag; both paths derive the same metadata from GitHub Release assets.

While `github.com/fiorix/chan` is still private during pre-release work, use `--skip-latest-download-heads` for asset-shape checks. The public launch requires that flag to be absent so unauthenticated latest-download URLs are checked.

## Install surface

`/install/` is desktop-first. The shell installer is CLI-only and supports only the active standalone CLI release targets:

- Linux x86_64
- Linux aarch64
- macOS aarch64

Desktop packages are downloaded directly as release artifacts. They are not installed by `install.sh`.

`install.sh` defaults to `https://chan.app/dl/cli/latest.json`. Download links on the site read `/dl/releases.json` at runtime and fall back to the GitHub Releases page if metadata is unavailable.

## Workspace boundary

This site is a member of the `./web` npm-workspaces monorepo, but unlike the
embedded SPAs it is not baked into any binary: it does not participate in `cargo
build`, `cargo test`, or the embedded editor bundle, and it deploys to `chan.app`
via the release/pages workflows.
