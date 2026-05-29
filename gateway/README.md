# chan-gateway

Identity and profile services behind `id.chan.app` and (later)
`workspace.chan.app`. Tracks [fiorix/chan#8][issue].

[issue]: https://github.com/fiorix/chan/issues/8

## Status

v0, scoped to the web flow:

- `profile`: internal HTTP API over Postgres. Users + linked OAuth
  identities + usernames. **(done)**
- `identity`: id.chan.app, OAuth2 sign-in (GitHub / Google /
  GitLab), Postgres-backed sessions, embedded Svelte SPA, personal
  access tokens. **(done)**
- `workspace-proxy`: workspace.chan.app (apex) +
  `*.workspace.chan.app` (wildcard). Each `chan serve` instance dials
  `POST /v1/tunnel` (raw h2c) and registers over an authenticated
  yamux tunnel; HTTP and WebSocket traffic at
  `{user}.workspace.chan.app/{workspace}/*` is reverse-proxied into it.
  Entry is gated by the workspace-gate handoff: identity mints an
  entry JWT, workspace-proxy verifies it and mints a host-only,
  path-scoped `workspace_gate` cookie. `--tunnel-public` registrations
  skip the gate; everything else without a valid token / cookie 404s
  (same shape as an unknown workspace, so probes can't enumerate).
  **(done)**

Personal access tokens (PATs) cover the chan CLI / chan-tunnel use
case today. Signed JWT access tokens, refresh-token rotation, and
entitlement bundles come back when desktop/mobile clients ship.
Adding another OAuth provider is one new file under
`crates/identity/src/providers/` plus wiring in `Config::from_env`.
Microsoft and Apple are intentionally excluded (Microsoft because
tenant admins can mint unverified-email accounts that defeat our
email-as-link key; Apple because the OAuth setup is high-touch for
the value at this scale).

## Layout

```
chan-gateway/
  Cargo.toml                       # workspace
  crates/identity/                 # bin: identity-service (id.chan.app)
  crates/identity/web/             # SPA embedded into identity-service
  crates/workspace-proxy/              # bin: workspace-proxy-service (workspace.chan.app)
  crates/workspace-proxy/web/          # SPA embedded into workspace-proxy-service
  crates/profile/                  # bin: profile-service (internal)
  migrations/                      # sqlx migrations (Postgres)
```

The frontends match `chan-writer/chan/web/` so id.chan.app,
workspace.chan.app and the editor read as the same product: Svelte 5 +
Vite + TypeScript, dark default with the same CSS variable palette.

## Dev

> macOS contributors: the recommended path runs Postgres inside a
> Lima VM via [sdme][sdme]; see [`docs/dev-setup.md`](docs/dev-setup.md)
> for the host-side aliases, container layout, and credentials.
> The rest of this section assumes a Postgres reachable on
> `127.0.0.1` either way.

[sdme]: https://github.com/fiorix/sdme

### Postgres

One database covers everything; `profile` owns users / identities /
workspaces, `identity` owns the `tower_sessions` table. Both auto-migrate
on boot.

```sh
createdb chan_gateway
createdb chan_gateway_test         # used by `cargo test`
```

### Frontends

The two SPAs share an npm workspace at the repo root, alongside a
small `web-common` package (shared theme CSS, fetch wrapper, topbar
component). One install builds both bundles:

```sh
npm install
npm run build --workspaces
```

`vite build` writes to `<crate>/web/dist/`, embedded by the matching
binary via `rust-embed`.

### GitHub OAuth app

Register one at https://github.com/settings/developers:

- Homepage URL: `http://127.0.0.1:7000`
- Authorization callback URL: `http://127.0.0.1:7000/auth/github/callback`

Save the client id and secret.

### Run

Three terminals; profile first.

Terminal 1 (profile-service, internal API on 7001):

```sh
export DATABASE_URL=postgres://chan:chan@127.0.0.1/chan_gateway
export BIND_ADDR=127.0.0.1:7001
export PROFILE_AUTH_TOKEN=dev-token
cargo run -p profile
```

Terminal 2 (identity-service, id.chan.app surface on 7000):

```sh
export DATABASE_URL=postgres://chan:chan@127.0.0.1/chan_gateway
export BIND_ADDR=127.0.0.1:7000
export BASE_URL=http://127.0.0.1:7000
export COOKIE_SECURE=false
export PROFILE_SERVICE_URL=http://127.0.0.1:7001
export PROFILE_AUTH_TOKEN=dev-token
export GITHUB_CLIENT_ID=...
export GITHUB_CLIENT_SECRET=...
cargo run -p identity
```

Open http://127.0.0.1:7000 and sign in with GitHub.

Terminal 3 (workspace-proxy-service, workspace.chan.app surface on 7002):

```sh
export BIND_ADDR=127.0.0.1:7002
export TUNNEL_BIND_ADDR=127.0.0.1:7100
export IDENTITY_URL=http://127.0.0.1:7000
export IDENTITY_INTERNAL_TOKEN=dev-internal-token
export WORKSPACE_GATE_SECRET=dev-workspace-gate-secret
cargo run -p workspace-proxy
```

workspace-proxy holds no database and no session cookie of its own; a
workspace is reached by following the "open workspace" link from the
id.chan.app dashboard, which carries the entry token. For the full
local stack use `scripts/dev/setup.sh` + `scripts/dev/run.sh`.

For frontend iteration without re-embedding:

```sh
npm run dev -w crates/identity/web      # :5173, proxies to :7000
npm run dev -w crates/workspace-proxy/web   # :5174, proxies to :7002
```

## Tests

```sh
export DATABASE_URL=postgres://chan:chan@127.0.0.1/chan_gateway
export TEST_DATABASE_URL=postgres://chan:chan@127.0.0.1/chan_gateway_test
cargo test
```

Tests use real Postgres (per-test schema isolation). Identity tests
mock the GitHub OAuth endpoints and profile-service via wiremock.

## Releases

The gateway ships on the monorepo's release line: the gateway
crates are versioned in lockstep with the root (`chan`), and a
`v*` tag triggers the repo-root `.github/workflows/release.yml`,
whose `gateway-linux-packages` job builds four .deb packages
(`chan-gateway-profile`, `chan-gateway-identity`,
`chan-gateway-workspace-proxy`, `chan-gateway-admin`) for amd64
and arm64 and uploads them alongside the rest of the release.

There is no gateway-local release script: bump
`gateway/Cargo.toml` in the same commit as the root `Cargo.toml`
version, then cut the release from the monorepo root. The release
workflow's `context` job asserts the tag matches the gateway
version.

To build .debs locally on macOS (one-off, before relying on CI):

```sh
brew install zig
rustup target add x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu
cargo install cargo-zigbuild cargo-deb
./scripts/build-debs.sh
ls dist/                                   # six .deb files (3 services x 2 archs)
```

### Install on a Debian/Ubuntu host

```sh
sudo apt install ./chan-gateway-profile_*.deb \
                 ./chan-gateway-identity_*.deb \
                 ./chan-gateway-workspace-proxy_*.deb
```

The packages share a system user (`chan-gateway`) and put env
templates at `/etc/chan-gateway/{profile,identity,workspace-proxy}.env`.
Edit those, then enable + start each service:

```sh
sudo systemctl enable --now chan-gateway-profile
sudo systemctl enable --now chan-gateway-identity
sudo systemctl enable --now chan-gateway-workspace-proxy
```

The binaries listen on `127.0.0.1:{7001,7000,7002}` by default;
front them with nginx + Let's Encrypt for `id.chan.app` and
`workspace.chan.app`.

## Admin

`chan-gateway-admin` (`crates/admin/`) is the operator CLI: list /
block / unblock users, inspect personal access tokens, snapshot or
kill live tunnels, read auth audit. It talks to profile-service's
`/v1/admin/*` tree and workspace-proxy's `/admin/v1/*` tree over plain
HTTP, so run it on a host that can reach the internal listeners.

### Setup

Two service env vars guard the admin tree; rotate them like any
other secret:

- profile-service: `PROFILE_ADMIN_TOKEN=<random>`
- workspace-proxy:    `WORKSPACE_ADMIN_TOKEN=<random>`

A single-token deployment shares one secret across both services;
`chan-gateway-admin` reads `CHAN_ADMIN_TOKEN` and sends it to each.

```sh
export CHAN_ADMIN_TOKEN=<same value as the service tokens>
export CHAN_ADMIN_PROFILE_URL=http://127.0.0.1:7001    # optional, default
export CHAN_ADMIN_WORKSPACE_URL=http://127.0.0.1:7002      # optional, default
```

Build / install:

```sh
cargo install --path crates/admin                 # local dev
# or use the .deb produced by scripts/build-debs.sh:
sudo apt install ./chan-gateway-admin_*.deb
```

### Recipes

```sh
# Block a user (revokes every live PAT, refuses fresh OAuth logins)
chan-gateway-admin user block alice@example.com --reason "spam reports"

# Reverse the block (existing tokens stay revoked; reissue if needed)
chan-gateway-admin user unblock alice@example.com

# Audit log for a user (login / logout / login_denied / blocked / ...)
chan-gateway-admin user audit alice@example.com

# Find a user (uuid, email substring, or exact username all work)
chan-gateway-admin user get alice

# List, with filters
chan-gateway-admin user list --blocked
chan-gateway-admin user list --email "@example.com"

# Personal access tokens
chan-gateway-admin token list alice@example.com
chan-gateway-admin token revoke <token-uuid>
chan-gateway-admin token audit  <token-uuid>

# Live tunnels (workspace-proxy in-memory registry)
chan-gateway-admin tunnel ps
chan-gateway-admin tunnel ps --user alice
chan-gateway-admin tunnel kill alice home          # force one workspace offline
chan-gateway-admin tunnel watch                    # SSE stream, top-style

# Feature flags. Fresh deploys ship oauth_login=off and
# share_workspaces=off so nobody can sign in until you enrol them.
chan-gateway-admin flag list
chan-gateway-admin flag grant oauth_login  alice@example.com
chan-gateway-admin flag grant share_workspaces alice@example.com
chan-gateway-admin flag overrides oauth_login         # who has access
chan-gateway-admin flag revoke share_workspaces alice@example.com
chan-gateway-admin flag create my_feature --default-on --description "..."
```

Add `--json` to any subcommand for jq-friendly output.

The two seeded flags workspace the rollout posture. `oauth_login`
gates the OAuth callback; an account without the override is
denied at sign-in with `?denied=oauth_login` in the redirect.
`share_workspaces` is the SPA-side toggle for the per-workspace sharing
UI; flipping it off hides the Workspaces tab and the share panel.
Both default to off so a fresh deploy has to grant the first user
out-of-band (or pre-create users via `chan-admin user create` and
then `flag grant`).

The user account survives a block; deletion is via the SPA's
"Delete account" disclosure (account holder only). Account delete
also evicts every live tunnel for that user.

## Troubleshooting

- `pool timed out while waiting for an open connection` even though
  TCP works: Postgres likely ran out of non-superuser connection
  slots (`FATAL: remaining connection slots are reserved for roles
  with the SUPERUSER attribute`). Restart it
  (`sudo systemctl restart postgresql`) and re-run. We cap each
  service at 4 connections, but leftover pools from prior runs +
  test crashes can pile up.
- `Access to localhost was denied` / 403 on macOS port 7000:
  AirPlay Receiver claims that port. Disable it in System Settings
  -> General -> AirDrop & Handoff, or change `BIND_ADDR` /
  `BASE_URL` to a different port (and re-register the OAuth
  callback URL).

## License

Apache-2.0. See [LICENSE](LICENSE).
