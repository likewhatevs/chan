# Phase 42 -- v0.52.0: the unification sweep

Experiment branch `v0.52.0-unification`, cut from post-v0.51.0 `main`. A phased, multi-lane round run by a four-member delivery team (a lead plus packaging, crates, and frontend tracks), not a single development line. The round is a hygiene sweep -- path moves, naming, docs, and frontend consolidation with no runtime behavior change -- landed as three sequential atomic phases, each ending on a green integrated gate, followed by a release-gating validation tail. The dev host can build neither the Tauri/GTK desktop nor a full release, so the desktop crate and the macOS-signing path are validated by the CI dry-run build; local proofs covered the cross-platform Rust, the gateway workspace, and the web and marketing surfaces.

## Theme

One tree, one shape. The frontend collapses into a single `./web` npm workspace; build and deploy tooling consolidates under `./packaging`; the crate layer gets a naming, docs, publish, and dependency-hygiene pass. The load-bearing invariants are frozen and proven untouched: the rust-embed bundle paths (`web/dist`, `web-launcher/dist`, `gateway/crates/identity/web/dist`), the `/dl` release-download contract, the Postgres-free root workspace, and every Makefile target and CI job name.

## What landed (by phase)

### Phase 1 -- packaging (`bd341769`, `4a896c3f`)

- **`bd341769` refactor(packaging): consolidate build/deploy infra under ./packaging.** `docker/`, `kube/`, `desktop/packaging/`, `scripts/dev/sdme/`, `gateway/packaging/`, and `gateway/scripts/` move under `packaging/{docker,kube,desktop,sdme,gateway,linux}` as one atomic `git mv` plus a deterministic, order-correct path-map rewrite, grep-zero of every old path.
- **`4a896c3f` build(packaging): retarget seam build files to packaging/ paths.** The lead-owned seam edits: the root Makefile sdme delegation, the `desktop/Makefile` build-dmg path (CHAN_REPO-anchored absolute), and the `release.yml` sdme comment. Make target and CI job names unchanged.

### Phase 2 -- crates (`491d9304`, `4eee5235`)

- **`491d9304` build(deps): hoist shared deps to [workspace.dependencies].** portable-pty, rand, rustix, and sha2 centralize in the root workspace dependency table; chrono and notify stay literal with a recorded reason (divergent feature sets that would change resolution).
- **`4eee5235` chore(crates): naming, docs, publish, and dependency hygiene.** The devserver-proxy naming alignment in the gateway docs, `publish = false` on the app-internal crates, the AI-native IDE product framing, three new crate `design.md` plus a rustdoc pass (broken intra-doc links, module-doc promotions), and the per-crate `workspace = true` conversions.

### Phase 3 -- frontend (`54eef711`, `0352d272`, `2d894cc6`, `ef56bf10`)

- **`54eef711` refactor(web): house workspace-app + launcher in a ./web npm workspace.** The root `./web` monorepo with `@chan/{workspace-app,launcher}` and one lockfile.
- **`0352d272` refactor(web): lift the gateway frontend into the ./web workspace.** `gateway/web-common` becomes `@chan/web-shared` and the identity SPA source becomes `@chan/profile`, still emitting to the frozen `gateway/crates/identity/web/dist`. The gateway npm root is removed; the gateway Rust crates and Cargo workspace are untouched.
- **`2d894cc6` refactor(web): house web-marketing in the ./web workspace.** Marketing becomes `@chan/marketing`; the `/dl` machinery and the `data-release-download` contract validators port unchanged.
- **`ef56bf10` build(web): retarget the build and CI graph to the ./web workspace.** The lead-owned seam edits plus the build and CI retarget (the Makefile web recipes, `release.yml`, `pages.yml`, `gateway-ci.yml`, `release-desktop.yml`) to the single `./web` workspace, preserving the launcher-then-app build order and the three frozen dist outputs.

### Validation tail (`dd4ca931`, `e2d89da5`, `29974c4f`)

- **`dd4ca931` refactor: retarget dangling references to the relocated web tree.** The exhaustive moved-path reference sweep over the final tree (Rust and desktop doc comments, design docs, dockerignores, the build-debs SPA build), plus the `attachments/` gitignore.
- **`e2d89da5` docs: complete the AI-native IDE definition sweep.** Three project-definition sites that still called chan a "notes app" (the contributing guide, the Arch PKGBUILD, the feature-request template) reframed to the AI-native IDE summary the crate description already carries.
- **`29974c4f` docs: retarget dangling gateway-frontend build instructions.** An adversarial completeness audit found that removing the gateway npm root left every "build the gateway SPA" contributor instruction broken (`npm run build --workspaces` from a `gateway/` with no `package.json`); retargeted to `cd web && npm run build -w @chan/profile` (or `make gateway-spa`) across the gateway README, the agent docs, the contributing guides, and the identity README.

### Release candidate (`f7743679`)

- **`f7743679` chore(release): bump version 0.51.0 -> 0.52.0-rc1.** Every pin to the prerelease; `release.yml` already accepts the `-rc1` tag, so no workflow change was needed.

## Notes

- **No behavior change.** This round restructures sources only. The embed bundle paths and the `/dl` download contract are byte-stable; every Makefile target and CI job name is unchanged.
- **Deferred to focused follow-ups.** The marketing Svelte render-layer port (the one from-scratch rewrite, over the load-bearing `/dl` contract and changing the public site's rendered output) is staged so v0.52.0 stays a clean hygiene sweep; marketing is already a `./web` member, so the port is cleanly additive later. The `crates/chan/src/lib.rs` module split is deferred because its behavior preservation on the `chan::run` to chan-desktop seam can only be proven on CI.
- **Pending host disposition.** A small set of dev-time "frontend not built" banner strings in the frozen embed files, a handful of product-voice descriptors ("markdown editor"), and the three held graph-demo HTML shells (`web/{d3-compare,graph-demo,sphere-tuner}.html`) are flagged for the host rather than changed in this sweep.
- **Host-validated build.** The desktop crate and the macOS sign/notarize/staple path are proven by the non-publishing `release.yml` dry-run build, the one place they validate off a workstation.
