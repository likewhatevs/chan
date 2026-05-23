# desktacean-4 — chan-desktop package version metadata

Owner: @@Desktacean
Phase: 8, Round 3
Date cut: 2026-05-23

## Goal

Fix chan-desktop package metadata so the next desktop release
artifacts carry the correct version for the release tag.

## Background

chan-core @@CI's `ci-15` workflow audit found that the
`chan-v0.12.0` release page shipped desktop artifacts named
`Chan_0.11.2.*` while the chan CLI artifacts correctly used
`0.12.0`.

chan-core routed this to @@Desktect through @@Alex as
chan-desktop-owned package metadata. Current suspect:
`desktop/src-tauri/tauri.conf.json` `version` still reads
`0.11.2`.

Do not retcon the already-shipped `chan-v0.12.0` artifacts. Fix
the next release path cleanly, expected `v0.13.0` per Round-3
public-flip planning.

## Acceptance Criteria

1. Identify the exact source of the `0.11.2` desktop artifact
   name.
2. Update desktop-local version metadata for the next cut if the
   source is under `./desktop`.
3. Confirm the expected artifact naming for the next build:
   `Chan_0.13.0.*` if the next release remains `chan-v0.13.0`.
4. Update desktop docs if they mention stale `0.11.2` package
   metadata.
5. Keep edits inside `./desktop` and this task/event journal.
   If release workflow logic is the real source, stop and poke
   @@Desktect so @@Alex can bridge to chan-core.
6. Run the smallest useful verification. At minimum:
   * `node -e "JSON.parse(require('fs').readFileSync('desktop/src-tauri/tauri.conf.json','utf8')); console.log('json ok')"`
   * `cargo test -p chan-desktop --bin chan-desktop`

## How To Start

1. Inspect `desktop/src-tauri/tauri.conf.json` and
   `desktop/src-tauri/Cargo.toml` for version fields.
2. Inspect `desktop/Makefile` for artifact naming.
3. Inspect `.github/workflows/release-desktop.yml` read-only if
   needed to understand how artifact names are collected. Do not
   edit workflow YAML in this task.
4. Patch desktop metadata only if the source is confirmed under
   `./desktop`.

## Coordination

* @@Desktect owns routing and commit clearance.
* @@Alex bridges any `.github/` or release-publisher change to
  chan-core.
* Do not edit outside `./desktop`.

## 2026-05-23 - @@Desktacean implementation report

Confirmed source of the `Chan_0.11.2.*` desktop artifact names:

* `desktop/src-tauri/tauri.conf.json` had `productName: "Chan"` and
  `version: "0.11.2"`.
* `desktop/Makefile` runs `cargo tauri build` / `cargo tauri build
  --bundles app,dmg` without overriding the version.
* `.github/workflows/release-desktop.yml` uploads the generated Tauri
  bundle files and only renames the macOS `_x64.dmg` suffix when
  present. It does not set desktop package version metadata.

Patch made:

* Updated `desktop/src-tauri/tauri.conf.json` `version` from `0.11.2`
  to `0.13.0` for the next planned `chan-v0.13.0` cut.
* Added a `desktop/CLAUDE.md` release package metadata note: Tauri
  artifact names derive from `src-tauri/tauri.conf.json`
  `productName` + `version`, and desktop release tags need that
  version updated before cut.

Expected next desktop artifact naming:

* Config now derives `Chan_0.13.0.*` from `productName` and `version`.
* Exact suffix still depends on Tauri bundle type and platform, with
  workflow-side macOS suffix cleanup still owned by
  `.github/workflows/release-desktop.yml`.

Boundary note:

* `desktop/src-tauri/Cargo.toml` still inherits
  `version.workspace = true`.
* Current root workspace version is still `0.11.2`, so
  `cargo test -p chan-desktop --bin chan-desktop` compiles the Rust
  package as `chan-desktop v0.11.2`.
* I did not edit root `Cargo.toml` because that is outside the
  desktop boundary and also controls the CLI/sidecar version. The
  release workspace bump should remain chan-core scope.

Verification:

* `node -e "JSON.parse(require('fs').readFileSync('desktop/src-tauri/tauri.conf.json','utf8')); console.log('json ok')"`:
  `json ok`.
* Derived artifact prefix check:
  `Chan_0.13.0.*`.
* `cargo test -p chan-desktop --bin chan-desktop`: 63 passed.

Ready for @@Desktect review / commit clearance.

## 2026-05-23 - @@Desktect approval + commit clearance

Approved.

Review notes:

* Root cause is confirmed: Tauri artifact names derive from
  `desktop/src-tauri/tauri.conf.json` `productName` + `version`.
* Updating `tauri.conf.json` to `0.13.0` is the right
  desktop-local fix for the next planned cut.
* `desktop/CLAUDE.md` now documents the guardrail so the desktop
  version is not missed again.
* The workspace package version is intentionally outside this
  task. It must still be bumped by chan-core at release cut so
  `env!("CARGO_PKG_VERSION")`, bundled `chan`, and
  chan-desktop's version probe stay aligned.

Commit clearance:

```
chan-desktop: bump package metadata for v0.13.0 artifacts (desktacean-4)
```

Path-scope the commit to:

* `desktop/src-tauri/tauri.conf.json`
* `desktop/CLAUDE.md`
* `docs/journals/phase-8/desktacean/desktacean-4.md`
* `docs/journals/phase-8/alex/event-desktacean-desktect.md`

Do not include unrelated chan-core work, bootstrap files, or other
agents' journals.
