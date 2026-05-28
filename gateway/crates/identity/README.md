# identity-service

Public-facing OAuth2 sign-in service for id.chan.app. Runs the
GitHub / Google / GitLab auth-code flow with PKCE, mints the
`id_session` cookie shared with workspace-proxy, and serves a Svelte
SPA where users manage their profile and personal access tokens
(PATs).

## Role in the system

First public touch-point of chan-gateway. After a successful
OAuth flow, the browser holds the `id_session` cookie and can
move between id.chan.app and workspace.chan.app without re-authing
(both services read the same `tower_sessions` Postgres table).

Identity-service owns:

- session table rows (via `tower_sessions_sqlx_store`)
- `api_tokens` (PAT issuance, revoke, audit)

It does not own user data. Every user lookup, write, or audit row
goes through profile-service over HTTP.

## Build

```bash
cargo build -p identity
```

Frontend baked in at build time via `rust_embed`. The two SPAs in
the workspace share one npm install at the repo root:

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
export GITHUB_CLIENT_ID=...
export GITHUB_CLIENT_SECRET=...
cargo run -p identity
```

Register a GitHub OAuth app at
`https://github.com/settings/developers` with callback
`http://127.0.0.1:7000/auth/github/callback`. The other providers
follow the same pattern.

## Env vars

Required:

| Name                    | Notes                                    |
|-------------------------|------------------------------------------|
| `DATABASE_URL`          | Postgres connection string               |
| `PROFILE_SERVICE_URL`   | profile-service HTTP base URL            |
| `PROFILE_AUTH_TOKEN`    | bearer for profile-service calls         |
| At least one provider's `*_CLIENT_ID` + `*_CLIENT_SECRET` pair  |

Provider credentials (each pair optional; leave both unset to
disable):

- `GITHUB_CLIENT_ID`, `GITHUB_CLIENT_SECRET`
- `GITHUB_ENTERPRISE_URL` (optional, switches to GHE endpoints)
- `GOOGLE_CLIENT_ID`, `GOOGLE_CLIENT_SECRET`
- `GITLAB_CLIENT_ID`, `GITLAB_CLIENT_SECRET`

Optional knobs:

| Name                    | Default                  | Purpose                  |
|-------------------------|--------------------------|--------------------------|
| `BIND_ADDR`             | `127.0.0.1:7000`         | listen address           |
| `BASE_URL`              | `http://localhost:7000`  | public URL for redirects |
| `COOKIE_SECURE`         | `false`                  | HTTPS-only cookie        |
| `COOKIE_DOMAIN`         | unset                    | `.chan.app` in prod      |
| `WORKSPACES_URL`            | `http://localhost:7002`  | shown in SPA topbar      |
| `WORKSPACE_ADMIN_URL`       | `WORKSPACES_URL`             | workspace-proxy admin base   |
| `WORKSPACE_ADMIN_TOKEN`     | unset                    | enables tunnel evict on  |
|                         |                          | account delete           |

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
| GET    | `/api/config`         | workspaces_url for the topbar               |
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
