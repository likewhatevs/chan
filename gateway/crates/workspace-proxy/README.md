# workspace-proxy

Public-facing service at workspace.chan.app (apex) and
`*.workspace.chan.app` (wildcard). Reverse-proxies HTTP / WebSocket
traffic into a user's running `chan serve` instances. Embeds
`chan-tunnel-server` to terminate registrations from those instances
on a separate h2c listener. No SPA, no database; it is a stateless
proxy.

## Role in the system

workspace-proxy is the surface where a workspace is served in the
browser. It does NOT read identity's `id_session` cookie. Entry is
gated by the workspace-gate handoff: identity mints a short-lived
entry JWT and 303s the browser to
`{user}.workspace.<domain>/{workspace}/?t=<jwt>`; workspace-proxy
verifies it (signature + `aud` = inbound host + `drv`), mints its own
host-only, path-scoped `workspace_gate` cookie, and forwards
authenticated traffic to the right `chan serve` peer through a yamux
substream owned by an active tunnel. The `aud`-equals-inbound-host
check is what enforces tenant isolation.

## Build

```bash
cargo build -p workspace-proxy
```

No frontend: workspace-proxy ships no SPA.

## Dev run

```bash
export BIND_ADDR=127.0.0.1:7002
export TUNNEL_BIND_ADDR=127.0.0.1:7100
export IDENTITY_URL=http://127.0.0.1:7000
export IDENTITY_INTERNAL_TOKEN=dev-internal-token
export WORKSPACE_GATE_SECRET=dev-workspace-gate-secret
cargo run -p workspace-proxy
```

For the full local stack (with identity + profile + Postgres), prefer
`scripts/dev/setup.sh` + `scripts/dev/run.sh`. Two listeners come up:

- `BIND_ADDR` (7002): public HTTP. workspace.chan.app sits behind
  nginx + TLS in production; loopback in dev.
- `TUNNEL_BIND_ADDR` (7100): h2c. nginx `grpc_pass`es `/v1/tunnel` on
  the apex here; `chan serve` instances dial it for the handshake.

## Env vars

Public hostnames come from the shared domain config
([`gateway/packaging/domain.env`](../../packaging/domain.env)).

Required:

| Name                      | Notes                                       |
|---------------------------|---------------------------------------------|
| `IDENTITY_INTERNAL_TOKEN` | bearer presented on identity's validate     |
| `WORKSPACE_GATE_SECRET`   | HS256 secret; equals identity's             |

Domain (single source):

| Name             | Default        | Purpose                        |
|------------------|----------------|--------------------------------|
| `CHAN_DOMAIN`    | `localtest.me` | base domain; derives the hosts |
| `PUBLIC_SCHEME`  | `http`         | scheme for built URLs          |

Optional:

| Name                       | Default                  | Purpose              |
|----------------------------|--------------------------|----------------------|
| `BIND_ADDR`                | `127.0.0.1:7002`         | public listener      |
| `TUNNEL_BIND_ADDR`         | `127.0.0.1:7100`         | tunnel listener (h2c)|
| `IDENTITY_URL`             | `http://127.0.0.1:7000`  | base for validate    |
| `APEX_HOST`                | `workspace.<domain>`     | apex host override   |
| `WILDCARD_SUFFIX`          | `.workspace.<domain>`    | wildcard override    |
| `DASHBOARD_URL`            | `<scheme>://id.<domain>/workspaces` | sign-in redirect |
| `WORKSPACE_ADMIN_TOKEN`    | unset                    | enables `/admin/v1/*`|
| `MAX_WORKSPACES_PER_USER`  | `0` (unlimited)          | concurrent tunnels   |
| `MAX_RESPONSE_BYTES`       | `100 MiB` (`0` disables) | response body cap    |
| `MAX_REQUEST_BYTES`        | `100 MiB` (`0` disables) | request body cap     |
| `REQUEST_TIMEOUT_SECS`     | `60` (`0` disables)      | end-to-end timeout   |
| `FORWARDED_PROTO`          | `https`                  | `X-Forwarded-Proto`  |

## Routes

- Apex (`workspace.<domain>`): `POST /v1/tunnel` (raw h2c, on the
  tunnel listener), the Bearer-gated `/admin/v1/*` tree (rate-limited
  via `tower_governor`), and `/healthz`.
- Wildcard (`{user}.workspace.<domain>`): the per-workspace reverse
  proxy under `/{workspace}/...`, gated by the `?t=` entry token /
  `workspace_gate` cookie.

See [`design.md`](design.md) for the authoritative route list, the
auth-gate order, and the reverse-proxy hygiene rules.

## Design rationale

See [`design.md`](design.md).
