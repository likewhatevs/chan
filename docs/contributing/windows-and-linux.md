# Building and testing chan on Windows (and Linux from a Windows host)

> **Status: design, not yet validated.** Nobody on the team has a Windows host, so the dev loop below is the *proposed* plan a Windows contributor would follow -- the companion to [`linux-and-macos.md`](linux-and-macos.md) for the third platform. The native-Windows **compile** path (`cargo-xwin`) and the **CI** path (the `windows-latest` job) are real and exercised; the WSL2 + `sdme` Linux-on-Windows loop is unverified and may need adjustment when first run on real hardware. Flag corrections back into this doc.

chan supports Windows on two fronts, kept deliberately separate:

1. **Running / building chan-desktop *for* Windows** -- the GUI app, whose terminal runs the user's default shell (PowerShell, or `cmd` / a `CHAN_SHELL` override). You do **not** need a Windows machine to *compile-check* this from macOS/Linux; see "Native Windows build" below.
2. **Developing chan *on* a Windows machine** -- running the gates, building the Linux artifacts (AppImage / `.deb` / `.rpm`, the static musl CLI tarball) and the chan-gateway, the same way a macOS contributor uses `lima` + `sdme`. On Windows the Linux environment comes from **WSL2** (recommended) or **Hyper-V**; `sdme` runs inside it. See "Linux dev loop on a Windows host" below.

For the core build/test commands on your host directly (fmt, clippy, tests, web), see [`CONTRIBUTING.md`](../../CONTRIBUTING.md).

## Native Windows build (no Windows host needed)

The fast local proof that the `cfg(windows)` arms compile is a cross-compile from macOS/Linux with [`cargo-xwin`](https://github.com/rust-cross/cargo-xwin), which downloads the MSVC CRT + Windows SDK headers on demand (no Windows host, and **no Wine** -- Wine cannot run WebView2, so a Wine-in-a-container GUI run is not viable):

```sh
# from the repo root or desktop/: installs cargo-xwin + the rust target on demand
make -C desktop xwin-check
```

This checks the core crates (`chan-server`, `chan-shell`, `chan-workspace`) for `x86_64-pc-windows-msvc`. The chan-desktop crate's full Windows build pulls the whole Tauri + WebView2 toolchain and is **not** part of `xwin-check`; it is built and bundled (NSIS) and headlessly smoked by the **`windows-latest`** job in [`.github/workflows/release-desktop.yml`](../../.github/workflows/release-desktop.yml). That CI job is the authoritative Windows build; the NSIS installer it uploads (`chan-desktop-windows-x86_64`) is what you download with `gh run download` to smoke on a real Windows box.

A clean Win11 needs the **WebView2 evergreen runtime**. It ships with current Windows 10/11 and the `windows-latest` runner has it; for an older or stripped image, install Microsoft's Evergreen Bootstrapper (`MicrosoftEdgeWebview2Setup.exe`) -- the NSIS installer can also bundle the bootstrapper check via Tauri's `webviewInstallMode`.

## Linux dev loop on a Windows host

chan's gates and Linux artifacts are built in `sdme` (the project's systemd-nspawn container manager), exactly as on macOS -- the only difference is how you reach a Linux kernel. On macOS that's `lima`; on Windows it's **WSL2**.

The Makefiles parameterize this with the **`SDME`** variable (how `sdme` is invoked) and **`DISTRO`** (which rootfs to build). On a native Linux host `SDME='sudo sdme'`; on macOS `SDME='limactl shell default sudo sdme'`. On Windows you have two equivalent options:

- **Recommended -- work *inside* WSL2.** Treat the WSL2 distro as a native Linux host: clone the repo into the WSL2 ext4 filesystem (your Linux `~`, **not** `/mnt/c`) and follow [`linux-and-macos.md`](linux-and-macos.md) verbatim with `SDME='sudo sdme'`. This is the simplest path and avoids every cross-filesystem and path-translation wrinkle below.
- **Drive WSL2 from the Windows shell.** Set `SDME='wsl sudo sdme'` so the Windows-side `make` shells into WSL for each `sdme` call. Convenient if you live in PowerShell, but the host↔WSL path translation (the build scripts do `git archive` on the host then `sdme cp` into the container) is the rough edge to expect first.

### Prerequisites

- **WSL2 with systemd enabled.** `sdme` is systemd-nspawn, so the WSL distro must run systemd. On WSL 0.67.6+ enable it once in `/etc/wsl.conf`:

  ```ini
  [boot]
  systemd=true
  ```

  then `wsl --shutdown` and reopen. Install a distro (`wsl --install -d Ubuntu`) and the systemd/nspawn host tooling `sdme` needs inside it, same as a Linux host.
- **Hyper-V alternative.** If WSL2 + nspawn proves troublesome (systemd-nspawn inside WSL2 leans on cgroup/namespace support that has historically lagged), a full Linux guest under Hyper-V behaves as a plain Linux host: install `sdme` there and use `SDME='sudo sdme'` inside the guest. Heavier than WSL2; reserve it for the case WSL2 can't do nspawn.
- **Git line endings.** Set `git config --global core.autocrlf input` (or rely on the repo's `.gitattributes`) so shell scripts and the `.cmd` shims keep their intended endings across the Windows/WSL boundary.

### Running the gate + building artifacts

With a working WSL2 (or Hyper-V) Linux environment, the same targets the macOS doc uses apply -- only `SDME` changes. From inside WSL2 (recommended):

```sh
# one-time: import the base image (inside WSL, sdme runs natively)
sudo sdme fs import docker.io/ubuntu --name ubuntu

# the Linux chan-desktop bundles (AppImage / .deb / .rpm)
make linux-chan-desktop DISTRO=ubuntu SDME='sudo sdme'

# the gateway .deb packages (separate Cargo workspace)
make linux-gateway SDME='sudo sdme'

# the static musl CLI tarball (host cross-compile, no container)
make linux-chan-tarball LINUX_TARGET=x86_64-unknown-linux-musl
```

Driving from the Windows shell instead, the same commands take `SDME='wsl sudo sdme'`.

The core gate itself (`make ci-linux`) runs inside an sdme container exactly as in [`linux-and-macos.md`](linux-and-macos.md#core-run-the-ci-gate-in-a-linux-container) -- seed the tree with `git archive HEAD`, install the deps, run the gate. Reuse those instructions; they are not duplicated here.

### Filesystem + performance notes

- **Stay on ext4.** WSL2 reaches Windows drives over a 9p mount at `/mnt/c`, which is slow for the many small files cargo and node touch. Keep the repo, `target/`, and `node_modules` on the WSL2 ext4 filesystem. This is the Windows analogue of the macOS caveat that `lima` mounts `/Users` read-only -- different cause (perf vs read-only), same conclusion: do the build inside the Linux fs.
- **x86_64 by default.** A Windows dev host is x86_64, so the WSL2 containers are x86_64 Linux -- matching CI's `ubuntu-latest` lane. There is no aarch64 fp16 build wrinkle to work around (that one only affects the Apple-Silicon lima/sdme flow); a plus of the Windows loop over the macOS one.

## How this maps to CI

CI does not use WSL2 or `sdme`. As on the other platforms, GitHub Actions runs natively:

- `.github/workflows/ci.yml` runs the Linux/macOS gates on their native runners.
- The `windows-latest` arm of [`release-desktop.yml`](../../.github/workflows/release-desktop.yml) builds and NSIS-bundles chan-desktop and runs the headless boot/`/api/health` smoke.

The WSL2 + `sdme` flow above is the *local* way to reproduce the Linux environment on a Windows machine; `cargo-xwin` is the *local* way to compile-check the native Windows build. Both are the fast loop -- CI is the canonical lane (and owns the authoritative Windows artifact).
