# Phase 10 Roadmap Track B: Public Site, Manual Pages, Install Split

Status: in progress.

Track B turns `web-marketing` into the public site source of truth,
publishes `docs/manual/` as public documentation, and cleans up the
install surface now that desktop and CLI have different release shapes.

## Current wave record

Landed in `915f5d9`:

- Added the `web-marketing` static generator and GitHub Pages workflow.
- Added the initial public manual under `docs/manual/`.
- Removed the public Windows installer surface.
- Rewired `chan upgrade` to GitHub Releases under `github.com/fiorix/chan`.

Follow-up in this wave:

- `chan.app` is a static GitHub Pages site. It serves the generated site and
  `/install.sh`.
- First-install binaries are fetched from GitHub Releases via
  `https://github.com/fiorix/chan/releases/latest/download/<asset>`.
- `/dl/latest/` is not part of the current public contract. Add it only if a
  generated static mirror becomes a deliberate later choice.
- The site generator validates generated release links, rejects stale
  `/dl/latest/` and `chan-writer/chan` public links, and verifies the
  `install.sh` default `BASE`.
- `.github/workflows/release.yml` now follows the fresh release contract:
  `chan-v*` tags only, no Windows or zip release path, no old `/dl/` mirror
  contract, and a `chan-manual-<version>.tar.gz` bundle built from
  `web-marketing/dist/`.
- The public desktop asset names were checked against the existing
  `chan-v0.14.0` GitHub Release, and `release-desktop.yml` now verifies those
  names before upload.
- `npm run verify:release` verifies the latest GitHub Release after a tag
  completes, including desktop downloads, standalone CLI tarballs, `VERSION`,
  `SHA256SUMS`, the manual bundle, and latest-download URLs.
- Because `github.com/fiorix/chan` is still private during pre-release work,
  current unauthenticated latest-download URL checks return 404. Use
  `--skip-latest-download-heads` only for private-repo asset-shape checks.
- The site generator now rejects stale generated public copy for CLI-only
  status, assistant-pane language, unaudited "no telemetry" claims, old org
  links, and old `/dl/` release routes.
- `npm run check` is now the single local and CI site gate: script syntax
  checks, static site build, generated link/copy/release-contract validation,
  and shell syntax for `dist/install.sh`.
- Committed follow-up checkpoints:
  - `7b6167f` guards generated public copy against stale claims.
  - `f949bcd` routes CI and Pages through `npm run check`.

Latest local checks:

- `npm run build` in `web-marketing/`
- `sh -n web-marketing/dist/install.sh`
- `git diff --check` on the Track B follow-up files
- YAML parse of `.github/workflows/release.yml`
- YAML parse of `.github/workflows/release-desktop.yml`
- Local manual-bundle tar smoke using `web-marketing/dist/`
- `gh api` asset-name check against `chan-v0.14.0`
- `npm run verify:release -- --allow-missing-manual
  --skip-latest-download-heads` against current latest release. This passes
  the release asset checks and reports the expected missing manual bundle
  because `chan-v0.14.0` predates this wave.
- `npm run build` verifies the stale-copy guards against generated output.
- `npm run check` in `web-marketing/`

Next wave:

- Add a local HTTP smoke for the generated `web-marketing/dist/` routes in
  the Track B test plan.
- Run `npm run verify:release` without `--allow-missing-manual` and without
  `--skip-latest-download-heads` after the next `chan-v*` tag includes the
  manual bundle and the repo is public.

## Objectives

- Generate the public website from in-repo sources.
- Compile `docs/manual/**/*.md` into pages at `chan.app/manual/`.
- Make desktop the primary public install path after Track A lands.
- Keep shell installation for the standalone CLI only.
- Remove Windows install support and the PowerShell installer.

## 1. Static site build

Starting state:

- `web-marketing/` is a copied static site with one `index.html`.
- There is no build step, no generated `dist/`, and no Pages workflow.
- `docs/manual/` does not exist yet.
- The homepage copy still describes stale product state, including an
  in-app assistant pane and CLI-only status.

Target state:

- `web-marketing/` has a small static build pipeline.
- The build produces `web-marketing/dist/` as the only publishable
  artifact.
- The public site remains independent from the Svelte editor app in
  `web/`.
- The generator renders the homepage, install page, manual pages, static
  assets, and public installer script into `dist/`.

Implementation notes:

- Prefer a small local Node build script and plain templates.
- Do not introduce a full application framework unless the static generator
  becomes unmaintainable.
- Keep assets and templates under `web-marketing/`.
- Keep generated output ignored by git.
- The build should fail on missing required inputs, broken local links, and
  invalid manual front matter if front matter is introduced later.

## 2. Manual publishing

Source contract:

- `docs/manual/` is the canonical manual source.
- `docs/manual/index.md` is required.
- Each markdown file renders to a clean URL under `/manual/`.
- Titles come from the first H1 unless a later front matter contract is
  deliberately added.
- Dotfiles and underscore-prefixed files are ignored.

Output contract:

- `/manual/` renders the manual landing page.
- `/manual/<page>/` renders nested manual pages.
- Manual navigation is derived from the markdown file tree.
- The same source tree is used by Track A for first-launch desktop seeding.
- Public manual pages and desktop-seeded manual content must not diverge.

Content expectations:

- Start with enough manual content to support first launch:
  - install choices
  - creating or opening a drive
  - editing markdown
  - wiki-links
  - search and graph basics
  - terminal and MCP discovery basics
  - tunnel basics
  - upgrade and troubleshooting
- Keep the manual factual and product-current.
- Do not document removed in-app assistant APIs or Windows install paths.

## 3. Install surface

Public UX:

- Make the main homepage install call-to-action lead to `/install/`.
- Make `/install/` desktop-first.
- Show CLI install as a clear secondary path for terminal and server users.
- Keep direct download links visible for users who do not want scripts.

Desktop install:

- Show active desktop artifacts:
  - macOS DMG
  - Linux AppImage
  - Linux deb
- Keep desktop packaging owned by `release-desktop.yml`.
- Do not install desktop packages through `install.sh`.
- Do not make site publishing depend on desktop packaging jobs.

CLI install:

- Keep `/install.sh` as the POSIX installer for the standalone `chan` CLI.
- `/install.sh` is first-install only. It is served by `chan.app`, but its
  default binary download base is GitHub's native latest-release asset route:
  `https://github.com/fiorix/chan/releases/latest/download/`.
- `BASE` overrides point at a directory containing release assets, for
  example
  `https://github.com/fiorix/chan/releases/download/chan-v0.14.0`.
- Support only active standalone CLI release targets:
  - Linux x86_64
  - Linux aarch64
  - macOS aarch64
- Keep `PREFIX` and `BASE` overrides.
- Keep unsupported OS and arch failures explicit.
- Do not add macOS x86_64 support.
- Do not add Windows support.

Removed install surface:

- Remove `/install.ps1` from the site source and public links.
- Remove Windows install copy from `web-marketing/README.md`.
- Remove Windows download claims from homepage and install pages.
- Keep any historical docs factual if they mention the old installer.

## 4. Site copy cleanup

- Replace stale "CLI only" status once desktop-first install is real.
- Remove stale in-app assistant pane language.
- Describe current external-agent behavior through MCP discovery variables.
- Keep the local-first claim precise:
  - local server by default
  - no account for local use
  - tunnel is opt-in
  - no telemetry claim only if still true after audit
- Keep install labels aligned with actual release artifacts.
- Keep `[[` language aligned with Track A's file/path picker contract.

## 5. CI and deployment

CI validation:

- Add a CI job that builds `web-marketing/dist/`.
- Validate generated local links.
- Validate that no Windows install links or PowerShell installer references
  remain in generated public pages.
- Validate that `/install.sh`, `/manual/`, and `/install/` are present.

GitHub Pages:

- Publish `web-marketing/dist/` to GitHub Pages.
- Include `CNAME` for `chan.app` in the published artifact.
- Keep Pages hosting as the only new hosting dependency.
- GitHub Pages serves the static site and `/install.sh`; it does not proxy
  GitHub Release assets.
- Do not depend on `/dl/latest/` under `chan.app` unless a static mirror is
  explicitly generated later.
- DNS cutover remains an operational step, not a build-script side effect.

Release integration:

- GitHub repository URLs now use the personal repo
  `github.com/fiorix/chan`, not the old `chan-writer` org.
- Keep `.github/workflows/release.yml` for standalone CLI artifacts.
- Keep `.github/workflows/release-desktop.yml` for desktop artifacts.
- First-install direct links and `install.sh` use GitHub's latest-download
  route:
  `https://github.com/fiorix/chan/releases/latest/download/<asset>`.
- `chan upgrade` uses GitHub Releases directly:
  - latest probe: `https://api.github.com/repos/fiorix/chan/releases/latest`
  - download base:
    `https://github.com/fiorix/chan/releases/download/chan-v<version>/`
  - release tags must use `chan-v<version>`.
  - `chan upgrade --version` takes a bare version such as `0.14.0`.
- Do not keep pre-release compatibility paths for old tag names, Windows
  release artifacts, or `/dl/v<version>/` upgrade URLs.
- Attach a release-tag manual bundle generated from the same `docs/manual/`
  source as `chan-manual-<version>.tar.gz`.
- Verify that latest-download GitHub URLs used by the site match the active
  first-install artifacts.
- Verify that GitHub Release URLs used by `chan upgrade` match the active
  standalone CLI artifacts.

## Interfaces

Public routes:

- `/`
- `/install/`
- `/manual/`
- `/manual/<page>/`
- `/install.sh`

Removed public route:

- `/install.ps1`

Source layout:

- `web-marketing/src/` holds templates and site source.
- `web-marketing/assets/` holds public image assets.
- `docs/manual/` holds markdown manual source.
- `web-marketing/dist/` is generated and ignored by git.

## Test plan

Local checks:

- Run the `web-marketing` build.
- Serve `web-marketing/dist/` locally.
- Verify `/`, `/install/`, `/manual/`, one nested manual page, and
  `/install.sh`.
- Verify `/install.ps1` is absent.
- Verify no generated page links to Windows installers.

CI checks:

- Build the site and manual on pull requests and pushes.
- Check generated local links.
- Check generated output contains `CNAME`.
- Check generated output contains no Windows install links.
- Check `install.sh` still passes shell syntax validation.

Release checks:

- Verify desktop links resolve to DMG, AppImage, and deb artifacts.
- Verify CLI links resolve to Linux x86_64, Linux aarch64, and macOS
  aarch64 artifacts.
- Verify `install.sh` installs the CLI from GitHub's latest-download route.
- Verify `chan upgrade` resolves `chan-v<version>` GitHub Release assets,
  verifies `SHA256SUMS`, and rejects unsupported targets.
- Verify the manual bundle is generated from the same `docs/manual/`
  source as the public manual pages.

## Assumptions and non-goals

- GitHub Pages with custom domain `chan.app` remains the hosting target.
- Track B does not support Windows.
- Track B does not add macOS x86_64 support.
- Phase 10 is pre-release work. Do not add migration or compatibility code
  for older unpublished installer or upgrade schemes.
- Desktop is the primary public install path after Track A.
- `install.sh` remains CLI-only.
- Desktop package installation is handled through OS package artifacts, not
  shell scripts.
- The public site stays independent from the Svelte editor app.
