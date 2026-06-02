# Gateway setup: local dev to prod-like

How to run the gateway (identity + profile + workspace-proxy + Postgres)
on your own machine, from a loopback dev loop up to a prod-LIKE local
stack with nginx, TLS, and real wildcard subdomains. The prod-like stack
exercises everything production does except real DNS and a real cloud
host, so what you validate locally is what runs on a VPS.

The recommended toolchain is the same one production runs:
[sdme][sdme] containers (Postgres in one of them), with the same
container configs `chan-prod-setup` ships in prod, so image drift between
dev and prod is zero. On Linux you run sdme directly. On macOS sdme runs
inside a [Lima][lima] VM and a one-line alias makes `sdme ...` Just Work;
see [macOS only: Lima shim](#macos-only-lima-shim).

This guide runs the three services as host binaries (`cargo run`) with
Postgres in an sdme container. Production runs the same binaries as
systemd units from the release `.deb` packages; that swap is the only
topology difference, and it lives in
[From local to a real VPS](#from-local-to-a-real-vps).

[lima]: https://github.com/lima-vm/lima
[sdme]: https://github.com/fiorix/sdme

## Why prod-like, not just loopback

The gateway's cross-tenant isolation is carried by two host-scoped
cookies: `id_session` (host-only on `id.<domain>`) and `workspace_gate`
(host-only and path-scoped on `{user}.workspace.<domain>/{workspace}/`).
No `.<domain>`-wide cookie exists, so a browser never auto-attaches an
identity session to a fetch on another tenant's subdomain. That design,
plus the reverse-proxy header hygiene (hop-by-hop stripping, dropped
inbound Host/Cookie/Authorization, recomputed `X-Forwarded-*`), only
fully exercises with real subdomains, TLS, and nginx in front.

The loopback runner ([Running the full stack](#running-the-full-stack-loopback))
already gets you real subdomains (via `localtest.me`) and the whole sign-in
plus workspace-gate flow over plain http. The
[prod-like front](#prod-like-front-nginx--tls) adds TLS, the nginx tunnel
`grpc_pass`, and the `Secure`-cookie / `https` shape on top, so you are
testing the same surface a VPS serves.

## Topology

```
  browser (https)
        |
        v
  nginx  (terminates TLS, one vhost per host below)
    |  id.<domain>          -> identity        127.0.0.1:17000
    |  <domain> (apex)      -> workspace-proxy 127.0.0.1:17002
    |    /v1/tunnel (h2c)   -> tunnel listener 127.0.0.1:17100
    |  *.workspace.<domain> -> workspace-proxy 127.0.0.1:17002
        |
        v
  identity:17000   profile:17001   workspace-proxy:17002
        |
        v
  Postgres 127.0.0.1:5432   (sdme container; chan_gateway[_test])
```

The ports above are the dev runner's (`17000`+). The services' own
defaults are `7000/7001/7002/7100`, but on macOS Apple's AirPlay Receiver
binds `:7000` (the identity default), so the dev runner offsets every port
by `+10000` to dodge that clash and to coexist with a prod-shaped
deployment on the defaults. This guide uses the `17xxx` ports throughout.

`chan serve --tunnel-url=https://<domain>/v1/tunnel` dials the apex; nginx
`grpc_pass`es `/v1/tunnel` (raw h2c) to the tunnel listener, and the
workspace is then reachable at `{user}.workspace.<domain>/{workspace}/`.

## Prerequisites + Postgres (sdme)

Install sdme first. On Linux, install it on the host:

```sh
curl -fsSL https://fiorix.github.io/sdme/install.sh | sudo sh
```

On macOS, sdme runs inside a Lima VM; install Lima and then sdme
*inside* the VM, per [macOS only: Lima shim](#macos-only-lima-shim).
Either way the `sdme ...` commands below then work (on macOS through the
alias). sdme containers run with host networking, so anything listening
inside a container is reachable from the host on `localhost`: Postgres
lands on `localhost:5432`, no port-forward, no DSN gymnastics.

One-time, build and start the Postgres container. The build assets live
in [`scripts/dev/sdme/`](../scripts/dev/sdme/) (`chan-psql.sdme` plus
`etc/postgresql/`), a sanitized copy of the prod `chan-psql` service: no
host bind mounts, no secrets, a throwaway `chan` superuser (password
`chan`), and both `chan_gateway` and `chan_gateway_test` seeded on first
boot.

```sh
cd gateway/scripts/dev/sdme
# Linux: drop the "limactl shell default sudo" prefix used on macOS.
limactl shell default sudo sdme fs import ubuntu docker.io/ubuntu   # base rootfs, one-time
limactl shell default sudo sdme fs build chan-psql-dev chan-psql.sdme
limactl shell default sudo sdme create chan-psql-1 -r chan-psql-dev
limactl shell default sudo sdme start  chan-psql-1
limactl shell default sudo sdme ps                                  # Running?
```

> DEV ONLY. The password is hardcoded and `pg_hba` accepts password auth
> from any address. Never build or run this where it is reachable beyond
> your own machine.

The container provisions role `chan` (password `chan`) and two databases:

- `chan_gateway`: the dev DB. `cargo run` against this one.
- `chan_gateway_test`: the integration-test DB. Tests create a fresh
  per-test schema (`t_<uuid>`) under it and drop it on teardown, so a
  `cargo test` run does not collide with a running dev server pointed at
  the dev DB.

Migrations apply on first service start; the test harness re-runs the
migration set into each fresh per-test schema. There is no host bind
mount, so `sdme rm chan-psql-1` discards the data; `sdme join chan-psql-1`
inspects or tweaks Postgres in place. To recreate the DBs from scratch:

```sh
sdme exec chan-psql-1 -- /usr/bin/runuser -u postgres -- \
    /usr/bin/psql -c 'DROP DATABASE IF EXISTS chan_gateway;'
sdme exec chan-psql-1 -- /usr/bin/runuser -u postgres -- \
    /usr/bin/psql -c 'CREATE DATABASE chan_gateway OWNER chan;'
# same for chan_gateway_test
```

(Full paths after `--` are required because `machinectl shell`, which
`sdme exec` runs under, sets no `PATH`.)

## Running the full stack (loopback)

[`scripts/dev/run.sh`](../scripts/dev/README.md) brings up profile,
identity, and workspace-proxy against the `cargo run` build over plain
http, so you can exercise the dashboard, OAuth, and the workspace-gate
handoff against the real binaries before adding TLS.

The trick that makes the wildcard-subdomain shape work with zero DNS
setup: `*.localtest.me` resolves to `127.0.0.1` for every subdomain via
public DNS. The dev runner's port layout:

| Service         | Dev port | URL                                       |
|-----------------|----------|-------------------------------------------|
| profile         | `17001`  | http://127.0.0.1:17001                    |
| identity        | `17000`  | http://id.localtest.me:17000              |
| workspace-proxy | `17002`  | http://workspace.localtest.me:17002 (apex)|
|                 |          | http://*.workspace.localtest.me:17002     |
| workspace tunnel| `17100`  | http://workspace.localtest.me:17100 (h2c) |

```sh
cd gateway
cp scripts/dev/env.example scripts/dev/.env   # paste GITHUB_CLIENT_ID + SECRET
scripts/dev/setup.sh                          # writes secrets/, applies migrations
scripts/dev/run.sh                            # profile + identity + workspace-proxy
```

Register a GitHub OAuth dev app first (Settings -> Developer settings ->
OAuth Apps) with homepage `http://id.localtest.me:17000` and callback
`http://id.localtest.me:17000/auth/github/callback`. Both feature flags
ship default-off, so enrol yourself after the first sign-in:

```sh
PT=$(grep PROFILE_ADMIN_TOKEN scripts/dev/secrets/profile.env | cut -d= -f2-)
A() { CHAN_ADMIN_TOKEN="$PT" CHAN_ADMIN_PROFILE_URL=http://127.0.0.1:17001 \
      target/debug/chan-gateway-admin "$@"; }
A flag grant oauth_login      <your-email>
A flag grant share_workspaces <your-email>
```

Register a test workspace from the sibling `chan` repo:

```sh
export CHAN_TUNNEL_TOKEN=chan_pat_...          # mint under the Tokens tab
cargo run -p chan -- serve <workspace-dir> \
  --tunnel-url=http://workspace.localtest.me:17100/v1/tunnel \
  --tunnel-workspace-name=blog
```

The `http://` scheme drives chan-tunnel-client's h2c path (no TLS);
clicking Open on the dashboard redirects through `/api/workspaces/open`
to `http://<user>.workspace.localtest.me:17002/blog/`. See
[`scripts/dev/README.md`](../scripts/dev/README.md) for the full runner
reference (port layout, secrets, lifecycle).

## Prod-like front: nginx + TLS

This is the only real gap between the loopback runner and production:
TLS, the nginx reverse proxy, and the `https`/`Secure`-cookie shape. Put
nginx in front of the same `run.sh` services and flip the scheme to https.

1. **A local wildcard cert.** Wildcard hosts (`*.workspace.<domain>`)
   cannot use an http-01 challenge, so locally use a local CA:
   [`mkcert`](https://github.com/FiloSottile/mkcert) issues a trusted
   `*.localtest.me` (or your chosen local domain) wildcard your browser
   accepts. The production equivalent is a real wildcard via Let's
   Encrypt dns-01; see [From local to a real VPS](#from-local-to-a-real-vps).

   ```sh
   mkcert -install
   mkcert "*.localtest.me" "*.workspace.localtest.me" localtest.me
   ```

2. **nginx vhosts.** A sketch mirroring the documented route split (apex
   = tunnel + admin + healthz; wildcard = tenant content; id = identity),
   proxying to the `run.sh` ports:

   ```nginx
   # id.<domain> -> identity
   server {
     listen 443 ssl;
     server_name id.localtest.me;
     ssl_certificate     /path/_wildcard.localtest.me.pem;
     ssl_certificate_key /path/_wildcard.localtest.me-key.pem;
     location / { proxy_pass http://127.0.0.1:17000; include proxy_hdr; }
   }

   # apex: tunnel registration (raw h2c), admin, healthz
   server {
     listen 443 ssl;
     server_name workspace.localtest.me;
     ssl_certificate     /path/_wildcard.workspace.localtest.me.pem;
     ssl_certificate_key /path/_wildcard.workspace.localtest.me-key.pem;
     location /v1/tunnel { grpc_pass grpc://127.0.0.1:17100; }   # h2c
     location /          { proxy_pass http://127.0.0.1:17002; include proxy_hdr; }
   }

   # *.workspace.<domain>: per-tenant content
   server {
     listen 443 ssl;
     server_name ~^[^.]+\.workspace\.localtest\.me$;
     ssl_certificate     /path/_wildcard.workspace.localtest.me.pem;
     ssl_certificate_key /path/_wildcard.workspace.localtest.me-key.pem;
     location / { proxy_pass http://127.0.0.1:17002; include proxy_hdr; }
   }
   ```

   `proxy_hdr` carries the standard reverse-proxy headers. workspace-proxy
   recomputes `X-Forwarded-*` itself and does not trust inbound
   `X-Forwarded-Host`/`-Proto` from clients, so nginx only needs to pass
   `Host` and the upgrade headers; set `FORWARDED_PROTO=https` on
   workspace-proxy so it stamps the right scheme. WebSocket upgrade
   (`Connection`/`Upgrade`) must be forwarded for the editor.

3. **Flip the scheme to https.** In the service env, set
   `COOKIE_SECURE=true` (identity) so `id_session` carries `Secure`, and
   set `WORKSPACE_PUBLIC_SCHEME=https` with an empty `WORKSPACE_PUBLIC_PORT`
   (or `:443`) so the gate-handoff URLs identity builds are https. Now
   `chan serve --tunnel-url=https://workspace.localtest.me/v1/tunnel`
   registers over the TLS apex, and the browser path is end-to-end https:
   sign in at `https://id.localtest.me`, Open a workspace, land on
   `https://<user>.workspace.localtest.me/blog/`.

## From local to a real VPS

The same nginx + service shape, with three production swaps:

- **DNS.** Point real records at the host: `id.<domain>`, `<domain>`
  (apex), and a wildcard `*.workspace.<domain>` (A/AAAA). You manage
  these at whatever DNS provider you use (for example, @@Host runs
  Cloudflare DNS; pick your own).
- **Certificates.** The wildcard `*.workspace.<domain>` forces the
  **dns-01** ACME challenge (http-01 cannot issue wildcards). Use certbot
  with your provider's dns-01 plugin to obtain and auto-renew the
  wildcard; `id.<domain>` and the apex can ride the same cert or a
  separate http-01 one. This is the one cert choice the local mkcert
  setup stands in for.
- **Install + run.** Install the services from the release `.deb`
  packages and run them under systemd, per
  [`../README.md`](../README.md) (## Releases, ## Admin). The services
  bind their default ports (`7000/7001/7002/7100`) on Linux, with no
  AirPlay clash. Set real secrets (`openssl rand -hex 32` for the bearer
  + gate secrets), `COOKIE_SECURE=true`, and `FORWARDED_PROTO=https`.
  Enrol the first user out-of-band exactly as in the loopback flow.

Everything else (the nginx vhost shape, the cookie isolation, the
`/v1/tunnel` `grpc_pass`) is identical to the prod-like local stack, which
is the point: validate it locally, then change only DNS, certs, and ports.

## sdme cheatsheet

- **Full container name**: pass the name you created (`chan-psql-1`).
  `sdme` also accepts an unambiguous prefix, but the full name keeps the
  examples copy-pasteable.
- **Full paths after `--`**: `machinectl shell` sets no `PATH`. Use
  `/usr/bin/psql`, `/usr/bin/runuser`, `/usr/bin/systemctl`.
- **Interactive shell**: `sdme join chan-psql-1` drops you into a real
  shell inside the container.
- **Restart a unit inside the container**: `sdme exec chan-psql-1 --
  /usr/bin/systemctl restart postgresql`.

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
isolation means a `cargo test` run never clobbers a running dev server
pointed at `chan_gateway`. CI (`gateway-ci.yml`) runs the same gate with a
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
sdme exec chan-psql-1 -- /bin/bash -c \
    "runuser -u postgres -- /usr/bin/psql -c \
        \"SELECT pg_terminate_backend(pid) \
            FROM pg_stat_activity WHERE usename='chan';\""
```

Safe whenever no live dev server is connected to `chan_gateway`; restart
it after if one was.

## Troubleshooting

- **`connection refused on localhost:5432`** — `sdme ps` should list the
  Postgres container Running; if stopped, `sdme start chan-psql-1`; if
  wedged under load, `sdme exec chan-psql-1 -- /usr/bin/systemctl restart
  postgresql`.
- **`password authentication failed` / `permission denied for database
  "postgres"`** — point `TEST_DATABASE_URL` at the `chan` role and a
  `chan_gateway[_test]` DB, not the `postgres` superuser/DB.
- **Browser rejects the local cert** — run `mkcert -install` so the local
  CA is trusted, and reissue the wildcard if you changed the domain.
- **Signed-in but the workspace 404s** — confirm `WORKSPACE_PUBLIC_SCHEME`
  matches the scheme nginx serves (https when TLS is on) and that
  `FORWARDED_PROTO=https` is set on workspace-proxy; a scheme mismatch
  makes the `workspace_gate` cookie fail to attach.
- **Tests pass locally but break on CI** — same migration set must run
  (`migrations/0001..N` in order); a forgotten file shows up as
  missing-column errors on first use.
