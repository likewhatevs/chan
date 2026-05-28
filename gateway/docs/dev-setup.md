# Dev setup (sdme + Postgres)

chan-gateway runs Postgres-backed services (identity, profile,
workspace-proxy) plus their integration tests. The recommended dev
layout uses the same toolchain production runs:
[sdme][sdme] containers, with Postgres in one of them. The same
container configs that `chan-prod-setup` ships in production are
reused here, so what you exercise locally is what runs in prod.

On Linux you run sdme directly. On macOS sdme runs inside a
[Lima][lima] VM, and a one-line shell alias makes `sdme ...` Just
Work from the macOS shell. Everything below the [macOS only](#macos-only-lima-shim)
section applies identically to both.

[lima]: https://github.com/lima-vm/lima
[sdme]: https://github.com/fiorix/sdme

## Layout

```
+----------------------+
| dev host             |
|                      |
|  cargo / rustc       |
|  sdme  -----------+--+--> postgres container (chan_gateway,
|                   |        chan_gateway_test, role chan)
|  reaches pg via   |
|  localhost:5432   |
+-------------------+--+
```

sdme containers in this layout run with **host networking**, so
anything listening inside a container is reachable from the dev
host on `localhost`. Postgres is on `localhost:5432`; no
port-forward, no DSN gymnastics.

## One-time setup

1. **Install sdme** following the [sdme README][sdme]. The
   `chan-prod-setup` repo at `../chan-prod-setup/` carries the same
   container configs used in production; the dev box reuses them so
   image drift between dev and prod is zero.

2. **Bring up the postgres container.** One-time, via sdme. Check
   it's running:

   ```sh
   sdme ps         # postgres should be Running
   ```

   If you do not yet have the container, the `chan-prod-setup`
   build files under `services/chan-psql.sdme` are the source of
   truth for its rootfs and unit files; copy / adapt as needed for
   dev.

3. **(Optional) macOS only**, set up the Lima shim. See
   [below](#macos-only-lima-shim).

## Postgres credentials

The dev postgres container is provisioned with role `chan`
(password `chan`) and two databases:

- `chan_gateway`: the dev DB. `cargo run` against this one.
- `chan_gateway_test`: the integration-test DB. Tests create a
  fresh per-test schema (`t_<uuid>`) under it and drop it on
  teardown, so a `cargo test` run does not collide with a running
  `cargo run` dev server pointed at the dev DB.

Verify by minting a one-off SQL against the container:

```sh
echo 'SELECT current_user, current_database();' > /tmp/probe.sql
sdme cp /tmp/probe.sql postgres:/tmp/probe.sql
sdme exec po -- /usr/bin/runuser -u postgres -- \
    /usr/bin/psql -d chan_gateway -f /tmp/probe.sql
```

The `po` prefix-match is enough; `sdme exec <prefix> ...` resolves
to `postgres` here. Full paths in the argv after `--` are required:
`sdme exec` uses `machinectl shell` under the hood and does not
inherit a sensible `PATH`.

If the dev DBs ever need recreating from scratch:

```sh
# Drop + create dev DB
sdme exec po -- /usr/bin/runuser -u postgres -- \
    /usr/bin/psql -c 'DROP DATABASE IF EXISTS chan_gateway;'
sdme exec po -- /usr/bin/runuser -u postgres -- \
    /usr/bin/psql -c 'CREATE DATABASE chan_gateway OWNER chan;'

# Same for the test DB
sdme exec po -- /usr/bin/runuser -u postgres -- \
    /usr/bin/psql -c 'DROP DATABASE IF EXISTS chan_gateway_test;'
sdme exec po -- /usr/bin/runuser -u postgres -- \
    /usr/bin/psql -c 'CREATE DATABASE chan_gateway_test OWNER chan;'
```

Migrations apply on first service start; the test harness re-runs
the migration set into each fresh per-test schema.

## Running services

The [`scripts/dev/README.md`](../scripts/dev/README.md) runner
already wires this up. Point both URLs at the sdme Postgres:

```sh
export DATABASE_URL=postgres://chan:chan@localhost:5432/chan_gateway
export TEST_DATABASE_URL=postgres://chan:chan@localhost:5432/chan_gateway_test
```

Everything else (bind addresses, OAuth client id/secret, the
service bearer tokens) is documented in the dev runner README.

## Running tests

```sh
export TEST_DATABASE_URL=postgres://chan:chan@localhost:5432/chan_gateway_test
cargo test --workspace
```

Per-test schema isolation means a `cargo test` run never clobbers
state created by a running dev server pointed at `chan_gateway`.

### Connection reaper (test infra)

A flaky `cargo test` run can panic mid-test and leave sqlx pool
connections orphaned: the role goes idle with N held slots and
pg's defaults never reclaim them, so the next run hits
`PoolTimedOut`.

`tests-shared/pg_reaper.rs` is wired into every DB-backed
`TestApp::new()`. On the first call per test process it:

1. opens one durable connection (3 retries × 2s);
2. runs `pg_terminate_backend()` on every peer session for
   `current_user` whose state is `idle` or `idle in transaction`
   (PG 13+ lets non-superusers reap their own role's peers); then
3. holds that one connection alive for the rest of the process so
   the role never falls fully idle.

The reaper recovers automatically from the realistic case: a
previous run leaked N < `max_connections - 3` slots. The next
test-process startup reaps them.

The one case it cannot recover is **full exhaustion**: all
non-superuser slots are pinned. The reaper can't even get its
first connection and panics with a clear message pointing at
this section. Run the manual reap below and re-try.

### Manual reap (slots fully exhausted)

When `cargo test` panics with `pg_reaper: could not open a
connection after 3 attempts: ... remaining connection slots are
reserved for roles with the SUPERUSER attribute`, kill every
`chan` session from inside the postgres container as the
postgres superuser:

```sh
sdme exec po -- /bin/bash -c \
    "runuser -u postgres -- /usr/bin/psql -c \\
        \"SELECT pg_terminate_backend(pid) \\
            FROM pg_stat_activity WHERE usename='chan';\""
```

This frees every `chan` slot regardless of state. Safe whenever
no live `cargo run` dev server is connected to `chan_gateway`; if
one is, restart it after.

## sdme cheatsheet

- **Prefix-match container names**: `sdme exec po` resolves to
  `postgres`, `sdme exec chan` resolves to `chan-gw`, etc.
  Unambiguous prefixes only.
- **Full paths required after `--`**: `machinectl shell` does not
  set `PATH`. `/usr/bin/psql`, `/usr/bin/runuser`,
  `/usr/bin/systemctl`. The doc snippets here use full paths
  throughout for the same reason.
- **PTY caveat**: `sdme exec` allocates a PTY via `machinectl
  shell`. Stdout from one-liners can get swallowed; for any
  command whose output you actually need, drop the SQL / shell
  command into a file with `sdme cp` and execute the file
  (`psql -f`, `bash <path>`).
- **Interactive shell**: `sdme join postgres` (also accepts
  prefixes) drops you in a real PTY inside the container.
- **Restart a service inside the container**: e.g.
  `sdme exec po -- /usr/bin/systemctl restart postgresql`.

## macOS only: Lima shim

On macOS, sdme runs inside a Lima VM (`default`) because sdme
needs systemd. Lima is configured with **host networking**, so
container ports show up on macOS `localhost` exactly as on a
native Linux host. macOS `$HOME` is bind-mounted into the VM at
the same path via virtiofs, **read-only**: edit + build on macOS,
the result is visible to sdme inside the VM. Anything sdme needs
to write (rootfs builds, postgres data) lands under
`/var/lib/sdme/` *inside* the VM.

One-time:

```sh
brew install lima
limactl start default       # Ubuntu, host networking
limactl shell default       # one-time: install sdme inside the VM
                            # (see sdme README)
```

Add the shell alias from the sdme macOS tutorial
([shell alias][sdme-alias]) so every subsequent invocation goes
through Lima + sudo transparently:

```sh
alias sdme='limactl shell default sudo sdme'
```

After that, every `sdme ...` example earlier in this doc runs
verbatim from a macOS Terminal tab. The bare form
`limactl shell default sudo sdme ...` works too, useful for
scripts and CI where shell aliases aren't visible.

[sdme-alias]: https://fiorix.github.io/sdme/tutorial/macos/#shell-alias

## Troubleshooting

- **`password authentication failed for user "postgres"`** — you
  hit the `postgres` superuser without a password. Tests want the
  `chan` role; double-check `TEST_DATABASE_URL` matches the
  `chan:chan@localhost/chan_gateway_test` form.

- **`permission denied for database "postgres"`** — same shape;
  you connected as `chan` but pointed at the `postgres` database
  the role can't create schemas in. Use `…/chan_gateway` or
  `…/chan_gateway_test` as the DB.

- **`connection refused on localhost:5432`** — `sdme ps` should
  list `postgres` as Running. If it's stopped,
  `sdme start postgres`. If it's running but unresponsive (heavy
  test load can wedge it), restart the systemd unit *inside* the
  container:

  ```sh
  sdme exec po -- /usr/bin/systemctl restart postgresql
  ```

- **`sdme exec` returns no output** — `machinectl shell` always
  attaches a PTY and drops stdout from one-liners. Use the
  `sdme cp file.sql container:/tmp/file.sql; sdme exec ... psql
  -f /tmp/file.sql` pattern, or `sdme join` for interactive work.

- **Tests pass on one machine but break on CI** — make sure the
  same migration set runs: `migrations/0001..N` applied in numeric
  order. The test harness re-runs the migrations into each fresh
  schema, so a forgotten file in `migrations/` shows up as
  missing-column / missing-table errors on first use.
