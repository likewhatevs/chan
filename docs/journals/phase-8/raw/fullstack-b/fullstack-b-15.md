# fullstack-b-15: Bundled chan binary inside chan-desktop app resources

Owner: @@FullStackB
Date: 2026-05-20

## Goal

Bundle a copy of the `chan` binary inside the chan-desktop app
package so chan-desktop can launch a chan server even when the
user has not separately installed chan via `cargo install` /
`brew install` / etc. This is item 7 piece 1 of the Round-2
north-star deliverable.

## Background

* **Round-2 plan**: [`../architect/round-2-plan.md`](../architect/round-2-plan.md)
  §"North-star through-line" lists this as item 7 piece 1.
* **Decision 3 LOCKED** (per round-2-plan decisions table):
  PATH-first with bundled fallback + version match. This task
  is the BUNDLING side; the LAUNCH-TIME PROBE LOGIC lives in
  the sibling task `fullstack-b-16`. The launch-time
  resolution shape is: `which chan` → check `chan --version`
  against the bundled version → if match use PATH, else fall
  back to bundled.
* **CLAUDE.md** (project root): chan is a single static
  binary; chan-desktop is the Tauri wrapper. The bundled binary
  goes inside the chan-desktop app, NOT replacing system
  installs.
* **Tauri bundle structure**: on macOS the app is
  `Chan.app/Contents/`. Executable code lives in
  `Contents/MacOS/`; non-executable resources in
  `Contents/Resources/`. The bundled chan binary is
  executable, so it likely lives in `Contents/MacOS/` alongside
  chan-desktop OR in a deliberately-marked subdirectory under
  `Contents/Resources/` that the launcher walks to invoke.
  Implementer picks the cleanest path with notarization in
  mind (everything signed under the app gets notarized
  together — both layouts work for signing as long as the
  bundled binary is in the `_CodeSignature` scope).
* **Build-time wiring**: `desktop/Makefile` already has
  recipes that build chan + chan-desktop together for the
  signed/notarized DMG path. This task extends those recipes
  to copy the chan binary into the bundle's right location +
  surfaces the bundled-binary path to chan-desktop's
  launcher code.

## Authorization

**Authorization: yes**, this task covers edits to:
* `desktop/Makefile` (bundle assembly recipes).
* `desktop/src-tauri/tauri.conf.json` (resource configuration
  if Tauri's resource manifest needs to know about the
  bundled binary).
* `desktop/src-tauri/src/serve.rs` (or wherever chan-desktop
  resolves the chan-binary path today) for the bundled-path
  helper.
* `.github/workflows/release-desktop.yml` if the CI bundle
  step needs adjustment (coordinate with @@CI on `ci-7` if
  so).
@@FullStackB may proceed without further in-chat confirmation
from @@Alex.

## Acceptance criteria

* A debug / dev build of chan-desktop (`cargo build -p
  chan-desktop` or `make app`) produces a `Chan.app` whose
  bundle contains a copy of the chan binary at a
  documented location.
* A release build (`make app-signed` / `make app-notarized`)
  embeds the bundled chan binary as well, with both binaries
  (chan-desktop + chan) signed under the same Developer ID
  identity so the notarization at `ci-7` covers both.
* The bundled binary has the right architecture for the
  target (universal2 fat binary on macOS preferred; per-arch
  also acceptable if universal2 is too much yak-shave for
  this task).
* A new helper function — `bundled_chan_path()` (or similar)
  — exposed to chan-desktop's launcher code, returning the
  absolute path of the bundled binary inside the running
  app. Robust against bundle relocation (drag-to-Applications
  / open-from-mounted-DMG / `xattr -cr` operations).
* Unit test in `desktop/src-tauri/src/` pinning the helper
  returns a path under the app bundle root.
* `chan --version` against the bundled binary matches the
  chan-desktop build's expected version (i.e. the bundle is
  built from this checkout's HEAD).
* `desktop/CLAUDE.md` updated with the bundled-binary
  layout decision + the rationale (notarization scope,
  arch handling).
* Pre-push gate: clean.

## How to start

1. Re-read `desktop/CLAUDE.md` for the existing chan-desktop
   build architecture.
2. Inspect `desktop/Makefile`'s `app-signed` / `app-notarized`
   recipes to understand the current bundle assembly.
3. Decide bundle location: `Contents/MacOS/chan` (sibling) or
   `Contents/Resources/bin/chan` (subdir). Recommend
   `Contents/MacOS/chan` for simplicity — Tauri's signing
   step covers everything under MacOS/ automatically; no
   custom code-sign-detached step needed. Document the
   choice in `desktop/CLAUDE.md`.
4. Wire the Makefile / Tauri config to copy the chan binary
   into the bundle on every build (debug + release paths).
5. Add the `bundled_chan_path()` helper in `serve.rs`
   (next to `drive_title` / similar existing helpers); use
   Tauri's `app_handle.path_resolver()` API or equivalent
   to derive the path from the running bundle root.
6. Add the unit test pinning the helper.
7. Append commit-readiness to the task tail.

## Coordination

* **Feeds `fullstack-b-16`**: the launch-time PATH-first
  probe consumes `bundled_chan_path()` as the fallback
  branch. Land `-15` first; `-16` sits on top.
* **Coordinates with `ci-7`**: @@CI's `release-desktop.yml`
  needs to know if the bundle-assembly step requires any new
  workflow plumbing (e.g. cross-compile chan for universal2
  before bundling). Surface the workflow needs in your task
  tail; @@CI absorbs them into ci-7.
* **Independent of `systacean-11`**: signing-key rotation
  doesn't affect this task's content; the binary just needs
  to be in the bundle's signed scope.

## Next in your queue

* `fullstack-b-16` — launch-time version probe + binary
  selection (PATH-first with bundled fallback + version
  match).

## Open questions

(populated as you investigate)

## 2026-05-20 — implementation note

**Pre-existing infrastructure**: bundling-the-binary is already
wired. `tauri.conf.json` declares
`bundle.externalBin = ["binaries/chan"]`, and the `chan-bin`
recipe in `desktop/Makefile` builds `target/release/chan` and
copies it to `src-tauri/binaries/chan-<target-triple>` before
every `cargo tauri dev` / `cargo tauri build`. `build.rs` writes
an empty executable placeholder when the staged binary is
absent so `cargo check` succeeds against a fresh checkout.
Tauri strips the target-triple suffix at bundle time and places
the sidecar at:

* `target/debug/chan` (dev)
* `Chan.app/Contents/MacOS/chan` (packaged macOS)
* sibling of `chan-desktop[.exe]` (packaged Linux / Windows)

What was missing for the round-2 north-star contract was:

1. A public, callable helper exposed to the launcher code — the
   existing `chan_bin()` was private in `main.rs`.
2. A version probe that matched chan-desktop's own version
   exactly — the existing probe used `MIN_CHAN_VERSION = "0.8.1"`
   as a floor, which silently passes any chan back to that
   version. The locked round-2 decision-3 contract is *exact
   match*; the PATH resolver in `fullstack-b-16` builds on that
   shape.
3. A unit test pinning the helper's path-resolution contract.
4. Documentation of the bundle layout in `desktop/CLAUDE.md`.

### Changes landed

* **`desktop/src-tauri/src/serve.rs`**
  * New `pub fn bundled_chan_path() -> Result<PathBuf, String>` next
    to `drive_title`. Pure path math over `current_exe()`; the
    existence check moved to the boot-time preflight.
  * New `pub fn probe_chan_version(&Path) -> Result<(), String>`.
    Runs `chan --version`, parses with `semver`, asserts
    *exact* equality against `env!("CARGO_PKG_VERSION")`. Drops
    the old `MIN_CHAN_VERSION` floor.
  * New unit test `bundled_chan_path_is_sibling_of_chan_desktop_executable`
    pinning the resolution contract (sibling of `current_exe()`,
    correct name per `target_os`). Pure path math, no filesystem
    access — the test passes on a fresh checkout that has not yet
    run `cargo build --release --bin chan`.

* **`desktop/src-tauri/src/main.rs`**
  * Removed the now-relocated `chan_bin()` and
    `probe_chan_version()` helpers + the `MIN_CHAN_VERSION`
    constant.
  * `compute_bin_status()` now calls
    `serve::bundled_chan_path()` for the path,
    `path.exists()` for the existence check, and
    `serve::probe_chan_version()` for the version check. Same
    three-way verdict (`ok` / `translocated` / `missing` /
    `version-mismatch`) preserved.
  * Three IPC handlers (`add_drive`, `remove_drive`,
    `set_drive_on`) now resolve via
    `serve::bundled_chan_path()?`. The `require_bin` gate is
    unchanged: every spawn path still checks
    `state.bin_status.ok` before resolving the path.

* **`desktop/CLAUDE.md`**
  * New "Bundled chan sidecar" section above the auto-upgrade
    notes. Documents the bundle layout per build profile, the
    sidecar placement rationale (macOS notarization scope), the
    public resolution + version-probe helpers, and the
    universal2 follow-up that belongs in `ci-7` (per the task
    body's "coordinate with @@CI" coordination note).

### Acceptance criteria — verification

| Criterion                                                              | State                                                                                          |
|------------------------------------------------------------------------|------------------------------------------------------------------------------------------------|
| Debug build produces `Chan.app` with chan inside                       | Pre-existing via `externalBin`; unchanged this commit.                                         |
| Release build embeds chan + signs both under same Developer ID         | Pre-existing via `externalBin` + `cargo tauri build --bundles app,dmg`; unchanged.             |
| Right arch for target (universal2 preferred, per-arch acceptable)      | Per-arch today (host triple). Universal2 deferred to `ci-7` per task body's coordination note. |
| `bundled_chan_path()` helper exposed to launcher code                  | Landed in `serve.rs` as `pub fn`; called from `main.rs` IPC handlers + `compute_bin_status`.   |
| Helper robust against bundle relocation                                | Uses `current_exe()` which Tauri / Tauri-Mac resolve correctly across drag-to-Applications.    |
| Unit test pinning the helper returns a path under app bundle root      | `bundled_chan_path_is_sibling_of_chan_desktop_executable`. chan-desktop 20 → 21 tests.         |
| `chan --version` against bundled matches chan-desktop's version        | `probe_chan_version` checks exact match against `env!("CARGO_PKG_VERSION")`.                   |
| `desktop/CLAUDE.md` updated                                            | New "Bundled chan sidecar" section.                                                            |
| Pre-push gate                                                          | Workspace fmt + clippy `-D warnings` + test (chan-desktop 20 → 21) + no-default-features build + svelte-check (3978 files / 0 errors) + npm build + vitest (544/544) all green. |

### Coordination footprint for @@CI

The Makefile's per-target-triple chan-bin recipe is the right
shape for local builds today, but a single universal2 fat binary
on macOS is the natural next step for distributing one DMG that
runs on both Apple Silicon and Intel. The `lipo`-merge belongs
in the GitHub Actions release workflow (`ci-7`) rather than in
this Makefile because CI already does per-arch matrix builds.

Documented in `desktop/CLAUDE.md` "Architecture handling"
subsection so @@CI's ci-7 work has a written reference.

### Suggested commit subject

```
chan-desktop: expose bundled_chan_path() + exact-match version probe (fullstack-b-15)
```

Touches:
* `desktop/src-tauri/src/serve.rs`
* `desktop/src-tauri/src/main.rs`
* `desktop/CLAUDE.md`

Holding for @@Architect commit clearance. Push held for the
Round-2 release tag (end of Round 2, not v0.11.x).

Next in queue: `fullstack-b-16` — PATH-first probe that consumes
`bundled_chan_path()` as the fallback branch. Holding the
implementation until @@Architect clears `-15` (hard-sequential
per the task brief): the public shape of `bundled_chan_path()` /
`probe_chan_version()` lands here, and `-16`'s implementation
hangs off both. If review changes either signature, `-16`
rebases trivially.
