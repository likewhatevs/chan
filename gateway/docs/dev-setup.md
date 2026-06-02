# Gateway setup: local, mirroring production

The gateway runs as a set of `sdme` containers on a private network zone,
in production and locally alike. This guide stands up that same
all-container stack on your machine (Linux directly, or macOS via a Lima
VM), so what you validate locally is the shape production runs.

It mirrors the production definitions in the sibling `chan-prod-setup`
repo. Per "show the pattern, copy little", it walks ONE worked service
container end to end and points at `chan-prod-setup` for the rest, rather
than duplicating every prod config here.

> A faster inner loop exists for rapid iteration: `scripts/dev/run.sh`
> runs the services as host `cargo run` binaries over `*.localtest.me`
> (see [`scripts/dev/README.md`](../scripts/dev/README.md)). That is handy
> while editing code, but it is NOT the prod-like shape. This guide is the
> all-container stack.

## Why the all-container, prod-like stack

The gateway's cross-tenant isolation is carried by two host-scoped
cookies: `id_session` (host-only on `id.<domain>`) and `workspace_gate`
(host-only and path-scoped on `{user}.workspace.<domain>/{workspace}/`).
No `.<domain>`-wide cookie exists, so a browser never auto-attaches an
identity session to a fetch on another tenant's subdomain. That design,
plus the reverse-proxy header hygiene (hop-by-hop stripping, dropped
inbound Host/Cookie/Authorization, recomputed `X-Forwarded-*`), only fully
exercises behind a real TLS terminator with real subdomains. Running the
same containers and the same nginx as prod is how you exercise it.

## Topology

```
  browser (https)
        |
        v  127.0.0.1:443  (Lima forwards the VM's :443 to the macOS host)
  +---------------------- chan-svc zone (private bridge, inside Lima) ------+
  |  chan-nginx   TLS terminator; the ONLY published container (:80,:443)  |
  |     id.<domain>          -> chan-id:7000                               |
  |     <domain> (apex)      -> chan-workspace-proxy:7002  (admin, healthz)|
  |       /v1/tunnel (h2c)   -> chan-workspace-proxy:7100  (grpc_pass)     |
  |     *.workspace.<domain> -> chan-workspace-proxy:7002  (tenant + WS)   |
  |                                                                        |
  |  chan-id:7000   chan-profile:7001   chan-workspace-proxy:7002 + :7100  |
  |  chan-psql:5432   (also published :5432 for host-side cargo test)      |
  +------------------------------------------------------------------------+
```

Services bind their default ports (`7000/7001/7002/7100`) INSIDE their
containers and resolve each other by container hostname on the `chan-svc`
zone (for example identity reads `chan-profile:7001`). Nothing binds on
the macOS host except what Lima forwards (nginx `:443`, and Postgres
`:5432` for host-run tests). Because no gateway port lands on the macOS
host, the macOS AirPlay `:7000` clash never arises and the code defaults
stay at the 7000 range, identical to prod.

## Prerequisites: sdme

Install sdme. On Linux, on the host:

```sh
curl -fsSL https://fiorix.github.io/sdme/install.sh | sudo sh
```

On macOS, sdme runs inside a Lima VM; install Lima and then sdme inside
the VM, per [macOS only: Lima shim](#macos-only-lima-shim). Either way the
`sdme ...` commands below then work (on macOS through the alias). The
examples use the explicit `limactl shell default sudo sdme ...` form; drop
that prefix on Linux.

## Build the gateway .deb packages

The service containers install the gateway `.deb`s, the same way prod
does, so build them first (once per source change) in the gateway-build
container:

```sh
make linux-gateway     # root Makefile -> build-gateway.sh, uses gateway-build.sdme
```

`gateway-build.sdme` (in `scripts/dev/sdme/`) bakes the Rust toolchain,
node/npm, and cargo-deb; no Postgres is needed at build time. The four
packages (identity, profile, workspace-proxy, admin) land in the build's
`dist/` staging dir, where the service containers pick them up.

## Postgres: chan-psql on the zone

Build and start the Postgres container on the `chan-svc` zone. The build
file is a sanitized dev copy of the prod one (no host bind-mount, a
throwaway `chan` superuser with password `chan`, both `chan_gateway` and
`chan_gateway_test` seeded on first boot).

```sh
cd gateway/scripts/dev/sdme
limactl shell default sudo sdme fs build chan-psql-dev chan-psql.sdme
limactl shell default sudo sdme create chan-psql -r chan-psql-dev \
    --network-zone chan-svc -p 5432:5432
limactl shell default sudo sdme start chan-psql
```

Services reach it as `chan-psql:5432` on the zone; the published `:5432`
(via Lima host networking) lets host-side `cargo test` use
`127.0.0.1:5432`. The dev `create` drops prod's `--hardened` and secret
bind (the dev rootfs self-seeds); `--network-zone chan-svc` is what puts
it on the zone.

## The service containers (pattern + one worked example)

Each gateway service is its own container built from a tiny `.sdme` file
that installs the matching `.deb` and enables its systemd unit. The prod
files live in `chan-prod-setup/services/` (`chan-id.sdme`,
`chan-profile.sdme`, `chan-workspace-proxy.sdme`); a dev-sanitized copy
differs only in where secrets come from. Worked example, identity:

```dockerfile
# chan-id-dev.sdme: chan-gateway-identity (id.<domain> on :7000)
FROM ubuntu
RUN apt-get update && \
    DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
        ca-certificates && rm -rf /var/lib/apt/lists/*
COPY dist /tmp/dist
RUN set -eux; \
    deb=$(ls /tmp/dist/chan-gateway-identity_*.deb | head -1); \
    apt-get update; DEBIAN_FRONTEND=noninteractive apt-get install -y "$deb"; \
    rm -rf /tmp/dist /var/lib/apt/lists/*; \
    install -d /etc/systemd/system/chan-gateway-identity.service.d; \
    # DEV: inline the env instead of prod's bind-mounted /run/chan-secrets.
    # Hostname-based cross-service URLs resolve on the chan-svc zone.
    printf '[Service]\n\
Environment=BIND_ADDR=0.0.0.0:7000\n\
Environment=BASE_URL=http://id.localtest.me\n\
Environment=PROFILE_SERVICE_URL=http://chan-profile:7001\n\
Environment=WORKSPACE_ADMIN_URL=http://chan-workspace-proxy:7002\n\
Environment=COOKIE_SECURE=false\n\
Environment=GITHUB_CLIENT_ID=...  GITHUB_CLIENT_SECRET=...\n' \
        > /etc/systemd/system/chan-gateway-identity.service.d/dev-env.conf; \
    systemctl enable chan-gateway-identity
```

Build, create on the zone, start:

```sh
limactl shell default sudo sdme fs build chan-id-dev chan-id-dev.sdme
limactl shell default sudo sdme create chan-id -r chan-id-dev --network-zone chan-svc
limactl shell default sudo sdme start chan-id
```

`chan-profile` and `chan-workspace-proxy` follow the identical shape:
install their `.deb`, set their bind addr and the hostname-based URLs
(`profile` needs `DATABASE_URL=postgres://chan:chan@chan-psql:5432/chan_gateway`;
`workspace-proxy` needs `IDENTITY_URL=http://chan-id:7000`,
`TUNNEL_BIND_ADDR=0.0.0.0:7100`, `FORWARDED_PROTO=https`, and the
`WORKSPACE_GATE_SECRET`/`IDENTITY_INTERNAL_TOKEN` shared secrets). Generate
the shared secrets with `openssl rand -hex 32` and reuse the matching
value across the two services that share each one. See
`chan-prod-setup/services/` for the prod versions and
`chan-prod-setup/bin/secrets-init.sh` for the full secret set.

## nginx container + TLS

nginx is its own container (`chan-nginx`), the TLS terminator and the only
one that publishes ports. Mirror `chan-prod-setup/services/chan-nginx.sdme`
and `chan-prod-setup/etc/nginx/`; the route map is:

```
id.<domain>                  -> chan-id:7000               (proxy_pass)
<domain> (apex)              -> chan-workspace-proxy:7002  (admin + healthz)
<domain>/v1/tunnel           -> chan-workspace-proxy:7100  (grpc_pass, h2c)
*.workspace.<domain>         -> chan-workspace-proxy:7002  (tenant + WS upgrade)
```

The one dev difference is the certificate. Prod uses certbot with the
dns-01 Cloudflare plugin to get a real `*.workspace.<domain>` wildcard
(http-01 cannot issue wildcards). Locally, issue a local-CA wildcard with
[`mkcert`](https://github.com/FiloSottile/mkcert) and mount it into the
nginx container in place of `/etc/letsencrypt`:

```sh
mkcert -install
mkcert "*.localtest.me" "*.workspace.localtest.me" localtest.me
```

Create chan-nginx on the zone, publishing `:443`, with the mkcert cert and
your `:443` vhosts bind-mounted in:

```sh
limactl shell default sudo sdme create chan-nginx -r chan-nginx-dev \
    --network-zone chan-svc -p 443:443 \
    --bind <mkcert-dir>:/etc/nginx/certs:ro
limactl shell default sudo sdme start chan-nginx
```

## Reach it from the browser (macOS)

`*.localtest.me` resolves to `127.0.0.1` for every subdomain via public
DNS, so no `/etc/hosts` or dnsmasq is needed. Lima host networking exposes
the chan-nginx `:443` on the macOS `localhost`, so the browser path is:
`https://id.localtest.me` -> `127.0.0.1:443` (Lima) -> chan-nginx ->
`chan-id:7000` on the zone. No `limactl` port-forward is needed: Lima
host networking surfaces the published `:443` on the macOS `localhost`
directly, the same way it does Postgres `:5432`.

Sign in at `https://id.localtest.me`. Both feature flags ship default-off,
so enrol yourself after the first sign-in (run the admin CLI inside the
profile container, or against the published profile port):

```sh
limactl shell default sudo sdme exec chan-profile -- \
    chan-gateway-admin flag grant oauth_login      <your-email>
limactl shell default sudo sdme exec chan-profile -- \
    chan-gateway-admin flag grant share_workspaces <your-email>
```

Register a test workspace from the sibling `chan` repo over the TLS apex:

```sh
export CHAN_TUNNEL_TOKEN=chan_pat_...     # mint under the dashboard Tokens tab
cargo run -p chan -- serve <workspace-dir> \
  --tunnel-url=https://workspace.localtest.me/v1/tunnel \
  --tunnel-workspace-name=blog
```

Clicking Open on the dashboard lands on
`https://<user>.workspace.localtest.me/blog/`.

## From local to a real VPS

Because the local stack already IS the prod container shape, going to a
real host changes only what is environment-specific, exactly as
`chan-prod-setup` automates (`configure.sh` then `make all`):

- **DNS.** Real records for `id.<domain>`, `<domain>` (apex), and a
  wildcard `*.workspace.<domain>` pointed at the host; inbound `:80/:443`
  DNAT to chan-nginx in the zone.
- **Certificates.** Swap mkcert for certbot with your provider's dns-01
  plugin to get the real `*.workspace.<domain>` wildcard (the wildcard
  forces dns-01; @@Host uses Cloudflare, pick your own provider).
- **Secrets.** Real per-service secrets bind-mounted from
  `/var/lib/chan/secrets` instead of the inlined dev values;
  `COOKIE_SECURE=true`.

The containers, the zone, the nginx routes, and the cookie isolation are
identical to what you ran locally.

## macOS only: Lima shim

On macOS, sdme runs inside a Lima VM because it needs systemd. Lima uses
host networking, so container ports show up on macOS `localhost` exactly
as on a native Linux host. macOS `$HOME` is bind-mounted into the VM
read-only via virtiofs: edit and build on macOS, sdme sees the result.

```sh
brew install lima
limactl start default        # Ubuntu, host networking
# install sdme inside the VM:
limactl shell default -- sh -c \
    'curl -fsSL https://fiorix.github.io/sdme/install.sh | sudo sh'
alias sdme='limactl shell default sudo sdme'   # then every sdme example runs verbatim
```

The bare `limactl shell default sudo sdme ...` form works too (useful for
scripts and agents, where the interactive alias does not resolve).

## Running tests

```sh
cd gateway
export TEST_DATABASE_URL=postgres://chan:chan@127.0.0.1:5432/chan_gateway_test
npm ci && npm run build --workspaces   # SPA; rust-embed needs web/dist
cargo test                             # profile + identity need the DB
```

`workspace-proxy` and all `cargo test --lib` unit tests need no database;
only `profile` and `identity` integration tests do. Per-test schema
isolation means a `cargo test` run never clobbers the `chan_gateway` DB a
running stack uses. CI (`gateway-ci.yml`) runs the same gate with a
`postgres:16` service on `ubuntu-latest` (x86_64), the canonical lane;
local sdme is the fast loop.

### Connection reaper (test infra)

A flaky `cargo test` can panic mid-test and orphan sqlx pool connections;
the role goes idle holding slots and the next run hits `PoolTimedOut`.
`tests-shared/pg_reaper.rs` (wired into every DB-backed `TestApp::new()`)
opens one durable connection and `pg_terminate_backend()`s its own role's
idle peers on first use, then holds that connection so the role never
falls fully idle. It recovers the realistic case automatically. The one
case it cannot is **full exhaustion** (all non-superuser slots pinned):
it panics pointing here. Reap manually as the postgres superuser:

```sh
limactl shell default sudo sdme exec chan-psql -- /bin/bash -c \
    "runuser -u postgres -- /usr/bin/psql -c \
        \"SELECT pg_terminate_backend(pid) \
            FROM pg_stat_activity WHERE usename='chan';\""
```

Safe whenever no live stack is connected to `chan_gateway`.

## sdme cheatsheet

- **Full container name**: pass the name you created (`chan-id`,
  `chan-psql`, ...). sdme also accepts an unambiguous prefix, but the full
  name keeps the examples copy-pasteable.
- **Full paths after `--`**: `machinectl shell` sets no `PATH`. Use
  `/usr/bin/psql`, `/usr/bin/runuser`, `/usr/bin/systemctl`.
- **Interactive shell**: `sdme join chan-id` drops you into a real shell
  inside the container; live `apt install ./chan-gateway-*.deb` works there
  without a rootfs rebuild.
- **Restart a unit**: `sdme exec chan-id -- /usr/bin/systemctl restart
  chan-gateway-identity`.

## Troubleshooting

- **`connection refused on localhost:5432`** — `sdme ps` should list
  chan-psql Running; if stopped, `sdme start chan-psql`; if wedged under
  load, `sdme exec chan-psql -- /usr/bin/systemctl restart postgresql`.
- **A service can't reach another** — they resolve by container hostname
  ON the `chan-svc` zone, so every service container (and chan-psql) must
  be created with `--network-zone chan-svc`; check `sdme ps` and the
  hostname-based URLs in each unit's env.
- **Browser rejects the local cert** — run `mkcert -install` so the local
  CA is trusted, and reissue the wildcard if you changed the domain.
- **Signed-in but the workspace 404s** — confirm nginx serves https and
  `FORWARDED_PROTO=https` is set on workspace-proxy; a scheme mismatch
  makes the `workspace_gate` cookie fail to attach.
- **Tests pass locally but break on CI** — same migration set must run
  (`migrations/0001..N` in order); a forgotten file shows up as
  missing-column errors on first use.
