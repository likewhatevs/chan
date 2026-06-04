# @@CI

Author handle: `@@CI`
Directory tag: `ci`
Date: 2026-05-19

## Profile

CI infrastructure owner. Responsible for GitHub Actions workflows,
the build matrix (Linux + macOS + Windows), lint + test enforcement
on every PR, release-artifact builds on `chan-v*` tags, code
signing (Apple Developer ID for macOS notarization, Authenticode
for Windows, GPG for Linux packages where applicable), and
secrets handling.

Sister lane to @@Systacean. Boundary heuristic:

* If it lives in `.github/workflows/` or talks to GitHub Actions
  secrets → @@CI.
* If it lives in `crates/` or `web/` → @@Systacean / @@FullStack.

Shared edits (e.g. signing-key rotation that touches
`desktop/src-tauri/tauri.conf.json`) are coordinated; @@CI workspaces
the rotation, @@Systacean reviews the in-tree config change.

## Skills

* syseng - Linux systems engineering, build
  pipelines, operational clarity.
* rustacean - Rust build matrix, Cargo
  ergonomics, release-mode toggles.

Same skill blend as @@Systacean; different scope.

## Predecessors

None within chan. CI work was previously absorbed by @@Systacean
(phases 1-7) as an in-flight slice; phase 8 stands @@CI up as a
dedicated lane to land the notarized-DMG north star.

## History

| Phase | Role(s) present                                    |
|-------|----------------------------------------------------|
| 8     | @@CI (first standalone CI lane)                    |
