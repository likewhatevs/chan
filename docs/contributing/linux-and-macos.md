# Building and testing chan on Linux and macOS

chan's gates run on Linux (the canonical CI target) and macOS. The
reproducible way to get a Linux environment **locally** on either host
is `sdme`, the project's systemd-nspawn container manager. This is the
same flow for everyone: on a Linux host you run `sdme` directly; on
macOS you run it inside a lightweight Linux VM via `lima`.

`sdme` is a local-development tool only. CI does not use it: GitHub
Actions runs on its own ubuntu runners (native, with apt-installed
deps and service containers). The dev setup and the CI setup are
intentionally different, the same as the rest of chan; see "How this
maps to CI" at the end.

This doc covers the local Linux flow. For the core build/test
commands themselves (fmt, clippy, tests, web) on your host directly,
see [`CONTRIBUTING.md`](../../CONTRIBUTING.md). For the gateway's
Postgres-backed tests, see
[`gateway/docs/testing-on-linux-and-macos.md`](../../gateway/docs/testing-on-linux-and-macos.md).

## Prerequisites

- **Linux**: install `sdme` and the systemd/nspawn host tooling it
  needs. Run every `sdme ...` command below directly.
- **macOS**: install `lima` and run `sdme` inside the VM. Every
  command below uses the explicit form
  `limactl shell default sudo sdme ...` (substitute your VM name for
  `default`; the docs name it `chan-dev`). Interactively you can set
  `alias sdme='limactl shell default sudo sdme'` and then the examples
  read verbatim.

Two macOS specifics worth knowing:

- `lima` exposes your home directory to the VM **read-only** via
  virtiofs, so the VM can read the repo at the same path. `sdme`
  containers, however, have their own rootfs and do not inherit that
  mount: move files in with `sdme cp` (see below).
- `sdme` containers use host networking, so a service listening on
  `:PORT` inside a container is reachable at `localhost:PORT` from
  macOS.

On Apple Silicon the containers are aarch64 Linux. That is sufficient
for distro portability, dependency availability, clippy, and tests;
x86_64-specific issues still ride CI's `ubuntu-latest`.

## One-time: import a base image

```sh
limactl shell default sudo sdme fs import ubuntu docker.io/ubuntu
```

## Core: run the CI gate in a Linux container

CI runs `make ci-linux` on `ubuntu-latest` after installing the Tauri
build dependencies. Mirror that locally:

```sh
# create + start a build container from the ubuntu base
limactl shell default sudo sdme create chan-build -r ubuntu
limactl shell default sudo sdme start  chan-build

# seed the repo (tracked files only) into the container
git archive HEAD -o ~/chan-src.tar
limactl shell default sudo sdme cp ~/chan-src.tar chan-build:/root/chan.tar
limactl shell default sudo sdme exec chan-build /bin/sh -c \
  'mkdir -p /root/chan && tar -xf /root/chan.tar -C /root/chan'

# install deps (same set CI installs) + the pinned Rust toolchain
limactl shell default sudo sdme exec chan-build /bin/sh -c '
  export DEBIAN_FRONTEND=noninteractive
  apt-get update -qq
  apt-get install -y build-essential pkg-config curl ca-certificates \
    nodejs npm libwebkit2gtk-4.1-dev libayatana-appindicator3-dev \
    librsvg2-dev libsoup-3.0-dev patchelf
  curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal'

# run the gate (reads rust-toolchain.toml -> 1.95.0)
limactl shell default sudo sdme exec chan-build /bin/sh -c '
  export HOME=/root; . /root/.cargo/env; cd /root/chan && make ci-linux'
```

`git archive HEAD` ships committed files only; re-run it (and re-`cp`)
after each commit you want reflected in the container. To iterate
without rebuilding the container, `sdme cp` individual files or
`sdme join chan-build` for an interactive shell.

## Desktop: build the chan-desktop AppImage and .deb

`make chan-desktop` runs `cargo tauri build` natively, so on macOS it
produces a macOS `.app`, not Linux bundles. To build the Linux
chan-desktop bundles (the AppImage and `.deb` that `release.yml`'s
`linux-desktop-artifacts` job ships, plus the `.rpm` Tauri's
`targets:"all"` emits for free) from a macOS workstation, run
`cargo tauri build` inside an sdme container:

```sh
# one-time: import the ubuntu base (shared with the core gate above)
limactl shell default sudo sdme fs import ubuntu docker.io/ubuntu

# build the bundles for one distro (default ubuntu)
make linux-chan-desktop DISTRO=ubuntu
```

`make linux-chan-desktop` drives
`scripts/dev/sdme/build-chan-desktop.sh`, which:

1. builds the `chan-desktop-<distro>` rootfs from
   `scripts/dev/sdme/chan-desktop-<distro>.sdme` on first use (it bakes
   the Tauri build deps and the Rust toolchain, so this cost is paid
   once),
2. creates or reuses a `chan-desktop-build-<distro>` container,
3. seeds the committed tree (`git archive HEAD`),
4. runs `make chan-desktop` inside it, and
5. copies the bundles out to `target/linux-desktop/<distro>/`.

The container is reused between runs so the cargo target cache
survives (a cold compile is around 20 minutes; an incremental rebuild
is a minute or two). Force a clean container with
`REBUILD_CONTAINER=1 make linux-chan-desktop`.

How sdme is reached is the `SDME` variable (a lima VM on macOS by
default, `sudo sdme` directly on a Linux host):

```sh
# native Linux host
make linux-chan-desktop SDME='sudo sdme'
```

Two container facts the driver accommodates, both worth knowing if you
script `cargo tauri build` in sdme yourself:

- sdme mounts a small (~800M) tmpfs over `/tmp`; the cold Rust +
  tauri-cli compile overflows it. The in-container build sets
  `TMPDIR=/var/tmp` (the disk-backed overlay).
- the chan-desktop rootfs needs `xdg-utils` on top of the core gate's
  Tauri deps: the AppImage plugin shells out to `xdg-mime`, and
  without it the bundle fails at the very end with
  `xdg-mime binary not found`.

On Apple Silicon the bundles are aarch64 Linux; CI's `ubuntu-latest`
owns the canonical x86_64 desktop build. The AppImage is built with
`linuxdeploy` in extract-and-run mode, so no FUSE or display is needed
in the container.

### Verifying the cs alias on the AppImage

chan-desktop runs as both `chan` and `cs` — the same binary re-execing
itself with `argv[0]` set to the name (the
`chan_shell::invoked_as_chan` / `invoked_as_cs` argv0 detection), so the
CLI / control client runs instead of the GUI. On boot it owns the
`~/.local/bin/{chan,cs}` shims (`desktop/src-tauri/src/cs_install.rs`):
real symlinks to the installed binary for a `.app` or deb/rpm install,
`exec -a` wrapper scripts for an AppImage (whose `current_exe()` is an
ephemeral mount). The shims self-heal on a move or self-upgrade, are
idempotent, and never clobber a `chan` / `cs` you installed yourself.
To check the client path on a built artifact, point the inner binary at
a running server's control socket and confirm it runs the client, not
the GUI:

```sh
# in the container, with a `chan serve` running and $CHAN_CONTROL_SOCKET
# pointed at its socket:
exec -a cs ./squashfs-root/usr/bin/chan-desktop terminal list   # rc=0, no GUI
```

Packaged-AppImage dispatch: invoking the AppImage through its `cs` / `chan`
wrapper (`exec -a cs "$APPIMAGE"`) DOES reach the inner binary as the right
name. linuxdeploy's `AppRun` re-execs `AppRun.wrapped` without preserving
argv[0], but the type-2 AppImage runtime exports the `exec -a` name as
`$ARGV0`, and the `cs` / `chan` stem probes read `$ARGV0` before `argv[0]`
(`chan_shell::invoked_arg0`), so the CLI / control client runs instead of the
GUI. Off an AppImage `$ARGV0` is unset and behavior is unchanged.

## CLI: build the static musl `chan` tarball

The standalone `chan` CLI tarball (what `install.sh` and the self-upgrade
download) is built fully static against musl, so a too-new build glibc does
not gate older Linux machines. The `.deb`/`.rpm` packages and the
chan-desktop AppImage stay gnu: the distro provides glibc, and webkit cannot
be static.

This is a host cross-compile, no container. cargo-zigbuild uses zig as the
cross C/C++ compiler so the C/C++ deps (ring, bundled SQLite, tokenizers'
esaxx-rs/onig) link static. Prerequisites on the host: zig, cargo-zigbuild,
and the musl rust targets.

```bash
# one-time: tooling (zig must already be on PATH) + the musl rust targets
cargo install cargo-zigbuild
rustup target add x86_64-unknown-linux-musl aarch64-unknown-linux-musl

# build the static tarball for an arch (from the repo root)
make linux-chan-tarball LINUX_TARGET=x86_64-unknown-linux-musl
make linux-chan-tarball LINUX_TARGET=aarch64-unknown-linux-musl

# the tarball lands at target/release/chan-<target>.tar.gz; confirm static
tar -xzf target/release/chan-x86_64-unknown-linux-musl.tar.gz -C /tmp
file /tmp/chan          # -> ... statically linked
# on Linux: ldd /tmp/chan -> "not a dynamic executable"
```

CI builds these in `release.yml`'s `linux-cli-artifacts` job (zig via
`mlugg/setup-zig` + cargo-zigbuild); the `.deb`/`.rpm` in that same job stay
gnu. The `chan-tarball` Make target uses `cargo zigbuild` for musl targets
and plain `cargo build` for gnu.

## Gateway: Postgres-backed tests

The gateway is a separate workspace with Postgres-backed integration
tests. Its container setup (a `chan-psql` Postgres rootfs) and the
test commands live in
[`gateway/docs/testing-on-linux-and-macos.md`](../../gateway/docs/testing-on-linux-and-macos.md).

The gateway's per-change test loop runs `cargo` + `npm` **on your
host** against the `chan-psql` container at `localhost:5432` (host
networking makes it reachable). That inner loop needs Rust + Node on
the host; only Postgres lives in a container. Binding the source into
a container instead does not help here: lima mounts `/Users`
read-only, so `cargo`/`npm` could not write `target/`, `node_modules`,
or `web/dist`. The in-container flows on this page (the core gate
above, the packaging validation below) are for CI parity and deploy
validation, not the inner loop.

## Packaging: validate a deploy locally

The gateway ships four `.deb` packages run under systemd. To verify
the prod path (packages -> postinst user -> systemd units ->
`configure.sh` -> running services) end to end, build and install them
in a systemd container with a reachable Postgres (the `chan-psql`
container from the gateway doc; host networking makes it reachable at
`localhost:5432`).

```sh
# create a build container and seed the repo (tracked files only)
limactl shell default sudo sdme create chan-gw-build -r ubuntu
limactl shell default sudo sdme start  chan-gw-build
git archive HEAD -o ~/chan-src.tar
limactl shell default sudo sdme cp ~/chan-src.tar chan-gw-build:/root/chan.tar

# drop into the container; everything below runs inside it
limactl shell default sudo sdme join chan-gw-build
```

Inside `chan-gw-build`:

```sh
mkdir -p /root/chan && tar -xf /root/chan.tar -C /root/chan

# build deps + the pinned toolchain. openssl + python3 are required by
# configure.sh (random secrets + password URL-encoding).
export DEBIAN_FRONTEND=noninteractive
apt-get update
apt-get install -y build-essential pkg-config libssl-dev curl \
  ca-certificates nodejs npm openssl python3
curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
. "$HOME/.cargo/env"
cargo install cargo-deb

# build the SPA, then the four .debs
cd /root/chan/gateway
npm ci && npm run build --workspaces
cargo build --release -p profile -p identity -p workspace-proxy -p admin
for c in profile identity workspace-proxy admin; do cargo deb --no-build -p "$c"; done

# install (postinst creates the chan-gateway user + units + default env),
# generate config, start, and check health
dpkg -i target/debian/*.deb || apt-get -f install -y
bash scripts/configure.sh   # answers: PG user/pass/db, base domain, scheme, >=1 provider
systemctl enable --now chan-gateway-profile chan-gateway-identity chan-gateway-workspace-proxy
systemctl is-active chan-gateway-profile chan-gateway-identity chan-gateway-workspace-proxy
for p in 7001 7000 7002; do curl -fsS "http://127.0.0.1:$p/healthz"; echo; done
```

`configure.sh` points `DATABASE_URL` at `127.0.0.1` for the answers
above, so the `chan-psql` container must be running on the same host
network. All three `/healthz` returning `ok` means the deb assets,
the systemd units (which load the shared `domain.env` first), the
generated secrets, and the workspace-gate wiring are all consistent.

## How this maps to CI

CI does not run `sdme`. GitHub Actions runs directly on its ubuntu
runners with the deps installed natively; the `sdme` flow above is
just the local way to reproduce that Linux environment on your
machine. The two are deliberately separate setups.

- `.github/workflows/ci.yml` runs `make ci-linux` then `make ci-macos`
  (the same Make targets the container runs above), with the Tauri
  build deps apt-installed on the runner.
- `.github/workflows/gateway-ci.yml` runs the gateway gate directly on
  the runner against a `postgres:16` **service container** (not
  `chan-psql`), scoped to `gateway/**`.

Local `sdme` is the fast loop; CI is the canonical lane (and owns
x86_64).

## aarch64 Linux build note

chan does not compile for aarch64-linux out of the box: gemm-common
0.19 (a candle dependency) uses fp16 inline asm (`fmla .8h`) that the
default `aarch64-unknown-linux-gnu` target lacks. On
Apple-Silicon-hosted VMs (the lima/sdme flow above) the CPU supports
it — build with:

    RUSTFLAGS="-C target-feature=+fp16" cargo build ...

x86_64 (CI's lane) is unaffected.
