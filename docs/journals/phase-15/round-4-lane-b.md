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

### Wave 2 - gateway + CI matrix (NOT fedora/arch - see the division)

SCOPE CHANGE (architect, round-4-status.md is authoritative): the fedora +
arch per-distro `.sdme` rootfs builds are now OWNED BY @@LaneA's lent
subagents (your driver is DISTRO-parameterized, so each distro is just a new
`scripts/dev/sdme/chan-desktop-<distro>.sdme`; A validates them on the lima
VM and integrates the templates). You do NOT redo fedora/arch - that avoids
A-subagent vs B duplication on scripts/dev/sdme/. Your Wave 2:

- The gateway linux build via sdme (same procedure; GATEWAY_RELEASE_CRATES).
- Extend `.github/workflows/release.yml`'s linux-desktop-artifacts into the
  multi-distro CI matrix. Land + gate this BEFORE @@LaneA's v0.23.0 cut (the
  B<->A seam). You may reference the fedora/arch `.sdme` files A integrates.
- Gate (incl. the gateway workspace); poke @@Architect "wave 2 done".

### Wave 3 - STATIC MUSL `chan` CLI binary (NEW, @@Host-approved; v0.23.0 holds)

Ship a fully-static standalone `chan` Linux binary so a too-new build glibc
does not block older machines. The old static blocker was CUDA, which is
already gone (embeddings default to pure-Rust candle CPU; `cuda` is an opt-in
feature). Grounded (see round-4-status.md cross-lane notes for the full
assessment):

- Favorable: TLS = rustls + ring, NO `openssl-sys` in the tree (the usual musl
  killer). The C/C++ deps the musl build cross-compiles: ring, libsqlite3-sys
  (bundled SQLite), tokenizers' esaxx-rs (C++) + onig (C).
- Tool = `cargo-zigbuild` (zig as the cross C/C++ compiler; both musl arches
  from one runner). ALREADY INSTALLED on this Mac: zig 0.15.2 + cargo-zigbuild
  + the `x86_64-unknown-linux-musl` target (A added it). Add
  `aarch64-unknown-linux-musl` too.
- SCOPE: standalone `chan` tarball -> musl static (the install.sh / self-upgrade
  download). `.deb`/`.rpm` stay gnu (distro provides glibc). chan-desktop stays
  gnu (webkit can't be static). The gnu-only guard to lift is in
  `packaging/linux/Makefile` (the `chan-tarball` target rejects non-gnu).

Sequence:
1. DE-RISK FIRST (the one unknown to retire): prove the full
   embeddings+tokenizers+candle tree links FULLY static. Run
   `cargo zigbuild --release --target x86_64-unknown-linux-musl -p chan`
   (then aarch64); confirm `file`/`ldd` report a static PIE ("not a dynamic
   executable"). This is your Wave-1-style de-risk; do it before touching CI.
2. Wire it: a musl tarball path (lift the Makefile gnu-only guard; use
   zigbuild for musl targets) + ADD musl CLI legs to `release.yml`
   (`linux-cli-artifacts`) producing the static standalone tarball. This is a
   SEPARATE release.yml edit, after the Wave-2 M1 matrix (you own release.yml;
   no collision).
3. Point the standalone Linux download (install.sh / latest.json) at the musl
   tarball; keep `.deb`/`.rpm` gnu. No back-compat shim (pre-release).
4. Gate; the authoritative release.yml check is A's pre-cut workflow_dispatch
   dry-run. The musl legs land BEFORE A cuts v0.23.0 (sequence with A).

## Completion (each wave)

Drive your files to gated-green + merge (pathspec commits), write your journal
(`round-4-lane-b-journal.md`), poke @@Architect. Record anything you could not
verify in this environment (e.g. fedora/arch if the sdme rootfs is
unavailable) as empirically-unverified -> round-5/phase-16 backlog.
