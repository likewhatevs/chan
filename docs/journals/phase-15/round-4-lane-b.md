# Phase-15 round-4 - @@LaneB (Linux build tooling)

You are @@LaneB. Read `round-4-bootstrap.md` (process) -> `round-4-status.md`
(active wave) -> this file -> `round-4-plan.md` (grounded anchors). You own the
round's LONG POLE + riskiest workstream. MAY spawn subagents for the per-distro
matrix.

## Goal

Build ALL linux components from a macOS machine via sdme/lima: chan-desktop for
ubuntu/debian, fedora/centos/almalinux, arch/cachyos + the `gateway/`
components + AppImage packaging + verify a `cs -> chan-desktop` symlink
dispatches. Launching the AppImage is OUT (no GUI; qemu later) - the goal is
the BUILD + the `cs` symlink.

## Your files (no other lane edits these)

- root `Makefile` (new linux-chan-desktop target)
- `desktop/Makefile` (linux cross/bundle support)
- `packaging/linux/Makefile` (chan-desktop targets, distinct from the existing
  CLI targets)
- `.github/workflows/release.yml` (the multi-distro matrix; the linux-desktop-
  artifacts job) - NOTE this is the B<->A seam: your release.yml change must
  land + gate BEFORE @@LaneA cuts v0.23.0 (Wave 2)
- `scripts/dev/sdme/*.sdme` (NEW: one rootfs per distro)
- `docs/contributing/linux-and-macos.md` (the build instructions)

DO NOT edit `cs_install.rs` / `chan-shell` argv0 detection - they are DONE +
unit-tested; you VERIFY them on a real AppImage.

## Grounded state (see round-4-plan.md for full anchors)

- No existing target builds chan-desktop for linux from macOS. `make dev` =
  CLI server; `desktop/Makefile build` = native `cargo tauri build`. The CLI
  linux packaging targets (root Makefile ~51-80) are the reference pattern.
- sdme: `docs/contributing/linux-and-macos.md` ~27-80 documents `limactl shell
  default sudo sdme ...`. The gateway ships `.sdme` templates
  (`gateway/scripts/dev/sdme/chan-psql.sdme`).
- AppImage auto-emits via Tauri `targets:"all"`. `release.yml` ~284-347 has a
  ubuntu-only linux-desktop-artifacts job to extend.
- `cs` symlink DONE: `chan-shell/src/lib.rs:35-46`, `cs_install.rs`,
  `chan/tests/cs_alias.rs`.

## Your work scope, by wave

### Wave 1 - DE-RISK ONE distro (the riskiest unknown first)

Prove the hard part before scaling: get **ubuntu** (matches CI) chan-desktop
building from macOS via sdme.
- Add a `.sdme` rootfs for ubuntu with the Tauri build deps (webkit2gtk-4.1,
  appindicator3, librsvg2, libsoup3, patchelf, rust toolchain).
- Add a `make` target that runs `cargo tauri build` inside the sdme container
  via `limactl shell ... sdme ...` and copies the artifacts out to the host.
- Confirm the emitted AppImage + .deb are valid; copy the AppImage to the host
  and verify the `cs -> chan-desktop` symlink dispatches: `cs terminal list`
  (and `cs t l`) against a running server returns rc=0 with NO GUI launch.
- Document the ubuntu path in `docs/contributing/linux-and-macos.md`.
- Gate your files; poke @@Architect "wave 1 done".

RISK: does `cargo tauri build` produce a valid AppImage inside a headless sdme
container (no X11/fonts)? If the bundler needs a display, find the headless
flag / fakeroot / xvfb path. This is the unknown to retire in Wave 1.

### Wave 2 - the full matrix + gateway + CI

- Add fedora (webkitgtk dep names differ) + arch/cachyos `.sdme` rootfs +
  targets. Subagents per distro are encouraged (the work parallelizes).
- Add the gateway linux build via sdme (same procedure;
  GATEWAY_RELEASE_CRATES).
- Extend `.github/workflows/release.yml`'s linux-desktop-artifacts into the
  multi-distro matrix. Land + gate this BEFORE @@LaneA's v0.23.0 cut.
- Gate (incl. the gateway workspace); poke @@Architect "wave 2 done".

## Completion (each wave)

Drive your files to gated-green + merge (pathspec commits), write your journal
(`round-4-lane-b-journal.md`), poke @@Architect. Record anything you could not
verify in this environment (e.g. fedora/arch if the sdme rootfs is
unavailable) as empirically-unverified -> round-5/phase-16 backlog.
