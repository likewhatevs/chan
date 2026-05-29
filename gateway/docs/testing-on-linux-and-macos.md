# Testing the gateway on Linux and macOS

The gateway is Postgres-backed. Its integration tests
(`profile`, `identity`) create a throwaway schema per test under
`TEST_DATABASE_URL`, so they need a reachable Postgres with a
`chan_gateway_test` database. Any Postgres works, but the supported,
reproducible path uses `sdme` (the project's container tool) to run
Postgres in a Linux container, the same way CI does.

`workspace-proxy` tests and all `cargo test --lib` unit tests need no
database; only `profile` and `identity` integration tests do.

The build assets for the dev Postgres container live in
[`scripts/dev/sdme/`](../scripts/dev/sdme/): `chan-psql.sdme` plus the
config under `etc/postgresql/`. It is a sanitized copy of the prod
`chan-psql` service: no host bind mounts and no secrets, a throwaway
`chan` superuser (password `chan`), and both `chan_gateway` and
`chan_gateway_test` seeded on first boot.

> DEV ONLY. The password is hardcoded and `pg_hba` accepts password
> auth from any address. Never build or run this where it is reachable
> beyond your own machine.

## Prerequisites

- Linux: `sdme` plus the systemd/nspawn host tooling it needs. Run the
  `sdme` commands below directly (drop the `limactl shell default`
  prefix).
- macOS: `lima-vm` hosting `sdme` inside a Linux VM. The commands below
  use the explicit `limactl shell default sudo sdme ...` form. (Alex's
  interactive shell uses `alias sdme='limactl shell default sudo
  sdme'`; the alias only resolves in an interactive session, so
  scripts and agents use the explicit form. Our docs name the VM
  `chan-dev`; substitute your VM name for `default`.)

On macOS, `lima` exposes the host home directory to the VM read-only
via virtiofs, so the `COPY` lines in `chan-psql.sdme` resolve against
this repo as long as you run `sdme fs build` from the
`scripts/dev/sdme/` directory. And because `sdme` containers default
to host networking, a Postgres listening on `:5432` in the container
is reachable at `localhost:5432` from macOS.

Arch note: on Apple Silicon the container is aarch64 Linux. That is
fine for the gateway (DB behavior is arch-independent); CI on
`ubuntu-latest` still owns the canonical x86_64 run.

## One-time: import the base image

```sh
# downloads and imports ubuntu (currently 26.04) as a base rootfs
limactl shell default sudo sdme fs import ubuntu docker.io/ubuntu
```

## Build the Postgres rootfs

```sh
cd gateway/scripts/dev/sdme
limactl shell default sudo sdme fs build chan-psql-dev chan-psql.sdme
# add -f to rebuild over an existing rootfs of the same name
```

This installs `postgresql-18` and enables three units that run on
first boot: `chan-pg-init` (initdb if empty), `postgresql`, and
`chan-pg-bootstrap` (creates the `chan` role and the two databases).

## Create, start, and use the container

```sh
limactl shell default sudo sdme create chan-psql-1 -r chan-psql-dev
limactl shell default sudo sdme start  chan-psql-1
```

Connection string for everything (host -> container over host net):

```
postgres://chan:chan@127.0.0.1:5432/chan_gateway_test
```

## Run the tests

```sh
cd gateway
export TEST_DATABASE_URL=postgres://chan:chan@127.0.0.1:5432/chan_gateway_test

npm ci && npm run build --workspaces   # SPA; rust-embed needs web/dist
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test                             # profile + identity need the DB
```

## Container lifecycle

```sh
limactl shell default sudo sdme ps                 # list containers
limactl shell default sudo sdme logs chan-psql-1   # journalctl
limactl shell default sudo sdme join chan-psql-1   # shell inside, to poke at config
limactl shell default sudo sdme stop chan-psql-1
limactl shell default sudo sdme rm   chan-psql-1   # discard (no bind mount; data is in-container)
```

There is no host bind mount, so removing the container discards its
data. `sdme join` is the way to inspect or tweak Postgres in place.

## How CI does the same

`.github/workflows/gateway-ci.yml` runs this gate on `gateway/**`
changes with a `postgres:16` service container and the same
`TEST_DATABASE_URL`, on `ubuntu-latest` (x86_64). Local `sdme` is the
fast loop; CI is the canonical lane.
