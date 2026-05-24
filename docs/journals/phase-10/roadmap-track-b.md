# Phase 10 Roadmap Track B: Public Site, Manual Pages, Install Split

Status: planning.

Track B turns `web-marketing` into the public site source of truth,
publishes `docs/manual/` as public documentation, and cleans up the
install surface now that desktop and CLI have different release shapes.

## Objectives

- Generate the public website from in-repo sources.
- Compile `docs/manual/**/*.md` into pages at `chan.app/manual/`.
- Make desktop the primary public install path after Track A lands.
- Keep shell installation for the standalone CLI only.
- Remove Windows install support and the PowerShell installer.

## 1. Static site build

Current state:

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
- `/install.sh` is first-install only and downloads from
  `https://chan.app/dl/latest/`.
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
- DNS cutover remains an operational step, not a build-script side effect.

Release integration:

- GitHub repository URLs now use the personal repo
  `github.com/fiorix/chan`, not the old `chan-writer` org.
- Keep `.github/workflows/release.yml` for standalone CLI artifacts.
- Keep `.github/workflows/release-desktop.yml` for desktop artifacts.
- `chan upgrade` uses GitHub Releases directly:
  - latest probe: `https://api.github.com/repos/fiorix/chan/releases/latest`
  - download base:
    `https://github.com/fiorix/chan/releases/download/chan-v<version>/`
  - release tags must use `chan-v<version>`.
  - `chan upgrade --version` takes a bare version such as `0.14.0`.
- Do not keep pre-release compatibility paths for old tag names, Windows
  release artifacts, or `/dl/v<version>/` upgrade URLs.
- Add a release-tag manual bundle generated from the same `docs/manual/`
  source, either attached to the GitHub release or published as part of the
  Pages artifact.
- Verify that `chan.app/dl/latest/` URLs used by the site match the active
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
- Verify `install.sh` installs the CLI from `/dl/latest/`.
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
