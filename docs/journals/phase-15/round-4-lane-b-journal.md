# Phase-15 round-4 - @@LaneB journal (Linux build tooling)

## Wave 1: DONE (ubuntu chan-desktop builds from macOS via sdme)

The Wave-1 RISK (does `cargo tauri build` produce a valid AppImage inside a
headless sdme container?) is RETIRED. On the aarch64 lima VM, a fresh
`chan-desktop-ubuntu` sdme rootfs builds chan-desktop end to end and emits all
three bundles with no X11, no display, no FUSE:

```
Chan_0.22.0_aarch64.AppImage   86M
Chan_0.22.0_arm64.deb          15M
Chan-0.22.0-1.aarch64.rpm      15M   (Tauri targets:"all" emits it for free)
```

linuxdeploy + the appimage plugin download and run in extract-and-run mode, so
the headless container is sufficient. CI's `ubuntu-latest` still owns the
canonical x86_64 build; the local sdme path is aarch64 on Apple Silicon.

### Delivered (my files, gated, ready to merge)

- `scripts/dev/sdme/chan-desktop-ubuntu.sdme` (new) - the rootfs template:
  ubuntu base + Tauri build deps + node/npm + rustup. Mirrors the apt deps the
  release.yml linux-desktop-artifacts job installs.
- `scripts/dev/sdme/build-chan-desktop.sh` (new) - the driver. Builds the
  rootfs on first use, creates/reuses a `chan-desktop-build-<distro>`
  container, seeds the committed tree (`git archive HEAD`), runs
  `make chan-desktop` inside, and copies the bundles out to
  `target/linux-desktop/<distro>/`. Host-portable via `$SDME`
  (lima on macOS, `sudo sdme` on Linux). Knobs: `REBUILD_CONTAINER=1`
  (fresh container) and `REBUILD_ROOTFS=1` (rebuild the rootfs after a
  .sdme edit - see the catch below).
- root `Makefile` - new `linux-chan-desktop` target (DISTRO + SDME knobs),
  delegating to packaging/linux like the existing linux-deb/rpm targets.
- `packaging/linux/Makefile` - new `chan-desktop` target (distinct from the CLI
  deb/rpm/archpkg) forwarding to the driver.
- `docs/contributing/linux-and-macos.md` - new "Desktop: build the chan-desktop
  AppImage and .deb" section + a "Verifying the cs alias" subsection.

`desktop/Makefile` was NOT touched: its existing `build` target
(`cargo tauri build`) works as-is inside the container. No linux-specific
bundle flags were needed (no NO_STRIP / APPIMAGE_EXTRACT_AND_RUN env). Revisit
only if cross-arch (x86_64 from aarch64) needs it in a later round.

`.github/workflows/release.yml` was NOT touched this wave: the multi-distro CI
matrix is Wave-2 scope (the B<->A seam). The existing ubuntu-only
linux-desktop-artifacts job is unchanged and still valid.

### Two environmental gaps found and codified

1. **tmpfs /tmp too small.** sdme mounts a ~800M tmpfs over `/tmp`; the cold
   Rust + tauri-cli compile overflowed it (`No space left on device` building
   zstd-sys). Fix: the in-container build sets `TMPDIR=/var/tmp` (the
   disk-backed overlay, 56G free). In the driver + documented.
2. **xdg-utils missing.** The AppImage plugin shells out to `xdg-mime`; without
   it the bundle fails at the very end with `xdg-mime binary not found`. Added
   `xdg-utils` to the .sdme rootfs. The deb + rpm emit fine without it; only
   the AppImage needs it.

Process catch: an existing rootfs is NOT auto-rebuilt when the .sdme template
changes, so after adding xdg-utils the first full `make linux-chan-desktop` run
still failed on xdg-mime (it reused the pre-edit rootfs). Added a
`REBUILD_ROOTFS=1` knob (forces `sdme fs build -f`) and documented it. The
clean validation run uses `REBUILD_ROOTFS=1 REBUILD_CONTAINER=1`.

## VERIFIED FINDING (escalation): the cs alias is broken on the AppImage

The lane doc asked me to VERIFY (not edit) the `cs -> chan-desktop` argv0
dispatch on a real AppImage. I did, and it is BROKEN as packaged. Precise,
fully-bounded result:

- The argv0 DETECTION is correct. Running the inner binary directly with
  `exec -a cs .../chan-desktop terminal list` takes the control-client path
  (no GUI).
- The cs CLIENT round-trips with a real server. Against a `chan serve` in the
  container with `$CHAN_CONTROL_SOCKET` set, both `cs terminal list` and the
  abbreviated `cs t l` returned `No live terminal sessions.` rc=0, no GUI.
- The ONE broken link is the AppImage AppRun. `cs_install.rs` drops a wrapper
  `exec -a cs "$APPIMAGE" "$@"`, but linuxdeploy's generated `AppRun` re-execs
  `exec "$this_dir"/AppRun.wrapped "$@"` WITHOUT `-a`, so argv0 is reset to the
  wrapped path before the inner binary runs. The inner binary sees
  argv[0]=chan-desktop, fails `invoked_as_cs`, and launches the GUI (GTK init
  panics headless). This affects FUSE-mount mode identically (same AppRun
  script).

Fix hook (for whoever owns this, NOT me - cs_install.rs / chan-shell detection
are flagged DONE/don't-edit): the AppImage type-2 runtime DOES export
`ARGV0=cs` into AppRun's env when invoked via `exec -a cs`. Proven by an
instrumented AppRun probe. So either a custom AppRun that honors `ARGV0`
(`exec -a "${ARGV0:-$0}" ...`, and AppRun.wrapped must preserve it too), or the
detection reads `ARGV0` / a dedicated `CHAN_INVOKED_AS_CS` env the wrapper
sets, would fix it. This is an architect-level cross-lane decision.

Status: the cs CLIENT and detection are sound on the real artifact; only the
AppImage argv0 plumbing is broken. The unit tests cover the pure
plan()/wrapper_script() logic, never the argv0 round-trip through AppRun, which
is exactly why the real-AppImage verification mattered.

## Empirically verified vs not, in this environment

- VERIFIED: ubuntu chan-desktop AppImage/.deb/.rpm build headless via sdme. A
  clean `make linux-chan-desktop DISTRO=ubuntu` (rootfs rebuild with xdg-utils
  -> fresh container -> cold build) emitted all three bundles. The cs client +
  control-socket round-trip on the real artifact (rc=0); the argv0 detection.
  The copy-out (sdme cp container -> VM stage -> `limactl copy` to the Mac) was
  validated against those exact bundles, landing them on the host:

  ```
  target/linux-desktop/ubuntu/Chan_0.22.0_aarch64.AppImage   86M  ELF aarch64
  target/linux-desktop/ubuntu/Chan_0.22.0_arm64.deb          15M  Debian pkg
  target/linux-desktop/ubuntu/Chan-0.22.0-1.aarch64.rpm      15M  RPM v3.0
  ```

  Copy-out catch (fixed): the first copy-out staged inside the CONTAINER's
  /var/tmp, but `limactl copy` reads the VM filesystem, not the container
  overlay, so it could not find the stage. Fixed to `sdme cp` each bundle from
  the container onto a VM stage, then `limactl copy` to the Mac. On a native
  Linux host sdme cp lands straight on the host path (one hop).
- NOT verifiable here: x86_64 bundles (this VM is aarch64; CI owns x86_64);
  launching the AppImage GUI (no display; out of scope, qemu later).

## Highlights / lowlights / contention (for @@Architect)

- Highlight: the long-pole build risk is fully retired on the first distro; the
  driver is distro-parameterized and ready to scale to fedora/arch in Wave 2.
- Highlight: the cs verification did its job - it caught a real shipped bug
  (cs-on-AppImage) that all the static gates and unit tests missed.
- Lowlight: the cs-on-AppImage bug means desktop-only Linux users cannot
  currently drive the window from a terminal via the packaged AppImage. Needs a
  follow-up in another lane's code; flagged, not fixed.
- No contention: my files are disjoint; no cross-lane seam touched this wave
  (release.yml untouched until Wave 2).

## Wave 2: in progress (gateway-linux + release.yml M1 matrix)

Scope (architect-divided in round-4-status.md): I do NOT redo fedora/arch
(A's lent subagents own those .sdme files; I consume the validated templates).
My Wave-2 = the gateway-linux build via sdme + extend release.yml into the
multi-arch CI matrix (the B<->A seam: lands + gates BEFORE A cuts v0.23.0).

### release.yml M1 matrix - WRITTEN + statically gated (this resume)

A DECIDED M1 (the B<->A mechanism call; see round-4-lane-b-release-matrix.md
for the M1/M2/M3 fork and the reasoning). GH-hosted runners are ubuntu-only,
so literal fedora/arch `container:` jobs would re-list distro deps into
release.yml = the single-source drift that killed the v0.19.0 cut, can't be
validated locally, and arch emits no native package. M1 instead extends
`linux-desktop-artifacts` to a multi-ARCH matrix (amd64: ubuntu-latest +
arm64: ubuntu-24.04-arm, matching the sibling CLI + gateway matrix jobs) and
STAGES the .rpm the old single-arch job discarded. CI now ships the universal
AppImage + debian .deb + fedora .rpm on amd64 AND arm64 (arm64 desktop was a
real gap). Zero drift, CI-native, fully gateable. The fedora/arch .sdme files
keep their purpose as the LOCAL multi-distro dev/QA build path, named in a
job comment.

Delivered (uncommitted, statically gated):
- `.github/workflows/release.yml` - `linux-desktop-artifacts` is now a
  matrix job. Staging globs per format dir
  (`target/release/bundle/{appimage,deb,rpm}/*`) so the arch filename skew
  (AppImage amd64/aarch64 vs deb amd64/arm64 vs rpm x86_64/aarch64) needs no
  bookkeeping. Upload name `release-linux-desktop-${{ matrix.package_arch }}`;
  `publish-release` already downloads `pattern: release-*` and globs
  `artifacts/**/*`, so both legs are picked up (distinct subdirs + distinct
  filenames, no collision). Downstream `needs: linux-desktop-artifacts`
  (macos-validate, publish-release) is matrix-safe (waits for all legs).
- Static gate: `ruby -ryaml` load OK; the parsed job shows both matrix legs +
  the templated name; the only `release-linux-desktop` ref is the suffixed
  upload name. actionlint not installed locally; YAML + structure verified.

MERGED: `06c371a6` (race-proof pathspec, release.yml only). A's CLEARANCE
(round-4-status.md) reviewed the M1 diff = CORRECT; seam condition met
(templates in main @ `7a27e191`, before the v0.23.0 cut). The authoritative
validation is A's pre-cut workflow_dispatch DRY-RUN (publish=false) before
tagging; the static YAML+structure gate was enough to merge.

NOT done in this edit (Wave-3, B owns): the musl CLI legs. release.yml is NOT
"done" after M1 - the static-musl `chan` tarball (cargo-zigbuild, both musl
arches from one runner, on the MAC not the VM) is a SEPARATE Wave-3 edit that
sequences after this Wave-2 barrier. @@Host approved it; v0.23.0 HOLDS for it.

### Gateway-linux build via sdme - MERGED + VM-VERIFIED

- `gateway/scripts/dev/sdme/gateway-build.sdme` (new) - rootfs for the gateway
  nested workspace (next to chan-psql.sdme).
- `gateway/scripts/dev/sdme/build-gateway.sh` (new) - the driver.
- root `Makefile` `linux-gateway` target (new) - delegates to the driver.

MERGED: `30a3347f` (race-proof pathspec; Makefile + the 2 gateway sdme files).

VM-VERIFIED end to end on the aarch64 lima VM (VM-free per A's signal). All
four gateway crates compile (release, 51.64s) and cargo-deb emits the four
.deb packages, copied out to the host:

```
target/linux-gateway/chan-gateway-admin_0.22.0-1_arm64.deb            1.4M
target/linux-gateway/chan-gateway-identity_0.22.0-1_arm64.deb         3.1M
target/linux-gateway/chan-gateway-profile_0.22.0-1_arm64.deb          1.9M
target/linux-gateway/chan-gateway-workspace-proxy_0.22.0-1_arm64.deb  1.8M
```

Bug caught + fixed by the empirical build (a static `bash -n` gate could not
see it): the .sdme rootfs RUN environment has no `$HOME`, so the original
`. "$HOME/.cargo/env"` resolved to `/.cargo/env` and failed (the first run
died at rootfs-bake, EXIT=2 - read from the log's MAKE_EXIT line, NOT the
background-task notification, which reported the trailing echo's 0; exactly
the masking gotcha A flagged on the arch run). Fix: pin `HOME=/root` in that
RUN step before sourcing cargo/env + installing cargo-deb. The clean rebuild
(`REBUILD_ROOTFS=1`) proves the fix end to end. CI still owns the canonical
native x86_64 + arm64 gateway .deb builds (gateway-linux-packages job); this
sdme path is the local dev/QA mirror.

### Wave-2: COMPLETE (both deliverables merged + verified)

1. release.yml M1 multi-arch matrix: MERGED `06c371a6`, statically gated.
2. gateway-linux sdme build: MERGED `30a3347f`, VM-verified (4 .deb packages).
Poked @@Architect wave-2 done. Per A's CLEARANCE this closes the Wave-2
barrier (A templates `7a27e191` + C spawn `626593e9` + D raw `e747f1d2` all
done). Remaining B work is Wave-3 (musl CLI legs) which sequences after.

## Wave 3: static musl `chan` CLI (GO, @@Host-approved; v0.23.0 holds)

### Step 1 DE-RISK: RETIRED (both arches, fully static + functional)

The unknown (does the full embeddings+tokenizers+candle+bundled-SQLite tree
link FULLY static under musl?) is retired. `cargo zigbuild --release --target
<triple> -p chan` for BOTH musl arches links static; the C/C++ deps (ring,
libsqlite3-sys bundled SQLite, tokenizers' esaxx-rs C++ + onig C) cross-
compile via zig with no openssl-sys in the tree (the usual musl killer is
absent; CUDA already gone since 044c23ff).

- x86_64-unknown-linux-musl: `file` = "ELF 64-bit ... x86-64 ... statically
  linked, stripped" (29M; release in 2m40s).
- aarch64-unknown-linux-musl: `file` = "statically linked, stripped" (26M).
  Functional proof on the aarch64 lima VM (real Linux, not just the ELF
  header): `ldd` = "not a dynamic executable"; `chan --version` = `chan
  0.22.0` rc=0; `chan --help` dispatches clap rc=0. The static binary RUNS.

Tooling: zig 0.15.2 + cargo-zigbuild already on the Mac; both musl rustup
targets installed (A added x86_64; I added aarch64). Done before touching CI,
per the lane doc's Wave-1-style de-risk-first.

### Step 2-3 WIRING: DONE + gated-green (11 files, one commit)

Build mechanism + CI:
- `packaging/linux/Makefile` chan-tarball: accept musl + `cargo zigbuild` for
  musl (a make conditional picks zigbuild for `%-unknown-linux-musl`, plain
  `cargo build` for gnu); guard now accepts gnu OR musl. VERIFIED locally:
  `make linux-chan-tarball LINUX_TARGET=x86_64-unknown-linux-musl` produced
  the tarball; the binary inside is `statically linked`.
- `.github/workflows/release.yml` linux-cli-artifacts: matrix gains
  `musl_target`; installs zig (mlugg/setup-zig@v2 0.15.2) + cargo-zigbuild;
  builds the tarball for the musl target; stages the musl tarball (the
  .deb/.rpm stay gnu, same job). YAML + job structure validated.

Download surface (install.sh + latest.json + self-upgrade), all consuming the
same musl asset names:
- `web-marketing/src/install.sh`: Linux arch -> musl target.
- `web-marketing/scripts/generate-release-metadata.mjs`: cliTargets +
  cliDownloads Linux entries -> musl.
- `web-marketing/scripts/collect-release-assets.mjs` + `verify-release-
  assets.mjs`: required-asset lists -> musl.
- `web-marketing/fixtures/release-assets/v0.15.4.json`: CLI tarball entries
  -> musl. smoke-install-sh.mjs + smoke-release-assets-manifest.mjs: fixtures
  -> musl. `npm run check` PASSED (all 4 smokes green).
- `crates/chan/src/update.rs` self-upgrade: `release_target_for` Linux ->
  musl + tests. CAUGHT by a repo-wide sweep, not the lane doc's named files:
  the self-upgrade is the OTHER latest.json consumer, so it had to flip too
  or `chan update` on Linux would 404 the (gone) gnu asset. cargo fmt/clippy
  clean; `cargo test -p chan update::` = 16 passed.
- `docs/contributing/linux-and-macos.md`: new musl-tarball build section.

NOT changed (correctly stay gnu/native): the .deb/.rpm targets +
gateway/scripts/build-debs.sh (gateway debs), chan-desktop AppImage, macOS.

UNVERIFIED-LOCALLY (one piece -> A's pre-cut workflow_dispatch dry-run is the
authoritative gate): the CI zig provisioning (mlugg/setup-zig@v2 + the 0.15.2
pin). The Makefile musl build is proven locally; only the runner's zig
install can't be exercised on this Mac. If the action/pin needs adjustment
the dry-run catches it before the tag (no tag-time surprise).

SCOPE NOTE: web-marketing + update.rs are beyond my lane's original "Your
files" list but are the named extent of step-3 ("install.sh / self-upgrade
download" + "latest.json"); the collect/verify/smoke/fixture edits are the
mechanical consequence of changing the published asset names. No back-compat
shim (pre-release).

## Carryover

- Wave 2 (mine): fedora + arch .sdme rootfs + driver coverage (the driver is
  already parameterized; each distro needs its .sdme dep names, e.g. fedora's
  webkitgtk package names differ); the gateway linux build via sdme; extend
  release.yml into the multi-distro matrix (land + gate BEFORE @@LaneA cuts
  v0.23.0).
- round-5 / phase-16 backlog: fix the cs-on-AppImage argv0 dispatch (ARGV0 hook
  above); x86_64 local-build path (cross or an x86_64 sdme rootfs); launch-the-
  AppImage GUI smoke (qemu).

## Round-4 retrospective (@@LaneB)

Done (all 3 waves merged, race-proof pathspec):
- W1 `bb1eed2f`: ubuntu chan-desktop AppImage/.deb/.rpm headless via sdme +
  the cs verification (which caught the AppImage argv0 bug).
- W2 `06c371a6` (release.yml M1 multi-arch desktop matrix) + `30a3347f`
  (gateway-linux sdme, VM-verified 4 .deb).
- W3 `101c0f66`: static musl `chan` CLI (de-risk both arches + full wiring).

Pending / not-mine-to-close:
- A owns the CI zig-provisioning dry-run + the v0.23.0 cut.
- round-5/phase-16: cs-on-AppImage argv0; arch AppImage linuxdeploy failure
  (A's backlog); x86_64 local sdme path; qemu GUI smoke.

Highlights:
- De-risk-first (W1 + W3) retired the two long-pole risks (headless AppImage
  bundling; full-tree musl static link) before any CI edit. Both held.
- Empirical builds caught THREE bugs the static gates could not: the
  cs-on-AppImage argv0 dispatch (W1), the gateway-rootfs `$HOME`-unset
  cargo-deb failure (W2), and (via a repo-wide sweep, not a build) the
  update.rs self-upgrade target, which would have half-shipped the musl
  repoint.

Lowlights / honest:
- The CI zig provisioning ships unverified-locally; it genuinely cannot be
  closed on a Mac without a CI run, so it rides on A's dry-run. Real residual
  risk, flagged not hidden.
- My first background build wrote the real exit only to the terminal, not the
  log, so the trailing-echo masked make's true EXIT=2 (A's arch-run gotcha
  bit me once). Fixed mid-round by writing MAKE_EXIT into the log; thereafter
  I always read the log's exit line, never the task-notification's.

Feedback:
- Me: should have logged the real exit code from the first build, and should
  default to a repo-wide consumer sweep (not just the lane-doc's named files)
  the moment a published-artifact name changes - the update.rs catch was luck
  of being thorough, not process.
- Alex: the relay-via-poke conduit was smooth; decisive GO + blocker-clearing
  kept the lane moving with zero stalls. No friction to report.
- Architect (A): crisp sequencing prevented scope-creep - the M1 fork
  resolution, the explicit "DE-RISK FIRST / musl is W3 don't add now", and the
  VM-ownership decision (A's fedora/arch own the 8GiB VM) all landed cleanly.
  One ask: the W3 spec named "install.sh / latest.json / self-upgrade" but not
  the full consumer set (the metadata generator + collector + verifier +
  smokes + fixtures + update.rs); listing them upfront would have turned a
  sweep-and-discover into a checklist.
