# workspace-proxy

Public-facing service at workspace.chan.app. Lists the signed-in
user's live workspaces and reverse-proxies HTTP / WebSocket traffic
into them. Embeds `chan-tunnel-server` to terminate registrations
from `chan serve` instances on a separate listener.

## Role in the system

workspace-proxy is the surface where users open their workspaces in a
browser. It reads the `id_session` cookie minted by
identity-service and forwards authenticated traffic to the right
`chan serve` peer through a yamux substream owned by an active
tunnel.

## Build

```bash
cargo build -p workspace-proxy
```

Frontend baked in at build time. The two SPAs in the workspace
share one npm install at the repo root:

```bash
npm install
npm run build -w crates/workspace-proxy/web
```

## Dev run

```bash
createdb chan_gateway
export DATABASE_URL=postgres://localhost/chan_gateway
export BIND_ADDR=127.0.0.1:7002
export TUNNEL_BIND_ADDR=127.0.0.1:7100
export IDENTITY_BASE_URL=http://127.0.0.1:7000
export IDENTITY_URL=http://127.0.0.1:7000
export PROFILE_SERVICE_URL=http://127.0.0.1:7001
export PROFILE_AUTH_TOKEN=dev-service-token
cargo run -p workspace-proxy
```

Two listeners come up:

- `BIND_ADDR` (7002): public HTTP. workspace.chan.app sits behind nginx
  + TLS in production; loopback in dev.
- `TUNNEL_BIND_ADDR` (7100): h2c. tunnel.chan.app sits behind
  `nginx grpc_pass` in production; `chan serve` instances dial
  here for the chan-tunnel handshake.

## Env vars

Required:

| Name                  | Notes                                         |
|-----------------------|-----------------------------------------------|
| `DATABASE_URL`        | Postgres connection string                    |
| `IDENTITY_AUTH_TOKEN` | bearer for identity validate (or fall back to |
|                       | `PROFILE_AUTH_TOKEN`)                         |
| `PROFILE_SERVICE_URL` | profile-service HTTP base URL                 |
| `PROFILE_AUTH_TOKEN`  | bearer for profile calls                      |

Optional:

| Name                   | Default                  | Purpose                |
|------------------------|--------------------------|------------------------|
| `BIND_ADDR`            | `127.0.0.1:7002`         | public listener        |
| `TUNNEL_BIND_ADDR`     | `127.0.0.1:7100`         | tunnel listener (h2c)  |
| `IDENTITY_BASE_URL`    | `http://127.0.0.1:7000`  | redirect target for    |
|                        |                          | anonymous visitors     |
| `IDENTITY_URL`         | `http://127.0.0.1:7000`  | base for token         |
|                        |                          | validate calls         |
| `COOKIE_SECURE`        | `false`                  | HTTPS-only cookie      |
| `COOKIE_DOMAIN`        | unset                    | `.chan.app` in prod    |
| `MAX_WORKSPACES_PER_USER`  | `0` (unlimited)          | concurrent registrations |
| `WORKSPACE_ADMIN_TOKEN`    | unset                    | enables `/admin/v1/*`  |
| `MAX_RESPONSE_BYTES`   | `100 MiB` (`0` disables) | reverse-proxy body cap |

## Routes

### Public listener (7002)

Session-gated SPA + reverse proxy:

| Method | Path                          | Purpose                       |
|--------|-------------------------------|-------------------------------|
| GET    | `/`                           | redirect to identity_base_url |
| GET    | `/healthz`                    | health check                  |
| GET    | `/api/config`                 | sign_in_url for the SPA       |
| GET    | `/api/me`                     | current user + workspaces list    |
| POST   | `/api/logout`                 | invalidate session            |
| GET    | `/assets/*path`               | embedded SPA bundle           |
| GET    | `/favicon.ico`, `/chan-mark.png` | embedded assets            |
| GET    | `/:user`                      | per-user dashboard SPA root   |
| GET    | `/:user/:workspace`               | 308 to canonical              |
|        |                               | `/:user/:workspace/`              |
| ANY    | `/:user/:workspace/`              | reverse-proxy entry           |
| ANY    | `/:user/:workspace/*path`         | reverse-proxy deeper paths    |

Admin (Bearer-gated by `WORKSPACE_ADMIN_TOKEN`, rate-limited):

| Method | Path                                          | Purpose            |
|--------|-----------------------------------------------|--------------------|
| GET    | `/admin/v1/tunnels`                           | snapshot of all   |
|        |                                               | registrations     |
| POST   | `/admin/v1/tunnels/:user/:workspace/kill`         | evict one tunnel  |
| POST   | `/admin/v1/users/:user/tunnels/kill`          | bulk evict        |
| GET    | `/admin/v1/tunnels/watch`                     | SSE snapshot stream |

The admin tree runs through `tower_governor` at 4 rps + 16 burst
per source IP.

### Tunnel listener (7100)

Embedded `chan-tunnel-server`. Handles the chan-tunnel handshake
and inserts authenticated registrations into the in-process
`Registry` shared with the public listener.

## Design rationale

See [`design.md`](design.md).
