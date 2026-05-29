# identity-service

Public-facing OAuth2 sign-in service for id.chan.app. Runs the
GitHub / Google / GitLab auth-code flow with PKCE, holds the
host-only `id_session` cookie, and serves a Svelte SPA where users
manage their profile, personal access tokens (PATs), and workspaces.
It mints the short-lived workspace-gate entry token that hands a user
off to workspace-proxy.

## Role in the system

First public touch-point of chan-gateway. After a successful OAuth
flow the browser holds the `id_session` cookie, which is host-only on
id.chan.app and is NOT shared with workspace-proxy. To open a
workspace, identity mints a short-lived workspace-gate entry token and
303s the browser to `{user}.workspace.<domain>/{workspace}/?t=<jwt>`;
workspace-proxy verifies it and mints its own host-scoped cookie. That
split is the load-bearing piece of cross-tenant isolation: no
`.chan.app`-scoped cookie exists.

Identity-service owns:

- session table rows (via `tower_sessions_sqlx_store`)
- `api_tokens` (PAT issuance, revoke, audit)

It does not own user data. Every user lookup, write, or audit row
goes through profile-service over HTTP.

## Build

```bash
cargo build -p identity
```

Frontend baked in at build time via `rust_embed`. identity is the
gateway's only SPA; it installs from the gateway npm workspace root:

```bash
npm install
npm run build -w crates/identity/web
```

A fresh checkout without `web/dist/` still builds; the SPA
endpoints render a "frontend not built" banner that points at the
build command.

## Dev run

```bash
createdb chan_gateway
export DATABASE_URL=postgres://localhost/chan_gateway
export BIND_ADDR=127.0.0.1:7000
export BASE_URL=http://127.0.0.1:7000
export PROFILE_SERVICE_URL=http://127.0.0.1:7001
export PROFILE_AUTH_TOKEN=dev-service-token
export IDENTITY_INTERNAL_TOKEN=dev-internal-token
export WORKSPACE_GATE_SECRET=dev-workspace-gate-secret
export GITHUB_CLIENT_ID=...
export GITHUB_CLIENT_SECRET=...
cargo run -p identity
```

Hostnames derive from `CHAN_DOMAIN` (default `localtest.me`) and
`PUBLIC_SCHEME` (default `http`); `BASE_URL` defaults to
`<scheme>://id.<domain>` and is set explicitly above only to pin the
loopback port. For the full local stack, prefer `scripts/dev/setup.sh`
+ `scripts/dev/run.sh`.

Register a GitHub OAuth app at
`https://github.com/settings/developers` with callback
`http://127.0.0.1:7000/auth/github/callback`. The other providers
follow the same pattern.

## Env vars

Required:

| Name                      | Notes                                       |
|---------------------------|---------------------------------------------|
| `DATABASE_URL`            | Postgres connection string                  |
| `PROFILE_SERVICE_URL`     | profile-service HTTP base URL               |
| `PROFILE_AUTH_TOKEN`      | bearer for profile-service calls            |
| `IDENTITY_INTERNAL_TOKEN` | bearer workspace-proxy presents on validate |
| `WORKSPACE_GATE_SECRET`   | HS256 secret; equals workspace-proxy's      |
| At least one provider's `*_CLIENT_ID` + `*_CLIENT_SECRET` pair        |

Provider credentials (each pair optional; leave both unset to
disable):

- `GITHUB_CLIENT_ID`, `GITHUB_CLIENT_SECRET`
- `GOOGLE_CLIENT_ID`, `GOOGLE_CLIENT_SECRET`
- `GITLAB_CLIENT_ID`, `GITLAB_CLIENT_SECRET`

Domain (single source; see [`gateway/packaging/domain.env`](../../packaging/domain.env)):

| Name                       | Default        | Purpose                     |
|----------------------------|----------------|-----------------------------|
| `CHAN_DOMAIN`              | `localtest.me` | base domain; derives hosts  |
| `PUBLIC_SCHEME`            | `http`         | scheme for built URLs       |

Optional knobs:

| Name                       | Default                   | Purpose               |
|----------------------------|---------------------------|-----------------------|
| `BIND_ADDR`                | `127.0.0.1:7000`          | listen address        |
| `BASE_URL`                 | `<scheme>://id.<domain>`  | OAuth callback origin |
| `COOKIE_SECURE`            | `false`                   | HTTPS-only cookie     |
| `WORKSPACE_WILDCARD_SUFFIX`| `.workspace.<domain>`     | redirect host suffix  |
| `WORKSPACE_PUBLIC_SCHEME`  | `PUBLIC_SCHEME`           | workspace redirect scheme |
| `WORKSPACE_PUBLIC_PORT`    | unset                     | `:port` for dev       |
| `WORKSPACE_ADMIN_URL`      | unset                     | workspace-proxy admin base |
| `WORKSPACE_ADMIN_TOKEN`    | unset                     | enables tunnel evict on revoke / delete |

## Routes

Public (no session required):

| Method | Path                          | Purpose                       |
|--------|-------------------------------|-------------------------------|
| GET    | `/`                           | SPA root (index.html)         |
| GET    | `/healthz`                    | health check                  |
| GET    | `/auth/:provider`             | OAuth start (PKCE)            |
| GET    | `/auth/:provider/callback`    | OAuth callback                |

Session-gated SPA API (`/api/*`):

| Method | Path                  | Purpose                                 |
|--------|-----------------------|-----------------------------------------|
| GET    | `/api/providers`      | list of enabled OAuth providers         |
| GET    | `/api/me`             | current user                            |
| PATCH  | `/api/me/username`    | rename handle                           |
| POST   | `/api/logout`         | invalidate session                      |
| DELETE | `/api/profile`        | account deletion                        |
| GET    | `/api/tokens`         | list PATs                               |
| POST   | `/api/tokens`         | mint a PAT (returns plaintext once)     |
| DELETE | `/api/tokens/:id`     | revoke a PAT                            |
| GET    | `/api/tokens/:id/audit` | per-token audit log                   |
| GET    | `/api/workspaces/open`    | mint workspace-gate entry token + 303       |
| POST   | `/api/workspaces`         | create a workspace in the user's namespace  |
| DELETE | `/api/workspaces/:d`      | delete a workspace (cascades all its grants)|
| GET    | `/api/workspaces/owned`   | workspaces the user owns                    |
| GET    | `/api/workspaces/incoming`| workspaces shared with the user             |
| POST   | `/api/workspaces/:d/grants`| share a workspace by email                 |
| GET    | `/api/workspaces/:d/grants`| list grants on the user's workspace        |
| DELETE | `/api/grants/:id`     | revoke a grant on the user's workspace      |

Public share landing (no auth at the door):

| Method | Path                  | Purpose                                 |
|--------|-----------------------|-----------------------------------------|
| GET    | `/s/:owner/:workspace`    | OAuth-then-mint entry token for grantees |

Internal (Bearer-gated by `PROFILE_AUTH_TOKEN`):

| Method | Path                                   | Purpose                |
|--------|----------------------------------------|------------------------|
| POST   | `/internal/v1/tokens/validate`         | validate a PAT         |

The internal route is called by chan-tunnel during tunnel
handshake. PAT brute-force throttling runs one hop earlier in
workspace-proxy, keyed on a hash of the candidate token; the previous
per-IP governor at this hop saw only workspace-proxy's container IP
and degenerated into a single global bucket. See identity's
`design.md` for the rationale.

## Design rationale

See [`design.md`](design.md).
