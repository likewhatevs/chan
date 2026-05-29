# Building and testing chan on Linux and macOS

chan's gates run on Linux (the canonical CI target) and macOS. The
reproducible way to get a Linux environment locally on either host is
`sdme`, the project's systemd-nspawn container manager. This is the
same flow for everyone: on a Linux host you run `sdme` directly; on
macOS you run it inside a lightweight Linux VM via `lima`.

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

## Gateway: Postgres-backed tests

The gateway is a separate workspace with Postgres-backed integration
tests. Its container setup (a `chan-psql` Postgres rootfs) and the
test commands live in
[`gateway/docs/testing-on-linux-and-macos.md`](../../gateway/docs/testing-on-linux-and-macos.md).
Because containers share host networking, the gateway gate can run on
your host against the `chan-psql` container at `localhost:5432`, or in
its own build container alongside it.

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

- `.github/workflows/ci.yml` runs `make ci-linux` then `make ci-macos`
  (the same targets above).
- `.github/workflows/gateway-ci.yml` runs the gateway gate against a
  `postgres:16` service container, scoped to `gateway/**`.

Local `sdme` is the fast loop; CI is the canonical lane (and owns
x86_64).
