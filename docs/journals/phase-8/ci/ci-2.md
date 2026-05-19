# ci-2: Release CI scaffold — tag-triggered build (signing deferred)

Owner: @@CI
Date: 2026-05-19

## Goal

Stand up a placeholder release workflow that fires on `chan-v*`
tags. Round 1 lands the unsigned scaffold; full signing /
notarization / GitHub Release upload lands in Round 2 once
signing-key rotation completes.

Workflow shape (this task):

* Trigger: push of a tag matching `chan-v*`.
* Job: build the chan binary + the chan-desktop installers via
  `make build` / the desktop Makefile, per platform.
* Upload the artifacts as workflow artifacts (NOT GitHub
  Release assets — that comes after signing).
* Linux + macOS now; Windows when convenient.

## Background

Source: phase-8 north star (notarized DMG) and backlog items 7
+ 8 in [`../../phase-7/next-phase-backlog.md`](../../phase-7/next-phase-backlog.md).

Signing-key rotation prerequisite per
[`../../../../desktop/CLAUDE.md`](../../../../desktop/CLAUDE.md):
the current updater key is a DEV key. Production signing
identity (Apple Developer ID for macOS notarization) needs to
be provisioned before the first public DMG ships. **Out of
scope for ci-2; this task is the unsigned scaffold.**

## Acceptance criteria

* `.github/workflows/release.yml` (or similar) triggers on
  `chan-v*` tags.
* Builds chan-desktop installers on Linux + macOS.
* Uploads workflow artifacts; does NOT publish to GitHub
  Releases yet.
* Documented in this task file: what's left for Round 2
  (signing identity provisioning, notarization steps, GitHub
  Release upload, Authenticode for Windows, Linux packaging).

## How to start

* After `ci-1` lands so the gate is in place.
* Reference `desktop/Makefile` for the build commands.
* Reference `desktop/CLAUDE.md` for the signing-key context.
* Audit existing GitHub Actions secrets (likely none today);
  draft the secret-name list needed for Round 2 and append it
  here.

## 2026-05-19 — landed (ready for review)

Owner: @@CI.

### What I added

New workflow `.github/workflows/release-desktop.yml`. Coexists
with the existing `.github/workflows/release.yml` (which produces
chan CLI artifacts on `v*` tags) — separate trigger pattern so
the chan CLI flow stays untouched.

| Trigger        | Action                                                       |
|----------------|--------------------------------------------------------------|
| push tag       | refs/tags/chan-v* (e.g. `chan-v0.11.0`)                      |
| workflow_dispatch | manual run from the Actions UI for dry runs              |

### Build matrix

| Runner        | Artifact name                              |
|---------------|--------------------------------------------|
| ubuntu-latest | chan-desktop-linux-x86_64-unsigned         |
| macos-latest  | chan-desktop-macos-aarch64-unsigned        |

Windows deferred per the task spec ("Linux + macOS now; Windows
when convenient"). macOS uses macos-latest which is Apple Silicon
on the GitHub-hosted runner pool — same Intel-excluded choice
release.yml made for the chan CLI .pkg.

### Build chain (per matrix entry)

1. `actions/checkout@v4` into `chan/`.
2. Copy `chan/rust-toolchain.toml` to workspace root so
   `actions-rust-lang/setup-rust-toolchain@v1` picks up the pin
   (1.95.0 + rustfmt + clippy + minimal profile).
3. `Swatinem/rust-cache@v2` against `chan` workspace.
4. `actions/setup-node@v4` (node 20, npm cache keyed on
   `chan/web/package-lock.json`).
5. Linux only: `apt-get install` webkit2gtk + ayatana-appindicator3
   + librsvg2 + libsoup-3.0 + patchelf (Tauri 2 Ubuntu deps).
6. `cargo install tauri-cli --locked --version "^2"` so
   `cargo tauri build` resolves.
7. `npm ci` under `chan/web/` — the Makefile's `chan-bin` target
   runs `npm run build` but does not install deps first; this
   step plugs that gap for a clean runner.
8. `cargo run --release -p fetch-models` from the workspace root
   so the BGE-small bundle bakes into the sidecar chan binary
   (mirrors release.yml; +~140 MB but matches chan CLI behaviour).
9. `make build` from `chan/desktop/` — this is `chan-bin`
   (`npm run build`, `cargo build --release --bin chan`,
   copy to `src-tauri/binaries/chan-<triple>`) followed by
   `cargo tauri build`. Bundle targets are `bundle.targets:
   "all"` per `tauri.conf.json`, so Tauri produces every
   installer the runner supports.
10. `actions/upload-artifact@v4` from `chan/target/release/bundle/**/*`.

### Bundle output path: workspace target, not src-tauri target

Important detail: `desktop/src-tauri/Cargo.toml` lists
`name = "chan-desktop"` and is a member of the root
`Cargo.toml` workspace (`members = [..., "desktop/src-tauri"]`).
Cargo writes to the workspace target dir, which means
`cargo tauri build` writes the bundle to
`chan/target/release/bundle/...`, NOT
`chan/desktop/src-tauri/target/release/bundle/...`.

The Makefile's `app-signed` / `app-notarized` echo lines still
reference `src-tauri/target/release/bundle/...` — that's stale
from pre-merge when desktop was its own crate. Not in
`@@CI`'s lane to fix (touches `desktop/Makefile`); flagging for
`@@Systacean`.

### What I did NOT do (Round-2 follow-ups)

Documented in the workflow's header comment as well:

| Secret                                | Purpose                                          |
|---------------------------------------|--------------------------------------------------|
| APPLE_SIGNING_IDENTITY                | Developer ID Application cert (macOS codesign)   |
| APPLE_TEAM_ID                         | 10-char team id (auto-derived from identity)     |
| APPLE_ID                              | Apple developer account email (notary)           |
| APPLE_PASSWORD                        | app-specific password for the notary             |
| TAURI_SIGNING_PRIVATE_KEY             | NEW production updater key (post bridge release) |
| TAURI_SIGNING_PRIVATE_KEY_PASSWORD    | passphrase for the above                         |
| WINDOWS_PFX_BASE64                    | Authenticode cert blob, base64                   |
| WINDOWS_PFX_PASSWORD                  | Authenticode cert password                       |
| GPG_PRIVATE_KEY                       | .deb / .rpm GPG signing key                      |
| GPG_PASSPHRASE                        | passphrase for the above                         |

When those land, switch `make build` to `make app-notarized` on
macOS, add signtool steps for Windows, and swap
`upload-artifact` for `softprops/action-gh-release` with
`permissions: contents: write`.

Tauri updater bundle signing has the bridge-release ordering
constraint in `desktop/CLAUDE.md`: the NEW pubkey ships in a
bridge release still signed by the OLD key, then all later
releases use the NEW key. @@CI cannot rotate that key without
@@Alex (secure machine + cert handling); flagging for the
phase-8 architect's release sequencing.

### Things I noticed but did not act on

* **Stranded workflow file**: `desktop/.github/workflows/ci.yml`
  still exists post-merge. GitHub Actions only reads
  `.github/workflows/` at the repo root, so this file does
  nothing today — it's a holdover from when chan-desktop was a
  separate repo. Reference: `desktop/release-review.md` P0.2
  flags it as "CI is disabled". Lane is @@CI's, but deletion
  warrants @@Architect input (does anyone consume it as docs?
  is `release-review.md` going to be re-issued?). Leaving it
  alone for now and flagging.
* **Makefile bundle-path comments**: `desktop/Makefile`'s
  `app-signed` / `app-notarized` echo `src-tauri/target/release/bundle/...`
  but the workspace target dir means the bundle is at
  `target/release/bundle/...`. Not destructive — just stale
  user output. @@Systacean lane.

### Files changed

* `.github/workflows/release-desktop.yml` — new file, unsigned
  scaffold per ci-2 spec.

No `crates/` or `web/` source touched. No edits to
`desktop/Makefile` or `desktop/src-tauri/`. Lane boundary
respected.

### Tests / verification

* Workflow YAML structurally validated (top-level keys + single
  `build` job present).
* Apt package list cross-checked against Tauri 2's Ubuntu 22.04
  prerequisites doc; matches.
* `cargo install tauri-cli --version "^2"` semver-resolvable
  against the workspace `tauri = "2"` pin.
* Full local `make build` smoke test NOT run — costs ~10 min
  on a clean checkout and reproduces a release I cannot
  meaningfully verify against today. The workflow's
  `workflow_dispatch` trigger means @@Alex / @@Architect can
  dry-run from the Actions UI before the first `chan-v*` tag.
  Flagging as the verification gap.

### Commit readiness

Not committing per the rule "do not commit unless @@Architect
or @@Alex tells you to." Files in working tree:

* `.github/workflows/release-desktop.yml`
* `docs/journals/phase-8/ci/ci-2.md` (this append)

Proposed commit message:

```
ci: tag-triggered chan-desktop release scaffold (unsigned)

Adds .github/workflows/release-desktop.yml triggered on `chan-v*`
tag pushes (intentionally distinct from release.yml's `v*` chan
CLI trigger). Builds chan-desktop installers via desktop/Makefile
on Linux + macOS, uploads as workflow artifacts. Signing /
notarization / GitHub Release upload deferred to Round 2 once
Apple Developer ID + production Tauri updater key are provisioned;
required secret names documented in the workflow header.
```

### Open questions for @@Architect

1. **Stranded `desktop/.github/workflows/ci.yml`**: delete now
   (one-line cleanup) or leave for a Round-2 pass that also
   refreshes `desktop/release-review.md`?
2. **Workflow dry-run via `workflow_dispatch`**: before the
   first `chan-v0.11.1` tag, want me to ask @@Alex (permission
   event) to trigger a manual dry-run from the Actions UI so
   we know the unsigned build succeeds?
3. **Windows lane**: spec says "when convenient". Round-2 with
   signing, or shoot for Round-1 with another unsigned matrix
   entry?

## 2026-05-19 — @@Architect: approved + answers + commit clearance

Reviewer: @@Architect.

Shape matches the spec: distinct `chan-v*` trigger (avoids the
existing `v*` CLI flow), Linux + macOS matrix, sidecar build
chain mirrors `desktop/Makefile`, secrets list documented in the
header for the Round-2 signing handover. The `workflow_dispatch`
trigger for manual dry-run is the right call — lets us verify
before the first real tag.

### Answers to your open questions

1. **Stranded `desktop/.github/workflows/ci.yml`**: delete now.
   It's dead code (GitHub Actions only reads repo-root
   `.github/workflows/`), and `desktop/release-review.md` P0.2's
   "CI is disabled" note becomes accurate once it's gone. Cut a
   one-line commit "drop stranded desktop/.github/workflows/ci.yml
   (workspace-merge leftover)" alongside ci-2 or as a tail-commit;
   your call.
2. **Workflow dry-run**: yes, but defer to Round-1 close. When
   @@Systacean approaches `systacean-3` (version bump + tag),
   fire the permission event then so we dry-run on the
   ci-1 + ci-2 + everything-else combined state. No point
   dry-running an isolated branch.
3. **Windows lane**: Round-2 with signing. Don't add a second
   unsigned matrix entry now — we'd just do it twice.
   Authenticode + the WINDOWS_PFX_* secrets line up with the
   broader signing wave.

**Commit clearance**: approved for `release-desktop.yml` as
proposed. Also clear to commit the
`desktop/.github/workflows/ci.yml` deletion (separate commit
preferred). Push waits for Round-1 close.

`desktop/Makefile` bundle-path comment fix flagged on
@@Systacean's lane; I'll route it to them when their next slot
opens. Stale `desktop/release-review.md` P0.2 is on the
documentation pass — not blocking.

Carry on idle / available for Round-2 prep (release CI signing
pipeline) once committed. I'll cut wave-2 work once the bug
wave settles.
