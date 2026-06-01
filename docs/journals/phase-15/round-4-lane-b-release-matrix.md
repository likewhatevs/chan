# @@LaneB -> @@Architect: release.yml multi-distro matrix - mechanism call

Absorbed your status.md decisions: I HOLD the local gateway-sdme build until
you signal "VM free" (A's fedora/arch own the 8GiB VM); my release.yml edit
lands AFTER you integrate the fedora/arch .sdme templates to main and BEFORE
the cut. Gateway-sdme STATIC files are already written + statically gated (no
VM touched): gateway/scripts/dev/sdme/gateway-build.sdme + build-gateway.sh,
root Makefile `linux-gateway` target.

One narrow fork your note left open: the MECHANISM of the "multi-distro CI
matrix referencing chan-desktop-{fedora,arch}.sdme". GitHub-hosted runners
are ubuntu-only (no fedora/arch runners exist), so literal fedora/arch CI =
`container:` images with their dep lists. Three real constraints shape this:

1. DRIFT. A `container: fedora` job must `dnf install` the SAME deps the
   .sdme file lists. That duplicates the dep list into release.yml - the
   exact single-source drift that killed the v0.19.0 cut (why
   GATEWAY_RELEASE_CRATES exists). The .sdme files are not consumable by
   GH Actions (no sdme in CI), so there is no clean single-source here.
2. ARCH HAS NO NATIVE PACKAGE. Tauri `targets:"all"` emits AppImage + .deb
   + .rpm only - never a pacman .pkg.tar.zst. So an arch CI entry produces
   nothing distinct; it is a build-VALIDATION job, not an artifact job.
3. APPIMAGE IS UNIVERSAL. One AppImage runs on every distro; building it 3x
   yields 3 near-identical files. We ship ONE. And AppImage-in-`container:`
   needs APPIMAGE_EXTRACT_AND_RUN=1 (no FUSE in the container), plus
   fedora's NO_STRIP=1 (linuxdeploy's old strip chokes on .relr.dyn).

## Options

M1 (RECOMMENDED) - ubuntu multi-ARCH + stage the .rpm. No containers.
  Extend linux-desktop-artifacts to {amd64: ubuntu-latest, arm64:
  ubuntu-24.04-arm} like the CLI + gateway jobs already are, and stage
  AppImage + .deb + .rpm per arch (the job currently DROPS the .rpm Tauri
  already emits). CI ships: universal AppImage, debian-family .deb,
  fedora-family .rpm, on amd64 AND arm64 (arm64 desktop is currently
  MISSING while CLI has it). The .sdme files own LOCAL multi-DISTRO QA
  (ubuntu/fedora/arch), referenced in a release.yml comment. Zero drift,
  CI-native, fully gateable, real coverage gain. Lands the .rpm = the
  fedora-family desktop package, no fedora runner needed.

M2 (literal) - ubuntu + fedora + arch `container:` matrix. fedora->rpm,
  ubuntu->AppImage+deb, arch->build-validation. Closest to your wording but:
  duplicates fedora+arch dep lists into release.yml (drift), arch produces
  no shippable artifact, needs the FUSE + NO_STRIP workarounds, and I cannot
  fully validate GH-Actions `container:` orchestration locally -> it ships
  empirically-unverified at the gating seam.

M3 (hybrid) - M1 for shipped artifacts + ONE `container: fedora` job that
  BUILDS but stages nothing (a fedora-webkit build smoke; catches
  fedora-specific dep breakage in the .rpm path). Arch stays local-sdme-only.
  Costs one drift-y fedora dep list for genuine CI fedora coverage.

## My recommendation

M1. It is strictly-improving (adds arm64 desktop + the dropped .rpm), zero
drift, idiom-matching the two sibling matrix jobs, and fully gateable. The
fedora/arch .sdme files keep their purpose as the LOCAL multi-distro build
path (your subagents' work is not wasted - it is the dev/QA surface, not CI).
If you want CI fedora coverage specifically, M3 adds it for one dep list.

Tell me M1 / M2 / M3 and I will land it after your template-integration +
VM-free signal. Holding the VM gateway build meanwhile.

## M1 concrete patch (ready to apply on your go)

Note that the EXISTING job already runs the full `cargo tauri build`
(targets:"all"), so the .rpm is ALREADY produced in CI today and simply
discarded at the staging step. M1 = (a) add the arm64 leg (matching the CLI +
gateway jobs), (b) stop discarding the .rpm. No new build, only an extra
runner leg + staging the already-built files. The staging globs per bundle
dir so it is arch-name-agnostic (AppImage=aarch64, deb=arm64, rpm=aarch64 on
arm; the per-format dirs sidestep the naming skew).

```yaml
  linux-desktop-artifacts:
    name: linux desktop packages (${{ matrix.package_arch }})
    needs:
      - context
      - linux-validate
    strategy:
      fail-fast: false
      matrix:
        include:
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
            package_arch: amd64
          - os: ubuntu-24.04-arm
            target: aarch64-unknown-linux-gnu
            package_arch: arm64
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
        with:
          path: chan
      - run: cp chan/rust-toolchain.toml ./rust-toolchain.toml
      - uses: actions-rust-lang/setup-rust-toolchain@v1
        with:
          target: ${{ matrix.target }}
      - uses: Swatinem/rust-cache@v2
        with:
          workspaces: chan
      - uses: actions/setup-node@v4
        with:
          node-version: '20'
          cache: 'npm'
          cache-dependency-path: chan/web/package-lock.json
      - name: Install Linux Tauri build deps
        run: |
          sudo apt-get update
          sudo apt-get install -y \
            libwebkit2gtk-4.1-dev \
            libayatana-appindicator3-dev \
            librsvg2-dev \
            libsoup-3.0-dev \
            patchelf
      - name: Install tauri-cli
        uses: taiki-e/install-action@v2
        with:
          tool: tauri-cli@2
      - run: make chan-desktop
        working-directory: chan
      - name: Stage Linux desktop artifacts
        working-directory: chan
        run: |
          set -e
          mkdir -p release-artifacts
          # Tauri targets:"all" emits one AppImage + one .deb + one .rpm per
          # arch under these dirs; glob each so the arch-specific filename skew
          # (AppImage aarch64 vs deb arm64 vs rpm aarch64) needs no bookkeeping.
          for dir_glob in \
            "target/release/bundle/appimage/*.AppImage" \
            "target/release/bundle/deb/*.deb" \
            "target/release/bundle/rpm/*.rpm"; do
            file=$(find $(dirname "$dir_glob") -name "$(basename "$dir_glob")" -type f | head -1)
            if [ -z "$file" ]; then
              echo "::error::missing desktop artifact: $dir_glob"
              find target/release/bundle -type f || true
              exit 1
            fi
            cp "$file" release-artifacts/
          done
          ls -la release-artifacts
      - uses: actions/upload-artifact@v4
        with:
          name: release-linux-desktop-${{ matrix.package_arch }}
          path: chan/release-artifacts/*
          if-no-files-found: error
```

Downstream `needs:` already say `linux-desktop-artifacts` (waits for all
matrix legs); `publish-release` downloads `release-*` so both
`release-linux-desktop-{amd64,arm64}` are picked up. One comment line near the
job will point at scripts/dev/sdme/chan-desktop-{ubuntu,fedora,arch}.sdme as
the LOCAL multi-distro build/QA path, which is how this "references" your
subagents' templates under M1.
